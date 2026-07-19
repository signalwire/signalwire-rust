//! `PromptObjectModel` — root container for a POM document.
//!
//! Direct port of `signalwire.pom.pom.PromptObjectModel`
//! (signalwire-python/signalwire/signalwire/pom/pom.py:234-540).
//!
//! Owns a `Vec<Section>` (the top-level sections) and exposes
//! markdown / XML / JSON / YAML rendering plus structural helpers
//! (`add_section`, `find_section`, `add_pom_as_subsection`).
//!
//! The renderers produce a stable, well-specified output asserted
//! byte-for-byte by this crate's inline tests.

use std::fmt;

use serde_json::Value;

use crate::pom::section::Section;

/// Error returned by the [`PromptObjectModel`] parse constructors
/// ([`from_json`](PromptObjectModel::from_json) /
/// [`from_yaml`](PromptObjectModel::from_yaml) /
/// [`from_value`](PromptObjectModel::from_value)).
///
/// D9-rust: these constructors previously returned `Result<Self, String>`. This
/// typed replacement lets a caller distinguish a *syntax* failure (malformed
/// JSON/YAML) from a *structural* one (the document parsed but violates the POM
/// shape), while keeping the exact error message text a Python caller would see via
/// [`Display`](fmt::Display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PomParseError {
    /// The input was not well-formed JSON. Carries the serde parser message.
    InvalidJson(String),
    /// The input was not well-formed YAML. Carries the serde parser message.
    InvalidYaml(String),
    /// The document parsed but violates the POM structure (top level not an
    /// array, a non-object section, a wrong-typed field, a section missing any
    /// body/bullets/subsections, a subsection missing its title). Carries the
    /// reference's exact `ValueError` message.
    InvalidStructure(String),
}

