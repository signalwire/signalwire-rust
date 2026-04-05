use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{json, Map, Value};

use crate::contexts::ContextBuilder;
use crate::security::SessionManager;
use crate::swaig::FunctionResult;
use crate::swml::service::{Service, ServiceOptions};

/// Handler type for SWAIG function callbacks.
///
/// Receives `(args, raw_data)` and returns a `FunctionResult`.
pub type FunctionHandler = Box<
    dyn Fn(&Map<String, Value>, &Map<String, Value>) -> FunctionResult + Send + Sync,
>;

/// Internal tool definition (function metadata + optional handler).
#[derive(Clone)]
struct ToolDef {
    definition: Value,
    handler: Option<Arc<FunctionHandler>>,
    secure: bool,
}

/// Options for constructing an `AgentBase`.
pub struct AgentOptions {
    pub name: String,
    pub route: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
    pub auto_answer: bool,
    pub record_call: bool,
    pub use_pom: bool,
}

impl AgentOptions {
    pub fn new(name: &str) -> Self {
        AgentOptions {
            name: name.to_string(),
            route: None,
            host: None,
            port: Some(3000),
            basic_auth_user: None,
            basic_auth_password: None,
            auto_answer: true,
            record_call: false,
            use_pom: true,
        }
    }
}

/// Callback types for agent events.
type DynamicConfigCallback = Box<
    dyn Fn(&Map<String, Value>, &Option<Value>, &HashMap<String, String>, &mut AgentBase)
        + Send
        + Sync,
>;

type SummaryCallback = Box<
    dyn Fn(&str, &Value, &HashMap<String, String>) + Send + Sync,
>;

type DebugEventCallback = Box<
    dyn Fn(&Value, &HashMap<String, String>) + Send + Sync,
>;

/// Core agent that extends `Service` with AI-specific capabilities.
///
/// Manages prompt configuration, tool registration, SWML rendering,
/// and HTTP request handling for AI agent endpoints.
pub struct AgentBase {
    // ── Service (composition, not inheritance) ───────────────────────────
    service: Service,

    // ── Call handling ────────────────────────────────────────────────────
    auto_answer: bool,
    record_call: bool,
    record_format: String,
    record_stereo: bool,

    // ── Prompt / POM ────────────────────────────────────────────────────
    use_pom: bool,
    pom_sections: Vec<Value>,
    prompt_text: String,
    post_prompt: String,

    // ── Tools / SWAIG ───────────────────────────────────────────────────
    tools: HashMap<String, ToolDef>,
    tool_order: Vec<String>,

    // ── Hints ───────────────────────────────────────────────────────────
    hints: Vec<String>,
    pattern_hints: Vec<String>,

    // ── Languages / pronunciations ──────────────────────────────────────
    languages: Vec<Value>,
    pronunciations: Vec<Value>,

    // ── Params / data ───────────────────────────────────────────────────
    params: Map<String, Value>,
    global_data: Map<String, Value>,

    // ── Native functions / fillers / debug ───────────────────────────────
    native_functions: Vec<String>,
    internal_fillers: Vec<String>,
    debug_events_level: Option<String>,

    // ── LLM params ──────────────────────────────────────────────────────
    prompt_llm_params: Map<String, Value>,
    post_prompt_llm_params: Map<String, Value>,

    // ── Verbs ───────────────────────────────────────────────────────────
    pre_answer_verbs: Vec<(String, Value)>,
    post_answer_verbs: Vec<(String, Value)>,
    post_ai_verbs: Vec<(String, Value)>,
    answer_config: Map<String, Value>,

    // ── Callbacks ───────────────────────────────────────────────────────
    dynamic_config_callback: Option<Arc<DynamicConfigCallback>>,
    summary_callback: Option<Arc<SummaryCallback>>,
    debug_event_handler: Option<Arc<DebugEventCallback>>,

    // ── Web / URLs ──────────────────────────────────────────────────────
    webhook_url: Option<String>,
    post_prompt_url: Option<String>,
    swaig_query_params: HashMap<String, String>,

    // ── Function includes ───────────────────────────────────────────────
    function_includes: Vec<Value>,

    // ── Session / context / skills ──────────────────────────────────────
    session_manager: SessionManager,
    context_builder: Option<ContextBuilder>,
    skills: Vec<String>,

    // ── Proxy override ──────────────────────────────────────────────────
    manual_proxy_url: Option<String>,
}

impl Clone for AgentBase {
    fn clone(&self) -> Self {
        AgentBase {
            service: Service::new(ServiceOptions {
                name: self.service.name().to_string(),
                route: Some(self.service.route().to_string()),
                host: Some(self.service.host().to_string()),
                port: Some(self.service.port()),
                basic_auth_user: Some(self.service.basic_auth_credentials().0.to_string()),
                basic_auth_password: Some(self.service.basic_auth_credentials().1.to_string()),
            }),
            auto_answer: self.auto_answer,
            record_call: self.record_call,
            record_format: self.record_format.clone(),
            record_stereo: self.record_stereo,
            use_pom: self.use_pom,
            pom_sections: self.pom_sections.clone(),
            prompt_text: self.prompt_text.clone(),
            post_prompt: self.post_prompt.clone(),
            tools: self.tools.clone(),
            tool_order: self.tool_order.clone(),
            hints: self.hints.clone(),
            pattern_hints: self.pattern_hints.clone(),
            languages: self.languages.clone(),
            pronunciations: self.pronunciations.clone(),
            params: self.params.clone(),
            global_data: self.global_data.clone(),
            native_functions: self.native_functions.clone(),
            internal_fillers: self.internal_fillers.clone(),
            debug_events_level: self.debug_events_level.clone(),
            prompt_llm_params: self.prompt_llm_params.clone(),
            post_prompt_llm_params: self.post_prompt_llm_params.clone(),
            pre_answer_verbs: self.pre_answer_verbs.clone(),
            post_answer_verbs: self.post_answer_verbs.clone(),
            post_ai_verbs: self.post_ai_verbs.clone(),
            answer_config: self.answer_config.clone(),
            dynamic_config_callback: self.dynamic_config_callback.clone(),
            summary_callback: self.summary_callback.clone(),
            debug_event_handler: self.debug_event_handler.clone(),
            webhook_url: self.webhook_url.clone(),
            post_prompt_url: self.post_prompt_url.clone(),
            swaig_query_params: self.swaig_query_params.clone(),
            function_includes: self.function_includes.clone(),
            session_manager: self.session_manager.clone(),
            context_builder: self.context_builder.clone(),
            skills: self.skills.clone(),
            manual_proxy_url: self.manual_proxy_url.clone(),
        }
    }
}

impl AgentBase {
    pub fn new(options: AgentOptions) -> Self {
        let service = Service::new(ServiceOptions {
            name: options.name,
            route: options.route,
            host: options.host,
            port: options.port,
            basic_auth_user: options.basic_auth_user,
            basic_auth_password: options.basic_auth_password,
        });

        AgentBase {
            service,
            auto_answer: options.auto_answer,
            record_call: options.record_call,
            record_format: "wav".to_string(),
            record_stereo: false,
            use_pom: options.use_pom,
            pom_sections: Vec::new(),
            prompt_text: String::new(),
            post_prompt: String::new(),
            tools: HashMap::new(),
            tool_order: Vec::new(),
            hints: Vec::new(),
            pattern_hints: Vec::new(),
            languages: Vec::new(),
            pronunciations: Vec::new(),
            params: Map::new(),
            global_data: Map::new(),
            native_functions: Vec::new(),
            internal_fillers: Vec::new(),
            debug_events_level: None,
            prompt_llm_params: Map::new(),
            post_prompt_llm_params: Map::new(),
            pre_answer_verbs: Vec::new(),
            post_answer_verbs: Vec::new(),
            post_ai_verbs: Vec::new(),
            answer_config: Map::new(),
            dynamic_config_callback: None,
            summary_callback: None,
            debug_event_handler: None,
            webhook_url: None,
            post_prompt_url: None,
            swaig_query_params: HashMap::new(),
            function_includes: Vec::new(),
            session_manager: SessionManager::with_defaults(),
            context_builder: None,
            skills: Vec::new(),
            manual_proxy_url: None,
        }
    }

