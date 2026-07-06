//! SWML verb handlers — interface and implementations for SWML verb handling.
//!
//! Port of Python `signalwire.core.swml_handler`. Defines the base interface
//! for SWML verb handlers ([`SwmlVerbHandler`]) and a concrete handler for the
//! complex `ai` verb ([`AiVerbHandler`]), plus the [`VerbHandlerRegistry`] that
//! maps verb names to their specialized handlers.
//!
//! Python models the interface as an ABC (`SWMLVerbHandler`) with abstract
//! `get_verb_name` / `validate_config` / `build_config`; Rust expresses the
//! same contract as a trait. The Python class names carry the `SWML` prefix;
//! the enumerator's class-rename table folds the Rust `SwmlVerbHandler` /
//! `AiVerbHandler` onto the reference `SWMLVerbHandler` / `AIVerbHandler`.

use std::collections::HashMap;

use serde_json::{Map, Value};

/// Base interface for SWML verb handlers.
///
/// Verb handlers provide specialized logic for complex SWML verbs that cannot
/// be handled generically (the `ai` verb, above all). Mirrors the abstract
/// `SWMLVerbHandler` interface in Python.
pub trait SwmlVerbHandler: Send + Sync {
    /// The name of the verb this handler handles.
    fn get_verb_name(&self) -> String;

    /// Validate the configuration for this verb.
    ///
    /// Returns `(is_valid, error_messages)`.
    fn validate_config(&self, config: &Value) -> (bool, Vec<String>);

    /// Build a configuration for this verb from provided arguments.
    fn build_config(&self, args: &Map<String, Value>) -> Value;

    /// Clone this handler into a fresh boxed trait object.
    ///
    /// Enables `VerbHandlerRegistry` (and therefore `Service`) to be `Clone`,
    /// which `as_router` relies on to hand a shared snapshot to the mountable
    /// axum handler.
    fn clone_box(&self) -> Box<dyn SwmlVerbHandler>;
}

/// Handler for the SWML `ai` verb.
///
/// The `ai` verb is complex and requires specialized handling, particularly
/// for managing prompts, SWAIG functions, and AI configurations.
#[derive(Debug, Default, Clone)]
pub struct AiVerbHandler;

impl AiVerbHandler {
    #[must_use]
    pub fn new() -> Self {
        AiVerbHandler
    }
}

impl SwmlVerbHandler for AiVerbHandler {
    fn get_verb_name(&self) -> String {
        "ai".to_string()
    }

    fn clone_box(&self) -> Box<dyn SwmlVerbHandler> {
        Box::new(self.clone())
    }

    fn validate_config(&self, config: &Value) -> (bool, Vec<String>) {
        let mut errors: Vec<String> = Vec::new();

        let Some(obj) = config.as_object() else {
            errors.push("Missing required field 'prompt'".to_string());
            return (false, errors);
        };

        // Check that prompt is present.
        let Some(prompt) = obj.get("prompt") else {
            errors.push("Missing required field 'prompt'".to_string());
            return (false, errors);
        };

        let Some(prompt_obj) = prompt.as_object() else {
            errors.push("'prompt' must be an object".to_string());
            return (false, errors);
        };

        // Require either text or pom (mutually exclusive).
        let has_text = prompt_obj.contains_key("text");
        let has_pom = prompt_obj.contains_key("pom");
        let has_contexts = prompt_obj.contains_key("contexts");

        let base_prompt_count = usize::from(has_text) + usize::from(has_pom);
        if base_prompt_count == 0 {
            errors.push("'prompt' must contain either 'text' or 'pom' as base prompt".to_string());
        } else if base_prompt_count > 1 {
            errors.push(
                "'prompt' can only contain one of: 'text' or 'pom' (mutually exclusive)"
                    .to_string(),
            );
        }

        // Contexts are optional and can be combined with text or pom.
        if has_contexts && !prompt_obj["contexts"].is_object() {
            errors.push("'prompt.contexts' must be an object".to_string());
        }

        // Validate SWAIG structure if present.
        if let Some(swaig) = obj.get("SWAIG")
            && !swaig.is_object()
        {
            errors.push("'SWAIG' must be an object".to_string());
        }

        (errors.is_empty(), errors)
    }

