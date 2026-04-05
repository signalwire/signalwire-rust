use std::collections::HashMap;

use serde_json::Value;

/// SWML document: version + named sections containing verb arrays.
pub struct Document {
    version: String,
    sections: HashMap<String, Vec<Value>>,
}

impl Document {
    pub fn new() -> Self {
        let mut sections = HashMap::new();
        sections.insert("main".to_string(), Vec::new());
        Document {
            version: "1.0.0".to_string(),
            sections,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Add a new named section. Returns `true` if created, `false` if it already existed.
    pub fn add_section(&mut self, name: &str) -> bool {
        if self.sections.contains_key(name) {
            return false;
        }
        self.sections.insert(name.to_string(), Vec::new());
        true
    }

    pub fn has_section(&self, name: &str) -> bool {
        self.sections.contains_key(name)
    }

    /// Get a clone of the verbs for a section (empty vec if section does not exist).
    pub fn get_verbs(&self, section: &str) -> Vec<Value> {
        self.sections.get(section).cloned().unwrap_or_default()
    }

    /// Append a verb to the `main` section.
    pub fn add_verb(&mut self, verb_name: &str, config: Value) {
        self.add_verb_to_section("main", verb_name, config);
    }

    /// Append a verb to a named section.
    ///
    /// # Panics
    ///
    /// Panics if the section does not exist.
    pub fn add_verb_to_section(&mut self, section: &str, verb_name: &str, config: Value) {
        let verbs = self
            .sections
            .get_mut(section)
            .unwrap_or_else(|| panic!("Section '{}' does not exist", section));
        let mut map = serde_json::Map::new();
        map.insert(verb_name.to_string(), config);
        verbs.push(Value::Object(map));
    }

    /// Append a pre-formatted verb value to a section.
    ///
    /// # Panics
    ///
    /// Panics if the section does not exist.
    pub fn add_raw_verb(&mut self, section: &str, verb_hash: Value) {
        let verbs = self
            .sections
            .get_mut(section)
            .unwrap_or_else(|| panic!("Section '{}' does not exist", section));
        verbs.push(verb_hash);
    }

    /// Clear all verbs in a section (keeps the section itself).
    pub fn clear_section(&mut self, section: &str) {
        if let Some(verbs) = self.sections.get_mut(section) {
            verbs.clear();
        }
    }

    /// Reset document to initial state (only `main` with no verbs).
    pub fn reset(&mut self) {
        self.sections.clear();
        self.sections.insert("main".to_string(), Vec::new());
    }

    /// Build the document as a `serde_json::Value`.
    pub fn to_value(&self) -> Value {
        let mut sections_map = serde_json::Map::new();
        // Sort keys for deterministic output
        let mut keys: Vec<&String> = self.sections.keys().collect();
        keys.sort();
        for key in keys {
            let verbs = &self.sections[key];
            sections_map.insert(key.clone(), Value::Array(verbs.clone()));
        }
        serde_json::json!({
            "version": self.version,
            "sections": sections_map,
        })
    }

    /// Compact JSON string.
    pub fn render(&self) -> String {
        serde_json::to_string(&self.to_value()).expect("Document serialisation should not fail")
    }

    /// Pretty-printed JSON string.
    pub fn render_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.to_value())
            .expect("Document serialisation should not fail")
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert_eq!(doc.version(), "1.0.0");
        assert!(doc.has_section("main"));
        assert!(doc.get_verbs("main").is_empty());
    }

    #[test]
    fn test_add_section() {
        let mut doc = Document::new();
        assert!(doc.add_section("intro"));
        assert!(doc.has_section("intro"));
        // Adding again returns false
        assert!(!doc.add_section("intro"));
    }

    #[test]
    fn test_add_section_does_not_replace_main() {
        let mut doc = Document::new();
        assert!(!doc.add_section("main"));
    }

    #[test]
    fn test_has_section_missing() {
        let doc = Document::new();
        assert!(!doc.has_section("nonexistent"));
    }

    #[test]
    fn test_add_verb_to_main() {
        let mut doc = Document::new();
        doc.add_verb("answer", serde_json::json!({"max_duration": 3600}));
        let verbs = doc.get_verbs("main");
        assert_eq!(verbs.len(), 1);
        assert_eq!(verbs[0]["answer"]["max_duration"], 3600);
    }

