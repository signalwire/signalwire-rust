//! Builder for structured POM prompts.
//!
//! Port of Python `signalwire.core.pom_builder.PomBuilder`. A flexible wrapper
//! around [`PromptObjectModel`] that supports dynamic section creation, adding
//! content to existing sections, nesting subsections, and rendering to
//! markdown / XML. There are no predefined section types.

use serde_json::Value;

use crate::pom::PromptObjectModel;
use crate::pom::section::Section;

/// Builder class for creating structured prompts using the Prompt Object Model.
#[derive(Default)]
pub struct PomBuilder {
    pom: PromptObjectModel,
    /// Titles of top-level sections, in insertion order — the Rust analog of
    /// Python's `self._sections` title→Section map (Rust stores sections in a
    /// `Vec` on the POM, so we track titles and resolve by lookup).
    section_titles: Vec<String>,
}

impl PomBuilder {
    /// Initialize a new POM builder with an empty POM.
    #[must_use]
    pub fn new() -> Self {
        PomBuilder {
            pom: PromptObjectModel::new(),
            section_titles: Vec::new(),
        }
    }

    /// Access the underlying [`PromptObjectModel`].
    pub fn pom(&self) -> &PromptObjectModel {
        &self.pom
    }

    /// Add a new section to the POM.
    ///
    /// Optional `bullets`, `numbered`, `numbered_bullets`, and `subsections`
    /// (each an object with `title`/`body`/`bullets`) further configure it.
    ///
    /// # Panics
    ///
    /// Panics if the underlying POM rejects the section (only the first
    /// section may be untitled) — the builder always passes a title, so this
    /// does not arise in practice.
    pub fn add_section(
        &mut self,
        title: &str,
        body: Option<&str>,
        bullets: Option<Vec<String>>,
        numbered: Option<bool>,
        numbered_bullets: Option<bool>,
        subsections: Option<Vec<Value>>,
    ) -> &mut Self {
        // `None` is the omit-it call; reference defaults are ""/false/false.
        let body = body.unwrap_or("");
        let numbered = numbered.unwrap_or(false);
        let numbered_bullets = numbered_bullets.unwrap_or(false);
        {
            let section = self
                .pom
                .add_section(Some(title.to_string()))
                .expect("titled section is always accepted");
            section.body = body.to_string();
            section.bullets = bullets.unwrap_or_default();
            section.numbered = Some(numbered);
            section.numbered_bullets = numbered_bullets;

            if let Some(subs) = subsections {
                for sub in subs {
                    if let Some(sub_title) = sub.get("title").and_then(Value::as_str) {
                        let sub_body = sub.get("body").and_then(Value::as_str).unwrap_or("");
                        let sub_bullets = sub
                            .get("bullets")
                            .and_then(Value::as_array)
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        section.add_subsection_full(
                            sub_title,
                            Some(sub_body.to_string()),
                            Some(sub_bullets),
                            None,
                            None,
                        );
                    }
                }
            }
        }
        if !self.section_titles.iter().any(|t| t == title) {
            self.section_titles.push(title.to_string());
        }
        self
    }

    /// Add content (body text and/or bullets) to an existing section,
    /// auto-creating it if it does not exist.
    pub fn add_to_section(
        &mut self,
        title: &str,
        body: Option<&str>,
        bullet: Option<&str>,
        bullets: Option<Vec<String>>,
    ) -> &mut Self {
        if !self.has_section(title) {
            self.add_section(title, None, None, None, None, None);
        }
        if let Some(section) = self.pom.find_section_mut(title) {
            if let Some(b) = body
                && !b.is_empty()
            {
                if section.body.is_empty() {
                    section.body = b.to_string();
                } else {
                    section.body = format!("{}\n\n{}", section.body, b);
                }
            }
            if let Some(one) = bullet {
                section.bullets.push(one.to_string());
            }
            if let Some(many) = bullets {
                section.bullets.extend(many);
            }
        }
        self
    }

    /// Add a subsection to an existing section, creating the parent if needed.
    pub fn add_subsection(
        &mut self,
        parent_title: &str,
        title: &str,
        body: Option<&str>,
        bullets: Option<Vec<String>>,
    ) -> &mut Self {
        // `None` is the omit-it call; the reference default is "".
        let body = body.map(str::to_string);
        if !self.has_section(parent_title) {
            self.add_section(parent_title, None, None, None, None, None);
        }
        if let Some(parent) = self.pom.find_section_mut(parent_title) {
            parent.add_subsection_full(title, body, bullets, None, None);
        }
        self
    }