    /// Build a configuration for the AI verb.
    ///
    /// Recognized keys in `args`:
    /// `prompt_text`, `prompt_pom`, `contexts`, `post_prompt`,
    /// `post_prompt_url`, `swaig`. Any other key is routed to `params`,
    /// except `languages`/`hints`/`pronounce`/`global_data` which are
    /// promoted to top-level keys (matching Python `build_config`).
    ///
    /// # Panics
    ///
    /// Panics if neither `prompt_text` nor `prompt_pom` is provided, or if
    /// both are provided (mutually exclusive), matching Python's `ValueError`.
    fn build_config(&self, args: &Map<String, Value>) -> Value {
        let prompt_text = args.get("prompt_text").filter(|v| !v.is_null());
        let prompt_pom = args.get("prompt_pom").filter(|v| !v.is_null());

        let base_prompt_count =
            usize::from(prompt_text.is_some()) + usize::from(prompt_pom.is_some());
        assert!(
            base_prompt_count != 0,
            "Either prompt_text or prompt_pom must be provided as base prompt"
        );
        assert!(
            base_prompt_count <= 1,
            "prompt_text and prompt_pom are mutually exclusive"
        );

        let mut config = Map::new();

        // Build prompt object with base prompt.
        let mut prompt_config = Map::new();
        if let Some(text) = prompt_text {
            prompt_config.insert("text".to_string(), text.clone());
        } else if let Some(pom) = prompt_pom {
            prompt_config.insert("pom".to_string(), pom.clone());
        }
        if let Some(contexts) = args.get("contexts").filter(|v| !v.is_null()) {
            prompt_config.insert("contexts".to_string(), contexts.clone());
        }
        config.insert("prompt".to_string(), Value::Object(prompt_config));

        if let Some(post_prompt) = args.get("post_prompt").filter(|v| !v.is_null()) {
            let mut pp = Map::new();
            pp.insert("text".to_string(), post_prompt.clone());
            config.insert("post_prompt".to_string(), Value::Object(pp));
        }
        if let Some(url) = args.get("post_prompt_url").filter(|v| !v.is_null()) {
            config.insert("post_prompt_url".to_string(), url.clone());
        }
        if let Some(swaig) = args.get("swaig").filter(|v| !v.is_null()) {
            config.insert("SWAIG".to_string(), swaig.clone());
        }

        // Add any additional parameters into params, with the four promoted keys.
        let mut params = Map::new();
        let reserved = [
            "prompt_text",
            "prompt_pom",
            "contexts",
            "post_prompt",
            "post_prompt_url",
            "swaig",
        ];
        for (key, value) in args {
            if reserved.contains(&key.as_str()) {
                continue;
            }
            match key.as_str() {
                "languages" | "hints" | "pronounce" | "global_data" => {
                    config.insert(key.clone(), value.clone());
                }
                _ => {
                    params.insert(key.clone(), value.clone());
                }
            }
        }
        config.insert("params".to_string(), Value::Object(params));

        Value::Object(config)
    }
}

/// Registry for SWML verb handlers.
///
/// Maintains a map of handlers for special SWML verbs and provides accessors.
/// Constructed pre-populated with the default [`AiVerbHandler`], matching
/// Python's `VerbHandlerRegistry.__init__`.
#[derive(Default)]
pub struct VerbHandlerRegistry {
    handlers: HashMap<String, Box<dyn SwmlVerbHandler>>,
}

impl Clone for VerbHandlerRegistry {
    fn clone(&self) -> Self {
        VerbHandlerRegistry {
            handlers: self
                .handlers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_box()))
                .collect(),
        }
    }
}

impl VerbHandlerRegistry {
    /// Initialize the registry with default handlers (the `ai` verb handler).
    #[must_use]
    pub fn new() -> Self {
        let mut reg = VerbHandlerRegistry {
            handlers: HashMap::new(),
        };
        reg.register_handler(Box::new(AiVerbHandler::new()));
        reg
    }

    /// Register a new verb handler, keyed by its verb name.
    pub fn register_handler(&mut self, handler: Box<dyn SwmlVerbHandler>) {
        let verb_name = handler.get_verb_name();
        self.handlers.insert(verb_name, handler);
    }

    /// Get the handler for a specific verb, if one is registered.
    #[must_use]
    pub fn get_handler(&self, verb_name: &str) -> Option<&dyn SwmlVerbHandler> {
        self.handlers.get(verb_name).map(AsRef::as_ref)
    }

    /// Whether a handler exists for a specific verb.
    #[must_use]
    pub fn has_handler(&self, verb_name: &str) -> bool {
        self.handlers.contains_key(verb_name)
    }

    /// The registered verb names, sorted. Python parity:
    /// `sorted(VerbHandlerRegistry._handlers.keys())`.
    #[must_use]
    pub fn handler_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.handlers.keys().cloned().collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ai_handler_verb_name() {
        assert_eq!(AiVerbHandler::new().get_verb_name(), "ai");
    }

    #[test]
    fn test_registry_has_default_ai_handler() {
        let reg = VerbHandlerRegistry::new();
        assert!(reg.has_handler("ai"));
        assert!(!reg.has_handler("play"));
        assert!(reg.get_handler("ai").is_some());
        assert!(reg.get_handler("missing").is_none());
    }

