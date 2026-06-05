//! Amazon Bedrock voice-to-voice agent.
//!
//! `BedrockAgent` extends [`AgentBase`] (via composition + `Deref`) so it
//! shares all of AgentBase's features (prompt building, skills, tools,
//! post-prompt, dynamic configuration) but emits an `amazon_bedrock`
//! verb in the rendered SWML document instead of the standard `ai`
//! verb.
//!
//! Mirrors the Python `signalwire.agents.bedrock.BedrockAgent`.

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::agent::{AgentBase, AgentOptions};
use crate::logging::Logger;

/// Voice-to-voice agent backed by Amazon Bedrock.
///
/// Wraps an [`AgentBase`]: every standard agent operation
/// (`set_prompt_text`, `prompt_add_section`, `define_tool`, …) is
/// available through `Deref`/`DerefMut`. The single divergence is at
/// SWML rendering time: [`BedrockAgent::render_swml`] takes the SWML
/// produced by `AgentBase::render_swml` and rewrites the `ai` verb
/// into an `amazon_bedrock` verb that carries Bedrock-specific
/// inference parameters (voice id, temperature, top-p) inside the
/// prompt object.
pub struct BedrockAgent {
    agent: AgentBase,
    voice_id: String,
    temperature: f64,
    top_p: f64,
    max_tokens: u32,
    logger: Logger,
}

/// Construction options for [`BedrockAgent`]. Mirrors the keyword
/// arguments of Python's `BedrockAgent.__init__`.
#[must_use]
pub struct BedrockOptions {
    /// Agent name (default `"bedrock_agent"`).
    pub name: String,
    /// HTTP route (default `"/bedrock"`).
    pub route: String,
    /// Optional system prompt to register with `set_prompt_text`.
    pub system_prompt: Option<String>,
    /// Bedrock voice id (default `"matthew"`).
    pub voice_id: String,
    /// Generation temperature 0..1 (default `0.7`).
    pub temperature: f64,
    /// Nucleus sampling top-p 0..1 (default `0.9`).
    pub top_p: f64,
    /// Max generation tokens (default `1024`).
    pub max_tokens: u32,
    /// Optional bind host override.
    pub host: Option<String>,
    /// Optional bind port override.
    pub port: Option<u16>,
    /// Optional basic-auth user override.
    pub basic_auth_user: Option<String>,
    /// Optional basic-auth password override.
    pub basic_auth_password: Option<String>,
}

impl Default for BedrockOptions {
    fn default() -> Self {
        BedrockOptions {
            name: "bedrock_agent".to_string(),
            route: "/bedrock".to_string(),
            system_prompt: None,
            voice_id: "matthew".to_string(),
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 1024,
            host: None,
            port: None,
            basic_auth_user: None,
            basic_auth_password: None,
        }
    }
}

impl BedrockOptions {
    /// Convenience: create with a name (other fields keep defaults).
    pub fn with_name(name: &str) -> Self {
        BedrockOptions {
            name: name.to_string(),
            ..BedrockOptions::default()
        }
    }
}

impl BedrockAgent {
    /// Construct a new BedrockAgent.
    ///
    /// Mirrors Python's
    /// `BedrockAgent(name=..., route=..., system_prompt=..., voice_id=...,
    /// temperature=..., top_p=..., max_tokens=..., **kwargs)`.
    pub fn new(options: BedrockOptions) -> Self {
        let mut agent_opts = AgentOptions::new(&options.name);
        agent_opts.route = Some(options.route);
        agent_opts.host = options.host;
        agent_opts.port = options.port;
        agent_opts.basic_auth_user = options.basic_auth_user;
        agent_opts.basic_auth_password = options.basic_auth_password;
        agent_opts.use_pom = true;
        // Bedrock agents typically don't need auto-answer recording defaults
        // beyond what AgentBase provides; keep AgentBase defaults.

        let mut agent = AgentBase::new(agent_opts);
        if let Some(ref sp) = options.system_prompt {
            agent.set_prompt_text(sp);
        }

        let logger = Logger::new("bedrock_agent");
        logger.info(&format!(
            "BedrockAgent initialized: {} on route {}",
            agent.service().name(),
            agent.service().route()
        ));

        BedrockAgent {
            agent,
            voice_id: options.voice_id,
            temperature: options.temperature,
            top_p: options.top_p,
            max_tokens: options.max_tokens,
            logger,
        }
    }

