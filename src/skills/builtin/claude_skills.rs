use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// A single Claude skill discovered from a `SKILL.md` file.
///
/// Discovery + declaration only. The `body` (the markdown instructions) and
/// the per-section `.md` files are read from disk and returned verbatim (with
/// argument / variable substitution) when the tool is invoked — this is the
/// Python reference's default, non-shell-injection behavior.
///
/// Python's `allow_shell_injection` path (running `!` \`cmd\` snippets authored
/// in a skill body) is a NATIVE-CODE-EXECUTION feature that is intentionally
/// NOT ported: Rust is AOT-compiled and cannot execute skill-authored code at
/// runtime, so this skill *discovers and declares* skills but never executes
/// arbitrary code from them. See `PORT_SIGNATURE_OMISSIONS.md`.
#[derive(Debug, Clone)]
struct DiscoveredSkill {
    /// Skill name (frontmatter `name`, else the directory name).
    name: String,
    /// Frontmatter `description`, if present.
    description: Option<String>,
    /// Frontmatter `argument-hint`, if present.
    argument_hint: Option<String>,
    /// The markdown body (everything after the frontmatter).
    body: String,
    /// Absolute path to the skill's directory.
    skill_dir: PathBuf,
    /// Supporting `.md` sections: `(section_key, absolute_path)`, sorted by key.
    sections: Vec<(String, PathBuf)>,
    /// `disable-model-invocation: true` → skip tool + prompt registration.
    skip_tool: bool,
    /// `user-invocable: false` → skip tool but keep the prompt section.
    skip_prompt: bool,
}

/// Load Claude SKILL.md files as agent tools (handler-based).
pub struct ClaudeSkills {
    sp: SkillParams,
    /// Populated by [`ClaudeSkills::setup`]; each becomes a SWAIG tool.
    skills: Vec<DiscoveredSkill>,
}

impl ClaudeSkills {
    pub fn new(params: Map<String, Value>) -> Self {
        ClaudeSkills {
            sp: SkillParams::new(params),
            skills: Vec::new(),
        }
    }

    /// Discover and parse every `<skills_path>/<dir>/SKILL.md`. Returns the
    /// parsed skills (may be empty — an empty skill set is valid).
    fn discover_skills(&self, skills_path: &Path) -> Vec<DiscoveredSkill> {
        let include = self.include_patterns();
        let exclude = self.exclude_patterns();
        let ignore_invocation_control = self.sp.get_bool("ignore_invocation_control");

        let mut skills = Vec::new();

        let Ok(entries) = std::fs::read_dir(skills_path) else {
            return skills;
        };

        // Sort directory entries for deterministic ordering.
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();

        for dir in dirs {
            let skill_file = dir.join("SKILL.md");
            if !skill_file.is_file() {
                continue;
            }

            let dir_name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if !matches_patterns(&dir_name, &include, &exclude) {
                continue;
            }

            let Some((frontmatter, body)) = parse_skill_md(&skill_file) else {
                continue;
            };

            let name = frontmatter
                .get("name")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map_or_else(|| dir_name.clone(), str::to_string);

            let description = frontmatter
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string);

            let argument_hint = frontmatter
                .get("argument-hint")
                .and_then(Value::as_str)
                .map(str::to_string);

            let sections = discover_sections(&dir);

            // Invocation control (mirrors Python `_apply_invocation_control`).
            let (skip_tool, skip_prompt) = if ignore_invocation_control {
                (false, false)
            } else {
                let disable_model = frontmatter
                    .get("disable-model-invocation")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let user_invocable = frontmatter
                    .get("user-invocable")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                if disable_model {
                    (true, true)
                } else if !user_invocable {
                    (true, false)
                } else {
                    (false, false)
                }
            };

            skills.push(DiscoveredSkill {
                name,
                description,
                argument_hint,
                body,
                skill_dir: dir,
                sections,
                skip_tool,
                skip_prompt,
            });
        }

        skills
    }

    fn include_patterns(&self) -> Vec<String> {
        let arr = self.sp.get_array("include");
        if arr.is_empty() {
            vec!["*".to_string()]
        } else {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        }
    }

    fn exclude_patterns(&self) -> Vec<String> {
        self.sp
            .get_array("exclude")
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    }
}

