use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Load Claude SKILL.md files as agent tools (handler-based).
pub struct ClaudeSkills {
    sp: SkillParams,
}

impl ClaudeSkills {
    pub fn new(params: Map<String, Value>) -> Self {
        ClaudeSkills {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for ClaudeSkills {
    fn name(&self) -> &str {
        "claude_skills"
    }

    fn description(&self) -> &str {
        "Load Claude SKILL.md files as agent tools"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("skills_path").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let skills_path = self.sp.get_str_or("skills_path", "");
        let tool_prefix = self.sp.get_str_or("tool_prefix", "claude_");
        let response_prefix = self.sp.get_str_or("response_prefix", "");
        let response_postfix = self.sp.get_str_or("response_postfix", "");

        let tool_name = format!("{}skill", tool_prefix);

        agent.define_tool(
            &tool_name,
            &format!("Execute a Claude skill from {}", skills_path),
            json!({
                "arguments": {
                    "type": "string",
                    "description": "Arguments to pass to the skill",
                    "required": true,
                },
                "section": {
                    "type": "string",
                    "description": "Optional section of the skill to invoke",
                },
            }),
            Box::new(move |args, _raw| {
                let mut result = FunctionResult::new();
                let arguments = args
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let section = args
                    .get("section")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut response = String::new();
                if !response_prefix.is_empty() {
                    response.push_str(&response_prefix);
                    response.push(' ');
                }

                response.push_str(&format!(
                    "Claude skill execution from \"{}\"",
                    skills_path
                ));
                if !section.is_empty() {
                    response.push_str(&format!(" (section: {})", section));
                }
                response.push_str(&format!(
                    " with arguments: {}. \
                     In production, this would parse SKILL.md files with YAML frontmatter and execute them.",
                    arguments
                ));

                if !response_postfix.is_empty() {
                    response.push(' ');
                    response.push_str(&response_postfix);
                }

                result.set_response(&response);
                result
            }),
            false,
        );
    }

    fn get_hints(&self) -> Vec<String> {
        vec!["claude".to_string(), "skill".to_string()]
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        let skills_path = self.sp.get_str_or("skills_path", "");

        vec![json!({
            "title": "Claude Skills",
            "body": format!("You have access to Claude skills loaded from {}.", skills_path),
            "bullets": [
                "Use claude skill tools to execute specialized tasks.",
                "Pass arguments as a string describing what you need.",
                "Optionally specify a section to target a specific part of the skill.",
            ],
        })]
    }
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
        params.insert("skills_path".to_string(), json!("/path/to/skills"));
        let mut skill2 = ClaudeSkills::new(params);
        assert!(skill2.setup());
    }

    #[test]
    fn test_claude_skills_hints() {
        let skill = ClaudeSkills::new(Map::new());
        let hints = skill.get_hints();
        assert!(hints.contains(&"claude".to_string()));
    }
}