    /// Access the underlying service.
    pub fn service(&self) -> &Service {
        &self.service
    }

    /// Access the underlying service mutably.
    pub fn service_mut(&mut self) -> &mut Service {
        &mut self.service
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Prompt Methods
    // ══════════════════════════════════════════════════════════════════════

    pub fn set_prompt_text(&mut self, text: &str) -> &mut Self {
        self.prompt_text = text.to_string();
        self
    }

    pub fn set_post_prompt(&mut self, text: &str) -> &mut Self {
        self.post_prompt = text.to_string();
        self
    }

    /// Add a top-level POM section with an optional body and bullets.
    pub fn prompt_add_section(
        &mut self,
        title: &str,
        body: &str,
        bullets: Vec<&str>,
    ) -> &mut Self {
        self.use_pom = true;
        let mut section = Map::new();
        section.insert("title".to_string(), json!(title));
        section.insert("body".to_string(), json!(body));
        if !bullets.is_empty() {
            section.insert("bullets".to_string(), json!(bullets));
        }
        self.pom_sections.push(Value::Object(section));
        self
    }

    /// Add a subsection nested under an existing parent section.
    pub fn prompt_add_subsection(
        &mut self,
        parent_title: &str,
        title: &str,
        body: &str,
    ) -> &mut Self {
        for section in &mut self.pom_sections {
            if let Value::Object(map) = section {
                if map.get("title").and_then(|t| t.as_str()) == Some(parent_title) {
                    let subsections = map
                        .entry("subsections".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(arr) = subsections {
                        arr.push(json!({"title": title, "body": body}));
                    }
                    break;
                }
            }
        }
        self
    }

    /// Append body text and/or bullets to an existing section.
    pub fn prompt_add_to_section(
        &mut self,
        title: &str,
        body: Option<&str>,
        bullets: Vec<&str>,
    ) -> &mut Self {
        for section in &mut self.pom_sections {
            if let Value::Object(map) = section {
                if map.get("title").and_then(|t| t.as_str()) == Some(title) {
                    if let Some(b) = body {
                        let existing = map
                            .get("body")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        map.insert("body".to_string(), json!(format!("{}{}", existing, b)));
                    }
                    if !bullets.is_empty() {
                        let existing_bullets = map
                            .entry("bullets".to_string())
                            .or_insert_with(|| Value::Array(Vec::new()));
                        if let Value::Array(arr) = existing_bullets {
                            for bullet in bullets {
                                arr.push(json!(bullet));
                            }
                        }
                    }
                    break;
                }
            }
        }
        self
    }

    /// Check whether a POM section with the given title exists.
    pub fn prompt_has_section(&self, title: &str) -> bool {
        self.pom_sections.iter().any(|s| {
            s.as_object()
                .and_then(|m| m.get("title"))
                .and_then(|t| t.as_str())
                == Some(title)
        })
    }

    /// Return the prompt payload: POM array if enabled and populated, otherwise raw text.
    pub fn get_prompt(&self) -> Value {
        if self.use_pom && !self.pom_sections.is_empty() {
            Value::Array(self.pom_sections.clone())
        } else {
            json!(self.prompt_text)
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Tool Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Define a tool with a callable handler.
    pub fn define_tool(
        &mut self,
        name: &str,
        description: &str,
        parameters: Value,
        handler: Box<
            dyn Fn(&Map<String, Value>, &Map<String, Value>) -> FunctionResult + Send + Sync,
        >,
        secure: bool,
    ) -> &mut Self {
        let mut definition = Map::new();
        definition.insert("function".to_string(), json!(name));
        definition.insert("purpose".to_string(), json!(description));
        definition.insert(
            "argument".to_string(),
            json!({"type": "object", "properties": parameters}),
        );

        self.tools.insert(
            name.to_string(),
            ToolDef {
                definition: Value::Object(definition),
                handler: Some(Arc::new(handler)),
                secure,
            },
        );

        if !self.tool_order.contains(&name.to_string()) {
            self.tool_order.push(name.to_string());
        }
        self
    }

    /// Register a raw SWAIG function definition (e.g. from DataMap).
    pub fn register_swaig_function(&mut self, func_def: Value) -> &mut Self {
        let name = func_def["function"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            return self;
        }
        self.tools.insert(
            name.clone(),
            ToolDef {
                definition: func_def,
                handler: None,
                secure: false,
            },
        );
        if !self.tool_order.contains(&name) {
            self.tool_order.push(name);
        }
        self
    }

    /// Register multiple tool definitions at once.
    pub fn define_tools(&mut self, tool_defs: Vec<Value>) -> &mut Self {
        for def in tool_defs {
            self.register_swaig_function(def);
        }
        self
    }

    /// Dispatch a function call to the registered handler.
    pub fn on_function_call(
        &self,
        name: &str,
        args: &Map<String, Value>,
        raw_data: &Map<String, Value>,
    ) -> Option<FunctionResult> {
        let tool = self.tools.get(name)?;
        let handler = tool.handler.as_ref()?;
        Some(handler(args, raw_data))
    }

    // ══════════════════════════════════════════════════════════════════════
    //  AI Config Methods
    // ══════════════════════════════════════════════════════════════════════

    pub fn add_hint(&mut self, hint: &str) -> &mut Self {
        self.hints.push(hint.to_string());
        self
    }

    pub fn add_hints(&mut self, hints: Vec<&str>) -> &mut Self {
        for h in hints {
            self.hints.push(h.to_string());
        }
        self
    }

    pub fn add_pattern_hint(&mut self, pattern: &str) -> &mut Self {
        self.pattern_hints.push(pattern.to_string());
        self
    }

    pub fn add_language(&mut self, name: &str, code: &str, voice: &str) -> &mut Self {
        self.languages
            .push(json!({"name": name, "code": code, "voice": voice}));
        self
    }

    pub fn set_languages(&mut self, languages: Vec<Value>) -> &mut Self {
        self.languages = languages;
        self
    }

    pub fn add_pronunciation(
        &mut self,
        replace: &str,
        with: &str,
        ignore: &str,
    ) -> &mut Self {
        let mut entry = Map::new();
        entry.insert("replace".to_string(), json!(replace));
        entry.insert("with".to_string(), json!(with));
        if !ignore.is_empty() {
            entry.insert("ignore".to_string(), json!(ignore));
        }
        self.pronunciations.push(Value::Object(entry));
        self
    }

    pub fn set_pronunciations(&mut self, pronunciations: Vec<Value>) -> &mut Self {
        self.pronunciations = pronunciations;
        self
    }

    pub fn set_param(&mut self, key: &str, value: Value) -> &mut Self {
        self.params.insert(key.to_string(), value);
        self
    }

    pub fn set_params(&mut self, params: Value) -> &mut Self {
        if let Value::Object(map) = params {
            self.params = map;
        }
        self
    }

    pub fn set_global_data(&mut self, data: Value) -> &mut Self {
        if let Value::Object(map) = data {
            self.global_data = map;
        }
        self
    }

    pub fn update_global_data(&mut self, data: Value) -> &mut Self {
        if let Value::Object(map) = data {
            for (k, v) in map {
                self.global_data.insert(k, v);
            }
        }
        self
    }

    pub fn set_native_functions(&mut self, functions: Vec<&str>) -> &mut Self {
        self.native_functions = functions.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn set_internal_fillers(&mut self, fillers: Vec<&str>) -> &mut Self {
        self.internal_fillers = fillers.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn add_internal_filler(&mut self, filler: &str) -> &mut Self {
        self.internal_fillers.push(filler.to_string());
        self
    }

    pub fn enable_debug_events(&mut self, level: &str) -> &mut Self {
        self.debug_events_level = Some(level.to_string());
        self
    }

    pub fn add_function_include(&mut self, include: Value) -> &mut Self {
        self.function_includes.push(include);
        self
    }

    pub fn set_function_includes(&mut self, includes: Vec<Value>) -> &mut Self {
        self.function_includes = includes;
        self
    }

    pub fn set_prompt_llm_params(&mut self, params: Value) -> &mut Self {
        if let Value::Object(map) = params {
            self.prompt_llm_params = map;
        }
        self
    }

    pub fn set_post_prompt_llm_params(&mut self, params: Value) -> &mut Self {
        if let Value::Object(map) = params {
            self.post_prompt_llm_params = map;
        }
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Verb Methods
    // ══════════════════════════════════════════════════════════════════════

    pub fn add_pre_answer_verb(&mut self, verb: &str, config: Value) -> &mut Self {
        self.pre_answer_verbs.push((verb.to_string(), config));
        self
    }

    pub fn add_post_answer_verb(&mut self, verb: &str, config: Value) -> &mut Self {
        self.post_answer_verbs.push((verb.to_string(), config));
        self
    }

    pub fn add_post_ai_verb(&mut self, verb: &str, config: Value) -> &mut Self {
        self.post_ai_verbs.push((verb.to_string(), config));
        self
    }

    pub fn clear_pre_answer_verbs(&mut self) -> &mut Self {
        self.pre_answer_verbs.clear();
        self
    }

    pub fn clear_post_answer_verbs(&mut self) -> &mut Self {
        self.post_answer_verbs.clear();
        self
    }

    pub fn clear_post_ai_verbs(&mut self) -> &mut Self {
        self.post_ai_verbs.clear();
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Context Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Return the ContextBuilder, creating it lazily on first access.
    pub fn define_contexts(&mut self) -> &mut ContextBuilder {
        if self.context_builder.is_none() {
            self.context_builder = Some(ContextBuilder::new());
        }
        self.context_builder.as_mut().unwrap()
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Skill Methods (stubs)
    // ══════════════════════════════════════════════════════════════════════

    pub fn add_skill(&mut self, name: &str, _params: Value) -> &mut Self {
        if !self.skills.contains(&name.to_string()) {
            self.skills.push(name.to_string());
        }
        self
    }

    pub fn remove_skill(&mut self, name: &str) -> &mut Self {
        self.skills.retain(|s| s != name);
        self
    }

    pub fn list_skills(&self) -> Vec<String> {
        self.skills.clone()
    }

    pub fn has_skill(&self, name: &str) -> bool {
        self.skills.contains(&name.to_string())
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Web / Callback Methods
    // ══════════════════════════════════════════════════════════════════════

    pub fn set_dynamic_config_callback(
        &mut self,
        callback: Box<
            dyn Fn(&Map<String, Value>, &Option<Value>, &HashMap<String, String>, &mut AgentBase)
                + Send
                + Sync,
        >,
    ) -> &mut Self {
        self.dynamic_config_callback = Some(Arc::new(callback));
        self
    }

    pub fn set_webhook_url(&mut self, url: &str) -> &mut Self {
        self.webhook_url = Some(url.to_string());
        self
    }

    pub fn set_post_prompt_url(&mut self, url: &str) -> &mut Self {
        self.post_prompt_url = Some(url.to_string());
        self
    }

    pub fn manual_set_proxy_url(&mut self, url: &str) -> &mut Self {
        self.manual_proxy_url = Some(url.trim_end_matches('/').to_string());
        self
    }

    pub fn add_swaig_query_params(&mut self, params: HashMap<String, String>) -> &mut Self {
        for (k, v) in params {
            self.swaig_query_params.insert(k, v);
        }
        self
    }

    pub fn clear_swaig_query_params(&mut self) -> &mut Self {
        self.swaig_query_params.clear();
        self
    }

    pub fn on_summary(
        &mut self,
        callback: Box<
            dyn Fn(&str, &Value, &HashMap<String, String>) + Send + Sync,
        >,
    ) -> &mut Self {
        self.summary_callback = Some(Arc::new(callback));
        self
    }

    pub fn on_debug_event(
        &mut self,
        callback: Box<
            dyn Fn(&Value, &HashMap<String, String>) + Send + Sync,
        >,
    ) -> &mut Self {
        self.debug_event_handler = Some(Arc::new(callback));
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  SIP Methods
    // ══════════════════════════════════════════════════════════════════════

    pub fn enable_sip_routing(&mut self) -> &mut Self {
        self.set_param("sip_routing", json!(true));
        self
    }

    pub fn register_sip_username(&mut self, username: &str, route: &str) -> &mut Self {
        self.set_param("sip_username", json!(username));
        if !route.is_empty() {
            self.set_param("sip_route", json!(route));
        }
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  SWML Rendering
    // ══════════════════════════════════════════════════════════════════════

    /// Build the complete SWML document for a request.
    ///
    /// Phases:
    ///   1. Pre-answer verbs
    ///   2. Answer verb (if auto_answer)
    ///   3. Record call verb (if record_call)
    ///   4. Post-answer verbs
    ///   5. AI verb (via build_ai_verb)
    ///   6. Post-AI verbs
    pub fn render_swml(&self, headers: &HashMap<String, String>) -> Value {
        let mut main = Vec::new();

        // Phase 1: Pre-answer verbs
        for (verb, config) in &self.pre_answer_verbs {
            main.push(json!({verb: config}));
        }

        // Phase 2: Answer verb
        if self.auto_answer {
            let mut answer_params = Map::new();
            answer_params.insert("max_duration".to_string(), json!(14400));
            for (k, v) in &self.answer_config {
                answer_params.insert(k.clone(), v.clone());
            }
            main.push(json!({"answer": Value::Object(answer_params)}));
        }

        // Phase 3: Record call verb
        if self.record_call {
            main.push(json!({
                "record_call": {
                    "format": self.record_format,
                    "stereo": self.record_stereo,
                }
            }));
        }

        // Phase 4: Post-answer verbs
        for (verb, config) in &self.post_answer_verbs {
            main.push(json!({verb: config}));
        }

        // Phase 5: AI verb
        main.push(json!({"ai": self.build_ai_verb(headers)}));

        // Phase 6: Post-AI verbs
        for (verb, config) in &self.post_ai_verbs {
            main.push(json!({verb: config}));
        }

        json!({
            "version": "1.0.0",
            "sections": {
                "main": main,
            }
        })
    }

    /// Build the AI verb configuration block.
    pub fn build_ai_verb(&self, headers: &HashMap<String, String>) -> Value {
        let mut ai = Map::new();

        // ── Prompt ──────────────────────────────────────────────────────
        let mut prompt = Map::new();
        if self.use_pom && !self.pom_sections.is_empty() {
            prompt.insert("pom".to_string(), Value::Array(self.pom_sections.clone()));
        } else {
            prompt.insert("text".to_string(), json!(self.prompt_text));
        }
        for (k, v) in &self.prompt_llm_params {
            prompt.insert(k.clone(), v.clone());
        }
        ai.insert("prompt".to_string(), Value::Object(prompt));

        // ── Post prompt ─────────────────────────────────────────────────
        if !self.post_prompt.is_empty() {
            let mut pp_block = Map::new();
            pp_block.insert("text".to_string(), json!(self.post_prompt));
            for (k, v) in &self.post_prompt_llm_params {
                pp_block.insert(k.clone(), v.clone());
            }
            ai.insert("post_prompt".to_string(), Value::Object(pp_block));
        }

        // ── Post prompt URL ─────────────────────────────────────────────
        if let Some(ref ppu) = self.post_prompt_url {
            ai.insert("post_prompt_url".to_string(), json!(ppu));
        } else {
            let proxy_base = self.resolve_proxy_base(headers);
            let route_segment = if self.service.route() == "/" {
                "".to_string()
            } else {
                self.service.route().to_string()
            };
            ai.insert(
                "post_prompt_url".to_string(),
                json!(format!("{}{}/post_prompt", proxy_base, route_segment)),
            );
        }

        // ── Params ──────────────────────────────────────────────────────
        let mut merged_params = self.params.clone();
        if !self.internal_fillers.is_empty() {
            merged_params.insert(
                "internal_fillers".to_string(),
                json!(self.internal_fillers),
            );
        }
        if let Some(ref level) = self.debug_events_level {
            merged_params.insert("debug_events".to_string(), json!(level));
        }
        if !merged_params.is_empty() {
            ai.insert("params".to_string(), Value::Object(merged_params));
        }

        // ── Hints ───────────────────────────────────────────────────────
        let mut all_hints: Vec<Value> = self.hints.iter().map(|h| json!(h)).collect();
        for ph in &self.pattern_hints {
            all_hints.push(json!(ph));
        }
        if !all_hints.is_empty() {
            ai.insert("hints".to_string(), Value::Array(all_hints));
        }

        // ── Languages ───────────────────────────────────────────────────
        if !self.languages.is_empty() {
            ai.insert("languages".to_string(), Value::Array(self.languages.clone()));
        }

        // ── Pronunciations ──────────────────────────────────────────────
        if !self.pronunciations.is_empty() {
            ai.insert(
                "pronounce".to_string(),
                Value::Array(self.pronunciations.clone()),
            );
        }

        // ── SWAIG ──────────────────────────────────────────────────────
        let swaig = self.build_swaig_block(headers);
        if !swaig.is_empty() {
            ai.insert("SWAIG".to_string(), Value::Object(swaig));
        }

        // ── Global data ─────────────────────────────────────────────────
        if !self.global_data.is_empty() {
            ai.insert(
                "global_data".to_string(),
                Value::Object(self.global_data.clone()),
            );
        }

        // ── Context switch ──────────────────────────────────────────────
        if let Some(ref cb) = self.context_builder {
            if cb.has_contexts() {
                let ctx_val = cb.to_value();
                ai.insert("context_switch".to_string(), ctx_val);
            }
        }

        Value::Object(ai)
    }

    // ══════════════════════════════════════════════════════════════════════
    //  HTTP Handling
    // ══════════════════════════════════════════════════════════════════════

    /// Handle an HTTP request. Overrides the service handler with agent-specific
    /// logic for SWML, SWAIG dispatch, and post-prompt callbacks.
    pub fn handle_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (u16, HashMap<String, String>, String) {
        // Health/ready: delegate to service
        if path == "/health" || path == "/ready" {
            return self.service.handle_request(method, path, headers, body);
        }

        // Determine sub-path relative to route
        let route = self.service.route();
        let sub_path = if route == "/" {
            Some(path.to_string())
        } else if path == route || path.starts_with(&format!("{}/", route)) {
            let rest = &path[route.len()..];
            if rest.is_empty() {
                Some("/".to_string())
            } else {
                Some(rest.to_string())
            }
        } else {
            None
        };

        let sub_path = match sub_path {
            Some(p) => p,
            None => return json_response(404, &json!({"error": "Not found"})),
        };

        // Auth
        if !self.check_auth(headers) {
            let mut resp_headers = HashMap::new();
            resp_headers.insert("Content-Type".to_string(), "text/plain".to_string());
            resp_headers.insert(
                "WWW-Authenticate".to_string(),
                "Basic realm=\"SignalWire Agent\"".to_string(),
            );
            return (401, resp_headers, "Unauthorized".to_string());
        }

        // Parse body
        let request_data: Option<Value> = if !body.is_empty() {
            serde_json::from_str(body).ok()
        } else {
            None
        };

        match sub_path.as_str() {
            "/" | "" => self.handle_swml_request(method, &request_data, headers),
            "/swaig" => self.handle_swaig_request(&request_data, headers),
            "/post_prompt" => self.handle_post_prompt(&request_data, headers),
            _ => json_response(404, &json!({"error": "Not found"})),
        }
    }

    /// Create a deep copy of this agent for per-request customisation.
    pub fn clone_for_request(&self) -> Self {
        self.clone()
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Private Helpers
    // ══════════════════════════════════════════════════════════════════════

    fn check_auth(&self, headers: &HashMap<String, String>) -> bool {
        // Delegate to service's handle_request for auth check by
        // using the service's basic_auth_credentials to validate
        let auth_header = headers
            .get("Authorization")
            .or_else(|| headers.get("authorization"));

        let auth_header = match auth_header {
            Some(h) => h,
            None => return false,
        };

        if !auth_header.starts_with("Basic ") {
            return false;
        }

        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;

        let decoded = match BASE64.decode(&auth_header[6..]) {
            Ok(d) => d,
            Err(_) => return false,
        };
        let decoded_str = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let colon_pos = match decoded_str.find(':') {
            Some(p) => p,
            None => return false,
        };
        let input_user = &decoded_str[..colon_pos];
        let input_pass = &decoded_str[colon_pos + 1..];

        let (expected_user, expected_pass) = self.service.basic_auth_credentials();
        input_user == expected_user && input_pass == expected_pass
    }

    fn handle_swml_request(
        &self,
        _method: &str,
        request_data: &Option<Value>,
        headers: &HashMap<String, String>,
    ) -> (u16, HashMap<String, String>, String) {
        if self.dynamic_config_callback.is_some() {
            let mut clone = self.clone_for_request();
            let query_params = request_data
                .as_ref()
                .and_then(|d| d.get("query_params"))
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            if let Some(ref cb) = self.dynamic_config_callback {
                cb(&query_params, request_data, headers, &mut clone);
            }

            let swml = clone.render_swml(headers);
            return json_response(200, &swml);
        }

        let swml = self.render_swml(headers);
        json_response(200, &swml)
    }

    fn handle_swaig_request(
        &self,
        request_data: &Option<Value>,
        _headers: &HashMap<String, String>,
    ) -> (u16, HashMap<String, String>, String) {
        let data = match request_data {
            Some(d) => d,
            None => return json_response(400, &json!({"error": "Missing request body"})),
        };

        let function_name = data["function"].as_str().unwrap_or("");
        if function_name.is_empty() {
            return json_response(400, &json!({"error": "Missing function name"}));
        }

        let args = data["argument"]["parsed"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let raw_data = data.as_object().cloned().unwrap_or_default();

        match self.on_function_call(function_name, &args, &raw_data) {
            Some(result) => json_response(200, &result.to_value()),
            None => {
                json_response(404, &json!({"error": format!("Unknown function: {}", function_name)}))
            }
        }
    }

    fn handle_post_prompt(
        &self,
        request_data: &Option<Value>,
        headers: &HashMap<String, String>,
    ) -> (u16, HashMap<String, String>, String) {
        if let (Some(cb), Some(data)) = (&self.summary_callback, request_data) {
            let summary = data["post_prompt_data"]["raw"]
                .as_str()
                .or_else(|| data["summary"].as_str())
                .unwrap_or("");
            cb(summary, data, headers);
        }
        json_response(200, &json!({"status": "ok"}))
    }

    fn build_swaig_block(&self, headers: &HashMap<String, String>) -> Map<String, Value> {
        let mut swaig = Map::new();

        let mut functions = Vec::new();
        for name in &self.tool_order {
            if let Some(tool) = self.tools.get(name) {
                let mut func_def = tool.definition.clone();

                // Add web_hook_url for tools with handlers
                if tool.handler.is_some() {
                    if let Some(ref wh_url) = self.webhook_url {
                        if let Value::Object(map) = &mut func_def {
                            map.insert("web_hook_url".to_string(), json!(wh_url));
                        }
                    } else {
                        let url = self.build_swaig_webhook_url(headers);
                        if let Value::Object(map) = &mut func_def {
                            map.insert("web_hook_url".to_string(), json!(url));
                        }
                    }
                }

                functions.push(func_def);
            }
        }

        if !functions.is_empty() {
            swaig.insert("functions".to_string(), Value::Array(functions));
        }

        if !self.native_functions.is_empty() {
            swaig.insert("native_functions".to_string(), json!(self.native_functions));
        }

        if !self.function_includes.is_empty() {
            swaig.insert(
                "includes".to_string(),
                Value::Array(self.function_includes.clone()),
            );
        }

        swaig
    }

    fn build_swaig_webhook_url(&self, headers: &HashMap<String, String>) -> String {
        let proxy_base = self.resolve_proxy_base(headers);
        let route_segment = if self.service.route() == "/" {
            "".to_string()
        } else {
            self.service.route().to_string()
        };

        let (user, pass) = self.service.basic_auth_credentials();

        // Parse proxy_base to extract host/port
        let mut auth_url = if proxy_base.starts_with("http://") || proxy_base.starts_with("https://") {
            let proto_end = proxy_base.find("://").unwrap() + 3;
            let proto = &proxy_base[..proto_end];
            let rest = &proxy_base[proto_end..];
            format!("{}{}:{}@{}{}/swaig", proto, user, pass, rest, route_segment)
        } else {
            format!("http://{}:{}@{}{}/swaig", user, pass, proxy_base, route_segment)
        };

        if !self.swaig_query_params.is_empty() {
            let params: Vec<String> = self
                .swaig_query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            auth_url = format!("{}?{}", auth_url, params.join("&"));
        }

        auth_url
    }

    fn resolve_proxy_base(&self, headers: &HashMap<String, String>) -> String {
        if let Some(ref manual) = self.manual_proxy_url {
            return manual.clone();
        }
        self.service.get_proxy_url_base(headers)
    }
}

/// Build a JSON HTTP response tuple.
fn json_response(status: u16, data: &Value) -> (u16, HashMap<String, String>, String) {
    let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert(
        "X-Content-Type-Options".to_string(),
        "nosniff".to_string(),
    );
    headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
    headers.insert("Cache-Control".to_string(), "no-store".to_string());
    (status, headers, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> AgentOptions {
        let mut opts = AgentOptions::new("test-agent");
        opts.basic_auth_user = Some("user".to_string());
        opts.basic_auth_password = Some("pass".to_string());
        opts.port = Some(3000);
        opts
    }

    fn authed_headers() -> HashMap<String, String> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;
        let mut h = HashMap::new();
        h.insert(
            "Authorization".to_string(),
            format!("Basic {}", BASE64.encode("user:pass")),
        );
        h
    }

    // ── Construction ─────────────────────────────────────────────────────

    #[test]
    fn test_construction() {
        let agent = AgentBase::new(default_options());
        assert_eq!(agent.service().name(), "test-agent");
        assert!(agent.auto_answer);
        assert!(!agent.record_call);
        assert!(agent.use_pom);
    }

    #[test]
    fn test_construction_custom() {
        let mut opts = default_options();
        opts.auto_answer = false;
        opts.record_call = true;
        opts.use_pom = false;
        let agent = AgentBase::new(opts);
        assert!(!agent.auto_answer);
        assert!(agent.record_call);
        assert!(!agent.use_pom);
    }

    // ── Prompt ───────────────────────────────────────────────────────────

    #[test]
    fn test_set_prompt_text() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_text("You are a helpful assistant");
        assert_eq!(agent.get_prompt(), json!("You are a helpful assistant"));
    }

    #[test]
    fn test_pom_sections() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Role", "You are an agent", vec![]);
        let prompt = agent.get_prompt();
        assert!(prompt.is_array());
        assert_eq!(prompt[0]["title"], "Role");
    }

    #[test]
    fn test_pom_with_bullets() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Rules", "Follow these rules:", vec!["Be polite", "Be helpful"]);
        let prompt = agent.get_prompt();
        let bullets = prompt[0]["bullets"].as_array().unwrap();
        assert_eq!(bullets.len(), 2);
    }

    #[test]
    fn test_prompt_add_subsection() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Role", "Main role", vec![]);
        agent.prompt_add_subsection("Role", "Details", "Extra detail");
        let prompt = agent.get_prompt();
        let subs = prompt[0]["subsections"].as_array().unwrap();
        assert_eq!(subs[0]["title"], "Details");
    }

    #[test]
    fn test_prompt_add_to_section() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Info", "Initial", vec![]);
        agent.prompt_add_to_section("Info", Some(" added"), vec!["bullet1"]);
        let prompt = agent.get_prompt();
        assert_eq!(prompt[0]["body"], "Initial added");
        assert_eq!(prompt[0]["bullets"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_prompt_has_section() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Role", "body", vec![]);
        assert!(agent.prompt_has_section("Role"));
        assert!(!agent.prompt_has_section("Missing"));
    }

    #[test]
    fn test_set_post_prompt() {
        let mut agent = AgentBase::new(default_options());
        agent.set_post_prompt("Summarise the call");
        assert_eq!(agent.post_prompt, "Summarise the call");
    }

    #[test]
    fn test_prompt_text_when_no_pom() {
        let mut opts = default_options();
        opts.use_pom = false;
        let mut agent = AgentBase::new(opts);
        agent.set_prompt_text("Plain text prompt");
        assert_eq!(agent.get_prompt(), json!("Plain text prompt"));
    }

    // ── Tool Registration ────────────────────────────────────────────────

    #[test]
    fn test_define_tool() {
        let mut agent = AgentBase::new(default_options());
        agent.define_tool(
            "lookup",
            "Look up data",
            json!({"query": {"type": "string"}}),
            Box::new(|_args, _raw| FunctionResult::with_response("found it")),
            false,
        );
        assert!(agent.tools.contains_key("lookup"));
        assert_eq!(agent.tool_order, vec!["lookup"]);
    }

    #[test]
    fn test_define_tool_dispatch() {
        let mut agent = AgentBase::new(default_options());
        agent.define_tool(
            "greet",
            "Greet user",
            json!({}),
            Box::new(|args, _raw| {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
                FunctionResult::with_response(&format!("Hello, {}!", name))
            }),
            false,
        );

        let mut args = Map::new();
        args.insert("name".to_string(), json!("Alice"));
        let raw = Map::new();
        let result = agent.on_function_call("greet", &args, &raw).unwrap();
        assert_eq!(result.to_value()["response"], "Hello, Alice!");
    }

    #[test]
    fn test_on_function_call_unknown() {
        let agent = AgentBase::new(default_options());
        let args = Map::new();
        let raw = Map::new();
        assert!(agent.on_function_call("nonexistent", &args, &raw).is_none());
    }

    #[test]
    fn test_register_swaig_function() {
        let mut agent = AgentBase::new(default_options());
        agent.register_swaig_function(json!({
            "function": "datamap_func",
            "purpose": "data lookup",
            "data_map": {"expressions": []}
        }));
        assert!(agent.tools.contains_key("datamap_func"));
    }

    #[test]
    fn test_register_swaig_function_empty_name() {
        let mut agent = AgentBase::new(default_options());
        agent.register_swaig_function(json!({"purpose": "no name"}));
        assert!(agent.tools.is_empty());
    }

    #[test]
    fn test_define_tools() {
        let mut agent = AgentBase::new(default_options());
        agent.define_tools(vec![
            json!({"function": "f1", "purpose": "p1"}),
            json!({"function": "f2", "purpose": "p2"}),
        ]);
        assert_eq!(agent.tools.len(), 2);
        assert_eq!(agent.tool_order, vec!["f1", "f2"]);
    }

    // ── AI Config ────────────────────────────────────────────────────────

    #[test]
    fn test_add_hints() {
        let mut agent = AgentBase::new(default_options());
        agent.add_hint("SignalWire");
        agent.add_hints(vec!["SWAIG", "AI"]);
        assert_eq!(agent.hints.len(), 3);
    }

    #[test]
    fn test_add_pattern_hint() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pattern_hint("[A-Z]{3}");
        assert_eq!(agent.pattern_hints.len(), 1);
    }

    #[test]
    fn test_add_language() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "Polly.Salli");
        assert_eq!(agent.languages[0]["name"], "English");
    }

    #[test]
    fn test_set_languages() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "Polly.Salli");
        agent.set_languages(vec![]);
        assert!(agent.languages.is_empty());
    }

    #[test]
    fn test_add_pronunciation() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pronunciation("SignalWire", "signal wire", "");
        assert_eq!(agent.pronunciations[0]["replace"], "SignalWire");
        assert!(agent.pronunciations[0].get("ignore").is_none());
    }

    #[test]
    fn test_add_pronunciation_with_ignore() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pronunciation("AI", "A.I.", "context");
        assert_eq!(agent.pronunciations[0]["ignore"], "context");
    }

    #[test]
    fn test_set_pronunciations() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pronunciation("a", "b", "");
        agent.set_pronunciations(vec![]);
        assert!(agent.pronunciations.is_empty());
    }

    #[test]
    fn test_set_param() {
        let mut agent = AgentBase::new(default_options());
        agent.set_param("temperature", json!(0.7));
        assert_eq!(agent.params["temperature"], 0.7);
    }

    #[test]
    fn test_set_params() {
        let mut agent = AgentBase::new(default_options());
        agent.set_params(json!({"a": 1, "b": 2}));
        assert_eq!(agent.params.len(), 2);
    }

    #[test]
    fn test_set_global_data() {
        let mut agent = AgentBase::new(default_options());
        agent.set_global_data(json!({"key": "value"}));
        assert_eq!(agent.global_data["key"], "value");
    }

    #[test]
    fn test_update_global_data() {
        let mut agent = AgentBase::new(default_options());
        agent.set_global_data(json!({"a": 1}));
        agent.update_global_data(json!({"b": 2}));
        assert_eq!(agent.global_data.len(), 2);
    }

    #[test]
    fn test_set_native_functions() {
        let mut agent = AgentBase::new(default_options());
        agent.set_native_functions(vec!["check_voicemail", "send_digits"]);
        assert_eq!(agent.native_functions.len(), 2);
    }

    #[test]
    fn test_internal_fillers() {
        let mut agent = AgentBase::new(default_options());
        agent.set_internal_fillers(vec!["one moment"]);
        agent.add_internal_filler("please hold");
        assert_eq!(agent.internal_fillers.len(), 2);
    }

    #[test]
    fn test_enable_debug_events() {
        let mut agent = AgentBase::new(default_options());
        agent.enable_debug_events("all");
        assert_eq!(agent.debug_events_level, Some("all".to_string()));
    }

    #[test]
    fn test_function_includes() {
        let mut agent = AgentBase::new(default_options());
        agent.add_function_include(json!({"url": "https://example.com/funcs"}));
        agent.add_function_include(json!({"url": "https://example.com/more"}));
        assert_eq!(agent.function_includes.len(), 2);
    }

    #[test]
    fn test_set_function_includes() {
        let mut agent = AgentBase::new(default_options());
        agent.add_function_include(json!({"url": "a"}));
        agent.set_function_includes(vec![]);
        assert!(agent.function_includes.is_empty());
    }

    #[test]
    fn test_set_prompt_llm_params() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_llm_params(json!({"temperature": 0.5}));
        assert_eq!(agent.prompt_llm_params["temperature"], 0.5);
    }

    #[test]
    fn test_set_post_prompt_llm_params() {
        let mut agent = AgentBase::new(default_options());
        agent.set_post_prompt_llm_params(json!({"top_p": 0.9}));
        assert_eq!(agent.post_prompt_llm_params["top_p"], 0.9);
    }

    // ── Verbs ────────────────────────────────────────────────────────────

    #[test]
    fn test_pre_answer_verbs() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pre_answer_verb("play", json!({"url": "ring.mp3"}));
        assert_eq!(agent.pre_answer_verbs.len(), 1);
    }

    #[test]
    fn test_post_answer_verbs() {
        let mut agent = AgentBase::new(default_options());
        agent.add_post_answer_verb("sleep", json!(1000));
        assert_eq!(agent.post_answer_verbs.len(), 1);
    }

    #[test]
    fn test_post_ai_verbs() {
        let mut agent = AgentBase::new(default_options());
        agent.add_post_ai_verb("hangup", json!({}));
        assert_eq!(agent.post_ai_verbs.len(), 1);
    }

    #[test]
    fn test_clear_verbs() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pre_answer_verb("play", json!({}));
        agent.add_post_answer_verb("sleep", json!(1));
        agent.add_post_ai_verb("hangup", json!({}));
        agent.clear_pre_answer_verbs();
        agent.clear_post_answer_verbs();
        agent.clear_post_ai_verbs();
        assert!(agent.pre_answer_verbs.is_empty());
        assert!(agent.post_answer_verbs.is_empty());
        assert!(agent.post_ai_verbs.is_empty());
    }