impl SkillBase for ClaudeSkills {
    fn name(&self) -> &'static str {
        "claude_skills"
    }

    fn description(&self) -> &'static str {
        "Load Claude SKILL.md files as agent tools"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn skill_state(&self) -> Option<&crate::skills::skill_base::SkillParams> {
        Some(&self.sp)
    }

    fn setup(&mut self) -> bool {
        let Some(skills_path) = self.sp.get_str("skills_path") else {
            return false;
        };

        let path = expand_user(skills_path);
        if !path.is_dir() {
            return false;
        }

        // Discover + parse; an empty skill set is still a valid setup.
        self.skills = self.discover_skills(&path);
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_prefix = self.sp.get_str_or("tool_prefix", "claude_");
        let response_prefix = self.sp.get_str_or("response_prefix", "");
        let response_postfix = self.sp.get_str_or("response_postfix", "");
        let skill_descriptions = self.sp.get_object("skill_descriptions");

        for skill in &self.skills {
            if skill.skip_tool {
                continue;
            }

            let tool_name = format!("{tool_prefix}{}", sanitize_tool_name(&skill.name));

            let description = skill_descriptions
                .get(&skill.name)
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| skill.description.clone())
                .unwrap_or_else(|| format!("Use the {} skill", skill.name));

            // Parameter schema: `arguments` (string) + optional `section` enum
            // when the skill ships supporting `.md` files.
            let mut parameters = Map::new();
            parameters.insert(
                "arguments".to_string(),
                json!({
                    "type": "string",
                    "description": skill
                        .argument_hint
                        .clone()
                        .unwrap_or_else(|| "Arguments or context to pass to the skill".to_string()),
                    "required": true,
                }),
            );
            let section_keys: Vec<String> = skill.sections.iter().map(|(k, _)| k.clone()).collect();
            if !section_keys.is_empty() {
                parameters.insert(
                    "section".to_string(),
                    json!({
                        "type": "string",
                        "description": "Which reference section to load",
                        "enum": section_keys,
                    }),
                );
            }

            // Capture what the handler needs (owned, so it's 'static + Send + Sync).
            let body = skill.body.clone();
            let skill_dir = skill.skill_dir.to_string_lossy().to_string();
            let sections: Vec<(String, PathBuf)> = skill.sections.clone();
            let rprefix = response_prefix.clone();
            let rpostfix = response_postfix.clone();

            agent.define_tool(
                &tool_name,
                &description,
                Value::Object(parameters),
                Box::new(move |args, raw| {
                    let arguments = args.get("arguments").and_then(Value::as_str).unwrap_or("");
                    let section = args.get("section").and_then(Value::as_str).unwrap_or("");

                    // Load the requested section file, else the SKILL.md body.
                    let mut content = if section.is_empty() {
                        body.clone()
                    } else {
                        sections
                            .iter()
                            .find(|(k, _)| k == section)
                            .and_then(|(_, p)| std::fs::read_to_string(p).ok())
                            .unwrap_or_else(|| body.clone())
                    };

                    // Variable substitution (${CLAUDE_SKILL_DIR}/${CLAUDE_SESSION_ID}).
                    let session_id = raw.get("call_id").and_then(Value::as_str).unwrap_or("");
                    content = content.replace("${CLAUDE_SKILL_DIR}", &skill_dir);
                    content = content.replace("${CLAUDE_SESSION_ID}", session_id);

                    // Argument substitution ($ARGUMENTS + $N positional).
                    content = substitute_arguments(&content, arguments);

                    // Prefix/postfix wrapping.
                    if !rprefix.is_empty() || !rpostfix.is_empty() {
                        let mut parts: Vec<String> = Vec::new();
                        if !rprefix.is_empty() {
                            parts.push(rprefix.clone());
                        }
                        parts.push(content);
                        if !rpostfix.is_empty() {
                            parts.push(rpostfix.clone());
                        }
                        content = parts.join("\n\n");
                    }

                    FunctionResult::with_response(&content)
                }),
                true,
            );
        }
    }

    fn get_hints(&self) -> Vec<String> {
        // Speech hints derived from the discovered skill names (Python parity).
        let mut hints: Vec<String> = Vec::new();
        for skill in &self.skills {
            for word in skill.name.replace(['-', '_'], " ").split_whitespace() {
                let w = word.to_string();
                if !hints.contains(&w) {
                    hints.push(w);
                }
            }
        }
        if hints.is_empty() {
            // No skills discovered yet (e.g. before setup): fall back to the
            // generic hints so the skill still contributes recognition context.
            return vec!["claude".to_string(), "skill".to_string()];
        }
        hints
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        // One prompt section per discovered skill (its SKILL.md body as a TOC).
        let tool_prefix = self.sp.get_str_or("tool_prefix", "claude_");
        let mut sections: Vec<Value> = Vec::new();
        for skill in &self.skills {
            if skill.skip_prompt {
                continue;
            }
            let mut body = skill.body.clone();
            if !skill.sections.is_empty() && !skill.skip_tool {
                let tool_name = format!("{tool_prefix}{}", sanitize_tool_name(&skill.name));
                let names: Vec<&str> = skill.sections.iter().map(|(k, _)| k.as_str()).collect();
                let _ = write!(
                    body,
                    "\n\nAvailable reference sections: {}\nCall {}(section=\"<name>\") to load a section.",
                    names.join(", "),
                    tool_name
                );
            }
            sections.push(json!({"title": skill.name, "body": body}));
        }

        if sections.is_empty() {
            let skills_path = self.sp.get_str_or("skills_path", "");
            return vec![json!({
                "title": "Claude Skills",
                "body": format!("You have access to Claude skills loaded from {skills_path}."),
                "bullets": [
                    "Use claude skill tools to execute specialized tasks.",
                    "Pass arguments as a string describing what you need.",
                    "Optionally specify a section to target a specific part of the skill.",
                ],
            })];
        }
        sections
    }
}

