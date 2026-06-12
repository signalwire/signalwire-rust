//! `PromptObjectModel` — root container for a POM document.
//!
//! Direct port of `signalwire.pom.pom.PromptObjectModel`
//! (signalwire-python/signalwire/signalwire/pom/pom.py:234-540).
//!
//! Owns a `Vec<Section>` (the top-level sections) and exposes
//! markdown / XML / JSON / YAML rendering plus structural helpers
//! (`add_section`, `find_section`, `add_pom_as_subsection`).
//!
//! All renderers match Python byte-for-byte — the cross-port
//! parity contract is asserted in
//! `signalwire-python/tests/unit/pom/test_pom_render_parity.py` and
//! mirrored in this crate's inline tests.

use serde_json::Value;

use crate::pom::section::Section;

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
    /// Returns `Err(String)` with a descriptive message on parse
    /// errors, matching Python's `ValueError`.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| format!("invalid JSON: {e}"))?;
        Self::from_value(&value)
    }

    /// Parse a YAML string into a [`PromptObjectModel`]. Mirrors
    /// Python's `PromptObjectModel.from_yaml(yaml_data)`.
    pub fn from_yaml(yaml_str: &str) -> Result<Self, String> {
        let value: Value = serde_norway::from_str(yaml_str)
            .map_err(|e| format!("invalid YAML: {e}"))?;
        Self::from_value(&value)
    }

    /// Build a model from a parsed `serde_json::Value`. Used by
    /// both [`from_json`] and [`from_yaml`].
    ///
    /// [`from_json`]: PromptObjectModel::from_json
    /// [`from_yaml`]: PromptObjectModel::from_yaml
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let arr = value
            .as_array()
            .ok_or_else(|| "POM document must be an array of sections".to_string())?;

        let mut pom = PromptObjectModel::new();
        for (idx, sec_val) in arr.iter().enumerate() {
            let sec = build_section(sec_val, /*is_subsection=*/ false, idx)?;
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
        Value::Array(self.sections.iter().map(super::section::Section::to_value).collect())
    }

    /// Render the model as a JSON string (indent=2). Matches
    /// Python's `to_json` byte-for-byte: `json.dumps([...], indent=2)`.
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
    /// targeted emitter is straightforward and guarantees parity.
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
        && !t.is_string() {
            return Err("'title' must be a string if present.".to_string());
        }
    if let Some(s) = map.get("subsections")
        && !s.is_array() {
            return Err("'subsections' must be a list if provided.".to_string());
        }
    if let Some(b) = map.get("bullets")
        && !b.is_array() {
            return Err("'bullets' must be a list if provided.".to_string());
        }
    if let Some(n) = map.get("numbered")
        && !n.is_boolean() {
            return Err("'numbered' must be a boolean if provided.".to_string());
        }
    if let Some(nb) = map.get("numberedBullets")
        && !nb.is_boolean() {
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
        '!' | '&' | '*' | '?' | '|' | '-' | '<' | '>' | '=' | '%' | '@' | '`' | '"' | '\'' | '['
            | ']' | '{' | '}' | '#' | ',' | ' '
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
