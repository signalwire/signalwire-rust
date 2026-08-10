//! SWML document rendering utilities.
//!
//! [`SwmlRenderer`] renders
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
///
/// The `bool` fields (`prompt_is_pom`, `add_answer`, `record_call`,
/// `record_stereo`) are independent render toggles mirroring Python's
/// keyword args — each a distinct option, not a state machine — so a flat
/// options struct is the right shape and `struct_excessive_bools` is
/// suppressed.
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
            // Text is played via the `say:` URL scheme — the SWML `play` verb has
            // NO `text` key. `$defs/Play` accepts only PlayWithURL / PlayWithURLS
            // under `unevaluatedProperties: {"not": {}}`, so `{"text": …}` is a
            // schema violation the validating `Service::add_verb` PANICS on.
            // Mirrors the Python reference (`swml_renderer.py:177`
            // `service.add_verb("play", {"url": f"say:{response_text}"})`).
            service.add_verb("play", json!({"url": format!("say:{response_text}")}));
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
        // Actions render as their own verbs. `hangup` reason must be a
        // schema-valid enum value (hangup|busy|decline) — the full validator
        // rejects "done" exactly as the Python reference does. Empty
        // response_text takes the no-play-verb branch, so the only verb is the
        // hangup action.
        let mut s = svc();
        let actions = vec![json!({"hangup": {"reason": "busy"}})];
        let out = SwmlRenderer::render_function_response_swml("", &mut s, Some(&actions));
        let doc: Value = serde_json::from_str(&out).unwrap();
        let main = doc["sections"]["main"].as_array().unwrap();
        assert_eq!(main[0]["hangup"]["reason"], "busy");
    }

    #[test]
    fn test_render_function_response_text_uses_say_url_scheme() {
        // A NON-empty response_text must render as the `say:` URL scheme.
        // The SWML `play` verb has NO `text` key: `$defs/Play` accepts only
        // PlayWithURL / PlayWithURLS under `unevaluatedProperties: {"not":{}}`,
        // so `{"play":{"text":…}}` is a schema violation the validating
        // `Service::add_verb` rejects. Spoken text goes through `url:
        // "say:<text>"` — matching the Python reference
        // (`swml_renderer.py:177  service.add_verb("play", {"url":
        // f"say:{response_text}"})`).
        //
        // This asserts THROUGH the validating path: `render_function_response_swml`
        // calls `Service::add_verb`, so a schema-forbidden key panics before it
        // can reach the document. The rendered doc is therefore proof of both
        // the correct key AND that the verb survived validation.
        let mut s = svc();
        let out = SwmlRenderer::render_function_response_swml("Hello there", &mut s, None);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let main = doc["sections"]["main"].as_array().unwrap();
        assert_eq!(main.len(), 1, "expected exactly one play verb: {main:?}");
        assert_eq!(main[0]["play"]["url"], "say:Hello there");
        assert!(
            main[0]["play"].get("text").is_none(),
            "`play` has no `text` key in the SWML schema: {:?}",
            main[0]
        );
    }

    #[test]
    #[should_panic(expected = "play")]
    fn test_play_with_text_key_is_rejected_by_the_validator() {
        // Negative control for the fix above: prove the validating entry point
        // actually REFUSES the schema-forbidden `text` key, so this class of
        // defect cannot silently reappear. `$defs/Play` closes its property set
        // with `unevaluatedProperties: {"not": {}}`.
        let mut s = svc();
        s.add_verb("play", json!({"text": "Hello there"}));
    }

    /// The `ai` verb's sub-object PLACEMENTS, pinned against the REFERENCE's own
    /// emission — the durable authority — not against any schema file's closure:
    ///   `contexts`                     -> inside `prompt`  (`swml_handler.py:191`)
    ///   `debug_webhook_url` / `_level` -> inside `params`  (`agent_base.py:1286/:1291`)
    ///   `temperature`                  -> `params`, because that is what the
    ///     reference does: `swml_handler.py`'s kwargs loop promotes only
    ///     `languages`/`hints`/`pronounce`/`global_data` and sends every other
    ///     key to `config["params"][key]`. Matching the reference wins.
    ///
    /// Deliberately NOT asserted here: the set of keys the `ai` TOP level admits.
    /// The vendored `schema.json` closes `$defs/AIObject` over nine keys, but the
    /// ENGINE (`mod_infrastructure/swml_schema.c:1880`) accepts FIFTEEN — the
    /// vendored file is a strict six-key-short subset, and a properly
    /// server-derived SWML spec is being vendored to replace it. Pinning the
    /// stale closure here would bake in a number known to be wrong and would go
    /// red when the real spec lands. The server is the spec; placements are what
    /// this test can assert durably.
    #[test]
    fn test_ai_verb_nests_contexts_and_debug_params_where_the_reference_puts_them() {
        // Drive the AI-verb HANDLER — the path that owns this routing
        // (`swml/handler.rs`, port of `swml_handler.py`). It promotes exactly
        // languages/hints/pronounce/global_data to the top level and sends every
        // other key to `params`; `contexts` goes into `prompt`.
        use crate::swml::handler::SwmlVerbHandler;
        let handler = crate::swml::handler::AiVerbHandler;
        let mut args = Map::new();
        args.insert("prompt_text".to_string(), json!("hi"));
        args.insert("contexts".to_string(), json!({"default": {"steps": []}}));
        args.insert("temperature".to_string(), json!(0.7));
        args.insert(
            "debug_webhook_url".to_string(),
            json!("https://x/debug_events"),
        );
        args.insert("debug_webhook_level".to_string(), json!(2));
        let cfg = handler.build_config(&args);

        let ai = cfg.as_object().unwrap();
        // Each key landed one level down, where the reference puts it — and NOT
        // at the top level, which is the misplacement other ports shipped.
        for k in [
            "contexts",
            "temperature",
            "debug_webhook_url",
            "debug_webhook_level",
        ] {
            assert!(
                !ai.contains_key(k),
                "`{k}` must be nested, not emitted at the `ai` top level: {ai:?}"
            );
        }
        assert!(ai["prompt"].get("contexts").is_some());
        assert_eq!(ai["params"]["debug_webhook_url"], "https://x/debug_events");
        assert_eq!(ai["params"]["debug_webhook_level"], 2);
        assert_eq!(ai["params"]["temperature"], 0.7);

        // record_call goes through the renderer's own emission.
        let mut s2 = svc();
        let opts = RenderSwmlOptions {
            record_call: true,
            ..Default::default()
        };
        let out = SwmlRenderer::render_swml(&json!("hi"), &mut s2, &opts);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let main = doc["sections"]["main"].as_array().unwrap();

        // $defs/RecordCall.stereo is anyOf<boolean, SWMLVar> — a bare `1`
        // JSON-encodes as a NUMBER and the schema rejects it (perl shipped
        // exactly that on every record_call). Confirm serde writes a real bool.
        let rc = main
            .iter()
            .find(|v| v.get("record_call").is_some())
            .unwrap();
        assert!(
            rc["record_call"]["stereo"].is_boolean(),
            "record_call.stereo must be a JSON boolean, got {:?}",
            rc["record_call"]["stereo"]
        );
    }

    #[test]
    fn test_render_function_response_empty_text() {
        let mut s = svc();
        let out = SwmlRenderer::render_function_response_swml("", &mut s, None);
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert!(doc["sections"]["main"].as_array().unwrap().is_empty());
    }
}