// ── Free helpers ────────────────────────────────────────────────────────────

/// Expand a leading `~` to the user's home directory.
fn expand_user(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home).join(rest);
        }
    } else if path == "~"
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home);
    }
    PathBuf::from(path)
}

/// Parse a `SKILL.md` file into `(frontmatter, body)`. The frontmatter is the
/// YAML block between the leading `---` fences; the body is everything after.
/// Returns `None` only when the file can't be read.
fn parse_skill_md(path: &Path) -> Option<(Map<String, Value>, String)> {
    let content = std::fs::read_to_string(path).ok()?;

    // No frontmatter: whole file is the body.
    if !content.starts_with("---") {
        return Some((Map::new(), content.trim().to_string()));
    }

    // Split on `---` into [before, frontmatter, body]. `splitn(3, ...)` keeps
    // any later `---` inside the body.
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        // Malformed frontmatter: treat the whole thing as body.
        return Some((Map::new(), content.trim().to_string()));
    }

    let frontmatter_str = parts[1].trim();
    let body = parts[2].trim().to_string();

    // Parse the YAML frontmatter into a JSON object (best-effort).
    let frontmatter: Map<String, Value> = serde_norway::from_str::<Value>(frontmatter_str)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    Some((frontmatter, body))
}

/// Discover supporting `.md` sections in a skill directory (recursive),
/// excluding `SKILL.md`. Section keys are the file stem, prefixed with the
/// parent folder for nested files (`references/api`). Sorted by key.
fn discover_sections(skill_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut sections: Vec<(String, PathBuf)> = Vec::new();
    collect_md_files(skill_dir, skill_dir, &mut sections);
    sections.sort_by(|a, b| a.0.cmp(&b.0));
    sections
}

fn collect_md_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(root, &path, out);
            continue;
        }
        let is_md = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if !is_md {
            continue;
        }
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name.eq_ignore_ascii_case("SKILL.md") {
            continue;
        }
        // Build the section key: `<parent>/<stem>` for nested, else `<stem>`.
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let key = match relative.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(parent) => format!("{}/{}", parent.to_string_lossy().replace('\\', "/"), stem),
            None => stem.to_string(),
        };
        out.push((key, path));
    }
}

/// Sanitize a skill name into a SWAIG tool-name-safe token: lowercase, hyphens
/// and whitespace → `_`, drop other non-`[a-z0-9_]`, no leading digit.
fn sanitize_tool_name(name: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c == '-' || c.is_whitespace() {
            if !prev_underscore {
                out.push('_');
                prev_underscore = true;
            }
        } else if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
            prev_underscore = c == '_';
        }
        // else: drop the character
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        "unnamed".to_string()
    } else {
        out
    }
}

/// Match a name against fnmatch-style include/exclude glob patterns. Excludes
/// win. Supports `*` and `?` wildcards (the only forms the reference uses).
fn matches_patterns(name: &str, include: &[String], exclude: &[String]) -> bool {
    if exclude.iter().any(|p| glob_match(p, name)) {
        return false;
    }
    include.iter().any(|p| glob_match(p, name))
}