    #[test]
    fn test_validate_missing_prompt() {
        let h = AiVerbHandler::new();
        let (ok, errs) = h.validate_config(&json!({}));
        assert!(!ok);
        assert!(
            errs.iter()
                .any(|e| e.contains("Missing required field 'prompt'"))
        );
    }

    #[test]
    fn test_validate_prompt_not_object() {
        let h = AiVerbHandler::new();
        let (ok, errs) = h.validate_config(&json!({"prompt": "hi"}));
        assert!(!ok);
        assert!(errs.iter().any(|e| e.contains("must be an object")));
    }

    #[test]
    fn test_validate_prompt_needs_text_or_pom() {
        let h = AiVerbHandler::new();
        let (ok, errs) = h.validate_config(&json!({"prompt": {}}));
        assert!(!ok);
        assert!(errs.iter().any(|e| e.contains("either 'text' or 'pom'")));
    }

    #[test]
    fn test_validate_text_and_pom_mutually_exclusive() {
        let h = AiVerbHandler::new();
        let (ok, errs) = h.validate_config(&json!({"prompt": {"text": "a", "pom": []}}));
        assert!(!ok);
        assert!(errs.iter().any(|e| e.contains("mutually exclusive")));
    }

    #[test]
    fn test_validate_ok_with_text() {
        let h = AiVerbHandler::new();
        let (ok, errs) = h.validate_config(&json!({"prompt": {"text": "hello"}}));
        assert!(ok, "errors: {errs:?}");
    }

    #[test]
    fn test_validate_contexts_must_be_object() {
        let h = AiVerbHandler::new();
        let (ok, errs) = h.validate_config(&json!({"prompt": {"text": "x", "contexts": []}}));
        assert!(!ok);
        assert!(
            errs.iter()
                .any(|e| e.contains("contexts' must be an object"))
        );
    }

    #[test]
    fn test_validate_swaig_must_be_object() {
        let h = AiVerbHandler::new();
        let (ok, errs) = h.validate_config(&json!({"prompt": {"text": "x"}, "SWAIG": []}));
        assert!(!ok);
        assert!(errs.iter().any(|e| e.contains("'SWAIG' must be an object")));
    }

    #[test]
    fn test_build_config_text() {
        let h = AiVerbHandler::new();
        let mut args = Map::new();
        args.insert("prompt_text".to_string(), json!("You are helpful"));
        args.insert("post_prompt".to_string(), json!("Summarize"));
        args.insert("temperature".to_string(), json!(0.7));
        let cfg = h.build_config(&args);
        assert_eq!(cfg["prompt"]["text"], "You are helpful");
        assert_eq!(cfg["post_prompt"]["text"], "Summarize");
        assert_eq!(cfg["params"]["temperature"], 0.7);
    }

    #[test]
    fn test_build_config_pom_and_promoted_keys() {
        let h = AiVerbHandler::new();
        let mut args = Map::new();
        args.insert("prompt_pom".to_string(), json!([{"title": "Role"}]));
        args.insert("hints".to_string(), json!(["one", "two"]));
        args.insert("swaig".to_string(), json!({"functions": []}));
        let cfg = h.build_config(&args);
        assert!(cfg["prompt"]["pom"].is_array());
        assert_eq!(cfg["hints"], json!(["one", "two"]));
        assert!(cfg["SWAIG"].is_object());
    }

    #[test]
    #[should_panic(expected = "must be provided")]
    fn test_build_config_requires_base_prompt() {
        let h = AiVerbHandler::new();
        h.build_config(&Map::new());
    }

    #[test]
    #[should_panic(expected = "mutually exclusive")]
    fn test_build_config_rejects_both() {
        let h = AiVerbHandler::new();
        let mut args = Map::new();
        args.insert("prompt_text".to_string(), json!("a"));
        args.insert("prompt_pom".to_string(), json!([]));
        h.build_config(&args);
    }

    #[test]
    fn test_register_custom_handler() {
        #[derive(Clone)]
        struct HangupHandler;
        impl SwmlVerbHandler for HangupHandler {
            fn get_verb_name(&self) -> String {
                "hangup".to_string()
            }
            fn validate_config(&self, _config: &Value) -> (bool, Vec<String>) {
                (true, Vec::new())
            }
            fn build_config(&self, _args: &Map<String, Value>) -> Value {
                json!({})
            }
            fn clone_box(&self) -> Box<dyn SwmlVerbHandler> {
                Box::new(self.clone())
            }
        }
        let mut reg = VerbHandlerRegistry::new();
        reg.register_handler(Box::new(HangupHandler));
        assert!(reg.has_handler("hangup"));
        assert_eq!(reg.get_handler("hangup").unwrap().get_verb_name(), "hangup");
    }
}