    // ── Context ──────────────────────────────────────────────────────────

    #[test]
    fn test_define_contexts() {
        let mut agent = AgentBase::new(default_options());
        agent
            .define_contexts()
            .add_context("default")
            .add_step("intro")
            .set_text("Hello");
        assert!(agent.context_builder.is_some());
    }

    #[test]
    fn test_define_contexts_lazy() {
        let mut agent = AgentBase::new(default_options());
        assert!(agent.context_builder.is_none());
        agent.define_contexts();
        assert!(agent.context_builder.is_some());
        // Second call returns same builder
        agent.define_contexts();
        assert!(agent.context_builder.is_some());
    }

    // ── Skills (stubs) ───────────────────────────────────────────────────

    #[test]
    fn test_skills() {
        let mut agent = AgentBase::new(default_options());
        assert!(!agent.has_skill("weather"));
        assert!(agent.list_skills().is_empty());

        agent.add_skill("weather", json!({}));
        assert!(agent.has_skill("weather"));
        assert_eq!(agent.list_skills(), vec!["weather"]);

        agent.remove_skill("weather");
        assert!(!agent.has_skill("weather"));
    }

    #[test]
    fn test_add_skill_idempotent() {
        let mut agent = AgentBase::new(default_options());
        agent.add_skill("s1", json!({}));
        agent.add_skill("s1", json!({}));
        assert_eq!(agent.list_skills().len(), 1);
    }