    /// Set the Bedrock voice id (e.g. `"matthew"`, `"joanna"`).
    pub fn set_voice(&mut self, voice_id: &str) -> &mut Self {
        self.voice_id = voice_id.to_string();
        self.logger.debug(&format!("Voice set to: {}", voice_id));
        self
    }

    /// Update Bedrock inference parameters. Pass `None` to keep an
    /// existing value untouched. Mirrors the Python signature.
    pub fn set_inference_params(
        &mut self,
        temperature: Option<f64>,
        top_p: Option<f64>,
        max_tokens: Option<u32>,
    ) -> &mut Self {
        if let Some(t) = temperature {
            self.temperature = t;
        }
        if let Some(p) = top_p {
            self.top_p = p;
        }
        if let Some(m) = max_tokens {
            self.max_tokens = m;
        }
        self.logger.debug(&format!(
            "Inference params updated: temp={}, top_p={}, max_tokens={}",
            self.temperature, self.top_p, self.max_tokens
        ));
        self
    }

    /// Set LLM model — not applicable for Bedrock. Logs a warning and
    /// is a no-op (Bedrock uses a fixed voice-to-voice model). Matches
    /// Python's documented behavior.
    pub fn set_llm_model(&mut self, model: &str) -> &mut Self {
        self.logger.warn(&format!(
            "set_llm_model('{}') called but Bedrock uses a fixed voice-to-voice model",
            model
        ));
        self
    }

    /// Set LLM temperature — redirects to `set_inference_params` for
    /// Bedrock. Matches Python's documented behavior.
    pub fn set_llm_temperature(&mut self, temperature: f64) -> &mut Self {
        self.set_inference_params(Some(temperature), None, None)
    }

    /// Set post-prompt LLM params — not applicable for Bedrock. Logs a
    /// warning and is a no-op (post-prompt summarisation runs on a
    /// platform-side model). Matches Python's documented behavior.
    pub fn set_post_prompt_llm_params(&mut self, _params: Value) -> &mut Self {
        self.logger.warn(
            "set_post_prompt_llm_params() called but Bedrock post-prompt uses OpenAI configured in C code",
        );
        self
    }

    /// Set prompt LLM params — Bedrock callers should use
    /// `set_inference_params` instead. Logs a warning and is a no-op.
    /// Matches Python's documented behavior.
    pub fn set_prompt_llm_params(&mut self, _params: Value) -> &mut Self {
        self.logger
            .warn("set_prompt_llm_params() called - use set_inference_params() for Bedrock");
        self
    }

    /// Render SWML, transforming the `ai` verb into an
    /// `amazon_bedrock` verb that carries the Bedrock voice and
    /// inference parameters. Mirrors Python's `_render_swml`.
    pub fn render_swml(&self, headers: &HashMap<String, String>) -> Value {
        let mut swml = self.agent.render_swml(headers);

        // Locate the `main` section list and rewrite the first `ai`
        // verb in-place into an `amazon_bedrock` verb.
        if let Some(sections) = swml.get_mut("sections").and_then(|v| v.as_object_mut()) {
            if let Some(main) = sections.get_mut("main").and_then(|v| v.as_array_mut()) {
                for item in main.iter_mut() {
                    let Some(obj) = item.as_object_mut() else {
                        continue;
                    };
                    if !obj.contains_key("ai") {
                        continue;
                    }
                    let ai_value = obj.remove("ai").unwrap_or(Value::Null);
                    let ai_obj = match ai_value {
                        Value::Object(m) => m,
                        _ => Map::new(),
                    };

                    let bedrock_obj = self.build_bedrock_block(ai_obj);
                    obj.insert("amazon_bedrock".to_string(), Value::Object(bedrock_obj));
                    break;
                }
            }
        }

        swml
    }