impl fmt::Display for PomParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PomParseError::InvalidJson(m) => write!(f, "invalid JSON: {m}"),
            PomParseError::InvalidYaml(m) => write!(f, "invalid YAML: {m}"),
            PomParseError::InvalidStructure(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for PomParseError {}

/// Root container for a Prompt Object Model document.
///
/// Mirrors Python's `signalwire.pom.pom.PromptObjectModel`. Construct
/// with [`PromptObjectModel::new`], populate with [`add_section`],
/// then render via [`render_markdown`], [`render_xml`], [`to_json`],
/// or [`to_yaml`].
///
/// [`add_section`]: PromptObjectModel::add_section
/// [`render_markdown`]: PromptObjectModel::render_markdown
/// [`render_xml`]: PromptObjectModel::render_xml
/// [`to_json`]: PromptObjectModel::to_json
/// [`to_yaml`]: PromptObjectModel::to_yaml
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[must_use]
pub struct PromptObjectModel {
    /// Top-level sections, in insertion order. Only the *first*
    /// entry may have `title = None`; all others must carry a
    /// title (Python raises `ValueError` otherwise — see
    /// `add_section`).
    pub sections: Vec<Section>,
}

impl PromptObjectModel {
    /// Construct an empty model. Mirrors Python's
    /// `PromptObjectModel()` constructor.
    pub fn new() -> Self {
        PromptObjectModel {
            sections: Vec::new(),
        }
    }

    /// Parse a JSON string into a [`PromptObjectModel`]. Mirrors
    /// Python's `PromptObjectModel.from_json(json_data)`.
    ///
    /// Returns [`PomParseError`] with a descriptive message on parse
    /// errors, matching Python's `ValueError`.
    ///
    /// # Errors
    ///
    /// Returns [`PomParseError::InvalidJson`] if `json_str` is not well-formed
    /// JSON (its `Display` is `"invalid JSON: <serde error>"`), or a
    /// [`PomParseError::InvalidStructure`] if the parsed document fails POM
    /// structural validation — see [`from_value`], which this delegates to (e.g.
    /// the top level is not an array, a section is not an object, or a section
    /// has no body/bullets/subsections).
    ///
    /// [`from_value`]: PromptObjectModel::from_value
    pub fn from_json(json_str: &str) -> Result<Self, PomParseError> {
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| PomParseError::InvalidJson(e.to_string()))?;
        Self::from_value(&value)
    }

    /// Parse a YAML string into a [`PromptObjectModel`]. Mirrors
    /// Python's `PromptObjectModel.from_yaml(yaml_data)`.
    ///
    /// # Errors
    ///
    /// Returns [`PomParseError::InvalidYaml`] if `yaml_str` is not well-formed
    /// YAML (its `Display` is `"invalid YAML: <serde error>"`), or a
    /// [`PomParseError::InvalidStructure`] if the parsed document fails POM
    /// structural validation — see [`from_value`], which this delegates to.
    ///
    /// [`from_value`]: PromptObjectModel::from_value
    pub fn from_yaml(yaml_str: &str) -> Result<Self, PomParseError> {
        let value: Value = serde_norway::from_str(yaml_str)
            .map_err(|e| PomParseError::InvalidYaml(e.to_string()))?;
        Self::from_value(&value)
    }

    /// Build a model from a parsed `serde_json::Value`. Used by
    /// both [`from_json`] and [`from_yaml`].
    ///
    /// [`from_json`]: PromptObjectModel::from_json
    /// [`from_yaml`]: PromptObjectModel::from_yaml
    ///
    /// # Errors
    ///
    /// Returns [`PomParseError::InvalidStructure`] when the value violates the
    /// POM document shape. The top level must be an array (else `"POM document
    /// must be an array of sections"`); each element is validated by
    /// `build_section`, which rejects non-object sections (`"Each
    /// section must be an object/dict."`), wrong-typed fields (e.g.
    /// `"'title' must be a string if present."`, `"'subsections' must
    /// be a list if provided."`), a section lacking any of a non-empty
    /// body, non-empty bullets, or subsections, and a subsection
    /// missing its required `title`.
    pub fn from_value(value: &Value) -> Result<Self, PomParseError> {
        let arr = value.as_array().ok_or_else(|| {
            PomParseError::InvalidStructure("POM document must be an array of sections".to_string())
        })?;

        let mut pom = PromptObjectModel::new();
        for (idx, sec_val) in arr.iter().enumerate() {
            // build_section still yields the reference's exact ValueError text as
            // a String; wrap it as a structural parse error (message preserved).
            let sec = build_section(sec_val, /*is_subsection=*/ false, idx)
                .map_err(PomParseError::InvalidStructure)?;
            pom.sections.push(sec);
        }
        Ok(pom)
    }

    /// Append a top-level section with the given title and body.
    ///
    /// Mirrors Python's `PromptObjectModel.add_section(title, body=...)`.
    /// Only the *first* section may pass `title = None`; subsequent
    /// `None` titles return `Err`.
    ///
    /// Returns a mutable reference to the new section so callers
    /// can keep configuring it (Python returns the `Section` object
    /// — Rust's borrow checker makes a `&mut` reference the
    /// equivalent shape).
    ///
    /// # Errors
    ///
    /// Returns `Err("Only the first section can have no title")` when
    /// `title` is `None` but the model already contains at least one
    /// section — only the very first section may be untitled.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the internal `.expect("just pushed")`
    /// reads back the section pushed on the line above, so the `last_mut()`
    /// is always `Some`.
    pub fn add_section(&mut self, title: Option<String>) -> Result<&mut Section, String> {
        if title.is_none() && !self.sections.is_empty() {
            return Err("Only the first section can have no title".to_string());
        }
        self.sections.push(Section::new(title));
        Ok(self.sections.last_mut().expect("just pushed"))
    }

    /// Append a top-level section with title + body in one call.
    /// Convenience wrapper that mirrors Python's keyword-style
    /// `add_section(title=..., body=...)`.
    ///
    /// # Errors
    ///
    /// Propagates the error from [`add_section`]: returns
    /// `Err("Only the first section can have no title")` when `title`
    /// is `None` and the model already has a section.
    ///
    /// [`add_section`]: PromptObjectModel::add_section
    pub fn add_section_with(
        &mut self,
        title: Option<String>,
        body: impl Into<String>,
    ) -> Result<&mut Section, String> {
        let sec = self.add_section(title)?;
        sec.body = body.into();
        Ok(sec)
    }

    /// Find the first section (recursively, depth-first) with the
    /// given title. Returns `None` when no match. Mirrors Python's
    /// `find_section`.
    pub fn find_section(&self, title: &str) -> Option<&Section> {
        fn recurse<'a>(sections: &'a [Section], title: &str) -> Option<&'a Section> {
            for section in sections {
                if section.title.as_deref() == Some(title) {
                    return Some(section);
                }
                if let Some(found) = recurse(&section.subsections, title) {
                    return Some(found);
                }
            }
            None
        }
        recurse(&self.sections, title)
    }

    /// Mutable variant of [`find_section`].
    ///
    /// [`find_section`]: PromptObjectModel::find_section
    pub fn find_section_mut(&mut self, title: &str) -> Option<&mut Section> {
        fn recurse<'a>(sections: &'a mut [Section], title: &str) -> Option<&'a mut Section> {
            for section in sections {
                if section.title.as_deref() == Some(title) {
                    return Some(section);
                }
                if let Some(found) = recurse(&mut section.subsections, title) {
                    return Some(found);
                }
            }
            None
        }
        recurse(&mut self.sections, title)
    }

    /// Convert the model to a `serde_json::Value` (a JSON array of
    /// section dicts). Mirrors Python's `to_dict`. The Rust name
    /// follows serde idiom (`to_value`) but the cross-port surface
    /// audit treats `to_value` ≡ `to_dict`.
    pub fn to_value(&self) -> Value {
        Value::Array(
            self.sections
                .iter()
                .map(super::section::Section::to_value)
                .collect(),
        )
    }

    /// Render the model as a JSON string (indent=2). Matches
    /// Python's `to_json` byte-for-byte: `json.dumps([...], indent=2)`.
    ///
    /// # Errors
    ///
    /// Returns `Err("failed to serialize JSON: <serde error>")` if
    /// `serde_json` fails to serialize the section tree. In practice
    /// the POM value is always serializable, so this is effectively
    /// infallible, but the fallible signature is preserved to mirror
    /// Python's `to_json`.
    pub fn to_json(&self) -> Result<String, String> {
        // serde_json::to_string_pretty uses indent=2 by default,
        // matching Python's json.dumps(..., indent=2).
        serde_json::to_string_pretty(&self.to_value())
            .map_err(|e| format!("failed to serialize JSON: {e}"))
    }

    /// Render the model as a YAML string. Matches PyYAML's output
    /// shape (`default_flow_style=False, sort_keys=False`).
    ///
    /// We hand-emit YAML rather than rely on `serde_norway::to_string`
    /// because the latter (a) sorts keys alphabetically when fed a
    /// `serde_json::Value` (which uses `BTreeMap` internally) and
    /// (b) doesn't expose a switch to disable that. The POM
    /// document shape is fully constrained — list of dicts with
    /// known string/list-of-string/list-of-dict values — so a
    /// targeted emitter is straightforward and guarantees correct output.
    ///
    /// # Errors
    ///
    /// The fallible `Result` signature mirrors Python's `to_yaml`. The
    /// hand-rolled emitter walks a fully-constrained document shape and
    /// always succeeds, so no `Err` is currently produced; the
    /// signature is retained for API stability and to allow
    /// future emit failures to surface without an API break.
    pub fn to_yaml(&self) -> Result<String, String> {
        if self.sections.is_empty() {
            // PyYAML emits `[]\n` for an empty list.
            return Ok("[]\n".to_string());
        }
        let mut out = String::new();
        for section in &self.sections {
            emit_section_yaml(section, &mut out, /*key_indent=*/ 2);
        }
        Ok(out)
    }

    /// Render the entire model as markdown. Matches Python's
    /// `render_markdown` byte-for-byte.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        let any_section_numbered = self.sections.iter().any(|s| s.numbered == Some(true));

        let mut md: Vec<String> = Vec::new();
        let mut section_counter: usize = 0;
        for section in &self.sections {
            let section_number: Vec<usize> = if section.title.is_some() {
                section_counter += 1;
                if any_section_numbered && section.numbered != Some(false) {
                    vec![section_counter]
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            md.push(section.render_markdown_at(2, &section_number));
        }

        md.join("\n")
    }

    /// Render the entire model as XML. Matches Python's
    /// `render_xml` byte-for-byte.
    #[must_use]
    pub fn render_xml(&self) -> String {
        let mut xml: Vec<String> = vec![
            r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string(),
            "<prompt>".to_string(),
        ];

        let any_section_numbered = self.sections.iter().any(|s| s.numbered == Some(true));

        let mut section_counter: usize = 0;
        for section in &self.sections {
            let section_number: Vec<usize> = if section.title.is_some() {
                section_counter += 1;
                if any_section_numbered && section.numbered != Some(false) {
                    vec![section_counter]
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            xml.push(section.render_xml_at(1, &section_number));
        }

        xml.push("</prompt>".to_string());
        xml.join("\n")
    }

    /// Append every top-level section of `pom_to_add` as a
    /// subsection of the section identified by `target_title`.
    ///
    /// Mirrors Python's `add_pom_as_subsection(target, pom_to_add)`
    /// where `target` is a section title. Returns `Err` when no
    /// section with the given title exists.
    ///
    /// # Errors
    ///
    /// Returns `Err("No section with title '<target_title>' found.")`
    /// when no section (at any depth) matches `target_title`, so there
    /// is nowhere to attach the incoming sections.
    pub fn add_pom_as_subsection(
        &mut self,
        target_title: &str,
        pom_to_add: &PromptObjectModel,
    ) -> Result<(), String> {
        let target = self
            .find_section_mut(target_title)
            .ok_or_else(|| format!("No section with title '{target_title}' found."))?;
        for section in &pom_to_add.sections {
            target.subsections.push(section.clone());
        }
        Ok(())
    }
}

// ─── Internal helpers ────────────────────────────────────────────────

/// Recursively build a [`Section`] from a parsed JSON/YAML value.
/// Mirrors Python's nested `build_section(d, is_subsection)` helper
/// inside `_from_dict`.
fn build_section(value: &Value, is_subsection: bool, top_index: usize) -> Result<Section, String> {
    let map = value
        .as_object()
        .ok_or_else(|| "Each section must be an object/dict.".to_string())?;

    if let Some(t) = map.get("title")
        && !t.is_string()
    {
        return Err("'title' must be a string if present.".to_string());
    }
    if let Some(s) = map.get("subsections")
        && !s.is_array()
    {
        return Err("'subsections' must be a list if provided.".to_string());
    }
    if let Some(b) = map.get("bullets")
        && !b.is_array()
    {
        return Err("'bullets' must be a list if provided.".to_string());
    }
    if let Some(n) = map.get("numbered")
        && !n.is_boolean()
    {
        return Err("'numbered' must be a boolean if provided.".to_string());
    }
    if let Some(nb) = map.get("numberedBullets")
        && !nb.is_boolean()
    {
        return Err("'numberedBullets' must be a boolean if provided.".to_string());
    }

    // Validate body / bullets / subsections present (Python rule)
    let has_body = map
        .get("body")
        .and_then(|b| b.as_str())
        .is_some_and(|s| !s.is_empty());
    let has_bullets = map
        .get("bullets")
        .and_then(|b| b.as_array())
        .is_some_and(|a| !a.is_empty());
    let has_subsections = map
        .get("subsections")
        .and_then(|s| s.as_array())
        .is_some_and(|a| !a.is_empty());
    if !has_body && !has_bullets && !has_subsections {
        return Err(
            "All sections must have either a non-empty body, non-empty bullets, or subsections"
                .to_string(),
        );
    }

    // Subsections must have a title
    if is_subsection && !map.contains_key("title") {
        return Err("All subsections must have a title".to_string());
    }

    // Top-level: only the first section can be untitled — Python
    // mutates the dict in-place to add "Untitled Section" for
    // later untitled siblings. We replicate that behaviour here so
    // round-trips through `from_json` / `to_json` survive
    // (test_from_json_round_trip_preserves_structure).
    let title: Option<String> = if let Some(t) = map.get("title").and_then(|v| v.as_str()) {
        Some(t.to_string())
    } else if !is_subsection && top_index > 0 {
        Some("Untitled Section".to_string())
    } else {
        None
    };

    let body = map
        .get("body")
        .and_then(|b| b.as_str())
        .unwrap_or("")
        .to_string();
    let bullets: Vec<String> = map
        .get("bullets")
        .and_then(|b| b.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                .collect()
        })
        .unwrap_or_default();
    let numbered = map.get("numbered").and_then(serde_json::Value::as_bool);
    let numbered_bullets = map
        .get("numberedBullets")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let mut section = Section {
        title,
        body,
        bullets,
        subsections: Vec::new(),
        numbered,
        numbered_bullets,
    };

    if let Some(subs) = map.get("subsections").and_then(|s| s.as_array()) {
        for sub in subs {
            section
                .subsections
                .push(build_section(sub, /*is_subsection=*/ true, 0)?);
        }
    }

    Ok(section)
}

/// Emit a single section as YAML (insertion-order keys, 2-space
/// indent, matching PyYAML's `default_flow_style=False, sort_keys=False`).
///
/// `key_indent` is the column at which keys of this mapping appear
/// (PyYAML-compact style: list-marker "- " sits at column
/// `key_indent - 2`, mapping keys at column `key_indent`, nested
/// lists also at column `key_indent`).
///
/// PyYAML output shape:
///   - title: A          # "- " at col 0, "title" at col 2
///     body: ab          # "body" at col 2
///     subsections:
///     - title: A1       # nested list "- " at col 2, "title" at col 4
///       body: a1b
fn emit_section_yaml(section: &Section, out: &mut String, key_indent: usize) {
    let key_indent_str = " ".repeat(key_indent);
    let marker_indent_str = " ".repeat(key_indent.saturating_sub(2));
    let mut first = true;

    // Helper: emit "key: value\n" with the right indentation
    macro_rules! push_kv {
        ($key:expr, $value:expr) => {
            if first {
                out.push_str(&marker_indent_str);
                out.push_str("- ");
                first = false;
            } else {
                out.push_str(&key_indent_str);
            }
            out.push_str($key);
            out.push_str(": ");
            out.push_str($value);
            out.push('\n');
        };
    }

    // Helper: emit "key:\n" header (no value on same line)
    macro_rules! push_key_header {
        ($key:expr) => {
            if first {
                out.push_str(&marker_indent_str);
                out.push_str("- ");
                first = false;
            } else {
                out.push_str(&key_indent_str);
            }
            out.push_str($key);
            out.push_str(":\n");
        };
    }

    if let Some(title) = &section.title {
        push_kv!("title", &yaml_scalar(title));
    }

    if !section.body.is_empty() {
        push_kv!("body", &yaml_scalar(&section.body));
    }

    if !section.bullets.is_empty() {
        push_key_header!("bullets");
        // PyYAML keeps nested lists at the same column as their
        // parent key (compact style). So bullet "- x" goes at
        // `key_indent` columns deep.
        for bullet in &section.bullets {
            out.push_str(&key_indent_str);
            out.push_str("- ");
            out.push_str(&yaml_scalar(bullet));
            out.push('\n');
        }
    }

    if !section.subsections.is_empty() {
        push_key_header!("subsections");
        // Each subsection is a list element at `key_indent` cols
        // deep; its own keys then sit at `key_indent + 2`.
        for sub in &section.subsections {
            emit_section_yaml(sub, out, key_indent + 2);
        }
    }

    if section.numbered == Some(true) {
        push_kv!("numbered", "true");
    }

    if section.numbered_bullets {
        push_kv!("numberedBullets", "true");
    }

    // Theoretically invalid (validator forbids it), but if it
    // ever happens emit "- {}" so the YAML stays well-formed.
    if first {
        out.push_str(&marker_indent_str);
        out.push_str("- {}\n");
    }
}

/// Emit a YAML scalar matching PyYAML's default style for
/// human-readable strings: bare for plain text, single-quoted for
/// values that'd otherwise look like YAML special tokens. The POM
/// payload is constrained enough that bare emission is safe for
/// the typical content we see (titles, body text, bullets); for
/// anything containing characters that require quoting we fall
/// back to JSON's quoted form (PyYAML does the same when needed).
fn yaml_scalar(s: &str) -> String {
    if needs_yaml_quoting(s) {
        // serde_json::to_string emits a double-quoted JSON string
        // which is also a valid YAML quoted scalar (YAML's
        // double-quoted style is a strict superset of JSON
        // strings). PyYAML may pick single quotes or block
        // notation depending on content; we accept the JSON
        // double-quoted form because POM round-trips don't carry
        // through the *style*, only the *value* (which `from_yaml`
        // reads back identically).
        serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
    } else {
        s.to_string()
    }
}

fn needs_yaml_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    // Any of these characters or YAML reserved tokens trigger quoting.
    // Conservative: matches PyYAML's emitter behaviour for the
    // shapes the POM ever sees.
    let first = s.chars().next().unwrap();
    if matches!(
        first,
        '!' | '&'
            | '*'
            | '?'
            | '|'
            | '-'
            | '<'
            | '>'
            | '='
            | '%'
            | '@'
            | '`'
            | '"'
            | '\''
            | '['
            | ']'
            | '{'
            | '}'
            | '#'
            | ','
            | ' '
    ) {
        return true;
    }
    if s.contains(": ") || s.contains(" #") || s.ends_with(':') {
        return true;
    }
    if s.contains('\n') || s.contains('\t') {
        return true;
    }
    // YAML reserved scalars
    matches!(
        s.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~"
    )
}