    // ── Web / Callbacks ──────────────────────────────────────────────────

    #[test]
    fn test_set_webhook_url() {
        let mut agent = AgentBase::new(default_options());
        agent.set_webhook_url("https://webhook.example.com/swaig");
        assert_eq!(agent.webhook_url, Some("https://webhook.example.com/swaig".to_string()));
    }

    #[test]
    fn test_set_post_prompt_url() {
        let mut agent = AgentBase::new(default_options());
        agent.set_post_prompt_url("https://example.com/post_prompt");
        assert_eq!(agent.post_prompt_url, Some("https://example.com/post_prompt".to_string()));
    }

    #[test]
    fn test_manual_proxy_url() {
        let mut agent = AgentBase::new(default_options());
        agent.manual_set_proxy_url("https://proxy.example.com/");
        assert_eq!(agent.manual_proxy_url, Some("https://proxy.example.com".to_string()));
    }

    #[test]
    fn test_swaig_query_params() {
        let mut agent = AgentBase::new(default_options());
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        agent.add_swaig_query_params(params);
        assert_eq!(agent.swaig_query_params.len(), 1);
        agent.clear_swaig_query_params();
        assert!(agent.swaig_query_params.is_empty());
    }

    // ── SIP ──────────────────────────────────────────────────────────────