/// Minimal fnmatch: `*` = any run, `?` = any single char. Case-sensitive
/// (mirrors `fnmatch.fnmatch` on POSIX for these skill dir names).
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    // Iterative wildcard match with backtracking.
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Substitute Claude skill argument placeholders in `body`:
/// `$ARGUMENTS` (full string), `$ARGUMENTS[N]` and `$N` (positional). If the
/// body has no bare `$ARGUMENTS` and `arguments` is non-empty, append them.
fn substitute_arguments(body: &str, arguments: &str) -> String {
    let positional: Vec<&str> = arguments.split_whitespace().collect();

    // Does the body contain a bare `$ARGUMENTS` (not `$ARGUMENTS[`)?
    let has_bare = {
        let mut found = false;
        let bytes = body.as_bytes();
        let needle = b"$ARGUMENTS";
        let mut i = 0;
        while i + needle.len() <= bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                let next = bytes.get(i + needle.len());
                if next != Some(&b'[') {
                    found = true;
                    break;
                }
            }
            i += 1;
        }
        found
    };

    // 1. `$ARGUMENTS[N]` → positional[N].
    let mut result = replace_indexed(body, "$ARGUMENTS[", &positional);
    // 2. `$N` → positional[N].
    result = replace_dollar_index(&result, &positional);
    // 3. `$ARGUMENTS` → full string.
    result = result.replace("$ARGUMENTS", arguments);

    if !has_bare && !arguments.is_empty() {
        let _ = write!(result, "\n\nARGUMENTS: {arguments}");
    }
    result
}

