//! SWML document rendering utilities.
//!
//! Port of Python `signalwire.core.swml_renderer`. [`SwmlRenderer`] renders
//! complete SWML documents (with AI + SWAIG components) and function-response
//! documents, driving an underlying [`Service`] through the fluent
//! [`SwmlBuilder`].

use serde_json::{Map, Value, json};

use crate::swml::builder::SwmlBuilder;
use crate::swml::service::Service;

/// Optional parameters for [`SwmlRenderer::render_swml`].
///
/// Mirrors the keyword arguments of Python `SwmlRenderer.render_swml`. All
/// fields default to the Python defaults via [`Default`].
// The bool fields (prompt_is_pom / add_answer / record_call / record_stereo)
// are independent render toggles mirroring Python's keyword args — each is a
// distinct option, not a state machine, so a flat options struct is the right
// shape.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct RenderSwmlOptions {
    pub post_prompt: Option<String>,
    pub post_prompt_url: Option<String>,
    pub swaig_functions: Option<Vec<Value>>,
    pub startup_hook_url: Option<String>,
    pub hangup_hook_url: Option<String>,
    pub prompt_is_pom: bool,
    pub params: Option<Map<String, Value>>,
    pub add_answer: bool,
    pub record_call: bool,
    pub record_format: String,
    pub record_stereo: bool,
    pub default_webhook_url: Option<String>,
}

impl Default for RenderSwmlOptions {
    fn default() -> Self {
        RenderSwmlOptions {
            post_prompt: None,
            post_prompt_url: None,
            swaig_functions: None,
            startup_hook_url: None,
            hangup_hook_url: None,
            prompt_is_pom: false,
            params: None,
            add_answer: false,
            record_call: false,
            record_format: "mp4".to_string(),
            record_stereo: true,
            default_webhook_url: None,
        }
    }
}

/// Renders SWML documents for SignalWire AI Agents with AI and SWAIG components.
pub struct SwmlRenderer;

impl SwmlRenderer {
    /// Generate a complete SWML document with AI configuration.
    ///
    /// `prompt` is either a text string (when `opts.prompt_is_pom` is false) or
    /// a POM array (when true). Returns the SWML document as a JSON string.
    #[must_use]
    pub fn render_swml(prompt: &Value, service: &mut Service, opts: &RenderSwmlOptions) -> String {
        let mut builder = SwmlBuilder::new(service);
        builder.reset();

        if opts.add_answer {
            builder.answer(None, None);
        }

        if opts.record_call {
            builder.service_mut().add_verb(
                "record_call",
                json!({"format": opts.record_format, "stereo": opts.record_stereo}),
            );
        }

        // Configure SWAIG object for the AI verb.
        let mut swaig_config = Map::new();
        let mut functions: Vec<Value> = Vec::new();

        if let Some(url) = &opts.startup_hook_url {
            functions.push(json!({
                "function": "startup_hook",
                "description": "Called when the call starts",
                "parameters": {"type": "object", "properties": {}},
                "web_hook_url": url,
            }));
        }
        if let Some(url) = &opts.hangup_hook_url {
            functions.push(json!({
                "function": "hangup_hook",
                "description": "Called when the call ends",
                "parameters": {"type": "object", "properties": {}},
                "web_hook_url": url,
            }));
        }
        if let Some(fns) = &opts.swaig_functions {
            for f in fns {
                let name = f.get("function").and_then(Value::as_str);
                if name != Some("startup_hook") && name != Some("hangup_hook") {
                    functions.push(f.clone());
                }
            }
        }

        if !functions.is_empty() || opts.default_webhook_url.is_some() {
            if let Some(url) = &opts.default_webhook_url {
                swaig_config.insert("defaults".to_string(), json!({"web_hook_url": url}));
            }
            if !functions.is_empty() {
                swaig_config.insert("functions".to_string(), Value::Array(functions));
            }
        }

        // Assemble the ai() args.
        let mut ai_args = Map::new();
        if opts.prompt_is_pom {
            ai_args.insert("prompt_pom".to_string(), prompt.clone());
        } else {
            ai_args.insert("prompt_text".to_string(), prompt.clone());
        }
        if let Some(pp) = &opts.post_prompt {
            ai_args.insert("post_prompt".to_string(), json!(pp));
        }
        if let Some(url) = &opts.post_prompt_url {
            ai_args.insert("post_prompt_url".to_string(), json!(url));
        }
        if !swaig_config.is_empty() {
            ai_args.insert("swaig".to_string(), Value::Object(swaig_config));
        }
        if let Some(params) = &opts.params {
            for (k, v) in params {
                ai_args.insert(k.clone(), v.clone());
            }
        }

        builder.ai(&ai_args);
        builder.render()
    }