    #[test]
    fn test_enable_sip_routing() {
        let mut agent = AgentBase::new(default_options());
        agent.enable_sip_routing();
        assert_eq!(agent.params["sip_routing"], true);
    }

    #[test]
    fn test_register_sip_username() {
        let mut agent = AgentBase::new(default_options());
        agent.register_sip_username("alice", "/custom");
        assert_eq!(agent.params["sip_username"], "alice");
        assert_eq!(agent.params["sip_route"], "/custom");
    }

    #[test]
    fn test_register_sip_username_no_route() {
        let mut agent = AgentBase::new(default_options());
        agent.register_sip_username("bob", "");
        assert_eq!(agent.params["sip_username"], "bob");
        assert!(agent.params.get("sip_route").is_none());
    }

    // ── SWML Rendering ───────────────────────────────────────────────────

    #[test]
    fn test_render_swml_basic() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_text("You are a bot");
        let swml = agent.render_swml(&HashMap::new());
        assert_eq!(swml["version"], "1.0.0");
        let main = swml["sections"]["main"].as_array().unwrap();
        // Should have answer + ai verbs
        assert!(main.len() >= 2);
        assert!(main[0].get("answer").is_some());
        assert!(main[1].get("ai").is_some());
    }

    #[test]
    fn test_render_swml_no_auto_answer() {
        let mut opts = default_options();
        opts.auto_answer = false;
        let mut agent = AgentBase::new(opts);
        agent.set_prompt_text("Bot");
        let swml = agent.render_swml(&HashMap::new());
        let main = swml["sections"]["main"].as_array().unwrap();
        // First verb should be ai (no answer)
        assert!(main[0].get("ai").is_some());
    }

    #[test]
    fn test_render_swml_with_record() {
        let mut opts = default_options();
        opts.record_call = true;
        let agent = AgentBase::new(opts);
        let swml = agent.render_swml(&HashMap::new());
        let main = swml["sections"]["main"].as_array().unwrap();
        // answer, record_call, ai
        assert!(main[1].get("record_call").is_some());
    }

    #[test]
    fn test_render_swml_with_verbs() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_text("Bot");
        agent.add_pre_answer_verb("play", json!({"url": "ring.mp3"}));
        agent.add_post_answer_verb("sleep", json!(1000));
        agent.add_post_ai_verb("hangup", json!({}));
        let swml = agent.render_swml(&HashMap::new());
        let main = swml["sections"]["main"].as_array().unwrap();
        // play, answer, sleep, ai, hangup
        assert!(main[0].get("play").is_some());
        assert!(main[1].get("answer").is_some());
        assert!(main[2].get("sleep").is_some());
        assert!(main[3].get("ai").is_some());
        assert!(main[4].get("hangup").is_some());
    }

    #[test]
    fn test_build_ai_verb_prompt_text() {
        let mut agent = AgentBase::new(default_options());
        agent.use_pom = false;
        agent.set_prompt_text("You are helpful");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["prompt"]["text"], "You are helpful");
    }

    #[test]
    fn test_build_ai_verb_prompt_pom() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Role", "Be helpful", vec![]);
        let ai = agent.build_ai_verb(&HashMap::new());
        assert!(ai["prompt"]["pom"].is_array());
    }

    #[test]
    fn test_build_ai_verb_post_prompt() {
        let mut agent = AgentBase::new(default_options());
        agent.set_post_prompt("Summarise the call");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["post_prompt"]["text"], "Summarise the call");
    }

    #[test]
    fn test_build_ai_verb_hints() {
        let mut agent = AgentBase::new(default_options());
        agent.add_hint("SignalWire");
        agent.add_pattern_hint("[0-9]+");
        let ai = agent.build_ai_verb(&HashMap::new());
        let hints = ai["hints"].as_array().unwrap();
        assert_eq!(hints.len(), 2);
    }

    #[test]
    fn test_build_ai_verb_languages() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "Polly.Salli");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["languages"][0]["name"], "English");
    }

    #[test]
    fn test_build_ai_verb_pronunciations() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pronunciation("AI", "A.I.", "");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["pronounce"][0]["replace"], "AI");
    }

    #[test]
    fn test_build_ai_verb_params() {
        let mut agent = AgentBase::new(default_options());
        agent.set_param("temperature", json!(0.7));
        agent.add_internal_filler("one moment");
        agent.enable_debug_events("all");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["params"]["temperature"], 0.7);
        assert_eq!(ai["params"]["internal_fillers"][0], "one moment");
        assert_eq!(ai["params"]["debug_events"], "all");
    }

    #[test]
    fn test_build_ai_verb_global_data() {
        let mut agent = AgentBase::new(default_options());
        agent.set_global_data(json!({"company": "SignalWire"}));
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["global_data"]["company"], "SignalWire");
    }

    #[test]
    fn test_build_ai_verb_swaig_functions() {
        let mut agent = AgentBase::new(default_options());
        agent.manual_set_proxy_url("https://proxy.example.com");
        agent.define_tool(
            "lookup",
            "Look up info",
            json!({}),
            Box::new(|_args, _raw| FunctionResult::with_response("result")),
            false,
        );
        let ai = agent.build_ai_verb(&HashMap::new());
        let funcs = ai["SWAIG"]["functions"].as_array().unwrap();
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0]["function"], "lookup");
        assert!(funcs[0]["web_hook_url"].as_str().unwrap().contains("/swaig"));
    }

    #[test]
    fn test_build_ai_verb_native_functions() {
        let mut agent = AgentBase::new(default_options());
        agent.set_native_functions(vec!["check_voicemail"]);
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["SWAIG"]["native_functions"][0], "check_voicemail");
    }

    #[test]
    fn test_build_ai_verb_includes() {
        let mut agent = AgentBase::new(default_options());
        agent.add_function_include(json!({"url": "https://example.com/funcs"}));
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["SWAIG"]["includes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_build_ai_verb_context_switch() {
        let mut agent = AgentBase::new(default_options());
        agent
            .define_contexts()
            .add_context("default")
            .add_step("intro")
            .set_text("Hello");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert!(ai["context_switch"]["default"].is_object());
    }

    #[test]
    fn test_build_ai_verb_post_prompt_url_custom() {
        let mut agent = AgentBase::new(default_options());
        agent.set_post_prompt_url("https://custom.example.com/pp");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["post_prompt_url"], "https://custom.example.com/pp");
    }

    #[test]
    fn test_build_ai_verb_post_prompt_url_auto() {
        let mut agent = AgentBase::new(default_options());
        agent.manual_set_proxy_url("https://proxy.example.com");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["post_prompt_url"], "https://proxy.example.com/post_prompt");
    }

    #[test]
    fn test_build_ai_verb_llm_params() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_llm_params(json!({"temperature": 0.5}));
        agent.set_post_prompt("Summarise");
        agent.set_post_prompt_llm_params(json!({"top_p": 0.9}));
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["prompt"]["temperature"], 0.5);
        assert_eq!(ai["post_prompt"]["top_p"], 0.9);
    }

    // ── Dynamic config isolation ─────────────────────────────────────────

    #[test]
    fn test_clone_for_request_isolation() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_text("Original prompt");
        agent.add_hint("hint1");

        let mut clone = agent.clone_for_request();
        clone.set_prompt_text("Modified prompt");
        clone.add_hint("hint2");

        // Original should be unchanged
        assert_eq!(agent.prompt_text, "Original prompt");
        assert_eq!(agent.hints.len(), 1);

        // Clone should have changes
        assert_eq!(clone.prompt_text, "Modified prompt");
        assert_eq!(clone.hints.len(), 2);
    }

    #[test]
    fn test_clone_preserves_tools() {
        let mut agent = AgentBase::new(default_options());
        agent.define_tool(
            "func1",
            "test",
            json!({}),
            Box::new(|_args, _raw| FunctionResult::with_response("ok")),
            false,
        );

        let clone = agent.clone_for_request();
        let args = Map::new();
        let raw = Map::new();
        let result = clone.on_function_call("func1", &args, &raw).unwrap();
        assert_eq!(result.to_value()["response"], "ok");
    }

    // ── HTTP Endpoints ───────────────────────────────────────────────────

    #[test]
    fn test_handle_request_health() {
        let agent = AgentBase::new(default_options());
        let (status, _, body) = agent.handle_request("GET", "/health", &HashMap::new(), "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["status"], "healthy");
    }

    #[test]
    fn test_handle_request_ready() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("GET", "/ready", &HashMap::new(), "");
        assert_eq!(status, 200);
    }

    #[test]
    fn test_handle_request_auth_required() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("POST", "/", &HashMap::new(), "");
        assert_eq!(status, 401);
    }

    #[test]
    fn test_handle_request_swml() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_text("Bot");
        let (status, _, body) = agent.handle_request("POST", "/", &authed_headers(), "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["version"], "1.0.0");
    }

    #[test]
    fn test_handle_request_swaig_dispatch() {
        let mut agent = AgentBase::new(default_options());
        agent.define_tool(
            "greet",
            "Greet",
            json!({}),
            Box::new(|_args, _raw| FunctionResult::with_response("Hello!")),
            false,
        );

        let body = json!({
            "function": "greet",
            "argument": {"parsed": [{}]}
        });
        let (status, _, resp_body) = agent.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            &body.to_string(),
        );
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&resp_body).unwrap();
        assert_eq!(parsed["response"], "Hello!");
    }

    #[test]
    fn test_handle_request_swaig_unknown_function() {
        let agent = AgentBase::new(default_options());
        let body = json!({"function": "nonexistent", "argument": {"parsed": [{}]}});
        let (status, _, _) = agent.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            &body.to_string(),
        );
        assert_eq!(status, 404);
    }

    #[test]
    fn test_handle_request_swaig_no_body() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("POST", "/swaig", &authed_headers(), "");
        assert_eq!(status, 400);
    }

    #[test]
    fn test_handle_request_swaig_no_function_name() {
        let agent = AgentBase::new(default_options());
        let body = json!({"argument": {}});
        let (status, _, _) = agent.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            &body.to_string(),
        );
        assert_eq!(status, 400);
    }

    #[test]
    fn test_handle_request_post_prompt() {
        let agent = AgentBase::new(default_options());
        let body = json!({"summary": "Call went well"});
        let (status, _, resp_body) = agent.handle_request(
            "POST",
            "/post_prompt",
            &authed_headers(),
            &body.to_string(),
        );
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&resp_body).unwrap();
        assert_eq!(parsed["status"], "ok");
    }

    #[test]
    fn test_handle_request_not_found() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("GET", "/unknown", &authed_headers(), "");
        assert_eq!(status, 404);
    }

    // ── Chaining ─────────────────────────────────────────────────────────

    #[test]
    fn test_chaining() {
        let mut agent = AgentBase::new(default_options());
        agent
            .set_prompt_text("Bot")
            .set_post_prompt("Summarise")
            .add_hint("hint1")
            .add_hints(vec!["hint2", "hint3"])
            .set_param("temperature", json!(0.7))
            .enable_debug_events("all")
            .add_pre_answer_verb("play", json!({"url": "ring.mp3"}))
            .add_post_answer_verb("sleep", json!(1000));

        assert_eq!(agent.prompt_text, "Bot");
        assert_eq!(agent.post_prompt, "Summarise");
        assert_eq!(agent.hints.len(), 3);
        assert_eq!(agent.params["temperature"], 0.7);
    }

    // ── Webhook URL construction ─────────────────────────────────────────

    #[test]
    fn test_build_swaig_webhook_url() {
        let mut agent = AgentBase::new(default_options());
        agent.manual_set_proxy_url("https://proxy.example.com");
        let url = agent.build_swaig_webhook_url(&HashMap::new());
        assert!(url.starts_with("https://user:pass@proxy.example.com"));
        assert!(url.ends_with("/swaig"));
    }

    #[test]
    fn test_build_swaig_webhook_url_with_query_params() {
        let mut agent = AgentBase::new(default_options());
        agent.manual_set_proxy_url("https://proxy.example.com");
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        agent.add_swaig_query_params(params);
        let url = agent.build_swaig_webhook_url(&HashMap::new());
        assert!(url.contains("?key=value"));
    }

    #[test]
    fn test_webhook_url_override() {
        let mut agent = AgentBase::new(default_options());
        agent.set_webhook_url("https://custom-webhook.example.com/swaig");
        agent.define_tool(
            "f1",
            "test",
            json!({}),
            Box::new(|_args, _raw| FunctionResult::with_response("ok")),
            false,
        );
        let ai = agent.build_ai_verb(&HashMap::new());
        let funcs = ai["SWAIG"]["functions"].as_array().unwrap();
        assert_eq!(
            funcs[0]["web_hook_url"],
            "https://custom-webhook.example.com/swaig"
        );
    }

    // ── Dynamic config callback ──────────────────────────────────────────

    #[test]
    fn test_dynamic_config_callback() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_text("Original");
        agent.set_dynamic_config_callback(Box::new(|_params, _data, _headers, clone| {
            clone.set_prompt_text("Dynamic prompt");
        }));

        let (status, _, body) = agent.handle_request("POST", "/", &authed_headers(), "{}");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        // The AI verb should have the dynamically modified prompt
        let ai_verb = &parsed["sections"]["main"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v.get("ai").is_some())
            .unwrap()["ai"];
        assert_eq!(ai_verb["prompt"]["text"], "Dynamic prompt");

        // Original agent should be unchanged
        assert_eq!(agent.prompt_text, "Original");
    }

    // ── Summary callback ─────────────────────────────────────────────────

    #[test]
    fn test_on_summary_callback() {
        use std::sync::Arc;
        use std::sync::Mutex;

        let captured = Arc::new(Mutex::new(String::new()));
        let captured_clone = captured.clone();

        let mut agent = AgentBase::new(default_options());
        agent.on_summary(Box::new(move |summary, _data, _headers| {
            let mut guard = captured_clone.lock().unwrap();
            *guard = summary.to_string();
        }));

        let body = json!({"summary": "Great call"});
        let (status, _, _) = agent.handle_request(
            "POST",
            "/post_prompt",
            &authed_headers(),
            &body.to_string(),
        );
        assert_eq!(status, 200);

        let guard = captured.lock().unwrap();
        assert_eq!(*guard, "Great call");
    }
}