    /// Build the Bedrock verb body from the AI-verb body, copying the
    /// fields that survive (prompt, SWAIG, params, global_data,
    /// post_prompt, post_prompt_url) and rewriting `prompt` so the
    /// voice configuration and inference params live inside it.
    fn build_bedrock_block(&self, ai: Map<String, Value>) -> Map<String, Value> {
        let mut out: Map<String, Value> = Map::new();

        let prompt = match ai.get("prompt") {
            Some(Value::Object(p)) => self.add_voice_to_prompt(p.clone()),
            _ => self.add_voice_to_prompt(Map::new()),
        };
        out.insert("prompt".to_string(), Value::Object(prompt));

        if let Some(swaig) = ai.get("SWAIG") {
            out.insert("SWAIG".to_string(), swaig.clone());
        } else {
            out.insert("SWAIG".to_string(), json!({}));
        }

        // Copy params explicitly (matches Python behaviour: include the
        // params object whether empty or not, mirroring Python's
        // `ai_config.get("params", {})`).
        if let Some(params) = ai.get("params") {
            out.insert("params".to_string(), params.clone());
        } else {
            out.insert("params".to_string(), json!({}));
        }

        if let Some(global_data) = ai.get("global_data") {
            out.insert("global_data".to_string(), global_data.clone());
        } else {
            out.insert("global_data".to_string(), json!({}));
        }

        if let Some(pp) = ai.get("post_prompt") {
            if !pp.is_null() {
                out.insert("post_prompt".to_string(), pp.clone());
            }
        }
        if let Some(ppu) = ai.get("post_prompt_url") {
            if !ppu.is_null() {
                out.insert("post_prompt_url".to_string(), ppu.clone());
            }
        }

        // Drop None-valued entries (mirrors Python's filter).
        out.retain(|_, v| !v.is_null());
        out
    }

    /// Inject voice id and inference params into the prompt object,
    /// stripping text-model-specific fields that don't apply to
    /// Bedrock voice-to-voice. Mirrors Python's `_add_voice_to_prompt`.
    fn add_voice_to_prompt(&self, prompt: Map<String, Value>) -> Map<String, Value> {
        let drop_keys: &[&str] = &["barge_confidence", "presence_penalty", "frequency_penalty"];
        let mut filtered: Map<String, Value> = prompt
            .into_iter()
            .filter(|(k, _)| !drop_keys.contains(&k.as_str()))
            .collect();

        filtered.insert("voice_id".to_string(), json!(self.voice_id));
        filtered.insert("temperature".to_string(), json!(self.temperature));
        filtered.insert("top_p".to_string(), json!(self.top_p));
        filtered
    }

    /// String representation matching Python's `__repr__`.
    pub fn repr(&self) -> String {
        format!(
            "BedrockAgent(name='{}', route='{}', voice='{}')",
            self.agent.service().name(),
            self.agent.service().route(),
            self.voice_id,
        )
    }

    /// Borrow the underlying `AgentBase` (read-only).
    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    /// Borrow the underlying `AgentBase` (mutable). Most consumers use
    /// `Deref`/`DerefMut` for chaining; this is occasionally useful
    /// when an explicit handle is needed.
    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    /// Current voice id.
    pub fn voice_id(&self) -> &str {
        &self.voice_id
    }

    /// Current generation temperature.
    pub fn temperature(&self) -> f64 {
        self.temperature
    }

    /// Current nucleus sampling parameter.
    pub fn top_p(&self) -> f64 {
        self.top_p
    }

    /// Current max-tokens cap.
    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }
}

impl std::ops::Deref for BedrockAgent {
    type Target = AgentBase;
    fn deref(&self) -> &AgentBase {
        &self.agent
    }
}

impl std::ops::DerefMut for BedrockAgent {
    fn deref_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }
}