    /// Generate a SWML document for a function response.
    ///
    /// Adds a `play` verb for `response_text` (when non-empty) followed by any
    /// `actions` (play / hangup / transfer / ai). Returns a JSON string.
    #[must_use]
    pub fn render_function_response_swml(
        response_text: &str,
        service: &mut Service,
        actions: Option<&[Value]>,
    ) -> String {
        service.reset_document();

        if !response_text.is_empty() {
            service.add_verb("play", json!({"text": response_text}));
        }

        if let Some(actions) = actions {
            for action in actions {
                if let Some(v) = action.get("play") {
                    service.add_verb("play", v.clone());
                } else if let Some(v) = action.get("hangup") {
                    service.add_verb("hangup", v.clone());
                } else if let Some(v) = action.get("transfer") {
                    service.add_verb("transfer", v.clone());
                } else if let Some(v) = action.get("ai") {
                    service.add_verb("ai", v.clone());
                }
            }
        }

        service.render_document()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swml::service::ServiceOptions;

    fn svc() -> Service {
        Service::new(ServiceOptions::new("renderer-test"))
    }

    #[test]
    fn test_render_swml_text_prompt() {
        let mut s = svc();
        let out = SwmlRenderer::render_swml(
            &json!("You are helpful"),
            &mut s,
            &RenderSwmlOptions::default(),
        );
        let doc: Value = serde_json::from_str(&out).unwrap();
        let main = doc["sections"]["main"].as_array().unwrap();
        let ai = main.iter().find(|v| v.get("ai").is_some()).unwrap();
        assert_eq!(ai["ai"]["prompt"]["text"], "You are helpful");
    }

    #[test]
    fn test_render_swml_with_answer_and_hooks() {
        let mut s = svc();
        let opts = RenderSwmlOptions {
            add_answer: true,
            startup_hook_url: Some("https://x/start".to_string()),
            default_webhook_url: Some("https://x/swaig".to_string()),
            ..Default::default()
        };
        let out = SwmlRenderer::render_swml(&json!("hi"), &mut s, &opts);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let main = doc["sections"]["main"].as_array().unwrap();
        assert!(main.iter().any(|v| v.get("answer").is_some()));
        let ai = main.iter().find(|v| v.get("ai").is_some()).unwrap();
        let fns = ai["ai"]["SWAIG"]["functions"].as_array().unwrap();
        assert!(fns.iter().any(|f| f["function"] == "startup_hook"));
        assert_eq!(
            ai["ai"]["SWAIG"]["defaults"]["web_hook_url"],
            "https://x/swaig"
        );
    }

    #[test]
    fn test_render_function_response() {
        let mut s = svc();
        let actions = vec![json!({"hangup": {"reason": "done"}})];
        let out = SwmlRenderer::render_function_response_swml("Goodbye", &mut s, Some(&actions));
        let doc: Value = serde_json::from_str(&out).unwrap();
        let main = doc["sections"]["main"].as_array().unwrap();
        assert_eq!(main[0]["play"]["text"], "Goodbye");
        assert_eq!(main[1]["hangup"]["reason"], "done");
    }

    #[test]
    fn test_render_function_response_empty_text() {
        let mut s = svc();
        let out = SwmlRenderer::render_function_response_swml("", &mut s, None);
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert!(doc["sections"]["main"].as_array().unwrap().is_empty());
    }
}