    #[test]
    fn test_add_verb_to_named_section() {
        let mut doc = Document::new();
        doc.add_section("intro");
        doc.add_verb_to_section("intro", "play", serde_json::json!({"url": "ring.mp3"}));
        let verbs = doc.get_verbs("intro");
        assert_eq!(verbs.len(), 1);
        assert_eq!(verbs[0]["play"]["url"], "ring.mp3");
    }

    #[test]
    #[should_panic(expected = "does not exist")]
    fn test_add_verb_to_missing_section_panics() {
        let mut doc = Document::new();
        doc.add_verb_to_section("nope", "answer", serde_json::json!({}));
    }

    #[test]
    fn test_sleep_integer_value() {
        let mut doc = Document::new();
        doc.add_verb("sleep", serde_json::json!(2000));
        let verbs = doc.get_verbs("main");
        assert_eq!(verbs[0]["sleep"], 2000);
    }

    #[test]
    fn test_add_raw_verb() {
        let mut doc = Document::new();
        let raw = serde_json::json!({"hangup": {"reason": "busy"}});
        doc.add_raw_verb("main", raw.clone());
        let verbs = doc.get_verbs("main");
        assert_eq!(verbs.len(), 1);
        assert_eq!(verbs[0], raw);
    }

    #[test]
    #[should_panic(expected = "does not exist")]
    fn test_add_raw_verb_missing_section_panics() {
        let mut doc = Document::new();
        doc.add_raw_verb("nope", serde_json::json!({}));
    }

    #[test]
    fn test_json_round_trip() {
        let mut doc = Document::new();
        doc.add_verb("answer", serde_json::json!({}));
        doc.add_verb("sleep", serde_json::json!(1000));

        let rendered = doc.render();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["version"], "1.0.0");
        let main_verbs = parsed["sections"]["main"].as_array().unwrap();
        assert_eq!(main_verbs.len(), 2);
        assert!(main_verbs[0].get("answer").is_some());
        assert_eq!(main_verbs[1]["sleep"], 1000);
    }

    #[test]
    fn test_render_pretty_contains_newlines() {
        let doc = Document::new();
        let pretty = doc.render_pretty();
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn test_clear_section() {
        let mut doc = Document::new();
        doc.add_verb("answer", serde_json::json!({}));
        assert_eq!(doc.get_verbs("main").len(), 1);
        doc.clear_section("main");
        assert!(doc.get_verbs("main").is_empty());
        assert!(doc.has_section("main"));
    }

    #[test]
    fn test_clear_nonexistent_section_is_noop() {
        let mut doc = Document::new();
        doc.clear_section("nope"); // should not panic
    }

    #[test]
    fn test_reset() {
        let mut doc = Document::new();
        doc.add_section("extra");
        doc.add_verb("answer", serde_json::json!({}));
        doc.add_verb_to_section("extra", "hangup", serde_json::json!({}));

        doc.reset();
        assert!(doc.has_section("main"));
        assert!(!doc.has_section("extra"));
        assert!(doc.get_verbs("main").is_empty());
    }

    #[test]
    fn test_get_verbs_missing_section_returns_empty() {
        let doc = Document::new();
        assert!(doc.get_verbs("nonexistent").is_empty());
    }

    #[test]
    fn test_to_value_structure() {
        let doc = Document::new();
        let val = doc.to_value();
        assert_eq!(val["version"], "1.0.0");
        assert!(val["sections"]["main"].is_array());
    }

    #[test]
    fn test_multiple_verbs_ordering() {
        let mut doc = Document::new();
        doc.add_verb("answer", serde_json::json!({}));
        doc.add_verb("play", serde_json::json!({"url": "test.mp3"}));
        doc.add_verb("hangup", serde_json::json!({}));
        let verbs = doc.get_verbs("main");
        assert_eq!(verbs.len(), 3);
        assert!(verbs[0].get("answer").is_some());
        assert!(verbs[1].get("play").is_some());
        assert!(verbs[2].get("hangup").is_some());
    }

    #[test]
    fn test_default_trait() {
        let doc = Document::default();
        assert_eq!(doc.version(), "1.0.0");
        assert!(doc.has_section("main"));
    }
}