impl std::fmt::Debug for BedrockAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.repr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fresh_agent() -> BedrockAgent {
        BedrockAgent::new(BedrockOptions::default())
    }

    #[test]
    fn test_construct_with_defaults() {
        let agent = fresh_agent();
        assert_eq!(agent.service().name(), "bedrock_agent");
        assert_eq!(agent.service().route(), "/bedrock");
        assert_eq!(agent.voice_id(), "matthew");
        assert!((agent.temperature() - 0.7).abs() < 1e-9);
        assert!((agent.top_p() - 0.9).abs() < 1e-9);
        assert_eq!(agent.max_tokens(), 1024);
    }

    #[test]
    fn test_construct_with_custom_options() {
        let mut opts = BedrockOptions::with_name("custom");
        opts.route = "/custom".to_string();
        opts.voice_id = "joanna".to_string();
        opts.temperature = 0.42;
        opts.top_p = 0.5;
        opts.max_tokens = 512;
        opts.system_prompt = Some("You are helpful".to_string());
        let agent = BedrockAgent::new(opts);
        assert_eq!(agent.service().name(), "custom");
        assert_eq!(agent.service().route(), "/custom");
        assert_eq!(agent.voice_id(), "joanna");
        assert_eq!(agent.max_tokens(), 512);
    }

    #[test]
    fn test_set_voice_updates_voice_id() {
        let mut agent = fresh_agent();
        agent.set_voice("joanna");
        assert_eq!(agent.voice_id(), "joanna");
    }

    #[test]
    fn test_set_inference_params_partial_update() {
        let mut agent = fresh_agent();
        agent.set_inference_params(Some(0.1), None, Some(2048));
        assert!((agent.temperature() - 0.1).abs() < 1e-9);
        // top_p should be unchanged (None passed)
        assert!((agent.top_p() - 0.9).abs() < 1e-9);
        assert_eq!(agent.max_tokens(), 2048);
    }

    #[test]
    fn test_set_llm_temperature_updates_temperature() {
        let mut agent = fresh_agent();
        agent.set_llm_temperature(0.25);
        assert!((agent.temperature() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn test_set_llm_model_is_noop() {
        let mut agent = fresh_agent();
        // No-op (logs a warning); just exercises the method.
        agent.set_llm_model("claude-3");
        // No state should have changed.
        assert_eq!(agent.voice_id(), "matthew");
    }

    #[test]
    fn test_set_post_prompt_llm_params_is_noop() {
        let mut agent = fresh_agent();
        agent.set_post_prompt_llm_params(json!({"temperature": 0.1}));
        // Bedrock ignores these — agent state unchanged.
        assert!((agent.temperature() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_set_prompt_llm_params_is_noop() {
        let mut agent = fresh_agent();
        agent.set_prompt_llm_params(json!({"temperature": 0.1}));
        assert!((agent.temperature() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_render_swml_emits_amazon_bedrock_verb() {
        let mut agent = fresh_agent();
        agent.set_prompt_text("You are a helpful Bedrock-backed assistant");
        let swml = agent.render_swml(&HashMap::new());
        let main = swml
            .get("sections")
            .and_then(|s| s.get("main"))
            .and_then(|m| m.as_array())
            .expect("main array present");

        // Find the Bedrock verb; it should not have the `ai` key.
        let bedrock_verb = main
            .iter()
            .find_map(|v| v.as_object().and_then(|o| o.get("amazon_bedrock")))
            .expect("amazon_bedrock verb present");

        let bedrock_obj = bedrock_verb.as_object().expect("verb is an object");
        assert!(bedrock_obj.contains_key("prompt"));
        assert!(bedrock_obj.contains_key("SWAIG"));

        // Ensure no item still carries an `ai` verb.
        assert!(
            main.iter()
                .all(|v| v.as_object().map(|o| !o.contains_key("ai")).unwrap_or(true)),
            "ai verb should have been replaced"
        );
    }

    #[test]
    fn test_render_swml_voice_id_in_prompt() {
        let mut agent = fresh_agent();
        agent.set_voice("joanna");
        agent.set_prompt_text("Hi");
        let swml = agent.render_swml(&HashMap::new());
        // Walk main entries to find the amazon_bedrock verb's prompt object.
        let main = swml
            .get("sections")
            .and_then(|s| s.get("main"))
            .and_then(|m| m.as_array())
            .expect("main array present");
        let prompt = main
            .iter()
            .find_map(|item| {
                item.as_object()
                    .and_then(|o| o.get("amazon_bedrock"))
                    .and_then(|b| b.get("prompt"))
                    .cloned()
            })
            .expect("amazon_bedrock prompt block");
        assert_eq!(prompt.get("voice_id"), Some(&json!("joanna")));
        assert_eq!(prompt.get("temperature"), Some(&json!(0.7)));
        assert_eq!(prompt.get("top_p"), Some(&json!(0.9)));
    }

    #[test]
    fn test_render_swml_strips_text_only_params_from_prompt() {
        let mut agent = fresh_agent();
        agent.set_prompt_text("Hi");
        // Simulate a previously-set text-model-only param landing on the prompt
        // by pushing it through prompt_llm_params (the AI verb pulls these in).
        agent.agent_mut().set_prompt_llm_params(json!({
            "barge_confidence": 0.5,
            "presence_penalty": 0.1,
            "frequency_penalty": 0.2,
            "barge_match_string": "stop",
        }));

        let swml = agent.render_swml(&HashMap::new());
        let main = swml.get("sections").unwrap().get("main").unwrap().as_array().unwrap();
        let prompt = main
            .iter()
            .find_map(|i| i.as_object().and_then(|o| o.get("amazon_bedrock")))
            .and_then(|b| b.get("prompt"))
            .and_then(|p| p.as_object())
            .expect("prompt object");
        assert!(!prompt.contains_key("barge_confidence"));
        assert!(!prompt.contains_key("presence_penalty"));
        assert!(!prompt.contains_key("frequency_penalty"));
        // Non-listed keys should survive.
        assert_eq!(prompt.get("barge_match_string"), Some(&json!("stop")));
    }

    #[test]
    fn test_repr_contains_name_route_voice() {
        let agent = fresh_agent();
        let r = agent.repr();
        assert!(r.contains("BedrockAgent"));
        assert!(r.contains("name='bedrock_agent'"));
        assert!(r.contains("route='/bedrock'"));
        assert!(r.contains("voice='matthew'"));
    }

    #[test]
    fn test_deref_to_agent_base_allows_chaining() {
        let mut agent = fresh_agent();
        // Use AgentBase methods through Deref.
        agent.set_prompt_text("Bot");
        agent.prompt_add_section("intro", "say hi", vec![]);
        // Functional: render still emits amazon_bedrock verb.
        let swml = agent.render_swml(&HashMap::new());
        assert!(swml
            .get("sections")
            .unwrap()
            .get("main")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i.as_object().map(|o| o.contains_key("amazon_bedrock")).unwrap_or(false)));
    }

    #[test]
    fn test_define_tool_via_deref_carries_into_swaig() {
        use crate::swaig::FunctionResult;
        let mut agent = fresh_agent();
        agent.define_tool(
            "lookup",
            "Look up a thing",
            json!({"q": {"type": "string"}}),
            Box::new(|_args, _raw| FunctionResult::with_response("ok")),
            false,
        );
        let swml = agent.render_swml(&HashMap::new());
        let main = swml.get("sections").unwrap().get("main").unwrap().as_array().unwrap();
        let bedrock = main
            .iter()
            .find_map(|i| i.as_object().and_then(|o| o.get("amazon_bedrock")))
            .expect("amazon_bedrock verb");
        let swaig = bedrock.get("SWAIG").and_then(|v| v.as_object()).expect("SWAIG");
        // The AgentBase build_ai_verb populates SWAIG.functions for registered tools.
        let funcs = swaig.get("functions").and_then(|v| v.as_array()).expect("functions");
        assert!(funcs.iter().any(|f| f.get("function").and_then(|n| n.as_str()) == Some("lookup")));
    }
}