/// Replace `${prefix}N]` occurrences with `positional[N]` (empty if OOB).
fn replace_indexed(input: &str, prefix: &str, positional: &[&str]) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(pos) = rest.find(prefix) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + prefix.len()..];
        if let Some(close) = after.find(']') {
            let idx_str = &after[..close];
            if let Ok(idx) = idx_str.parse::<usize>() {
                out.push_str(positional.get(idx).copied().unwrap_or(""));
                rest = &after[close + 1..];
                continue;
            }
        }
        // Not a valid `[N]` form — emit the prefix literally and advance.
        out.push_str(prefix);
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Replace `$N` (a `$` followed by digits, not followed by more digits) with
/// `positional[N]`.
fn replace_dollar_index(input: &str, positional: &[&str]) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            let num: String = chars[i + 1..j].iter().collect();
            if let Ok(idx) = num.parse::<usize>() {
                out.push_str(positional.get(idx).copied().unwrap_or(""));
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_skills_metadata() {
        let skill = ClaudeSkills::new(Map::new());
        assert_eq!(skill.name(), "claude_skills");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_claude_skills_setup_needs_path() {
        let mut skill = ClaudeSkills::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("skills_path".to_string(), json!("/nonexistent/path/xyz"));
        let mut skill2 = ClaudeSkills::new(params);
        // A path that does not exist is a failed setup.
        assert!(!skill2.setup());
    }

    #[test]
    fn test_claude_skills_hints() {
        let skill = ClaudeSkills::new(Map::new());
        let hints = skill.get_hints();
        // No skills discovered → generic fallback hints.
        assert!(hints.contains(&"claude".to_string()));
    }

    #[test]
    fn test_sanitize_tool_name() {
        assert_eq!(sanitize_tool_name("PDF Processing"), "pdf_processing");
        assert_eq!(sanitize_tool_name("my-cool-skill"), "my_cool_skill");
        assert_eq!(sanitize_tool_name("2fast"), "_2fast");
        assert_eq!(sanitize_tool_name("!!!"), "unnamed");
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("pdf*", "pdf_processing"));
        assert!(!glob_match("pdf*", "csv_processing"));
        assert!(glob_match("skill?", "skill1"));
        assert!(!glob_match("skill?", "skill12"));
    }

    #[test]
    fn test_substitute_arguments() {
        // Bare $ARGUMENTS replaced inline, no append.
        assert_eq!(
            substitute_arguments("Process $ARGUMENTS now", "the file"),
            "Process the file now"
        );
        // No bare placeholder → arguments appended.
        assert_eq!(
            substitute_arguments("Do the thing", "extra"),
            "Do the thing\n\nARGUMENTS: extra"
        );
        // Positional $N and $ARGUMENTS[N]. No BARE $ARGUMENTS present (only the
        // indexed form), so Python's fallback appends the args (parity).
        assert_eq!(
            substitute_arguments("first=$0 second=$ARGUMENTS[1]", "alpha beta"),
            "first=alpha second=beta\n\nARGUMENTS: alpha beta"
        );
    }

    /// The core Contract-adjacent test: given a temp dir with a sample
    /// SKILL.md (YAML frontmatter + body + a supporting section), the skill
    /// DISCOVERS it, parses name/description, and registers a SWAIG tool that
    /// returns the substituted body — proving discovery+declaration works, not
    /// the old "In production, this would parse SKILL.md files" stub.
    #[test]
    fn test_discovery_registers_tool_from_sample_skill_md() {
        use std::io::Write as _;

        // Build a temp skills dir: <base>/pdf-processing/{SKILL.md, refs/api.md}
        let base = std::env::temp_dir().join(format!(
            "sw_claude_skills_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let skill_dir = base.join("pdf-processing");
        std::fs::create_dir_all(skill_dir.join("refs")).unwrap();

        let mut f = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        write!(
            f,
            "---\nname: pdf-processing\ndescription: Extract text from PDFs\nargument-hint: the PDF path\n---\n\nExtract text from $ARGUMENTS using the tools in ${{CLAUDE_SKILL_DIR}}."
        )
        .unwrap();
        drop(f);

        let mut sf = std::fs::File::create(skill_dir.join("refs").join("api.md")).unwrap();
        write!(sf, "# API reference\nThe extract() call...").unwrap();
        drop(sf);

        // Setup discovers the skill.
        let mut params = Map::new();
        params.insert(
            "skills_path".to_string(),
            json!(base.to_string_lossy().to_string()),
        );
        let mut skill = ClaudeSkills::new(params);
        assert!(skill.setup(), "setup must succeed for a real skills dir");
        assert_eq!(skill.skills.len(), 1, "one SKILL.md must be discovered");

        let discovered = &skill.skills[0];
        assert_eq!(discovered.name, "pdf-processing");
        assert_eq!(
            discovered.description.as_deref(),
            Some("Extract text from PDFs")
        );
        assert_eq!(discovered.argument_hint.as_deref(), Some("the PDF path"));
        // The supporting refs/api.md is discovered as a section.
        assert_eq!(discovered.sections.len(), 1);
        assert_eq!(discovered.sections[0].0, "refs/api");

        // Registering tools declares a SWAIG tool named `claude_pdf_processing`.
        let mut agent = AgentBase::new(crate::agent::AgentOptions::new("t"));
        skill.register_tools(&mut agent);
        assert!(
            agent.tools.contains_key("claude_pdf_processing"),
            "a SWAIG tool must be declared for the discovered skill"
        );

        // Invoking the tool returns the substituted SKILL.md body (discovery +
        // declaration + text substitution — NOT a stub string).
        let mut args = Map::new();
        args.insert("arguments".to_string(), json!("report.pdf"));
        let result = agent
            .on_function_call("claude_pdf_processing", &args, Some(&Map::new()))
            .expect("tool must dispatch");
        let response = result.to_value()["response"].as_str().unwrap().to_string();
        assert!(
            response.contains("Extract text from report.pdf"),
            "body must have $ARGUMENTS substituted; got: {response}"
        );
        assert!(
            response.contains(&skill_dir.to_string_lossy().to_string()),
            "body must have ${{CLAUDE_SKILL_DIR}} substituted; got: {response}"
        );
        assert!(
            !response.contains("In production"),
            "must NOT be the old stub string"
        );

        // Loading a section returns that file's content.
        let mut sargs = Map::new();
        sargs.insert("arguments".to_string(), json!(""));
        sargs.insert("section".to_string(), json!("refs/api"));
        let sresult = agent
            .on_function_call("claude_pdf_processing", &sargs, Some(&Map::new()))
            .expect("section dispatch");
        let sresp = sresult.to_value()["response"].as_str().unwrap().to_string();
        assert!(sresp.contains("API reference"), "section body must load");

        // A prompt section is produced from the SKILL.md body.
        let prompts = skill.get_prompt_sections();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0]["title"], "pdf-processing");

        std::fs::remove_dir_all(&base).ok();
    }

    /// `disable-model-invocation: true` → the skill is discovered but NO tool
    /// is registered (invocation control).
    #[test]
    fn test_disable_model_invocation_skips_tool() {
        use std::io::Write as _;
        let base = std::env::temp_dir().join(format!(
            "sw_claude_skills_dmi_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let skill_dir = base.join("internal");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut f = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        write!(
            f,
            "---\nname: internal\ndescription: hidden\ndisable-model-invocation: true\n---\n\nbody"
        )
        .unwrap();
        drop(f);

        let mut params = Map::new();
        params.insert(
            "skills_path".to_string(),
            json!(base.to_string_lossy().to_string()),
        );
        let mut skill = ClaudeSkills::new(params);
        assert!(skill.setup());
        assert_eq!(skill.skills.len(), 1);
        assert!(skill.skills[0].skip_tool);

        let mut agent = AgentBase::new(crate::agent::AgentOptions::new("t"));
        skill.register_tools(&mut agent);
        assert!(
            !agent.tools.contains_key("claude_internal"),
            "disable-model-invocation must skip tool registration"
        );

        std::fs::remove_dir_all(&base).ok();
    }
}