    /// Whether a section with the given title exists.
    #[must_use]
    pub fn has_section(&self, title: &str) -> bool {
        self.pom.find_section(title).is_some()
    }

    /// Get a section by title.
    #[must_use]
    pub fn get_section(&self, title: &str) -> Option<&Section> {
        self.pom.find_section(title)
    }

    /// Render the POM as markdown.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        self.pom.render_markdown()
    }

    /// Render the POM as XML.
    #[must_use]
    pub fn render_xml(&self) -> String {
        self.pom.render_xml()
    }

    /// Convert the POM to a list of section values.
    #[must_use]
    pub fn to_value(&self) -> Value {
        self.pom.to_value()
    }

    /// Convert the POM to a JSON string.
    ///
    /// # Panics
    ///
    /// Panics if the underlying POM fails to serialize (a well-formed POM does
    /// not), matching the fail-loud contract.
    #[must_use]
    pub fn to_json(&self) -> String {
        self.pom
            .to_json()
            .expect("POM serialization should not fail")
    }

    /// Create a `PomBuilder` from a list of section values.
    ///
    /// # Panics
    ///
    /// Panics if `sections` is not a valid POM section list.
    #[must_use]
    pub fn from_sections(sections: &Value) -> Self {
        let pom = PromptObjectModel::from_value(sections)
            .expect("from_sections requires a valid section list");
        let mut titles = Vec::new();
        for section in &pom.sections {
            if let Some(t) = &section.title {
                titles.push(t.clone());
            }
        }
        PomBuilder {
            pom,
            section_titles: titles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_add_section_with_bullets() {
        let mut b = PomBuilder::new();
        b.add_section(
            "Role",
            Some("You are helpful."),
            Some(vec!["Be concise".to_string()]),
            None,
            None,
            None,
        );
        assert!(b.has_section("Role"));
        let s = b.get_section("Role").unwrap();
        assert_eq!(s.body, "You are helpful.");
        assert_eq!(s.bullets, vec!["Be concise".to_string()]);
    }

    #[test]
    fn test_add_section_with_subsections() {
        let mut b = PomBuilder::new();
        b.add_section(
            "Rules",
            None,
            None,
            None,
            None,
            Some(vec![
                json!({"title": "Sub", "body": "sub body", "bullets": ["x"]}),
            ]),
        );
        let s = b.get_section("Rules").unwrap();
        assert_eq!(s.subsections.len(), 1);
        assert_eq!(s.subsections[0].title.as_deref(), Some("Sub"));
        assert_eq!(s.subsections[0].bullets, vec!["x".to_string()]);
    }

    #[test]
    fn test_add_to_section_autovivifies_and_appends() {
        let mut b = PomBuilder::new();
        b.add_to_section("Notes", Some("first"), None, None);
        b.add_to_section("Notes", Some("second"), Some("bullet1"), None);
        let s = b.get_section("Notes").unwrap();
        assert_eq!(s.body, "first\n\nsecond");
        assert_eq!(s.bullets, vec!["bullet1".to_string()]);
    }

    #[test]
    fn test_add_subsection_autovivifies_parent() {
        let mut b = PomBuilder::new();
        b.add_subsection(
            "Parent",
            "Child",
            Some("child body"),
            Some(vec!["b".to_string()]),
        );
        assert!(b.has_section("Parent"));
        let p = b.get_section("Parent").unwrap();
        assert_eq!(p.subsections[0].title.as_deref(), Some("Child"));
    }

    #[test]
    fn test_has_get_section_missing() {
        let b = PomBuilder::new();
        assert!(!b.has_section("Nope"));
        assert!(b.get_section("Nope").is_none());
    }

    #[test]
    fn test_render_and_to_json() {
        let mut b = PomBuilder::new();
        b.add_section("Role", Some("You help."), None, None, None, None);
        assert!(b.render_markdown().contains("You help."));
        assert!(b.render_xml().contains("Role") || !b.render_xml().is_empty());
        let json_str = b.to_json();
        assert!(json_str.contains("Role"));
    }

    #[test]
    fn test_to_value_is_section_array() {
        let mut b = PomBuilder::new();
        b.add_section("Role", Some("You help."), None, None, None, None);
        let v = b.to_value();
        assert!(v.is_array());
        assert_eq!(v[0]["title"], "Role");
    }

    #[test]
    fn test_from_sections_round_trip() {
        let sections = json!([{"title": "Role", "body": "You help."}]);
        let b = PomBuilder::from_sections(&sections);
        assert!(b.has_section("Role"));
        assert_eq!(b.get_section("Role").unwrap().body, "You help.");
    }
}
