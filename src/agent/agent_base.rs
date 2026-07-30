use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::contexts::ContextBuilder;
use crate::security::SessionManager;
use crate::swaig::FunctionResult;
use crate::swml::service::{Service, ServiceOptions};

// FunctionHandler and ToolDef are now declared on swml::Service so that
// tools registered on Service-the-sidecar and AgentBase share storage.
// Re-exported here for backward compatibility with consumers that referenced
// `crate::agent::agent_base::FunctionHandler`.
pub use crate::swml::service::{FunctionHandler, ToolDef};

/// Options for constructing an `AgentBase`.
///
/// Doubles as an idiomatic **builder**: every field has a fluent
/// `with_*`/setter that takes and returns `self`, so an agent can be configured
/// in one expression and handed straight to [`AgentBase::new`]:
///
/// ```no_run
/// use signalwire::agent::{AgentBase, AgentOptions};
///
/// let agent = AgentBase::new(
///     AgentOptions::new("receptionist")
///         .route("/reception")
///         .basic_auth("user", "secret")
///         .auto_answer(true)
///         .signing_key("whsec_…"),
/// );
/// ```
///
/// Direct field assignment (`opts.route = Some(...)`) still works — the builder
/// methods are an additive convenience. `#[must_use]` flags an `AgentOptions`
/// that is built but never passed to [`AgentBase::new`]; under
/// `#[deny(unused_must_use)]` that is a hard compile error, as this
/// `compile_fail` doctest proves:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use signalwire::agent::AgentOptions;
/// // Building an AgentOptions and dropping it on the floor is a compile error
/// // because the type is `#[must_use]` — you must feed it to AgentBase::new.
/// AgentOptions::new("oops").route("/x");
/// ```
#[must_use]
#[allow(clippy::struct_excessive_bools)] // orthogonal reference kwargs, 1:1
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
    /// SignalWire Signing Key used to validate the
    /// `X-SignalWire-Signature` header on incoming POST webhooks
    /// (`POST /`, `POST /swaig`, `POST /post_prompt`).
    ///
    /// When `None`, the agent falls back to the
    /// `SIGNALWIRE_SIGNING_KEY` environment variable. When neither is
    /// set, the agent logs a prominent warning at startup and accepts
    /// unsigned requests — see the webhook validator.
    pub signing_key: Option<String>,

    // ── Forwarded to `Service` (the reference's `super().__init__`,
    //    `agent_base.py:205-207`) ──────────────────────────────────────────
    /// Path to a SWML schema file; `None` uses the embedded `schema.json`.
    /// Forwarded to `Service` → `SchemaUtils`.
    pub schema_path: Option<String>,
    /// Path to a JSON configuration file. Its `service` section supplies
    /// `name`/`route`/`host`/`port` (explicit options win) and its `security`
    /// section supplies SSL/CORS/basic-auth. Forwarded to `Service`.
    pub config_file: Option<String>,
    /// Enable SWML schema validation on `add_verb` (default `true`). Also
    /// disablable process-wide via `SWML_SKIP_SCHEMA_VALIDATION=1`. Forwarded
    /// to `Service` → `SchemaUtils`.
    pub schema_validation: bool,

    // ── Forwarded to `SessionManager` (`agent_base.py:247`) ───────────────
    /// Seconds until a per-call SWAIG function token expires (default 3600).
    pub token_expiry_secs: u64,

    // ── Stored on the agent ──────────────────────────────────────────────
    /// Recording container format for the `record_call` verb. The reference
    /// default is `"mp4"` (`agent_base.py:131`).
    pub record_format: String,
    /// Whether `record_call` records in stereo. The reference default is
    /// `true` (`agent_base.py:132`).
    pub record_stereo: bool,
    /// Default `web_hook_url` applied to every SWAIG function that does not
    /// set its own, emitted as `ai.SWAIG.defaults.web_hook_url`.
    pub default_webhook_url: Option<String>,
    /// Stable identifier for this agent. `None` generates a UUID v4.
    pub agent_id: Option<String>,
    /// Server-side native functions listed in `ai.SWAIG.native_functions`.
    pub native_functions: Option<Vec<String>>,
    /// Suppress the agent's structured request/response logs.
    pub suppress_logs: bool,
    /// Enable the post-prompt override hook.
    pub enable_post_prompt_override: bool,
    /// Enable the check-for-input override hook.
    pub check_for_input_override: bool,
    /// Honor `X-Forwarded-Proto` / `X-Forwarded-Host` when reconstructing the
    /// URL for webhook signature validation.
    ///
    /// Defaults to `false`: those headers are attacker-controllable, so an
    /// attacker who can reach the agent directly could otherwise mint a
    /// signature for a host they chose and have it accepted. Opt in only when
    /// you control the proxy chain in front of the agent (reference
    /// `agent_base.py` `trust_proxy_for_signature` docstring).
    pub trust_proxy_for_signature: bool,
}

impl AgentOptions {
    /// Start a new options set for an agent named `name`.
    ///
    /// `name` is the agent's identity: it is the key [`AgentServer`] routes
    /// on and, when no explicit `route` is given, the default mount path is
    /// derived from it. Every other field takes its reference default —
    /// notably `auto_answer` and `use_pom` are `true`, `record_call` is
    /// `false`, `schema_validation` is `true`, and `token_expiry_secs` is
    /// 3600 — so only the fields you actually want to change need setting.
    ///
    /// [`AgentServer`]: crate::server::AgentServer
    pub fn new(name: &str) -> Self {
        AgentOptions {
            name: name.to_string(),
            route: None,
            host: None,
            port: None,
            basic_auth_user: None,
            basic_auth_password: None,
            auto_answer: true,
            record_call: false,
            use_pom: true,
            signing_key: None,
            schema_path: None,
            config_file: None,
            schema_validation: true,
            token_expiry_secs: 3600,
            record_format: "mp4".to_string(),
            record_stereo: true,
            default_webhook_url: None,
            agent_id: None,
            native_functions: None,
            suppress_logs: false,
            enable_post_prompt_override: false,
            check_for_input_override: false,
            trust_proxy_for_signature: false,
        }
    }

    // ── Fluent builder methods (idiomatic chaining; each returns Self) ────
    //
    // These are an additive Rust-idiom convenience over direct field
    // assignment. They take and return `self` so configuration reads as one
    // expression feeding `AgentBase::new`. Python configures these as keyword
    // arguments to `AgentBase.__init__`; the Rust port carries them on
    // `AgentOptions`, and these methods are the builder face of that struct.

    /// Set the HTTP route this agent serves (e.g. `"/reception"`).
    pub fn route(mut self, route: &str) -> Self {
        self.route = Some(route.to_string());
        self
    }

    /// Set the bind host (e.g. `"0.0.0.0"`).
    pub fn host(mut self, host: &str) -> Self {
        self.host = Some(host.to_string());
        self
    }

    /// Set the bind port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set HTTP Basic-Auth credentials guarding this agent's endpoints.
    pub fn basic_auth(mut self, user: &str, password: &str) -> Self {
        self.basic_auth_user = Some(user.to_string());
        self.basic_auth_password = Some(password.to_string());
        self
    }

    /// Toggle automatic call answering (default `true`).
    pub fn auto_answer(mut self, auto_answer: bool) -> Self {
        self.auto_answer = auto_answer;
        self
    }

    /// Toggle call recording (default `false`).
    pub fn record_call(mut self, record_call: bool) -> Self {
        self.record_call = record_call;
        self
    }

    /// Toggle Prompt-Object-Model prompt rendering (default `true`).
    pub fn use_pom(mut self, use_pom: bool) -> Self {
        self.use_pom = use_pom;
        self
    }

    /// Set the SignalWire signing key used to validate inbound
    /// `X-SignalWire-Signature` webhook headers. See [`AgentOptions::signing_key`].
    pub fn signing_key(mut self, signing_key: &str) -> Self {
        self.signing_key = Some(signing_key.to_string());
        self
    }

    /// Load the SWML schema from `path` instead of the embedded `schema.json`.
    pub fn schema_path(mut self, path: &str) -> Self {
        self.schema_path = Some(path.to_string());
        self
    }

    /// Read service + security configuration from the JSON file at `path`.
    pub fn config_file(mut self, path: &str) -> Self {
        self.config_file = Some(path.to_string());
        self
    }

    /// Toggle SWML schema validation on `add_verb` (default `true`).
    pub fn schema_validation(mut self, enabled: bool) -> Self {
        self.schema_validation = enabled;
        self
    }

    /// Set the SWAIG function-token lifetime in seconds (default 3600).
    pub fn token_expiry_secs(mut self, secs: u64) -> Self {
        self.token_expiry_secs = secs;
        self
    }

    /// Set the `record_call` container format (default `"mp4"`).
    pub fn record_format(mut self, format: &str) -> Self {
        self.record_format = format.to_string();
        self
    }

    /// Toggle stereo recording for `record_call` (default `true`).
    pub fn record_stereo(mut self, stereo: bool) -> Self {
        self.record_stereo = stereo;
        self
    }

    /// Set the default SWAIG `web_hook_url` for functions that do not set one.
    pub fn default_webhook_url(mut self, url: &str) -> Self {
        self.default_webhook_url = Some(url.to_string());
        self
    }

    /// Pin this agent's id instead of generating a UUID v4.
    pub fn agent_id(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.to_string());
        self
    }

    /// List the server-side native functions to advertise in
    /// `ai.SWAIG.native_functions`.
    pub fn native_functions(mut self, functions: Vec<String>) -> Self {
        self.native_functions = Some(functions);
        self
    }

    /// Suppress the agent's structured request/response logs (default `false`).
    pub fn suppress_logs(mut self, suppress: bool) -> Self {
        self.suppress_logs = suppress;
        self
    }

    /// Enable the post-prompt override hook (default `false`).
    pub fn enable_post_prompt_override(mut self, enabled: bool) -> Self {
        self.enable_post_prompt_override = enabled;
        self
    }

    /// Enable the check-for-input override hook (default `false`).
    pub fn check_for_input_override(mut self, enabled: bool) -> Self {
        self.check_for_input_override = enabled;
        self
    }

    /// Honor `X-Forwarded-*` when reconstructing the URL for webhook signature
    /// validation. Defaults to `false` — see
    /// [`AgentOptions::trust_proxy_for_signature`].
    pub fn trust_proxy_for_signature(mut self, trust: bool) -> Self {
        self.trust_proxy_for_signature = trust;
        self
    }
}

/// The `service` section of a config file, used by [`AgentBase::new`] with
/// constructor params taking precedence. Mirrors the reference's
/// `AgentBase._load_service_config` (`agent_base.py:359-382`), which reads the
/// `service` section and only consults it where the caller left the param at
/// its default. Private, like its leading-underscore reference counterpart.
fn load_service_config(config_file: Option<&str>, service_name: &str) -> Map<String, Value> {
    let path = match config_file {
        Some(p) => Some(p.to_string()),
        None => {
            crate::core::config_loader::ConfigLoader::find_config_file(Some(service_name), None)
        }
    };
    let Some(path) = path else {
        return Map::new();
    };
    let loader = crate::core::config_loader::ConfigLoader::new(Some(vec![path]));
    if !loader.has_config() {
        return Map::new();
    }
    // ConfigLoader::get_section returns a bare `Value` (an empty object when the
    // section is absent), not an Option — so narrow it to a Map directly.
    loader
        .get_section("service")
        .as_object()
        .cloned()
        .unwrap_or_default()
}

/// Generate a UUID v4 for the default `agent_id` (reference
/// `agent_base.py:229` — `agent_id or str(uuid.uuid4())`).
fn generate_uuid_v4() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let mut data = [0u8; 16];
    rng.fill(&mut data);
    data[6] = (data[6] & 0x0f) | 0x40; // version 4
    data[8] = (data[8] & 0x3f) | 0x80; // variant RFC 4122
    let hex: String = data.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Callback types for agent events.
type DynamicConfigCallback = Box<
    dyn Fn(&Map<String, Value>, &Option<Value>, &HashMap<String, String>, &mut AgentBase)
        + Send
        + Sync,
>;

type SummaryCallback = Box<dyn Fn(&str, &Value, &HashMap<String, String>) + Send + Sync>;

type DebugEventCallback = Box<dyn Fn(&Value, &HashMap<String, String>) + Send + Sync>;

/// Core agent that extends `Service` with AI-specific capabilities.
///
/// Manages prompt configuration, tool registration, SWML rendering,
/// and HTTP request handling for AI agent endpoints.
///
/// `AgentBase` implements `Deref<Target = Service>` (Rust's idiomatic
/// equivalent of inheritance) so `Service` methods like `set_route`,
/// `define_tool`, `on_function_call`, etc. are usable on `AgentBase`
/// instances directly without needing forwarding wrappers.
///
/// The several `bool` fields (`auto_answer`, `record_call`, `record_stereo`,
/// `use_pom`, …) are independent feature flags mirroring Python
/// `AgentBase`'s boolean `__init__` kwargs 1:1, so
/// `clippy::struct_excessive_bools` is suppressed: folding them into a state
/// struct would diverge from the reference's flat flag surface for no
/// behavioral gain — they are orthogonal toggles, not a state machine.
#[allow(clippy::struct_excessive_bools)]
pub struct AgentBase {
    // ── Service (composition + Deref<Service> for inheritance shape) ────
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
    // tools / tool_order live on the embedded Service (single registry,
    // shared between Service-the-sidecar and AgentBase). Accessed via
    // self.tools / self.tool_order through Deref<Target=Service>.

    // ── Hints ───────────────────────────────────────────────────────────
    hints: Vec<String>,
    /// Structured pattern hints. Each entry mirrors Python's
    /// `{hint, pattern, replace, ignore_case}` object (`ai_config_mixin`
    /// `add_pattern_hint`). Built via `add_pattern_hint(pattern)` + the
    /// fluent `set_pattern_hint_*` setters (Rust builder idiom for Python's
    /// all-args-inline call).
    pattern_hints: Vec<Value>,

    // ── Languages / pronunciations ──────────────────────────────────────
    languages: Vec<Value>,
    pronunciations: Vec<Value>,
    /// ASR-driven multilingual (Mode B) config, emitted as a top-level
    /// `multilingual` object on the AI verb. Mutually exclusive with
    /// `languages` (the server prefers `multilingual` when both are set).
    multilingual: Option<Value>,

    // ── Params / data ───────────────────────────────────────────────────
    params: Map<String, Value>,
    global_data: Map<String, Value>,
    /// SIP usernames routed to this agent: a case-folded, deduplicated set —
    /// `register_sip_username` lowercases each name before inserting. A
    /// `BTreeSet` keeps `sip_usernames()` sorted.
    sip_usernames: std::collections::BTreeSet<String>,

    // ── Native functions / fillers / debug ───────────────────────────────
    native_functions: Vec<String>,
    internal_fillers: Vec<String>,
    /// Structured internal fillers keyed by `function_name` → `language_code`
    /// → phrases. Populated by `set_internal_fillers_map` and
    /// `add_internal_filler_for`. Separate from the legacy
    /// `internal_fillers: Vec<String>` above to preserve backward
    /// compatibility.
    internal_fillers_map: HashMap<String, HashMap<String, Vec<String>>>,
    debug_events_level: Option<i64>,

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

    // ── Webhook signature validation ────────────────────────────────────
    /// Resolved signing key (from options, then env). `None` means
    /// signature validation is disabled and a startup warning was
    /// emitted. See `AgentOptions::signing_key`.
    signing_key: Option<String>,

    // ── MCP (Model Context Protocol) ────────────────────────────────────
    /// External MCP servers registered via [`AgentBase::add_mcp_server`].
    mcp_servers: Vec<Value>,
    /// Whether this agent exposes its tools as an MCP endpoint
    /// ([`AgentBase::enable_mcp_server`]).
    mcp_server_enabled: bool,

    // ── Web-hook URL override / debug routes ────────────────────────────
    /// Override for the default SWAIG `web_hook_url`
    /// ([`AgentBase::set_web_hook_url`]).
    web_hook_url_override: Option<String>,
    /// Whether debug routes are enabled ([`AgentBase::enable_debug_routes`]).
    debug_routes_enabled: bool,

    // ── Construction params stored on the agent ─────────────────────────
    /// Stable id for this agent — the `agent_id` option, or a generated
    /// UUID v4 (reference `AgentBase.agent_id`, `agent_base.py:229`).
    agent_id: String,
    /// Default SWAIG `web_hook_url` (`_default_webhook_url`).
    default_webhook_url: Option<String>,
    /// Suppress structured request/response logs (`_suppress_logs`).
    suppress_logs: bool,
    /// Post-prompt override hook flag (`enable_post_prompt_override`).
    enable_post_prompt_override: bool,
    /// Check-for-input override hook flag (`check_for_input_override`).
    check_for_input_override: bool,
    /// Honor `X-Forwarded-*` during signature URL reconstruction
    /// (`_trust_proxy_for_signature`). Default `false`.
    trust_proxy_for_signature: bool,
}

impl Clone for AgentBase {
    fn clone(&self) -> Self {
        // `Service` is itself `Clone`, so clone it directly rather than
        // rebuilding from `ServiceOptions`. Rebuilding silently DROPPED the
        // schema-utils override and the resolved `SecurityConfig` (an agent
        // built with `schema_validation=false` came back with validation ON),
        // and re-ran the config-file load. The direct clone carries the whole
        // resolved service — tools and tool_order included — so the ephemeral
        // dynamic-config copy is a faithful duplicate.
        let service = self.service.clone();
        AgentBase {
            service,
            auto_answer: self.auto_answer,
            record_call: self.record_call,
            record_format: self.record_format.clone(),
            record_stereo: self.record_stereo,
            use_pom: self.use_pom,
            pom_sections: self.pom_sections.clone(),
            prompt_text: self.prompt_text.clone(),
            post_prompt: self.post_prompt.clone(),
            hints: self.hints.clone(),
            pattern_hints: self.pattern_hints.clone(),
            languages: self.languages.clone(),
            pronunciations: self.pronunciations.clone(),
            multilingual: self.multilingual.clone(),
            params: self.params.clone(),
            global_data: self.global_data.clone(),
            sip_usernames: self.sip_usernames.clone(),
            native_functions: self.native_functions.clone(),
            internal_fillers: self.internal_fillers.clone(),
            internal_fillers_map: self.internal_fillers_map.clone(),
            debug_events_level: self.debug_events_level,
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
            signing_key: self.signing_key.clone(),
            mcp_servers: self.mcp_servers.clone(),
            mcp_server_enabled: self.mcp_server_enabled,
            web_hook_url_override: self.web_hook_url_override.clone(),
            debug_routes_enabled: self.debug_routes_enabled,
            agent_id: self.agent_id.clone(),
            default_webhook_url: self.default_webhook_url.clone(),
            suppress_logs: self.suppress_logs,
            enable_post_prompt_override: self.enable_post_prompt_override,
            check_for_input_override: self.check_for_input_override,
            trust_proxy_for_signature: self.trust_proxy_for_signature,
        }
    }
}

// Deref/DerefMut to Service is Rust's idiomatic equivalent of inheritance:
// callers can invoke any &Service or &mut Service method on an AgentBase
// directly, and field access through agent.field works for Service fields
// too. Removes the need for forwarding wrappers like `agent.service().route()`.
impl std::ops::Deref for AgentBase {
    type Target = Service;
    fn deref(&self) -> &Service {
        &self.service
    }
}

impl std::ops::DerefMut for AgentBase {
    fn deref_mut(&mut self) -> &mut Service {
        &mut self.service
    }
}

impl AgentBase {
    /// Construct an agent from `options`.
    ///
    /// Wraps a freshly built [`Service`] (reachable via `Deref`) and
    /// initialises the prompt store, tool registry, skill manager, and AI
    /// configuration.
    ///
    /// Where `options` carries a `config_file`, its `service` section is
    /// consulted for `name` / `route` / `host` / `port`, but **constructor
    /// values win**: the file is only read where the caller left the field
    /// unset, matching the reference (`agent_base.py:189-196`).
    ///
    /// Basic-auth credentials are taken from `options` when supplied,
    /// otherwise from `SWML_BASIC_AUTH_USER` / `SWML_BASIC_AUTH_PASSWORD`,
    /// otherwise randomly generated — so an agent is never served
    /// unauthenticated by accident.
    pub fn new(options: AgentOptions) -> Self {
        // Config-file `service` section, applied with CONSTRUCTOR params
        // taking precedence — the reference consults the file only where the
        // caller left the param at its default (`agent_base.py:189-196`).
        let service_config = load_service_config(options.config_file.as_deref(), &options.name);
        let cfg_str = |key: &str| {
            service_config
                .get(key)
                .and_then(Value::as_str)
                .map(std::string::ToString::to_string)
        };

        let final_name = cfg_str("name").unwrap_or(options.name);
        let final_route = options.route.or_else(|| cfg_str("route"));
        let final_host = options.host.or_else(|| cfg_str("host"));
        let final_port = options.port.or_else(|| {
            service_config
                .get("port")
                .and_then(Value::as_u64)
                .and_then(|p| u16::try_from(p).ok())
        });

        let service = Service::new(ServiceOptions {
            name: final_name,
            route: final_route,
            host: final_host,
            port: final_port,
            basic_auth_user: options.basic_auth_user,
            basic_auth_password: options.basic_auth_password,
            schema_path: options.schema_path,
            config_file: options.config_file,
            schema_validation: options.schema_validation,
        });

        // Resolve the signing key: explicit option, then environment.
        // When neither produces a non-empty value, log a prominent
        // startup warning and continue without validation — matching
        // Python's `[signalwire] webhook signature validation is disabled`
        // banner.
        let signing_key = options
            .signing_key
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("SIGNALWIRE_SIGNING_KEY").ok())
            .filter(|s| !s.is_empty());

        if signing_key.is_none() {
            log::warn!(
                "[signalwire] webhook signature validation is disabled — set AgentOptions::signing_key or SIGNALWIRE_SIGNING_KEY to enable"
            );
        }

        AgentBase {
            service,
            auto_answer: options.auto_answer,
            record_call: options.record_call,
            record_format: options.record_format,
            record_stereo: options.record_stereo,
            use_pom: options.use_pom,
            pom_sections: Vec::new(),
            prompt_text: String::new(),
            post_prompt: String::new(),
            hints: Vec::new(),
            pattern_hints: Vec::new(),
            languages: Vec::new(),
            pronunciations: Vec::new(),
            multilingual: None,
            params: Map::new(),
            global_data: Map::new(),
            sip_usernames: std::collections::BTreeSet::new(),
            native_functions: options.native_functions.unwrap_or_default(),
            internal_fillers: Vec::new(),
            internal_fillers_map: HashMap::new(),
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
            session_manager: SessionManager::new(options.token_expiry_secs),
            context_builder: None,
            skills: Vec::new(),
            manual_proxy_url: None,
            signing_key,
            mcp_servers: Vec::new(),
            mcp_server_enabled: false,
            web_hook_url_override: None,
            debug_routes_enabled: false,
            agent_id: options.agent_id.unwrap_or_else(generate_uuid_v4),
            default_webhook_url: options.default_webhook_url,
            suppress_logs: options.suppress_logs,
            enable_post_prompt_override: options.enable_post_prompt_override,
            check_for_input_override: options.check_for_input_override,
            trust_proxy_for_signature: options.trust_proxy_for_signature,
        }
    }

    /// This agent's stable identifier — the `agent_id` option, or the UUID v4
    /// generated at construction. Mirrors the reference's `agent.agent_id`.
    #[must_use]
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// The default SWAIG `web_hook_url` applied to functions that do not set
    /// their own (`_default_webhook_url`).
    #[must_use]
    pub fn default_webhook_url(&self) -> Option<&str> {
        self.default_webhook_url.as_deref()
    }

    /// Whether structured request/response logging is suppressed
    /// (`_suppress_logs`).
    #[must_use]
    pub fn suppress_logs(&self) -> bool {
        self.suppress_logs
    }

    /// Whether the post-prompt override hook is enabled.
    #[must_use]
    pub fn enable_post_prompt_override(&self) -> bool {
        self.enable_post_prompt_override
    }

    /// Whether the check-for-input override hook is enabled.
    #[must_use]
    pub fn check_for_input_override(&self) -> bool {
        self.check_for_input_override
    }

    /// Whether `X-Forwarded-*` headers are honored when reconstructing the URL
    /// for webhook signature validation (`_trust_proxy_for_signature`).
    #[must_use]
    pub fn trust_proxy_for_signature(&self) -> bool {
        self.trust_proxy_for_signature
    }

    /// The server-side native functions advertised in
    /// `ai.SWAIG.native_functions`.
    #[must_use]
    pub fn native_functions(&self) -> &[String] {
        &self.native_functions
    }

    /// Access the underlying service.
    pub fn service(&self) -> &Service {
        &self.service
    }

    /// Access the underlying service mutably.
    pub fn service_mut(&mut self) -> &mut Service {
        &mut self.service
    }

    /// Return the signing key resolved from `AgentOptions::signing_key`
    /// or the `SIGNALWIRE_SIGNING_KEY` environment variable. `None`
    /// means signature validation is disabled.
    pub fn signing_key(&self) -> Option<&str> {
        self.signing_key.as_deref()
    }

    /// Set or clear the signing key after construction. Useful for
    /// tests and dynamic-config flows. Pass an empty string or
    /// `None`-equivalent to disable.
    pub fn set_signing_key(&mut self, key: Option<&str>) -> &mut Self {
        self.signing_key = key
            .map(std::string::ToString::to_string)
            .filter(|s| !s.is_empty());
        self
    }

    /// Mint a per-call SWAIG-function token via the agent's `SessionManager`.
    pub fn create_tool_token(&self, tool_name: &str, call_id: &str) -> String {
        self.session_manager.create_token(tool_name, call_id)
    }

    /// Validate a per-call SWAIG-function token. Returns `false` when the
    /// function is not registered or when the `SessionManager` rejects the
    /// token. Rust's
    /// `SessionManager::validate_token` returns `bool` (no panics on bad
    /// input — see `security/session_manager.rs`), so no try/catch is
    /// required.
    pub fn validate_tool_token(&self, function_name: &str, token: &str, call_id: &str) -> bool {
        if !self.service.has_function(function_name) {
            return false;
        }
        self.session_manager
            .validate_token(function_name, call_id, token)
    }

    /// Enforce `secure` for ONE SWAIG call, independent of transport.
    ///
    /// A tool registered with `secure = true` REQUIRES a valid `__token`. An
    /// ABSENT token is refused exactly like a forged one — omitting the
    /// credential must never be weaker than presenting a wrong one, or
    /// `secure` would be a flag that permits anonymous calls. A token can only
    /// be checked against a `call_id`, so a missing `call_id` counts as
    /// unvalidated rather than as a bypass.
    ///
    /// Takes three nullable strings and no transport type, so the HTTP server
    /// and all four serverless envelopes reach the identical decision.
    ///
    /// Returns `None` to proceed, or the refusal to return instead. The
    /// refusal is a `FunctionResult` body served with HTTP 200, never an error
    /// status: the engine has no handling for a SWAIG refusal status, so the
    /// tool reports that it cannot execute and the model relays it.
    pub(crate) fn swaig_validate_token(
        &self,
        function_name: &str,
        token: Option<&str>,
        call_id: Option<&str>,
    ) -> Option<FunctionResult> {
        // Unknown function: not this check's business — dispatch reports it.
        let tool = self.get_function(function_name)?;
        if !tool.secure {
            return None;
        }
        let is_valid = match (token, call_id) {
            (Some(t), Some(c)) if !t.is_empty() => self.validate_tool_token(function_name, t, c),
            _ => false,
        };
        if is_valid {
            return None;
        }
        Some(FunctionResult::with_response(
            "I'm sorry, the security token for this function is invalid \
             or expired. I cannot execute this action.",
        ))
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Prompt Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Set the agent's system prompt as raw text.
    ///
    /// Renders as `ai.prompt.text` in the SWML document. This is the
    /// alternative to the structured POM path — if POM sections have been
    /// added, `ai.prompt.pom` is emitted instead and this text is unused.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_prompt_text(&mut self, text: &str) -> &mut Self {
        self.prompt_text = text.to_string();
        self
    }

    /// Set the post-prompt: the instruction the AI follows *after* the
    /// conversation ends, typically to produce a summary.
    ///
    /// Renders as `ai.post_prompt.text`. The whole `post_prompt` block is
    /// omitted from the SWML when `text` is empty. The resulting summary is
    /// POSTed to the agent's `/post_prompt` endpoint, where it reaches the
    /// handler registered with [`on_summary`](AgentBase::on_summary).
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_post_prompt(&mut self, text: &str) -> &mut Self {
        self.post_prompt = text.to_string();
        self
    }

    /// Add a top-level POM section with an optional body and bullets.
    pub fn prompt_add_section(&mut self, title: &str, body: &str, bullets: Vec<&str>) -> &mut Self {
        self.use_pom = true;
        let mut section = Map::new();
        section.insert("title".to_string(), json!(title));
        // POM parity (Python `Section.to_dict`): an empty body is OMITTED, not
        // emitted as `"body": ""`. A section may carry bullets with no body.
        if !body.is_empty() {
            section.insert("body".to_string(), json!(body));
        }
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
        // #182: auto-create the parent section if it does not exist yet, matching
        // the TS reference (`addSubsection` calls `addSection(parentTitle)` when
        // missing) — previously this was a no-op for an unknown parent.
        if !self.prompt_has_section(parent_title) {
            self.prompt_add_section(parent_title, "", Vec::new());
        }
        for section in &mut self.pom_sections {
            if let Value::Object(map) = section
                && map.get("title").and_then(|t| t.as_str()) == Some(parent_title)
            {
                let subsections = map
                    .entry("subsections".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = subsections {
                    arr.push(json!({"title": title, "body": body}));
                }
                break;
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
        // #182: auto-create the section if it does not exist yet, matching the TS
        // reference (`addToSection` calls `addSection(title)` when missing) —
        // previously this was a no-op for an unknown section.
        if !self.prompt_has_section(title) {
            self.prompt_add_section(title, "", Vec::new());
        }
        for section in &mut self.pom_sections {
            if let Value::Object(map) = section
                && map.get("title").and_then(|t| t.as_str()) == Some(title)
            {
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

    /// Read-only snapshot of the agent's POM as a typed
    /// [`PromptObjectModel`]. Returns `None` when `use_pom` is `false` (mirroring
    /// Python's `self.pom = None`); otherwise returns a freshly built
    /// [`PromptObjectModel`] populated from the agent's stored
    /// section list.
    ///
    /// Returning a typed POM (rather than the raw `Vec<Value>`) lets
    /// callers reach for `render_markdown` / `render_xml` / `to_json`
    /// directly without re-implementing the renderers — matching
    /// Python's `agent.pom.render_markdown()` ergonomics.
    pub fn pom(&self) -> Option<crate::pom::PromptObjectModel> {
        if !self.use_pom {
            return None;
        }
        // Build a PromptObjectModel from the stored section dicts.
        // pom_sections is always a JSON array of section objects; on
        // the off chance the stored shape is invalid (caller passed
        // garbage to set_prompt_pom) fall back to an empty model so
        // we never panic from inside the public accessor.
        let arr = Value::Array(self.pom_sections.clone());
        crate::pom::PromptObjectModel::from_value(&arr)
            .ok()
            .or_else(|| Some(crate::pom::PromptObjectModel::new()))
    }

    /// Returns the post-prompt text whatever `set_post_prompt` stored, or
    /// `None` when no post-prompt has been set.
    ///
    /// Mirrors Python's `PromptManager.get_post_prompt` /
    /// `PromptMixin.get_post_prompt` — used by SWML rendering when a
    /// post-prompt is configured.
    pub fn get_post_prompt(&self) -> Option<&str> {
        if self.post_prompt.is_empty() {
            None
        } else {
            Some(&self.post_prompt)
        }
    }

    /// Returns the raw prompt text whatever `set_prompt_text` stored, or
    /// `None` when no raw prompt has been set. Distinct from `get_prompt`
    /// which may return the POM array when `use_pom` is `true`.
    ///
    /// Mirrors Python's `PromptManager.get_raw_prompt`.
    pub fn get_raw_prompt(&self) -> Option<&str> {
        if self.prompt_text.is_empty() {
            None
        } else {
            Some(&self.prompt_text)
        }
    }

    /// Sets the prompt as a list of POM section objects. Each section
    /// supports keys "title", "body", "bullets", "numbered",
    /// `"numbered_bullets"`, and "subsections". Switches the agent to POM
    /// mode.
    ///
    /// Mirrors Python's `PromptManager.set_prompt_pom` — accepts a list
    /// of section dicts and stores them in `pom_sections`.
    pub fn set_prompt_pom(&mut self, pom: Vec<Value>) -> &mut Self {
        self.use_pom = true;
        self.pom_sections = pom;
        self
    }

    /// Returns the contexts dictionary as a serialised `Value::Object`,
    /// or `None` when no contexts have been defined yet.
    ///
    /// Mirrors Python's `PromptManager.get_contexts` which returns the
    /// contexts dict or `None`.
    pub fn get_contexts(&self) -> Option<Value> {
        self.context_builder
            .as_ref()
            .map(super::super::contexts::context_builder::ContextBuilder::to_value)
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Tool Methods
    // ══════════════════════════════════════════════════════════════════════

    // Register a SWAIG tool (function) that the AI can invoke during a
    // call.
    //
    // # How this becomes a tool the model sees
    //
    // A SWAIG function is **exactly the same concept** as a "tool" in
    // native OpenAI / Anthropic tool calling. On every LLM turn, the
    // SDK renders each registered SWAIG function into the OpenAI tool
    // schema:
    //
    // ```text
    // {
    //   "type": "function",
    //   "function": {
    //     "name":        "your_name_here",
    //     "description": "your description text",
    //     "parameters":  { ... your JSON schema ... }
    //   }
    // }
    // ```
    //
    // That schema is sent to the model as part of the same API call
    // that produces the next assistant message. The model reads:
    //
    //   - the function `description` to decide WHEN to call this tool
    //   - each parameter `description` (inside `parameters`) to decide
    //     HOW to fill in that argument from the user's utterance
    //
    // This means **descriptions are prompt engineering**, not developer
    // comments. A vague description is the #1 cause of "the model has
    // the right tool but doesn't call it" failures.
    //
    // # Bad vs good descriptions
    //
    // ```text
    // BAD : description: "Lookup function"
    // GOOD: description: "Look up a customer's account details by "
    //                  + "account number. Use this BEFORE quoting any "
    //                  + "account-specific info (balance, plan, status). "
    //                  + "Do not use for general product questions."
    //
    // BAD : parameters: json!({"id": {"type": "string", "description": "the id"}})
    // GOOD: parameters: json!({"account_number": {"type": "string",
    //         "description": "The customer's 8-digit account number, "
    //                       "no dashes or spaces. Ask the user if they "
    //                       "don't provide it."}})
    // ```
    //
    // # Tool count matters
    //
    // LLM tool selection accuracy degrades past ~7-8
    // simultaneously-active tools per call. Use
    // [`crate::contexts::Step::set_functions`] to partition tools
    // across steps so only the relevant subset is active at any moment.
    // define_tool, register_swaig_function, define_tools, on_function_call
    // are provided by swml::Service and accessible on AgentBase via the
    // `Deref<Target=Service>` impl. No agent-level wrapping is needed:
    // tools registered via `agent.define_tool(...)` and via
    // `service.define_tool(...)` go to the same registry.

    /// Convenience: register multiple raw SWAIG function descriptors.
    /// Wraps the inherited `register_swaig_function` for each.
    pub fn define_tools(&mut self, tool_defs: Vec<Value>) -> &mut Self {
        for def in tool_defs {
            self.register_swaig_function(def);
        }
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  AI Config Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Add one speech-recognition hint.
    ///
    /// Hints bias the ASR toward words it would otherwise mis-transcribe —
    /// product names, jargon, proper nouns. They accumulate and render as
    /// string entries in the `ai.hints` array, alongside any structured
    /// pattern hints; the array is omitted entirely when empty.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_hint(&mut self, hint: &str) -> &mut Self {
        self.hints.push(hint.to_string());
        self
    }

    /// Add several speech-recognition hints at once.
    ///
    /// Equivalent to calling [`add_hint`](AgentBase::add_hint) for each
    /// entry; hints are appended, not replaced.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_hints(&mut self, hints: Vec<&str>) -> &mut Self {
        for h in hints {
            self.hints.push(h.to_string());
        }
        self
    }

    /// Add a structured pattern hint. Rust's builder idiom seeds the entry from `pattern`
    /// (used as both the initial `hint` and `pattern`, `replace` defaults to
    /// the pattern, `ignore_case` false); refine with `set_pattern_hint_hint`
    /// / `set_pattern_hint_replace` / `set_pattern_hint_ignore_case`, which
    /// mutate the most-recently-added pattern hint. The rendered SWML carries
    /// the full structured object, not a bare string.
    pub fn add_pattern_hint(&mut self, pattern: &str) -> &mut Self {
        self.pattern_hints.push(json!({
            "hint": pattern,
            "pattern": pattern,
            "replace": pattern,
            "ignore_case": false,
        }));
        self
    }

    /// Set the `hint` (the text to match) on the most-recently-added pattern
    /// hint. No-op if none has been added.
    pub fn set_pattern_hint_hint(&mut self, hint: &str) -> &mut Self {
        if let Some(obj) = self.pattern_hints.last_mut().and_then(Value::as_object_mut) {
            obj.insert("hint".to_string(), json!(hint));
        }
        self
    }

    /// Set the `replace` (replacement text) on the most-recently-added
    /// pattern hint. No-op if none has been added.
    pub fn set_pattern_hint_replace(&mut self, replace: &str) -> &mut Self {
        if let Some(obj) = self.pattern_hints.last_mut().and_then(Value::as_object_mut) {
            obj.insert("replace".to_string(), json!(replace));
        }
        self
    }

    /// Set the `ignore_case` flag on the most-recently-added pattern hint.
    /// No-op if none has been added.
    pub fn set_pattern_hint_ignore_case(&mut self, ignore_case: bool) -> &mut Self {
        if let Some(obj) = self.pattern_hints.last_mut().and_then(Value::as_object_mut) {
            obj.insert("ignore_case".to_string(), json!(ignore_case));
        }
        self
    }

    /// Add a language configuration. Rust's
    /// builder idiom takes the core three args here and attaches the optional
    /// `engine` / `model` / `speech_fillers` / `function_fillers` / `params`
    /// via the fluent `set_language_*` setters (or the combined
    /// `engine.voice:model` string form Python also parses). All of these
    /// survive into the rendered SWML `ai.languages` entry.
    pub fn add_language(&mut self, name: &str, code: &str, voice: &str) -> &mut Self {
        let mut language = serde_json::Map::new();
        language.insert("name".to_string(), json!(name));
        language.insert("code".to_string(), json!(code));

        // Parse the combined "engine.voice:model" string form (Python parity).
        if voice.contains('.') && voice.contains(':') {
            if let Some((engine_voice, model_part)) = voice.split_once(':')
                && let Some((engine_part, voice_part)) = engine_voice.split_once('.')
            {
                language.insert("voice".to_string(), json!(voice_part));
                language.insert("engine".to_string(), json!(engine_part));
                language.insert("model".to_string(), json!(model_part));
            } else {
                language.insert("voice".to_string(), json!(voice));
            }
        } else {
            language.insert("voice".to_string(), json!(voice));
        }

        self.languages.push(Value::Object(language));
        self
    }

    /// Set the TTS `engine` on the most-recently-added language. No-op if none
    /// has been added.
    pub fn set_language_engine(&mut self, engine: &str) -> &mut Self {
        if let Some(obj) = self.languages.last_mut().and_then(Value::as_object_mut) {
            obj.insert("engine".to_string(), json!(engine));
        }
        self
    }

    /// Set the TTS `model` on the most-recently-added language. No-op if none
    /// has been added.
    pub fn set_language_model(&mut self, model: &str) -> &mut Self {
        if let Some(obj) = self.languages.last_mut().and_then(Value::as_object_mut) {
            obj.insert("model".to_string(), json!(model));
        }
        self
    }

    /// Attach filler phrases to the most-recently-added language: if both
    /// `speech_fillers` and
    /// `function_fillers` are given they are emitted as separate keys;
    /// if only one is given it goes to the deprecated combined `fillers` key.
    /// No-op if no language has been added.
    pub fn set_language_fillers(
        &mut self,
        speech_fillers: Option<Vec<&str>>,
        function_fillers: Option<Vec<&str>>,
    ) -> &mut Self {
        if let Some(obj) = self.languages.last_mut().and_then(Value::as_object_mut) {
            match (speech_fillers, function_fillers) {
                (Some(speech), Some(func)) => {
                    obj.insert("speech_fillers".to_string(), json!(speech));
                    obj.insert("function_fillers".to_string(), json!(func));
                }
                (Some(fillers), None) | (None, Some(fillers)) => {
                    obj.insert("fillers".to_string(), json!(fillers));
                }
                (None, None) => {}
            }
        }
        self
    }

    /// Set (or replace) the per-language `params` dict on an already-added
    /// language. Mirrors Python's `AIConfigMixin.set_language_params` —
    /// engine-specific tuning (voice stability/similarity, model knobs,
    /// etc.) can be attached after the language entry was created.
    ///
    /// Behavior:
    ///   - If `params` is a non-empty JSON object, store it under the
    ///     `params` key on the matching language entry (replacing any
    ///     prior value).
    ///   - If `params` is an empty object (or any non-object value),
    ///     remove the `params` key (treated as unset).
    ///   - If no language with the given code exists, this is a no-op.
    ///   - Returns `&mut Self` for chaining.
    ///
    /// object's `params` key in SWML and use `snake_case` wire shape.
    pub fn set_language_params(&mut self, code: &str, params: Value) -> &mut Self {
        for language in &mut self.languages {
            if let Some(obj) = language.as_object_mut()
                && obj.get("code").and_then(|v| v.as_str()) == Some(code)
            {
                let non_empty = match &params {
                    Value::Object(m) => !m.is_empty(),
                    _ => false,
                };
                if non_empty {
                    obj.insert("params".to_string(), params);
                } else {
                    obj.remove("params");
                }
                break;
            }
        }
        self
    }

    /// Read the per-language `params` dict for a previously-added
    /// language. Mirrors Python's `AIConfigMixin.get_language_params`.
    ///
    /// Returns `Some(&Value)` (always a JSON object) when params were set,
    /// `None` otherwise — including when the language code is unknown.
    /// No error path.
    pub fn get_language_params(&self, code: &str) -> Option<&Value> {
        for language in &self.languages {
            if language.get("code").and_then(|v| v.as_str()) == Some(code) {
                return language.get("params");
            }
        }
        None
    }

    /// Replace the whole language list with `languages`.
    ///
    /// Each entry is a raw language object as it will appear in the
    /// `ai.languages` array (`name`, `code`, `voice`, optional `params`, …).
    /// Unlike [`add_language`](AgentBase::add_language) this **replaces**
    /// rather than appends, and performs no shape validation — the caller
    /// owns the wire shape. The array is omitted from the SWML when empty.
    ///
    /// Mutually exclusive with
    /// [`set_multilingual`](AgentBase::set_multilingual): if both are
    /// configured the server honours `multilingual` and ignores `languages`.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_languages(&mut self, languages: Vec<Value>) -> &mut Self {
        self.languages = languages;
        self
    }

    /// Configure ASR-driven multilingual mode (Mode B).
    ///
    /// Emits a top-level `multilingual` object on the AI verb: the recognizer
    /// runs in code-switching mode and the agent answers in whatever language
    /// the caller actually spoke. Mutually exclusive with [`set_languages`] —
    /// if both are set the server uses `multilingual` and ignores `languages`.
    ///
    pub fn set_multilingual(&mut self, config: Value) -> &mut Self {
        if config.is_object() {
            self.multilingual = Some(config);
        }
        self
    }

    /// Add a pronunciation rule. Mirrors Python
    /// `add_pronunciation(replace, with_text, ignore_case=False)`: the SWML
    /// wire keys are `replace`, `with`, and `ignore_case` (a bool, emitted
    /// only when true — matches signalwire-agents schema.json `Pronounce`).
    /// `ignore_case` is `Option<bool>` because the reference declares it
    /// optional (`ignore_case: bool = False`); `None` takes `false`.
    pub fn add_pronunciation(
        &mut self,
        replace: &str,
        with: &str,
        ignore_case: Option<bool>,
    ) -> &mut Self {
        let ignore_case = ignore_case.unwrap_or(false);
        let mut entry = Map::new();
        entry.insert("replace".to_string(), json!(replace));
        entry.insert("with".to_string(), json!(with));
        if ignore_case {
            entry.insert("ignore_case".to_string(), json!(true));
        }
        self.pronunciations.push(Value::Object(entry));
        self
    }

    /// Replace the whole pronunciation list with `pronunciations`.
    ///
    /// Each entry is a raw object as it will appear in the `ai.pronounce`
    /// array (`replace`, `with`, and `ignore_case` only when `true`).
    /// Unlike [`add_pronunciation`](AgentBase::add_pronunciation) this
    /// **replaces** rather than appends and does no shape validation — the
    /// caller owns the wire shape. The array is omitted when empty.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_pronunciations(&mut self, pronunciations: Vec<Value>) -> &mut Self {
        self.pronunciations = pronunciations;
        self
    }

    /// Set a single AI parameter, overwriting any previous value for `key`.
    ///
    /// Parameters land in the `ai.params` object and tune engine behaviour
    /// (timeouts, barge-in, verbosity, and so on). Other parameters already
    /// set are left untouched.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_param(&mut self, key: &str, value: Value) -> &mut Self {
        self.params.insert(key.to_string(), value);
        self
    }

    /// Replace the entire `ai.params` object with `params`.
    ///
    /// Any parameters previously set with
    /// [`set_param`](AgentBase::set_param) are discarded. A `params` value
    /// that is not a JSON object is **ignored silently** — the existing map
    /// is left as it was.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_params(&mut self, params: Value) -> &mut Self {
        if let Value::Object(map) = params {
            self.params = map;
        }
        self
    }

    /// Merge `data` into the agent's global data.
    ///
    /// Despite the `set_` name this is a **shallow merge**, not a replace:
    /// keys in `data` overwrite same-named keys and every other key
    /// survives. This matches the TypeScript reference, whose `setGlobalData`
    /// merges exactly like `updateGlobalData` (issue #190) — so
    /// [`update_global_data`](AgentBase::update_global_data) is a synonym
    /// here, not a different operation.
    ///
    /// Global data renders as the `ai.global_data` object and is visible to
    /// every SWAIG function invocation on the call. A `data` value that is
    /// not a JSON object is ignored silently.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_global_data(&mut self, data: Value) -> &mut Self {
        // #190: merge incoming keys over existing global data (shallow), matching
        // the TS reference (`setGlobalData` shallow-merges like `updateGlobalData`)
        // rather than replacing the whole map.
        if let Value::Object(map) = data {
            for (k, v) in map {
                self.global_data.insert(k, v);
            }
        }
        self
    }

    /// Shallow-merge `data` into the agent's global data.
    ///
    /// Keys present in `data` overwrite same-named existing keys; all other
    /// keys are preserved. Nested objects are replaced wholesale, not merged
    /// recursively. A non-object `data` is ignored silently.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn update_global_data(&mut self, data: Value) -> &mut Self {
        if let Value::Object(map) = data {
            for (k, v) in map {
                self.global_data.insert(k, v);
            }
        }
        self
    }

    /// The accumulated global-data map, as a JSON object.
    #[must_use]
    pub fn get_global_data(&self) -> Value {
        Value::Object(self.global_data.clone())
    }

    /// Replace the list of server-side native functions the AI may call.
    ///
    /// Native functions execute on SignalWire's infrastructure rather than
    /// against this agent's `/swaig` webhook, so they need no handler here.
    /// The list renders as `ai.SWAIG.native_functions`; passing an empty
    /// vector clears it and the key is omitted from the SWML.
    ///
    /// This **replaces** the whole list rather than appending.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_native_functions(&mut self, functions: Vec<&str>) -> &mut Self {
        self.native_functions = functions
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect();
        self
    }

    /// The complete set of internal SWAIG function names that accept
    /// fillers, matching the `SWAIGInternalFiller` schema definition.
    ///
    /// Any name outside this set is silently ignored by the runtime —
    /// [`Self::set_internal_fillers_map`] and
    /// [`Self::add_internal_filler_for`] warn if you pass an unknown
    /// name.
    ///
    /// Notable absences: `change_step`, `gather_submit`, and arbitrary
    /// user-defined SWAIG function names are NOT supported.
    pub const SUPPORTED_INTERNAL_FILLER_NAMES: &'static [&'static str] = &[
        "hangup",                  // AI is hanging up the call
        "check_time",              // AI is checking the time
        "wait_for_user",           // AI is waiting for user input
        "wait_seconds",            // deliberate pause / wait period
        "adjust_response_latency", // AI is adjusting response timing
        "next_step",               // transitioning between steps in prompt.contexts
        "change_context",          // switching between contexts in prompt.contexts
        "get_visual_input",        // processing visual input (enable_vision)
        "get_ideal_strategy",      // thinking (enable_thinking)
    ];

    /// Replace the flat list of internal filler phrases.
    ///
    /// These are short phrases the agent speaks while an internal/native
    /// function runs, so the caller does not hear dead air. This flat form
    /// renders as the `ai.params.internal_fillers` array and applies to
    /// internal work generally — it is **not** the per-function,
    /// per-language form; for that, use
    /// [`set_internal_fillers_map`](AgentBase::set_internal_fillers_map),
    /// which is stored and rendered separately.
    ///
    /// Replaces the whole list; the key is omitted when the list is empty.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_internal_fillers(&mut self, fillers: Vec<&str>) -> &mut Self {
        self.internal_fillers = fillers
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect();
        self
    }

    /// Set internal fillers for native SWAIG functions (structured form).
    ///
    /// Internal fillers are short phrases the AI agent speaks (via TTS)
    /// while an internal/native function is running, so the caller
    /// doesn't hear dead air during transitions or background work.
    ///
    /// Supported function names (match the `SWAIGInternalFiller` schema):
    /// `hangup`, `check_time`, `wait_for_user`, `wait_seconds`,
    /// `adjust_response_latency`, `next_step`, `change_context`,
    /// `get_visual_input`, `get_ideal_strategy`. See
    /// [`Self::SUPPORTED_INTERNAL_FILLER_NAMES`].
    ///
    /// Notably NOT supported: `change_step`, `gather_submit`, or
    /// arbitrary user-defined SWAIG function names. The runtime only
    /// honors fillers for the names listed above; everything else is
    /// silently ignored at the SWML level. This method warns at
    /// registration time if you pass an unknown name so you catch the
    /// typo early.
    pub fn set_internal_fillers_map(
        &mut self,
        fillers: HashMap<String, HashMap<String, Vec<String>>>,
    ) -> &mut Self {
        let mut unknown: Vec<String> = fillers
            .keys()
            .filter(|k| !Self::SUPPORTED_INTERNAL_FILLER_NAMES.contains(&k.as_str()))
            .cloned()
            .collect();
        unknown.sort();
        if !unknown.is_empty() {
            log::warn!(
                "unknown_internal_filler_names: {:?}. set_internal_fillers_map received \
                 names that the SWML schema does not recognize. Those entries will be \
                 ignored by the runtime. Supported names: {:?}",
                unknown,
                Self::SUPPORTED_INTERNAL_FILLER_NAMES
            );
        }
        self.internal_fillers_map = fillers;
        self
    }

    /// Append one phrase to the flat internal-filler list.
    ///
    /// Accumulates onto whatever
    /// [`set_internal_fillers`](AgentBase::set_internal_fillers) established;
    /// the combined list renders as `ai.params.internal_fillers`. No name
    /// validation applies here — this form is not keyed by function name.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_internal_filler(&mut self, filler: &str) -> &mut Self {
        self.internal_fillers.push(filler.to_string());
        self
    }

    /// Add internal fillers for a single internal function and language.
    ///
    /// See [`Self::set_internal_fillers_map`] for the complete list of
    /// supported `function_name` values
    /// ([`Self::SUPPORTED_INTERNAL_FILLER_NAMES`]) and what fillers do.
    /// Names outside the supported set log a warning.
    pub fn add_internal_filler_for(
        &mut self,
        function_name: &str,
        language_code: &str,
        fillers: Vec<String>,
    ) -> &mut Self {
        if !Self::SUPPORTED_INTERNAL_FILLER_NAMES.contains(&function_name) {
            log::warn!(
                "unknown_internal_filler_name: '{}'. add_internal_filler_for received a \
                 function name the SWML schema does not recognize. The entry will be \
                 stored but the runtime will not play these fillers. Supported names: {:?}",
                function_name,
                Self::SUPPORTED_INTERNAL_FILLER_NAMES
            );
        }
        self.internal_fillers_map
            .entry(function_name.to_string())
            .or_default()
            .insert(language_code.to_string(), fillers);
        self
    }

    /// Enable the debug-event webhook at `level`.
    ///
    /// `level` is `Option<i64>` because the reference declares it optional
    /// (`level: int = 1`): `None` is the omit-it call and, exactly like the
    /// reference's no-arg call, enables debug events at the base tier `1`.
    ///
    /// The level is an INTEGER tier, not a label: the reference emits it as
    /// `params.debug_webhook_level` (`core/agent_base.py:1259`) and this port's
    /// own `schema.json` types `debug_webhook_level` as `integer`.
    pub fn enable_debug_events(&mut self, level: Option<i64>) -> &mut Self {
        self.debug_events_level = Some(level.unwrap_or(1));
        self
    }

    /// Append one remote SWAIG function-include entry.
    ///
    /// An include points the AI at SWAIG functions hosted somewhere other
    /// than this agent: the object carries a `url` and the `functions` array
    /// naming which of that endpoint's functions to expose. Includes render
    /// into `ai.SWAIG.includes`.
    ///
    /// Unlike [`set_function_includes`](AgentBase::set_function_includes),
    /// this appends and applies **no** well-formedness filtering — a
    /// malformed entry added here reaches the wire as given.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_function_include(&mut self, include: Value) -> &mut Self {
        self.function_includes.push(include);
        self
    }

    /// Replace the SWAIG function-include list with `includes`.
    ///
    /// Entries are filtered to the well-formed ones — a non-empty string
    /// `url` **and** an array `functions` — matching the TypeScript
    /// reference's `inc.url && Array.isArray(inc.functions)` check (issue
    /// #191). Each rejected entry is logged rather than silently dropped;
    /// the filtering is log-only and wire-neutral.
    ///
    /// The surviving entries render into `ai.SWAIG.includes`.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_function_includes(&mut self, includes: Vec<Value>) -> &mut Self {
        // #191: keep only well-formed entries — a non-empty string `url` AND an
        // array `functions` — matching the TS reference's filter
        // (`inc.url && Array.isArray(inc.functions)`). Each dropped entry is
        // logged (log-only, wire-neutral) so a misconfigured include is not
        // silently discarded.
        let mut kept = Vec::with_capacity(includes.len());
        for inc in includes {
            let url_ok = inc
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty());
            let functions_ok = inc.get("functions").is_some_and(Value::is_array);
            if url_ok && functions_ok {
                kept.push(inc);
            } else {
                log::warn!(
                    "set_function_includes: dropping invalid include (needs a non-empty string `url` and an array `functions`): {inc}"
                );
            }
        }
        self.function_includes = kept;
        self
    }

    /// Merge LLM tuning parameters into the main prompt block.
    ///
    /// Keys such as `temperature`, `top_p`, and `confidence` are flattened
    /// **into** `ai.prompt` alongside `text`/`pom` — they are not nested
    /// under a sub-object.
    ///
    /// Despite the `set_` name this **merges**: repeated calls with distinct
    /// keys accumulate and a repeated key overwrites its previous value,
    /// mirroring Python's `self._prompt_llm_params.update(params)`
    /// (`ai_config_mixin.py:669`). A non-object `params` is ignored
    /// silently.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_prompt_llm_params(&mut self, params: Value) -> &mut Self {
        // MERGE, not replace — mirrors Python's
        // `self._prompt_llm_params.update(params)` (ai_config_mixin.py:669).
        // Repeated calls with distinct keys accumulate; a repeated key
        // overwrites its previous value.
        if let Value::Object(map) = params {
            for (k, v) in map {
                self.prompt_llm_params.insert(k, v);
            }
        }
        self
    }

    /// Merge LLM tuning parameters into the post-prompt block.
    ///
    /// Keys are flattened into `ai.post_prompt` alongside its `text`. The
    /// post-prompt block itself is only emitted when
    /// [`set_post_prompt`](AgentBase::set_post_prompt) supplied non-empty
    /// text, so params set here have no effect without it.
    ///
    /// **Merges** rather than replaces, mirroring Python's
    /// `self._post_prompt_llm_params.update(params)`
    /// (`ai_config_mixin.py:703`). A non-object `params` is ignored
    /// silently.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_post_prompt_llm_params(&mut self, params: Value) -> &mut Self {
        // MERGE, not replace — mirrors Python's
        // `self._post_prompt_llm_params.update(params)` (ai_config_mixin.py:703).
        if let Value::Object(map) = params {
            for (k, v) in map {
                self.post_prompt_llm_params.insert(k, v);
            }
        }
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Verb Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Append a SWML verb to run **before** the call is answered.
    ///
    /// Emitted as `{verb: config}` in `sections.main`, in insertion order,
    /// ahead of the `answer` verb — so this is where pre-answer signalling
    /// belongs. Note that verbs which need answered media (`play`, `record`)
    /// will not behave here.
    ///
    /// `verb` is the SWML verb name and `config` its parameter object; the
    /// pair is passed through verbatim without validation.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_pre_answer_verb(&mut self, verb: &str, config: Value) -> &mut Self {
        self.pre_answer_verbs.push((verb.to_string(), config));
        self
    }

    /// Append a SWML verb to run **after** the call is answered but before
    /// the `ai` verb.
    ///
    /// Emitted as `{verb: config}` in `sections.main`, in insertion order,
    /// after `answer` (and after `record_call` when recording is enabled)
    /// and immediately before the AI verb. Typical use is a greeting `play`
    /// that must complete before the AI takes the turn.
    ///
    /// `verb` and `config` are passed through verbatim without validation.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_post_answer_verb(&mut self, verb: &str, config: Value) -> &mut Self {
        self.post_answer_verbs.push((verb.to_string(), config));
        self
    }

    /// Append a SWML verb to run **after** the `ai` verb completes.
    ///
    /// Emitted as `{verb: config}` in `sections.main`, in insertion order,
    /// last in the document. This is what runs once the AI conversation
    /// ends — a transfer, a closing message, or a hangup.
    ///
    /// `verb` and `config` are passed through verbatim without validation.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_post_ai_verb(&mut self, verb: &str, config: Value) -> &mut Self {
        self.post_ai_verbs.push((verb.to_string(), config));
        self
    }

    /// Remove every pre-answer verb.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn clear_pre_answer_verbs(&mut self) -> &mut Self {
        self.pre_answer_verbs.clear();
        self
    }

    /// Remove every post-answer verb.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn clear_post_answer_verbs(&mut self) -> &mut Self {
        self.post_answer_verbs.clear();
        self
    }

    /// Remove every post-AI verb.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn clear_post_ai_verbs(&mut self) -> &mut Self {
        self.post_ai_verbs.clear();
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Context Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Return the `ContextBuilder`, creating it lazily on first access.
    ///
    /// The builder's tool-name supplier is set to a snapshot of the
    /// currently registered tool names so [`ContextBuilder::validate`]
    /// can check for collisions with reserved native tool names
    /// (`next_step`, `change_context`, `gather_submit`). Tools added to
    /// the agent after the first `define_contexts()` call will not be
    /// included in that snapshot — call [`AgentBase::refresh_context_tools`]
    /// to update it, or call `define_contexts` only after defining all
    /// tools.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the internal `.unwrap()` reads back the
    /// `context_builder` that is initialised to `Some` immediately above when
    /// it was `None`, so it is always `Some` by that point.
    pub fn define_contexts(&mut self) -> &mut ContextBuilder {
        let tool_names: Vec<String> = self.tool_order.clone();
        if self.context_builder.is_none() {
            let mut cb = ContextBuilder::new();
            cb.attach_tool_name_supplier(move || tool_names.clone());
            self.context_builder = Some(cb);
        }
        self.context_builder.as_mut().unwrap()
    }

    /// Refresh the `ContextBuilder`'s tool-name supplier with the current
    /// list of registered SWAIG tools. Call this if you define new tools
    /// after the first `define_contexts()` call and want the next
    /// `validate()` to see them.
    pub fn refresh_context_tools(&mut self) -> &mut Self {
        let tool_names: Vec<String> = self.tool_order.clone();
        if let Some(ref mut cb) = self.context_builder {
            cb.attach_tool_name_supplier(move || tool_names.clone());
        }
        self
    }

    /// Return the names of every registered SWAIG tool in insertion order.
    pub fn list_tool_names(&self) -> Vec<String> {
        self.tool_order.clone()
    }

    /// Remove all contexts, returning the agent to a no-contexts state.
    /// This is a convenience wrapper around `define_contexts().reset()`.
    /// Use it in a dynamic config callback when you need to rebuild
    /// contexts from scratch for a specific request.
    pub fn reset_contexts(&mut self) -> &mut Self {
        if let Some(ref mut cb) = self.context_builder {
            cb.reset();
        }
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Skill Methods (stubs)
    // ══════════════════════════════════════════════════════════════════════

    /// `params` is `Option<Value>` because the reference declares it optional
    /// (`params: dict | None = None`); `None` is the omit-it call.
    pub fn add_skill(&mut self, name: &str, params: Option<Value>) -> &mut Self {
        let params = params.unwrap_or(Value::Null);
        if !self.skills.contains(&name.to_string()) {
            self.skills.push(name.to_string());
        }
        // Register built-in skill functions so they appear in rendered SWML.
        match name {
            "datetime" => {
                let tz = params
                    .get("default_timezone")
                    .or_else(|| params.get("timezone"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("UTC")
                    .to_string();
                self.register_swaig_function(json!({
                    "function": "get_current_time",
                    "purpose": "Get the current time",
                    "argument": {"type": "object", "properties": {
                        "timezone": {"type": "string", "description": "Timezone (e.g. America/New_York)"}
                    }},
                    "data_map": {
                        "expressions": [{
                            "string": "${args.timezone}",
                            "pattern": ".*",
                            "output": {"response": "The current time is now."}
                        }]
                    }
                }));
                self.register_swaig_function(json!({
                    "function": "get_current_date",
                    "purpose": "Get the current date",
                    "argument": {"type": "object", "properties": {
                        "timezone": {"type": "string", "description": "Timezone"}
                    }},
                    "data_map": {
                        "expressions": [{
                            "string": "${args.timezone}",
                            "pattern": ".*",
                            "output": {"response": "Today's date is now."}
                        }]
                    }
                }));
                let _ = tz; // suppress unused warning
            }
            "math" => {
                self.register_swaig_function(json!({
                    "function": "calculate",
                    "purpose": "Perform mathematical calculations",
                    "argument": {"type": "object", "properties": {
                        "expression": {"type": "string", "description": "Math expression to evaluate"}
                    }},
                    "data_map": {
                        "expressions": [{
                            "string": "${args.expression}",
                            "pattern": ".*",
                            "output": {"response": "The result is: calculated."}
                        }]
                    }
                }));
            }
            _ => {}
        }
        self
    }

    /// Remove `name` from the agent's list of loaded skills.
    ///
    /// A no-op when the skill was never added. Note this drops the skill
    /// from the tracking list only — tools the skill already registered on
    /// the agent are not unregistered.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn remove_skill(&mut self, name: &str) -> &mut Self {
        self.skills.retain(|s| s != name);
        self
    }

    /// The names of the skills loaded on this agent, in the order they were
    /// added.
    pub fn list_skills(&self) -> Vec<String> {
        self.skills.clone()
    }

    /// Whether a skill named `name` is loaded. The comparison is exact and
    /// case-sensitive.
    pub fn has_skill(&self, name: &str) -> bool {
        self.skills.contains(&name.to_string())
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Web / Callback Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Install a callback that reconfigures the agent per SWML request.
    ///
    /// When set, each request to the SWML endpoint **clones** the agent,
    /// invokes `callback` with the request's query params, the parsed body,
    /// the headers, and `&mut` access to that clone, then renders SWML from
    /// the clone and discards it. The agent this method was called on is
    /// never mutated by a request, so per-caller configuration cannot leak
    /// between concurrent calls.
    ///
    /// Setting a second callback replaces the first.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_dynamic_config_callback(&mut self, callback: DynamicConfigCallback) -> &mut Self {
        self.dynamic_config_callback = Some(Arc::new(callback));
        self
    }

    /// Point SWAIG function callbacks at an external `url` instead of this
    /// agent's own `/swaig` endpoint.
    ///
    /// When set, every tool with a handler emits `web_hook_url: url`
    /// **verbatim** — no per-tool `__token` is appended and no SWAIG query
    /// params are added, matching the reference (`agent_base.py:1085`).
    ///
    /// Security consequence: because no token is minted, an external
    /// webhook is not protected by the per-call token the platform would
    /// otherwise validate; the external endpoint is responsible for
    /// authenticating requests itself.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_webhook_url(&mut self, url: &str) -> &mut Self {
        self.webhook_url = Some(url.to_string());
        self
    }

    /// Send the post-prompt summary to `url` instead of this agent's own
    /// `/post_prompt` endpoint.
    ///
    /// Emitted verbatim as `ai.post_prompt_url`. When unset, the agent
    /// derives that URL from its proxy base, route, and basic-auth
    /// credentials — so overriding this means the summary POST no longer
    /// carries the agent's embedded credentials, and the receiving endpoint
    /// must authenticate the request on its own.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn set_post_prompt_url(&mut self, url: &str) -> &mut Self {
        self.post_prompt_url = Some(url.to_string());
        self
    }

    /// Override the base URL the agent advertises in the webhook URLs it
    /// emits.
    ///
    /// Use this when the agent sits behind a proxy or tunnel whose public
    /// address it cannot infer. Any trailing `/` is stripped. This base
    /// takes precedence over both `SWML_PROXY_URL_BASE` and the
    /// `X-Forwarded-*` headers.
    ///
    /// Because it is operator-supplied configuration rather than request
    /// data, it is also honoured when reconstructing the URL an inbound
    /// webhook signature was computed over — regardless of the
    /// `trust_proxy_for_signature` setting.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn manual_set_proxy_url(&mut self, url: &str) -> &mut Self {
        self.manual_proxy_url = Some(url.trim_end_matches('/').to_string());
        self
    }

    /// Merge `params` into the query string appended to the agent's
    /// generated webhook URLs.
    ///
    /// The params are appended to **every** generated endpoint URL — the
    /// `swaig` callback and the `debug_events` callback alike — matching the
    /// reference, which passes the same map to both. Keys already present
    /// are overwritten; others are preserved.
    ///
    /// Setting any query param also forces a per-tool `web_hook_url` to be
    /// emitted for tools that would otherwise fall back to the shared
    /// `SWAIG.defaults.web_hook_url`.
    ///
    /// Has no effect when [`set_webhook_url`](AgentBase::set_webhook_url)
    /// has redirected callbacks to an external URL, which is used verbatim.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn add_swaig_query_params(&mut self, params: HashMap<String, String>) -> &mut Self {
        for (k, v) in params {
            self.swaig_query_params.insert(k, v);
        }
        self
    }

    /// Remove every SWAIG webhook query param previously added.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn clear_swaig_query_params(&mut self) -> &mut Self {
        self.swaig_query_params.clear();
        self
    }

    /// Register the handler invoked when the platform POSTs the
    /// post-prompt summary to `/post_prompt`.
    ///
    /// The callback receives the summary text, the full request body, and
    /// the request headers. The summary text is read from
    /// `post_prompt_data.raw`, falling back to a top-level `summary` field,
    /// and is the empty string when neither is present. The endpoint always
    /// answers `200` whether or not a handler is registered.
    ///
    /// Setting a second handler replaces the first.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn on_summary(&mut self, callback: SummaryCallback) -> &mut Self {
        self.summary_callback = Some(Arc::new(callback));
        self
    }

    /// Register the handler invoked when the platform POSTs a debug event
    /// to `/debug_events`.
    ///
    /// The callback receives the event body and the request headers. Debug
    /// events are only delivered once
    /// [`enable_debug_events`](AgentBase::enable_debug_events) has emitted
    /// the `debug_webhook_url` / `debug_webhook_level` params into the SWML.
    ///
    /// Setting a second handler replaces the first.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn on_debug_event(&mut self, callback: DebugEventCallback) -> &mut Self {
        self.debug_event_handler = Some(Arc::new(callback));
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  SIP Methods
    // ══════════════════════════════════════════════════════════════════════

    /// Enable SIP routing for this agent.
    ///
    /// Sets the `sip_routing` AI param to `true`, which is what tells the
    /// platform to route SIP traffic addressed to this agent's registered
    /// usernames here. Register the usernames themselves with
    /// [`register_sip_username`](AgentBase::register_sip_username) or
    /// [`auto_map_sip_usernames`](AgentBase::auto_map_sip_usernames).
    ///
    /// Returns `&mut Self` for chaining.
    pub fn enable_sip_routing(&mut self) -> &mut Self {
        self.set_param("sip_routing", json!(true));
        self
    }

    /// Register a SIP `username` that should route to this agent, optionally
    /// on a specific `route`.
    ///
    /// The name is stored case-folded and deduplicated, so registering
    /// `"Bob"`, `"BOB"`, and `"bob"` collapses to one `bob` entry;
    /// [`sip_usernames`](AgentBase::sip_usernames) reads the set back.
    ///
    /// On the wire this sets the `sip_username` AI param to the username as
    /// **given** (not lower-cased), and sets `sip_route` when `route` is
    /// non-empty. Because both are single-valued params, registering several
    /// usernames leaves the last one in `sip_username` — the accumulated set
    /// is local bookkeeping.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn register_sip_username(&mut self, username: &str, route: &str) -> &mut Self {
        // Python parity (`AgentBase.register_sip_username`): accumulate the name
        // into a case-folded, deduplicated set. Registering "Bob"/"BOB"/"bob"
        // collapses to a single "bob" entry; `sip_usernames()` reads it sorted.
        self.sip_usernames.insert(username.to_lowercase());
        self.set_param("sip_username", json!(username));
        if !route.is_empty() {
            self.set_param("sip_route", json!(route));
        }
        self
    }

    /// The registered SIP usernames, lowercased and sorted.
    #[must_use]
    pub fn sip_usernames(&self) -> Vec<String> {
        self.sip_usernames.iter().cloned().collect()
    }

    /// Automatically register common SIP usernames derived from this agent's
    /// name and route.
    pub fn auto_map_sip_usernames(&mut self) -> &mut Self {
        let clean = |s: &str| -> String {
            s.to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                .collect()
        };
        let name = self.service.name().to_string();
        let route = self.service.route().to_string();
        let clean_name = clean(&name);
        if !clean_name.is_empty() {
            self.register_sip_username(&clean_name, "");
        }
        let clean_route = clean(&route);
        if !clean_route.is_empty() && clean_route != clean_name {
            self.register_sip_username(&clean_route, "");
        }
        if clean_name.len() > 3 {
            let no_vowels: String = clean_name
                .chars()
                .filter(|c| !"aeiou".contains(*c))
                .collect();
            if no_vowels != clean_name && no_vowels.len() > 2 {
                self.register_sip_username(&no_vowels, "");
            }
        }
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Naming / URL helpers (Python AgentBase parity)
    // ══════════════════════════════════════════════════════════════════════

    /// Get the agent name.
    #[must_use]
    pub fn get_name(&self) -> String {
        self.service.name().to_string()
    }

    /// Get the full URL for this agent's endpoint (host, port, route), with
    /// optional embedded basic-auth credentials.
    ///
    /// Prefers the manual proxy-URL override when set; otherwise composes from
    /// the service host/port/route.
    ///
    /// `include_auth` is `Option<bool>` because the reference declares it
    /// optional (`include_auth: bool = False`); `None` takes `false`.
    #[must_use]
    pub fn get_full_url(&self, include_auth: Option<bool>) -> String {
        let include_auth = include_auth.unwrap_or(false);
        let base = if let Some(proxy) = &self.manual_proxy_url {
            proxy.trim_end_matches('/').to_string()
        } else {
            let host = self.service.host();
            let host = if host == "0.0.0.0" { "localhost" } else { host };
            let route = self.service.route();
            format!("http://{host}:{}{route}", self.service.port())
        };
        if include_auth {
            let (user, pass) = self.service.get_basic_auth_credentials();
            if !user.is_empty() && !pass.is_empty() {
                if let Some(rest) = base.strip_prefix("http://") {
                    return format!("http://{user}:{pass}@{rest}");
                }
                if let Some(rest) = base.strip_prefix("https://") {
                    return format!("https://{user}:{pass}@{rest}");
                }
            }
        }
        base
    }

    /// Override the default SWAIG `web_hook_url`.
    pub fn set_web_hook_url(&mut self, url: &str) -> &mut Self {
        self.web_hook_url_override = Some(url.to_string());
        self
    }

    /// Configure the `answer` verb.
    pub fn add_answer_verb(&mut self, config: Option<Value>) -> &mut Self {
        self.answer_config = match config {
            Some(Value::Object(m)) => m,
            _ => Map::new(),
        };
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  MCP (Model Context Protocol) — Python AIConfigMixin parity
    // ══════════════════════════════════════════════════════════════════════

    /// Add an external MCP server for tool discovery and invocation.
    pub fn add_mcp_server(
        &mut self,
        url: &str,
        headers: Option<Map<String, Value>>,
        resources: Option<bool>,
        resource_vars: Option<Map<String, Value>>,
    ) -> &mut Self {
        // `None` is the omit-it call; the reference default is `False`.
        let resources = resources.unwrap_or(false);
        let mut server = Map::new();
        server.insert("url".to_string(), json!(url));
        if let Some(h) = headers
            && !h.is_empty()
        {
            server.insert("headers".to_string(), Value::Object(h));
        }
        if resources {
            server.insert("resources".to_string(), json!(true));
        }
        if let Some(rv) = resource_vars
            && !rv.is_empty()
        {
            server.insert("resource_vars".to_string(), Value::Object(rv));
        }
        self.mcp_servers.push(Value::Object(server));
        self
    }

    /// Expose this agent's tools as an MCP server endpoint.
    pub fn enable_mcp_server(&mut self) -> &mut Self {
        self.mcp_server_enabled = true;
        self
    }

    /// The registered external MCP servers.
    #[must_use]
    pub fn mcp_servers(&self) -> &[Value] {
        &self.mcp_servers
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Web surface — Python WebMixin parity
    // ══════════════════════════════════════════════════════════════════════

    /// Return a mountable [`axum::Router`] serving this agent's HTTP routes.
    ///
    /// Rust equivalent of Python's `WebMixin.as_router` (a FastAPI
    /// `APIRouter`): the "embed my routes in a host app" unit. The returned
    /// router can be [`axum::Router::nest`]ed into a caller's own axum/hyper
    /// application or served directly. Gated behind the default
    /// `tower-middleware` feature (which provides `axum`).
    #[cfg(feature = "tower-middleware")]
    pub fn as_router(&self) -> axum::Router {
        self.service.as_router()
    }

    /// Return the app mount identifier (Rust idiom for Python's `get_app`,
    /// which returns a FastAPI application object). The Rust port has no
    /// single baked-in web-application object, so `get_app` yields the mount
    /// route string; use [`AgentBase::as_router`] for the mountable handler.
    #[must_use]
    pub fn get_app(&self) -> String {
        self.service.route().to_string()
    }

    /// Enable debug routes.
    pub fn enable_debug_routes(&mut self) -> &mut Self {
        self.debug_routes_enabled = true;
        self
    }

    /// Whether debug routes are enabled.
    #[must_use]
    pub fn debug_routes_enabled(&self) -> bool {
        self.debug_routes_enabled
    }

    /// Start a web server for this agent (Python `WebMixin.serve`). Delegates
    /// to [`AgentBase::run`], optionally overriding host/port first.
    pub fn serve(&mut self, host: Option<&str>, port: Option<u16>) {
        if let Some(h) = host {
            self.service.set_host(h);
        }
        if let Some(p) = port {
            self.service.set_port(p);
        }
        self.run();
    }

    /// Set up graceful shutdown signal handling (Python
    /// `WebMixin.setup_graceful_shutdown`). The Rust `run` blocks
    /// synchronously; this is the entry point (a no-op placeholder
    /// until an async server backend is wired).
    pub fn setup_graceful_shutdown(&self) {}

    /// Register a routing callback for `path`.
    ///
    /// `path` is `Option<&str>` because the reference declares it optional
    /// (`path: str = "/sip"`); `None` takes `"/sip"`.
    pub fn register_routing_callback<F>(&mut self, callback: F, path: Option<&str>) -> &mut Self
    where
        F: Fn(&Value, &HashMap<String, String>) -> Option<String> + Send + Sync + 'static,
    {
        let path = path.unwrap_or("/sip");
        self.service.register_routing_callback(callback, Some(path));
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Serverless — Python ServerlessMixin parity
    // ══════════════════════════════════════════════════════════════════════

    /// Handle a request in a serverless environment. Renders the SWML document
    /// for the given (optional) request headers and returns it as a JSON
    /// string.
    #[must_use]
    pub fn handle_serverless_request(&self, headers: Option<&HashMap<String, String>>) -> String {
        let empty = HashMap::new();
        let headers = headers.unwrap_or(&empty);
        self.render_swml(headers).to_string()
    }

    /// Get the [`ContextBuilder`] for this agent (alias for
    /// [`AgentBase::define_contexts`]).
    pub fn contexts(&mut self) -> &mut ContextBuilder {
        self.define_contexts()
    }

    /// Define a SWAIG tool. The Rust idiom for Python's `@AgentBase.tool(...)`
    /// class-method decorator: Rust has no runtime method decorators, so `tool`
    /// registers a handler directly (same effect as the decorated function
    /// being registered).
    pub fn tool(
        &mut self,
        name: &str,
        description: &str,
        parameters: Value,
        handler: FunctionHandler,
        secure: bool,
    ) -> &mut Self {
        self.service
            .define_tool(name, description, parameters, handler, secure);
        self
    }

    // ══════════════════════════════════════════════════════════════════════
    //  SWML Rendering
    // ══════════════════════════════════════════════════════════════════════

    /// Build the complete SWML document for a request.
    ///
    /// Phases:
    ///   1. Pre-answer verbs
    ///   2. Answer verb (if `auto_answer`)
    ///   3. Record call verb (if `record_call`)
    ///   4. Post-answer verbs
    ///   5. AI verb (via `build_ai_verb`)
    ///   6. Post-AI verbs
    #[must_use]
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
    #[must_use]
    pub fn build_ai_verb(&self, headers: &HashMap<String, String>) -> Value {
        let mut ai = Map::new();

        // ── Prompt ──────────────────────────────────────────────────────
        let mut prompt = Map::new();
        if self.use_pom && !self.pom_sections.is_empty() {
            prompt.insert("pom".to_string(), Value::Array(self.pom_sections.clone()));
        } else if self.context_builder.is_some() && self.prompt_text.is_empty() {
            // #185: when contexts drive the conversation and no prompt text was
            // set, emit a default fallback rather than an empty string — matching
            // the TS reference (`text: prompt || "You are ${name}, a helpful AI
            // assistant."`), which applies the fallback only in the contexts
            // branch. Without contexts, an empty prompt is passed through as-is.
            prompt.insert(
                "text".to_string(),
                json!(format!(
                    "You are {}, a helpful AI assistant.",
                    self.service.name()
                )),
            );
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
                String::new()
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
            merged_params.insert("internal_fillers".to_string(), json!(self.internal_fillers));
        }
        if let Some(level) = self.debug_events_level {
            // Debug events emit a PAIR of params, exactly as the reference does
            // (`core/agent_base.py:1254-1261`): the auth-embedded webhook URL
            // built from the `debug_events` endpoint, and the INTEGER tier.
            //
            // The wire key for the tier is `debug_webhook_level` — see this
            // crate's embedded `schema.json`, which types it as `integer` and
            // has no `debug_events` param at all. `debug_events` is the webhook
            // PATH segment, never a params key.
            merged_params.insert(
                "debug_webhook_url".to_string(),
                json!(self.build_webhook_url("debug_events", headers)),
            );
            merged_params.insert("debug_webhook_level".to_string(), json!(level));
        }
        if !merged_params.is_empty() {
            ai.insert("params".to_string(), Value::Object(merged_params));
        }

        // ── Hints ───────────────────────────────────────────────────────
        let mut all_hints: Vec<Value> = self.hints.iter().map(|h| json!(h)).collect();
        for ph in &self.pattern_hints {
            all_hints.push(ph.clone());
        }
        if !all_hints.is_empty() {
            ai.insert("hints".to_string(), Value::Array(all_hints));
        }

        // ── Languages ───────────────────────────────────────────────────
        if !self.languages.is_empty() {
            ai.insert(
                "languages".to_string(),
                Value::Array(self.languages.clone()),
            );
        }

        // ── Multilingual (Mode B) — top-level `multilingual` on the AI verb.
        if let Some(ml) = &self.multilingual {
            ai.insert("multilingual".to_string(), ml.clone());
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
        if let Some(ref cb) = self.context_builder
            && cb.has_contexts()
        {
            let ctx_val = cb.to_value();
            ai.insert("context_switch".to_string(), ctx_val);
        }

        Value::Object(ai)
    }

    // ══════════════════════════════════════════════════════════════════════
    //  HTTP Handling
    // ══════════════════════════════════════════════════════════════════════

    /// Handle an HTTP request. Overrides the service handler with agent-specific
    /// logic for SWML, SWAIG dispatch, and post-prompt callbacks.
    ///
    /// `body` is `Option<&str>` to match [`SWMLService::handle_request`], which
    /// this overrides — a GET carries no body at all.
    pub fn handle_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> (u16, HashMap<String, String>, String) {
        // `unwrap_or_default()` (not `unwrap_or("")`) for the same reason as
        // `SWMLService::handle_request`: the reference's default is `null`, and
        // `unwrap_or(<literal>)` is the form the enumerator records a default from.
        let body = body.unwrap_or_default();

        // Split the query string off the path. `path` arrives as the RAW
        // request target on every transport (the built-in server hands over
        // `tiny_http`'s `request.url()`, which retains `?a=b`), so routing must
        // match on the path alone — otherwise `/swaig?__token=…` misses the
        // route and 404s, and the credential could never be read at all. The
        // parsed query is merged into the SWAIG body under `query_params`
        // below, which is where `swaig_request_token` looks for `__token`.
        let (path, raw_query) = path.split_once('?').map_or((path, ""), |(p, q)| (p, q));

        // Health/ready: delegate to service
        if path == "/health" || path == "/ready" {
            return self
                .service
                .handle_request(method, path, headers, Some(body));
        }

        // Determine sub-path relative to route
        let route = self.service.route();
        let sub_path = if route == "/" {
            Some(path.to_string())
        } else if path == route || path.starts_with(&format!("{route}/")) {
            let rest = &path[route.len()..];
            if rest.is_empty() {
                Some("/".to_string())
            } else {
                Some(rest.to_string())
            }
        } else {
            None
        };

        let Some(sub_path) = sub_path else {
            return json_response(404, &json!({"error": "Not found"}));
        };

        // Auth — framework-free contract (Python `_handle_request_core`): a
        // 401 is `(401, {"WWW-Authenticate": "Basic"}, {"error":
        // "Unauthorized"})` — JSON body, bare `WWW-Authenticate: Basic` header,
        // no Content-Type/security headers (the HTTP adapter re-adds them).
        if !self.check_auth(headers) {
            let mut resp_headers = HashMap::new();
            resp_headers.insert("WWW-Authenticate".to_string(), "Basic".to_string());
            return (
                401,
                resp_headers,
                json!({"error": "Unauthorized"}).to_string(),
            );
        }

        // Webhook signature validation. Mounted on POSTs to the
        // signed routes (`/`, `/swaig`, `/post_prompt`) when a
        // signing key is configured. Unsigned / invalid requests are
        // rejected with 403; signed-and-valid fall through to the
        // normal dispatch.
        if method.eq_ignore_ascii_case("POST")
            && matches!(sub_path.as_str(), "/" | "" | "/swaig" | "/post_prompt")
            && let Some(ref key) = self.signing_key
            && !self.verify_request_signature(key, headers, path, body)
        {
            return json_response(403, &json!({"error": "Invalid signature"}));
        }

        // Parse body
        let mut request_data: Option<Value> = if body.is_empty() {
            None
        } else {
            serde_json::from_str(body).ok()
        };

        // Merge the request's own query string into `query_params`. A caller
        // that already embedded `query_params` in the body keeps precedence
        // (that shape is what the dynamic-config callback and the existing
        // dispatch tests drive), so this only ADDS the transport-supplied
        // parameters that were previously dropped on the floor.
        if !raw_query.is_empty() {
            let parsed = parse_query_string(raw_query);
            if !parsed.is_empty() {
                let target = request_data.get_or_insert_with(|| json!({}));
                if let Some(obj) = target.as_object_mut() {
                    let existing = obj
                        .get("query_params")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    let mut merged = parsed;
                    for (k, v) in existing {
                        merged.insert(k, v);
                    }
                    obj.insert("query_params".to_string(), Value::Object(merged));
                }
            }
        }

        match sub_path.as_str() {
            "/" | "" => self.handle_swml_request(method, &request_data, headers),
            "/swaig" => self.handle_swaig_request(&request_data, headers),
            "/post_prompt" => self.handle_post_prompt(&request_data, headers),
            _ => json_response(404, &json!({"error": "Not found"})),
        }
    }

    /// Validate the SignalWire signature header against the URL the
    /// platform POSTed to and the raw body. Reconstructs the public URL via
    /// [`Self::resolve_signature_base`] plus the request path, so
    /// `SWML_PROXY_URL_BASE` / `manual_set_proxy_url` always apply but the
    /// spoofable `X-Forwarded-*` headers apply ONLY when the agent was built
    /// with `trust_proxy_for_signature = true`. Returns `false` for missing
    /// header, missing key (which shouldn't happen — caller already checked),
    /// or any validator error.
    fn verify_request_signature(
        &self,
        signing_key: &str,
        headers: &HashMap<String, String>,
        path: &str,
        raw_body: &str,
    ) -> bool {
        // Header lookup is case-insensitive in practice — try both.
        let signature = headers
            .get("X-SignalWire-Signature")
            .or_else(|| headers.get("x-signalwire-signature"))
            .or_else(|| headers.get("X-Twilio-Signature"))
            .or_else(|| headers.get("x-twilio-signature"));
        let signature = match signature {
            Some(s) => s.as_str(),
            None => return false,
        };

        let url_base = self.resolve_signature_base(headers);
        // Strip a trailing slash on the base so we don't double-up.
        let base = url_base.trim_end_matches('/');
        let full_url = format!("{base}{path}");

        crate::security::webhook::validate_webhook_signature(
            signing_key,
            signature,
            &full_url,
            raw_body,
        )
        .unwrap_or(false)
    }

    /// Create a deep copy of this agent for per-request customisation.
    #[must_use]
    pub fn clone_for_request(&self) -> Self {
        self.clone()
    }

    // ══════════════════════════════════════════════════════════════════════
    //  Private Helpers
    // ══════════════════════════════════════════════════════════════════════

    fn check_auth(&self, headers: &HashMap<String, String>) -> bool {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;

        // Delegate to service's handle_request for auth check by
        // using the service's basic_auth_credentials to validate
        let auth_header = headers
            .get("Authorization")
            .or_else(|| headers.get("authorization"));

        let Some(auth_header) = auth_header else {
            return false;
        };

        if !auth_header.starts_with("Basic ") {
            return false;
        }

        let Ok(decoded) = BASE64.decode(&auth_header[6..]) else {
            return false;
        };
        let Ok(decoded_str) = String::from_utf8(decoded) else {
            return false;
        };
        let Some(colon_pos) = decoded_str.find(':') else {
            return false;
        };
        let input_user = &decoded_str[..colon_pos];
        let input_pass = &decoded_str[colon_pos + 1..];

        let (expected_user, expected_pass) = self.service.basic_auth_credentials();
        input_user == expected_user && input_pass == expected_pass
    }

    // `request_data: &Option<Value>` (rather than the lint's preferred
    // `Option<&Value>`) is kept across this private handler trio because
    // `handle_swml_request` threads it straight into `DynamicConfigCallback`,
    // whose public signature takes `&Option<Value>` to mirror Python's
    // dynamic-config callback (which receives the possibly-absent request
    // body). Keeping the three dispatched-from-one-match handlers uniform —
    // and matching the parity callback shape — is worth the lint allow.
    #[allow(clippy::ref_option)]
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

    #[allow(clippy::ref_option)] // uniform with the handler trio; see handle_swml_request
    fn handle_swaig_request(
        &self,
        request_data: &Option<Value>,
        _headers: &HashMap<String, String>,
    ) -> (u16, HashMap<String, String>, String) {
        let Some(data) = request_data else {
            return json_response(400, &json!({"error": "Missing request body"}));
        };

        let function_name = data["function"].as_str().unwrap_or("");
        if function_name.is_empty() {
            return json_response(400, &json!({"error": "Missing function name"}));
        }

        // Enforce `secure` through the transport-agnostic core, so this path
        // and every serverless path reach the identical decision. The token
        // rides the query string (`__token`, legacy `token`); the call_id rides
        // the POST body (flat `call_id`, or nested `call.call_id`).
        let token = swaig_request_token(data);
        let call_id = data
            .get("call_id")
            .and_then(Value::as_str)
            .or_else(|| data.get("call").and_then(|c| c.get("call_id"))?.as_str());
        if let Some(refusal) = self.swaig_validate_token(function_name, token.as_deref(), call_id) {
            return json_response(200, &refusal.to_value());
        }

        let args = data["argument"]["parsed"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let raw_data = data.as_object().cloned().unwrap_or_default();

        match self.on_function_call(function_name, &args, Some(&raw_data)) {
            Some(result) => json_response(200, &result.to_value()),
            None => json_response(
                404,
                &json!({"error": format!("Unknown function: {}", function_name)}),
            ),
        }
    }

    #[allow(clippy::ref_option)] // uniform with the handler trio; see handle_swml_request
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

        // Every render is scoped to a call. The reference GENERATES a call_id when
        // the caller supplied none (`agent_base.py:958` →
        // `_session_manager.create_session()`), so a `secure` tool ALWAYS renders
        // with a per-tool token. Mirror that: mint the render's call_id up front so
        // the secure branch below always has one.
        let call_id = self.session_manager.create_session(None);

        let mut functions = Vec::new();
        for name in &self.tool_order {
            if let Some(tool) = self.tools.get(name) {
                let mut func_def = tool.definition.clone();

                // Add web_hook_url for tools with handlers
                if tool.handler.is_some() {
                    if let Some(ref wh_url) = self.webhook_url {
                        // EXTERNAL webhook: the reference uses the provided URL
                        // verbatim and never appends a token (agent_base.py:1085).
                        if let Value::Object(map) = &mut func_def {
                            map.insert("web_hook_url".to_string(), json!(wh_url));
                        }
                    } else if tool.secure || !self.swaig_query_params.is_empty() {
                        // LOCAL webhook, emitted ONLY when there is something to
                        // carry: a per-tool `__token` (the WIRE manifestation of
                        // `secure` the platform validates on the callback) or the
                        // agent's SWAIG query params. Mirrors the reference's
                        // `elif token or agent_to_use._swaig_query_params:`
                        // (agent_base.py:1087).
                        //
                        // An INSECURE tool with no query params gets NO
                        // `web_hook_url` key AT ALL — not an empty string, not a
                        // tokenless URL. It falls back to the shared
                        // `SWAIG.defaults.web_hook_url`. Emitting a per-tool
                        // callback here would publish an UNAUTHENTICATED,
                        // function-specific endpoint.
                        let mut url = self.build_swaig_webhook_url(headers);
                        if tool.secure {
                            let token = self.create_tool_token(name, &call_id);
                            let sep = if url.contains('?') { '&' } else { '?' };
                            url = format!("{url}{sep}__token={token}");
                        }
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
            // The SHARED fallback callback, emitted WHENEVER functions exist
            // (reference agent_base.py:1109-1113). This is what an INSECURE tool
            // — which correctly carries no per-tool `web_hook_url` — actually
            // dispatches to; without it such a tool would render with NO
            // reachable callback at all.
            //
            // Composed like the reference (agent_base.py:972-979): the local
            // `/swaig` URL carrying the agent's SWAIG query params, replaced
            // wholesale by the `set_web_hook_url` override when one is set.
            let default_webhook_url = self
                .web_hook_url_override
                .clone()
                .unwrap_or_else(|| self.build_swaig_webhook_url(headers));
            swaig.insert(
                "defaults".to_string(),
                json!({"web_hook_url": default_webhook_url}),
            );
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
        self.build_webhook_url("swaig", headers)
    }

    /// Build the auth-embedded webhook URL for `endpoint`, the port's
    /// `_build_webhook_url(endpoint, query_params)`
    /// (`core/swml_service.py:1615`). The `swaig` query params are appended for
    /// every endpoint, matching the reference — it passes the same
    /// `_swaig_query_params` copy to the `swaig` and `debug_events` calls alike.
    fn build_webhook_url(&self, endpoint: &str, headers: &HashMap<String, String>) -> String {
        let proxy_base = self.resolve_proxy_base(headers);
        let route_segment = if self.service.route() == "/" {
            String::new()
        } else {
            self.service.route().to_string()
        };

        let (user, pass) = self.service.basic_auth_credentials();

        // Parse proxy_base to extract host/port
        let mut auth_url =
            if proxy_base.starts_with("http://") || proxy_base.starts_with("https://") {
                let proto_end = proxy_base.find("://").unwrap() + 3;
                let proto = &proxy_base[..proto_end];
                let rest = &proxy_base[proto_end..];
                format!("{proto}{user}:{pass}@{rest}{route_segment}/{endpoint}")
            } else {
                format!("http://{user}:{pass}@{proxy_base}{route_segment}/{endpoint}")
            };

        if !self.swaig_query_params.is_empty() {
            let params: Vec<String> = self
                .swaig_query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
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

    /// Base URL used to reconstruct the URL a webhook signature was computed
    /// over. Unlike [`Self::resolve_proxy_base`] — which builds the outbound
    /// `web_hook_url` we ADVERTISE and may legitimately reflect the public
    /// proxy hostname — this is a SECURITY decision on INBOUND, attacker-
    /// influenced data.
    ///
    /// `X-Forwarded-Proto` / `X-Forwarded-Host` are set by whoever can reach
    /// the agent. If we always honored them, an attacker able to reach the
    /// agent directly could compute a valid signature over a hostname of their
    /// choosing, inject it as `X-Forwarded-Host`, and have the agent validate
    /// their request. The reference therefore defaults
    /// `trust_proxy_for_signature=False` and only honors those headers when the
    /// operator has explicitly opted in ("proxy headers are spoofable, so opt
    /// in only when you control the proxy chain" —
    /// `agent_base.py` `trust_proxy_for_signature` docstring).
    ///
    /// An explicit `manual_set_proxy_url` / `SWML_PROXY_URL_BASE` is operator-
    /// supplied configuration, not request data, so both are honored either way.
    fn resolve_signature_base(&self, headers: &HashMap<String, String>) -> String {
        if let Some(ref manual) = self.manual_proxy_url {
            return manual.clone();
        }
        if self.trust_proxy_for_signature {
            return self.service.get_proxy_url_base(headers);
        }
        // Untrusted path: strip the spoofable forwarded headers before asking
        // the service to derive a base, so only operator-supplied config
        // (SWML_PROXY_URL_BASE) and the service's own host:port can win.
        let filtered: HashMap<String, String> = headers
            .iter()
            .filter(|(k, _)| {
                let k = k.to_ascii_lowercase();
                k != "x-forwarded-proto" && k != "x-forwarded-host" && k != "x-original-url"
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.service.get_proxy_url_base(&filtered)
    }

    // ══════════════════════════════════════════════════════════════════════
    //  HTTP Server
    // ══════════════════════════════════════════════════════════════════════

    /// Return `(user, password)` for the agent's basic auth.
    pub fn get_basic_auth_credentials(&self) -> (String, String) {
        let (u, p) = self.service.basic_auth_credentials();
        (u.to_string(), p.to_string())
    }

    /// Start a blocking HTTP server on the configured host:port.
    ///
    /// Serves HTTPS instead when `SWML_SSL_ENABLED` is set together with
    /// `SWML_SSL_CERT_PATH` / `SWML_SSL_KEY_PATH` (mirrors Python's
    /// `SecurityConfig` / uvicorn `ssl_*` contract).
    ///
    /// # Panics
    ///
    /// Panics if the configured `host:port` cannot be bound (e.g. the port
    /// is already in use or permission is denied).
    pub fn run(&self) {
        // Introspect paths (compiled-example CLI support, mirrors Service::run):
        // SWAIG_LIST_TOOLS=1 dumps the tool registry, SWML_DUMP=1 renders the
        // agent's SWML — both between sentinel markers, then exit before binding
        // a port. This is how `swaig-test --example <NAME> --list-tools`/
        // `--dump-swml` introspect a compiled AgentBase example in-process.
        if std::env::var("SWAIG_LIST_TOOLS").is_ok() {
            let signatures: Vec<&Value> = self
                .tool_order
                .iter()
                .filter_map(|name| self.service.tools.get(name).map(|td| &td.definition))
                .collect();
            let body = serde_json::json!({ "tools": signatures });
            println!("__SWAIG_TOOLS_BEGIN__");
            println!("{}", serde_json::to_string(&body).unwrap_or_default());
            println!("__SWAIG_TOOLS_END__");
            std::process::exit(0);
        }
        if std::env::var("SWML_DUMP").is_ok() {
            let swml = self.render_swml(&HashMap::new());
            println!("__SWML_DUMP_BEGIN__");
            println!(
                "{}",
                serde_json::to_string_pretty(&swml).unwrap_or_else(|_| swml.to_string())
            );
            println!("__SWML_DUMP_END__");
            std::process::exit(0);
        }
        let addr = format!("{}:{}", self.service.host(), self.service.port());
        let (server, _is_https) = crate::server::tls::bind_server(&addr)
            .unwrap_or_else(|e| panic!("Failed to bind {addr}: {e}"));

        for mut request in server.incoming_requests() {
            let method = request.method().as_str().to_string();
            let path = request.url().to_string();

            let mut req_headers = HashMap::new();
            for h in request.headers() {
                req_headers.insert(
                    h.field.as_str().as_str().to_string(),
                    h.value.as_str().to_string(),
                );
            }

            let mut body_buf = String::new();
            let _ = request.as_reader().read_to_string(&mut body_buf);

            let (status, resp_headers, resp_body) =
                self.handle_request(&method, &path, &req_headers, Some(&body_buf));

            let mut response =
                tiny_http::Response::from_string(&resp_body).with_status_code(status);
            for (k, v) in &resp_headers {
                if let Ok(header) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                    response = response.with_header(header);
                }
            }
            let _ = request.respond(response);
        }
    }
}

/// The per-tool security token supplied on a SWAIG callback, if any.
///
/// The platform appends the token the render minted as a `__token` query
/// parameter (see `build_swaig_block`). The reference reads
/// `query_params["__token"]`, falling back to the legacy unprefixed `token`
/// (`agent_base.py:1414`); the HTTP adapter surfaces the parsed query string on
/// the request body under `query_params`.
/// Parse a raw `a=b&c=d` query string into a JSON object of string values.
///
/// A leading `?` is tolerated so a full `?a=b` fragment parses the same as
/// `a=b`. Percent-escapes are decoded and `+` is read as a space, matching how
/// the platform encodes a minted `__token` into the webhook URL. A repeated key
/// keeps the FIRST value, and a valueless key maps to the empty string.
fn parse_query_string(raw: &str) -> Map<String, Value> {
    let mut out = Map::new();
    for pair in raw.trim_start_matches('?').split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(k);
        if key.is_empty() || out.contains_key(&key) {
            continue;
        }
        out.insert(key, Value::String(percent_decode(v)));
    }
    out
}

/// Decode `%XX` escapes and `+`-as-space in one query-string component.
/// An invalid escape is left verbatim rather than dropped.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn swaig_request_token(data: &Value) -> Option<String> {
    let params = data.get("query_params")?.as_object()?;
    params
        .get("__token")
        .or_else(|| params.get("token"))
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

/// Build a JSON HTTP response tuple.
fn json_response(status: u16, data: &Value) -> (u16, HashMap<String, String>, String) {
    let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
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

    // ──── pom() accessor (Python parity: agent.pom)
    //
    // Mirrors signalwire-python tests/unit/core/test_agent_base.py::
    //   TestAgentBasePromptMethods::test_set_prompt_pom_succeeds_when_use_pom_true

    #[test]
    fn test_pom_returns_sections_after_prompt_add_section() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Greeting", "Hello", vec![]);
        let pom = agent.pom().expect("pom must be Some when use_pom is true");
        assert_eq!(pom.sections.len(), 1);
        assert_eq!(pom.sections[0].title.as_deref(), Some("Greeting"));
        assert_eq!(pom.sections[0].body, "Hello");
    }

    #[test]
    fn test_pom_none_when_use_pom_false() {
        let mut opts = default_options();
        opts.use_pom = false;
        let agent = AgentBase::new(opts);
        assert!(
            agent.pom().is_none(),
            "pom() must return None when use_pom is false"
        );
    }

    #[test]
    fn test_pom_returns_clone_not_internal_vec() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section("Original", "Body", vec![]);

        let mut pom = agent.pom().unwrap();
        // Mutate the returned PromptObjectModel; agent state must be unaffected.
        pom.add_section_with(
            Some("Injected".to_string()),
            Some("ib".into()),
            None,
            None,
            None,
        )
        .unwrap();
        pom.sections[0].title = Some("Hijacked".to_string());

        let fresh = agent.pom().unwrap();
        assert_eq!(
            fresh.sections.len(),
            1,
            "caller mutation leaked into agent state"
        );
        assert_eq!(
            fresh.sections[0].title.as_deref(),
            Some("Original"),
            "caller mutation leaked into agent state"
        );
    }

    #[test]
    fn test_pom_with_bullets() {
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_section(
            "Rules",
            "Follow these rules:",
            vec!["Be polite", "Be helpful"],
        );
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
                FunctionResult::with_response(&format!("Hello, {name}!"))
            }),
            false,
        );

        let mut args = Map::new();
        args.insert("name".to_string(), json!("Alice"));
        let raw = Map::new();
        let result = agent.on_function_call("greet", &args, Some(&raw)).unwrap();
        assert_eq!(result.to_value()["response"], "Hello, Alice!");
    }

    #[test]
    fn test_on_function_call_unknown() {
        let agent = AgentBase::new(default_options());
        let args = Map::new();
        let raw = Map::new();
        assert!(
            agent
                .on_function_call("nonexistent", &args, Some(&raw))
                .is_none()
        );
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

    // ── Behavioral Contract 8: AI/LLM structured add_pattern_hint / add_language ──
    //
    // Python (ai_config mixin): `add_pattern_hint` attaches a STRUCTURED hint
    // ({hint, pattern, replace, ignore_case}), not a bare string; `add_language`
    // carries engine + model + fillers (list) into the rendered SWML
    // `ai.languages` entry. A degraded body (bare-string hint / no
    // engine/model/fillers) would drop these — this test asserts every field
    // survives into the rendered document. FAILS against the old bare-string
    // `pattern_hints: Vec<String>` + 3-field-only `add_language` body.
    #[test]
    fn test_contract8_structured_pattern_hint_and_language_fillers_survive_render() {
        let mut agent = AgentBase::new(default_options());

        // Structured pattern hint WITH a replacement + hint text + ignore_case.
        agent
            .add_pattern_hint("AI")
            .set_pattern_hint_hint("AI")
            .set_pattern_hint_replace("Artificial Intelligence")
            .set_pattern_hint_ignore_case(true);

        // Language WITH engine + model + fillers.
        agent
            .add_language("English", "en-US", "josh")
            .set_language_engine("elevenlabs")
            .set_language_model("eleven_turbo_v2_5")
            .set_language_fillers(
                Some(vec!["um", "let me think"]),
                Some(vec!["one moment", "checking"]),
            );

        let ai = agent.build_ai_verb(&HashMap::new());

        // ── Structured pattern hint ──────────────────────────────────────
        let hints = ai["hints"].as_array().expect("ai.hints must be present");
        let structured = hints
            .iter()
            .find(|h| h.is_object())
            .expect("pattern hint must render as a STRUCTURED object, not a bare string");
        assert_eq!(structured["hint"], "AI");
        assert_eq!(structured["pattern"], "AI");
        assert_eq!(
            structured["replace"], "Artificial Intelligence",
            "replacement must survive into the rendered hint"
        );
        assert_eq!(structured["ignore_case"], true);

        // ── Language engine + model + fillers ────────────────────────────
        let langs = ai["languages"]
            .as_array()
            .expect("ai.languages must be present");
        let lang = &langs[0];
        assert_eq!(lang["name"], "English");
        assert_eq!(lang["code"], "en-US");
        assert_eq!(lang["voice"], "josh");
        assert_eq!(
            lang["engine"], "elevenlabs",
            "engine must survive into the rendered language"
        );
        assert_eq!(
            lang["model"], "eleven_turbo_v2_5",
            "model must survive into the rendered language"
        );
        assert_eq!(
            lang["speech_fillers"],
            json!(["um", "let me think"]),
            "speech_fillers must survive"
        );
        assert_eq!(
            lang["function_fillers"],
            json!(["one moment", "checking"]),
            "function_fillers must survive"
        );
    }

    // The combined "engine.voice:model" voice string is parsed into separate
    // engine / voice / model keys (Python parity).
    #[test]
    fn test_contract8_combined_voice_string_parsed() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "elevenlabs.josh:eleven_turbo_v2_5");
        let lang = &agent.languages[0];
        assert_eq!(lang["engine"], "elevenlabs");
        assert_eq!(lang["voice"], "josh");
        assert_eq!(lang["model"], "eleven_turbo_v2_5");
    }

    // Single filler kind uses the deprecated combined `fillers` key.
    #[test]
    fn test_contract8_single_filler_kind_uses_combined_key() {
        let mut agent = AgentBase::new(default_options());
        agent
            .add_language("English", "en-US", "josh")
            .set_language_fillers(Some(vec!["um"]), None);
        let lang = &agent.languages[0];
        assert_eq!(lang["fillers"], json!(["um"]));
        assert!(lang.get("speech_fillers").is_none());
    }

    #[test]
    fn test_set_languages() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "Polly.Salli");
        agent.set_languages(vec![]);
        assert!(agent.languages.is_empty());
    }

    #[test]
    fn test_set_multilingual_emits_ai_verb_object() {
        let mut agent = AgentBase::new(default_options());
        agent.set_multilingual(serde_json::json!({
            "languages": ["en", "es"],
            "start_language": "en",
        }));
        let swml = agent.render_swml(&HashMap::new());
        // Locate the AI verb's `multilingual` object in the rendered document.
        let doc = swml.to_string();
        assert!(
            doc.contains("multilingual"),
            "AI verb should carry multilingual"
        );
        assert!(doc.contains("start_language"));
    }

    #[test]
    fn test_set_multilingual_ignores_non_object() {
        let mut agent = AgentBase::new(default_options());
        agent.set_multilingual(serde_json::json!("not-an-object"));
        assert!(agent.multilingual.is_none());
    }

    // -------------------------------------------------------------------
    // Per-language params — mirrors Python's TestPerLanguageParams.
    //
    // Rust idiom difference: Python's `add_language(..., params=...)` keyword
    // arg has no Rust equivalent (Rust doesn't support kwargs and the
    // existing `add_language(name, code, voice)` signature is widely used).
    // The functional parity is provided via the fluent
    // `add_language(...).set_language_params(code, params)` chain. Tests
    // that exercise the "inline params" path on the Python side are
    // mirrored here as the equivalent two-call chain.
    // -------------------------------------------------------------------

    #[test]
    fn test_add_language_then_set_params_attaches_params() {
        let mut agent = AgentBase::new(default_options());
        agent
            .add_language("English", "en-US", "josh")
            .set_language_params("en-US", json!({"stability": 0.5, "similarity_boost": 0.75}));
        assert_eq!(
            agent.languages[0]["params"],
            json!({"stability": 0.5, "similarity_boost": 0.75})
        );
    }

    #[test]
    fn test_add_language_without_params_omits_key() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("French", "fr-FR", "fr-FR-Neural2-A");
        assert!(agent.languages[0].get("params").is_none());
    }

    #[test]
    fn test_set_language_params_empty_object_omits_key() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("French", "fr-FR", "v");
        agent.set_language_params("fr-FR", json!({}));
        assert!(agent.languages[0].get("params").is_none());
    }

    #[test]
    fn test_get_language_params_returns_set_dict() {
        let mut agent = AgentBase::new(default_options());
        agent
            .add_language("English", "en-US", "v")
            .set_language_params("en-US", json!({"a": 1}));
        assert_eq!(agent.get_language_params("en-US"), Some(&json!({"a": 1})));
    }

    #[test]
    fn test_get_language_params_returns_none_when_unset() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "v");
        assert!(agent.get_language_params("en-US").is_none());
    }

    #[test]
    fn test_get_language_params_returns_none_for_unknown_code() {
        let agent = AgentBase::new(default_options());
        assert!(agent.get_language_params("zh-CN").is_none());
    }

    #[test]
    fn test_set_language_params_replaces_existing() {
        let mut agent = AgentBase::new(default_options());
        agent
            .add_language("English", "en-US", "v")
            .set_language_params("en-US", json!({"a": 1}))
            .set_language_params("en-US", json!({"b": 2}));
        assert_eq!(agent.get_language_params("en-US"), Some(&json!({"b": 2})));
    }

    #[test]
    fn test_set_language_params_adds_when_unset() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "v");
        agent.set_language_params("en-US", json!({"c": 3}));
        assert_eq!(agent.get_language_params("en-US"), Some(&json!({"c": 3})));
    }

    #[test]
    fn test_set_language_params_empty_dict_removes_key() {
        let mut agent = AgentBase::new(default_options());
        agent
            .add_language("English", "en-US", "v")
            .set_language_params("en-US", json!({"a": 1}));
        agent.set_language_params("en-US", json!({}));
        assert!(agent.get_language_params("en-US").is_none());
        assert!(agent.languages[0].get("params").is_none());
    }

    #[test]
    fn test_set_language_params_unknown_code_is_noop() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "v");
        agent.set_language_params("zh-CN", json!({"a": 1}));
        // The known language stays untouched.
        assert!(agent.languages[0].get("params").is_none());
        // No new languages were added.
        assert_eq!(agent.languages.len(), 1);
    }

    #[test]
    fn test_set_language_params_returns_self_for_chaining() {
        let mut agent = AgentBase::new(default_options());
        agent.add_language("English", "en-US", "v");
        // Chain two setters together; the second only compiles when the
        // first returns &mut Self.
        agent
            .set_language_params("en-US", json!({"a": 1}))
            .set_language_params("en-US", json!({"b": 2}));
        assert_eq!(agent.get_language_params("en-US"), Some(&json!({"b": 2})));
    }

    #[test]
    fn test_set_language_params_emitted_in_ai_verb() {
        // The per-language `params` key is wired into the SWML `languages`
        // array unchanged — build_ai_verb clones `self.languages` on
        // emission, so any object key (including `params`) flows through.
        let mut agent = AgentBase::new(default_options());
        agent
            .add_language("English", "en-US", "josh")
            .set_language_params("en-US", json!({"stability": 0.5}));
        let ai = agent.build_ai_verb(&HashMap::new());
        let lang0 = &ai["languages"][0];
        assert_eq!(lang0["code"], "en-US");
        assert_eq!(lang0["params"], json!({"stability": 0.5}));
    }

    #[test]
    fn test_add_pronunciation() {
        // Wire keys: replace, with; `ignore_case` (bool) omitted when false.
        let mut agent = AgentBase::new(default_options());
        agent.add_pronunciation("SignalWire", "signal wire", None);
        assert_eq!(agent.pronunciations[0]["replace"], "SignalWire");
        assert_eq!(agent.pronunciations[0]["with"], "signal wire");
        assert!(agent.pronunciations[0].get("ignore_case").is_none());
        // The old (wrong) `ignore` string key must never be emitted.
        assert!(agent.pronunciations[0].get("ignore").is_none());
    }

    #[test]
    fn test_add_pronunciation_with_ignore_case() {
        // ignore_case=true emits the bool wire key `ignore_case: true`
        // (matches signalwire-agents schema.json + Python add_pronunciation).
        let mut agent = AgentBase::new(default_options());
        agent.add_pronunciation("AI", "A.I.", Some(true));
        assert_eq!(agent.pronunciations[0]["ignore_case"], json!(true));
        assert!(agent.pronunciations[0].get("ignore").is_none());
    }

    #[test]
    fn test_set_pronunciations() {
        let mut agent = AgentBase::new(default_options());
        agent.add_pronunciation("a", "b", None);
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

    // ── Behavior parity bundle (#190/#191/#185/#182) regression tests ──────

    #[test]
    fn test_set_global_data_merges_not_replaces() {
        // #190: a second set_global_data must MERGE over the first, not replace.
        let mut agent = AgentBase::new(default_options());
        agent.set_global_data(json!({"a": 1}));
        agent.set_global_data(json!({"b": 2}));
        assert_eq!(
            agent.global_data.len(),
            2,
            "second set must merge, not replace"
        );
        assert_eq!(agent.global_data["a"], 1);
        assert_eq!(agent.global_data["b"], 2);
    }

    #[test]
    fn test_set_function_includes_drops_invalid() {
        // #191: keep only entries with a non-empty string `url` AND an array
        // `functions`; drop the rest.
        let mut agent = AgentBase::new(default_options());
        agent.set_function_includes(vec![
            json!({"url": "https://x/swaig", "functions": ["a", "b"]}), // valid
            json!({"url": "https://y/swaig"}),                          // no functions
            json!({"functions": ["c"]}),                                // no url
            json!({"url": "", "functions": ["d"]}),                     // empty url
            json!({"url": "https://z/swaig", "functions": "nope"}),     // functions not array
        ]);
        assert_eq!(
            agent.function_includes.len(),
            1,
            "only the well-formed entry survives"
        );
        assert_eq!(agent.function_includes[0]["url"], "https://x/swaig");
    }

    #[test]
    fn test_default_prompt_fallback_with_contexts() {
        // #185: with contexts and no prompt text, render emits the fallback.
        let mut agent = AgentBase::new(default_options());
        agent
            .define_contexts()
            .add_context("default")
            .add_step("intro")
            .set_text("Hi");
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(
            ai["prompt"]["text"],
            json!("You are test-agent, a helpful AI assistant.")
        );
    }

    #[test]
    fn test_no_default_prompt_fallback_without_contexts() {
        // #185: WITHOUT contexts, an empty prompt is passed through (no fallback).
        let agent = AgentBase::new(default_options());
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["prompt"]["text"], json!(""));
    }

    #[test]
    fn test_prompt_add_to_section_autocreates() {
        // #182: appending to a missing section auto-creates it.
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_to_section("Fresh", Some("body text"), vec!["b1"]);
        assert!(agent.prompt_has_section("Fresh"));
        let prompt = agent.get_prompt();
        assert_eq!(prompt[0]["title"], "Fresh");
        assert_eq!(prompt[0]["body"], "body text");
        assert_eq!(prompt[0]["bullets"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_prompt_add_subsection_autocreates() {
        // #182: adding a subsection under a missing parent auto-creates the parent.
        let mut agent = AgentBase::new(default_options());
        agent.prompt_add_subsection("Parent", "Child", "detail");
        assert!(agent.prompt_has_section("Parent"));
        let prompt = agent.get_prompt();
        let subs = prompt[0]["subsections"].as_array().unwrap();
        assert_eq!(subs[0]["title"], "Child");
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
        agent.enable_debug_events(Some(3));
        assert_eq!(agent.debug_events_level, Some(3));
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

    // Tier-2 behavioral contract #2: set_prompt_llm_params / set_post_prompt_llm_params
    // MERGE (not replace). Two calls with distinct keys must both survive into the
    // rendered AI verb — a replace-stub would drop the first key. (Python:
    // ai_config_mixin.py:669,703 `.update(params)`.)
    #[test]
    fn test_set_prompt_llm_params_merges_across_calls() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_llm_params(json!({"temperature": 0.5}));
        agent.set_prompt_llm_params(json!({"top_p": 0.9}));
        // Both retained on the struct...
        assert_eq!(agent.prompt_llm_params["temperature"], 0.5);
        assert_eq!(agent.prompt_llm_params["top_p"], 0.9);
        // ...and both rendered into the AI verb's prompt block.
        let ai = agent.build_ai_verb(&HashMap::new());
        let prompt = &ai["prompt"];
        assert_eq!(
            prompt["temperature"], 0.5,
            "temperature must not be dropped by the 2nd call"
        );
        assert_eq!(prompt["top_p"], 0.9);
    }

    #[test]
    fn test_set_post_prompt_llm_params_merges_across_calls() {
        let mut agent = AgentBase::new(default_options());
        agent.set_post_prompt("Summarize the call.");
        agent.set_post_prompt_llm_params(json!({"temperature": 0.3}));
        agent.set_post_prompt_llm_params(json!({"top_p": 0.8}));
        assert_eq!(agent.post_prompt_llm_params["temperature"], 0.3);
        assert_eq!(agent.post_prompt_llm_params["top_p"], 0.8);
        let ai = agent.build_ai_verb(&HashMap::new());
        let pp = &ai["post_prompt"];
        assert_eq!(
            pp["temperature"], 0.3,
            "temperature must not be dropped by the 2nd call"
        );
        assert_eq!(pp["top_p"], 0.8);
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

        agent.add_skill("weather", None);
        assert!(agent.has_skill("weather"));
        assert_eq!(agent.list_skills(), vec!["weather"]);

        agent.remove_skill("weather");
        assert!(!agent.has_skill("weather"));
    }

    #[test]
    fn test_add_skill_idempotent() {
        let mut agent = AgentBase::new(default_options());
        agent.add_skill("s1", None);
        agent.add_skill("s1", None);
        assert_eq!(agent.list_skills().len(), 1);
    }

    #[test]
    fn test_add_skill_via_skill_name_enum_loads_identical_skill() {
        use crate::skills::SkillName;

        // The enum's as_str() is the canonical wire string.
        assert_eq!(SkillName::Datetime.as_str(), "datetime");

        // add_skill() driven by the typed enum loads the *identical* skill as
        // the bare string: same bookkeeping entry AND the same SWAIG functions
        // get registered (real behaviour, not just the name list).
        let mut enum_agent = AgentBase::new(default_options());
        enum_agent.add_skill(SkillName::Datetime.as_str(), None);
        assert!(enum_agent.has_skill("datetime")); // string lookup
        assert!(enum_agent.has_skill(SkillName::Datetime.as_str())); // enum lookup — same skill
        assert!(enum_agent.has_tool("get_current_time"));
        assert!(enum_agent.has_tool("get_current_date"));

        // Parity: the bare string still works identically (Python uses str).
        let mut string_agent = AgentBase::new(default_options());
        string_agent.add_skill("datetime", None);

        // Both paths produce the same loaded-skill set and the same tool set.
        assert_eq!(enum_agent.list_skills(), string_agent.list_skills());
        assert_eq!(enum_agent.list_tool_names(), string_agent.list_tool_names());

        // remove_skill() accepts the enum's str too (mirrors PHP removeSkill).
        enum_agent.remove_skill(SkillName::Datetime.as_str());
        assert!(!enum_agent.has_skill("datetime"));
    }

    // ── Web / Callbacks ──────────────────────────────────────────────────

    #[test]
    fn test_set_webhook_url() {
        let mut agent = AgentBase::new(default_options());
        agent.set_webhook_url("https://webhook.example.com/swaig");
        assert_eq!(
            agent.webhook_url,
            Some("https://webhook.example.com/swaig".to_string())
        );
    }

    #[test]
    fn test_set_post_prompt_url() {
        let mut agent = AgentBase::new(default_options());
        agent.set_post_prompt_url("https://example.com/post_prompt");
        assert_eq!(
            agent.post_prompt_url,
            Some("https://example.com/post_prompt".to_string())
        );
    }

    #[test]
    fn test_manual_proxy_url() {
        let mut agent = AgentBase::new(default_options());
        agent.manual_set_proxy_url("https://proxy.example.com/");
        assert_eq!(
            agent.manual_proxy_url,
            Some("https://proxy.example.com".to_string())
        );
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
        agent.add_pronunciation("AI", "A.I.", None);
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["pronounce"][0]["replace"], "AI");
    }

    #[test]
    fn test_build_ai_verb_params() {
        let mut agent = AgentBase::new(default_options());
        agent.set_param("temperature", json!(0.7));
        agent.add_internal_filler("one moment");
        agent.enable_debug_events(Some(3));
        let ai = agent.build_ai_verb(&HashMap::new());
        assert_eq!(ai["params"]["temperature"], 0.7);
        assert_eq!(ai["params"]["internal_fillers"][0], "one moment");
        // The reference emits `params.debug_webhook_level` as an INT
        // (`core/agent_base.py:1259`); `debug_events` is the webhook PATH
        // segment and must never appear as a params key.
        assert_eq!(ai["params"]["debug_webhook_level"], 3);
        assert!(ai["params"].get("debug_events").is_none());
    }

    #[test]
    fn test_enable_debug_events_defaults_to_tier_one() {
        let mut agent = AgentBase::new(default_options());
        agent.enable_debug_events(None);
        let ai = agent.build_ai_verb(&HashMap::new());
        // The reference's `level: int = 1` default.
        assert_eq!(ai["params"]["debug_webhook_level"], 1);
    }

    /// WIRE CONTRACT: enabling debug events emits BOTH params — the reference
    /// sets `params["debug_webhook_url"] = _build_webhook_url("debug_events",
    /// …)` alongside `params["debug_webhook_level"]`
    /// (`core/agent_base.py:1254-1261`). Emitting only the level leaves the
    /// platform with nowhere to deliver the events.
    #[test]
    fn test_debug_events_emit_both_url_and_level() {
        let mut agent = AgentBase::new(default_options());
        agent.enable_debug_events(Some(2));
        let ai = agent.build_ai_verb(&HashMap::new());

        assert_eq!(ai["params"]["debug_webhook_level"], 2);
        let url = ai["params"]["debug_webhook_url"]
            .as_str()
            .expect("debug_webhook_url must be emitted alongside the level");
        // The `debug_events` ENDPOINT segment (not `swaig`), and the same
        // auth-embedded shape every webhook URL carries.
        assert!(
            url.ends_with("/debug_events"),
            "debug_webhook_url must address the debug_events endpoint, got {url}"
        );
        assert!(
            url.contains('@'),
            "debug_webhook_url must embed basic-auth credentials, got {url}"
        );
    }

    /// The debug webhook URL carries the same `swaig` query params the
    /// reference passes into its `_build_webhook_url("debug_events", …)` call.
    #[test]
    fn test_debug_webhook_url_carries_swaig_query_params() {
        let mut agent = AgentBase::new(default_options());
        let mut qp = HashMap::new();
        qp.insert("tenant".to_string(), "acme".to_string());
        agent.add_swaig_query_params(qp);
        agent.enable_debug_events(Some(1));
        let ai = agent.build_ai_verb(&HashMap::new());

        let url = ai["params"]["debug_webhook_url"].as_str().unwrap();
        assert!(
            url.contains("/debug_events?tenant=acme"),
            "query params must ride along on the debug webhook URL, got {url}"
        );
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
        // SECURE (the define_tool default) — only a tool with a token (or SWAIG
        // query params) gets its own per-tool web_hook_url; see
        // test_render_emits_token_only_for_secure_tools for the insecure side.
        agent.define_tool(
            "lookup",
            "Look up info",
            json!({}),
            Box::new(|_args, _raw| FunctionResult::with_response("result")),
            true,
        );
        let ai = agent.build_ai_verb(&HashMap::new());
        let funcs = ai["SWAIG"]["functions"].as_array().unwrap();
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0]["function"], "lookup");
        assert!(
            funcs[0]["web_hook_url"]
                .as_str()
                .unwrap()
                .contains("/swaig")
        );
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
        assert_eq!(
            ai["post_prompt_url"],
            "https://proxy.example.com/post_prompt"
        );
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
        let result = clone.on_function_call("func1", &args, Some(&raw)).unwrap();
        assert_eq!(result.to_value()["response"], "ok");
    }

    // ── HTTP Endpoints ───────────────────────────────────────────────────

    #[test]
    fn test_handle_request_health() {
        let agent = AgentBase::new(default_options());
        let (status, _, body) = agent.handle_request("GET", "/health", &HashMap::new(), None);
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["status"], "healthy");
    }

    #[test]
    fn test_handle_request_ready() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("GET", "/ready", &HashMap::new(), None);
        assert_eq!(status, 200);
    }

    #[test]
    fn test_handle_request_auth_required() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("POST", "/", &HashMap::new(), None);
        assert_eq!(status, 401);
    }

    #[test]
    fn test_handle_request_swml() {
        let mut agent = AgentBase::new(default_options());
        agent.set_prompt_text("Bot");
        let (status, _, body) = agent.handle_request("POST", "/", &authed_headers(), None);
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
        let (status, _, resp_body) =
            agent.handle_request("POST", "/swaig", &authed_headers(), Some(&body.to_string()));
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&resp_body).unwrap();
        assert_eq!(parsed["response"], "Hello!");
    }

    #[test]
    fn test_handle_request_swaig_unknown_function() {
        let agent = AgentBase::new(default_options());
        let body = json!({"function": "nonexistent", "argument": {"parsed": [{}]}});
        let (status, _, _) =
            agent.handle_request("POST", "/swaig", &authed_headers(), Some(&body.to_string()));
        assert_eq!(status, 404);
    }

    #[test]
    fn test_handle_request_swaig_no_body() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("POST", "/swaig", &authed_headers(), None);
        assert_eq!(status, 400);
    }

    #[test]
    fn test_handle_request_swaig_no_function_name() {
        let agent = AgentBase::new(default_options());
        let body = json!({"argument": {}});
        let (status, _, _) =
            agent.handle_request("POST", "/swaig", &authed_headers(), Some(&body.to_string()));
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
            Some(&body.to_string()),
        );
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&resp_body).unwrap();
        assert_eq!(parsed["status"], "ok");
    }

    #[test]
    fn test_handle_request_not_found() {
        let agent = AgentBase::new(default_options());
        let (status, _, _) = agent.handle_request("GET", "/unknown", &authed_headers(), None);
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
            .enable_debug_events(Some(3))
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

        let (status, _, body) = agent.handle_request("POST", "/", &authed_headers(), Some("{}"));
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
            Some(&body.to_string()),
        );
        assert_eq!(status, 200);

        let guard = captured.lock().unwrap();
        assert_eq!(*guard, "Great call");
    }

    // ── Tool tokens ──────────────────────────────────────────────────────
    //
    // Parity: signalwire-python tests/unit/core/test_agent_base.py
    //   ::TestAgentBaseTokenMethods::test_validate_tool_token
    //   ::TestAgentBaseTokenMethods::test_create_tool_token

    fn agent_with_tool() -> AgentBase {
        let mut a = AgentBase::new(default_options());
        a.define_tool(
            "test_tool",
            "t",
            json!({}),
            Box::new(|_a, _r| FunctionResult::with_response("ok")),
            true,
        );
        a
    }

    #[test]
    fn test_create_tool_token_round_trip() {
        let a = agent_with_tool();
        let token = a.create_tool_token("test_tool", "call_123");
        assert!(
            !token.is_empty(),
            "expected non-empty SessionManager-issued token"
        );
        assert!(
            a.validate_tool_token("test_tool", &token, "call_123"),
            "validate_tool_token rejected the token we just created"
        );
    }

    #[test]
    fn test_validate_tool_token_rejects_unknown_function() {
        let a = AgentBase::new(default_options());
        assert!(
            !a.validate_tool_token("not_registered", "any_token", "call_123"),
            "expected false for unregistered function"
        );
    }

    #[test]
    fn test_validate_tool_token_rejects_bad_token() {
        let a = agent_with_tool();
        assert!(
            !a.validate_tool_token("test_tool", "garbage_token_value", "call_123"),
            "expected false for garbage token"
        );
    }

    #[test]
    fn test_validate_tool_token_rejects_wrong_call_id() {
        let a = agent_with_tool();
        let token = a.create_tool_token("test_tool", "call_A");
        assert!(!token.is_empty());
        assert!(
            !a.validate_tool_token("test_tool", &token, "call_B"),
            "expected false when token bound to a different call_id"
        );
    }

    // ── The `__token` wire manifestation of `secure` ─────────────────────
    //
    // Parity: signalwire-python `agent_base.py:1040`/`1097` — a SECURE tool's
    // rendered SWAIG webhook carries the per-tool `__token`; an INSECURE one
    // does not. This is the wire property the cross-port SECURE-DEFAULT gate
    // (porting-sdk `diff_port_secure_default.py`) compares.

    /// The rendered `SWAIG.functions[]` entry for `tool_name`, if any.
    fn rendered_entry<'a>(doc: &'a Value, tool_name: &str) -> Option<&'a Value> {
        doc["sections"]["main"]
            .as_array()?
            .iter()
            .find_map(|sec| sec.get("ai"))?["SWAIG"]["functions"]
            .as_array()?
            .iter()
            .find(|f| f["function"].as_str() == Some(tool_name))
    }

    /// The `web_hook_url` the render emitted for `tool_name`, if any.
    fn rendered_webhook_url(doc: &Value, tool_name: &str) -> Option<String> {
        rendered_entry(doc, tool_name)?["web_hook_url"]
            .as_str()
            .map(str::to_string)
    }

    fn agent_with_secure_and_insecure_tools() -> AgentBase {
        let mut a = AgentBase::new(default_options());
        a.define_tool(
            "secure_tool",
            "s",
            json!({}),
            Box::new(|_a, _r| FunctionResult::with_response("ok")),
            true,
        );
        a.define_tool(
            "insecure_tool",
            "i",
            json!({}),
            Box::new(|_a, _r| FunctionResult::with_response("ok")),
            false,
        );
        a
    }

    #[test]
    fn test_render_emits_token_only_for_secure_tools() {
        let a = agent_with_secure_and_insecure_tools();
        let doc = a.render_swml(&HashMap::new());

        let secure_url = rendered_webhook_url(&doc, "secure_tool")
            .expect("secure tool must render a web_hook_url");
        assert!(
            secure_url.contains("__token="),
            "a secure tool's rendered webhook must carry the per-tool __token, got {secure_url}"
        );

        // The reference emits NO web_hook_url KEY AT ALL for an insecure tool
        // with no token and no SWAIG query params (agent_base.py:1084-1099: the
        // `elif token or _swaig_query_params` has no else). A per-tool callback
        // here would be an UNAUTHENTICATED function-specific endpoint; the
        // insecure tool falls back to the shared SWAIG defaults instead.
        assert!(
            rendered_entry(&doc, "insecure_tool")
                .expect("insecure tool must still render a function entry")
                .get("web_hook_url")
                .is_none(),
            "an insecure tool must have NO web_hook_url key at all, got {:?}",
            rendered_webhook_url(&doc, "insecure_tool")
        );
    }

    /// The shared `ai.SWAIG.defaults` block the render emitted, if any.
    fn rendered_swaig_defaults(doc: &Value) -> Option<&Value> {
        let ai = doc["sections"]["main"]
            .as_array()?
            .iter()
            .find_map(|sec| sec.get("ai"))?;
        ai["SWAIG"].get("defaults")
    }

    #[test]
    fn test_render_emits_shared_swaig_defaults_webhook() {
        // The reference adds `SWAIG.defaults.web_hook_url` WHENEVER functions
        // exist (agent_base.py:1109-1113). This is the endpoint an INSECURE tool
        // — which correctly has no per-tool web_hook_url — actually dispatches
        // to. Without it, dropping the per-tool key would leave an insecure tool
        // with NO reachable callback at all, which the SECURE-DEFAULT gate
        // cannot see (it inspects only the functions[] entries).
        let a = agent_with_secure_and_insecure_tools();
        let doc = a.render_swml(&HashMap::new());

        let defaults =
            rendered_swaig_defaults(&doc).expect("functions exist, so SWAIG.defaults must be too");
        let url = defaults["web_hook_url"]
            .as_str()
            .expect("SWAIG.defaults.web_hook_url must be a string");
        assert!(
            url.contains("/swaig"),
            "the shared fallback must point at the agent's /swaig endpoint, got {url}"
        );
        // The SHARED endpoint is not per-tool, so it carries no per-tool token.
        assert!(
            !url.contains("__token="),
            "the shared defaults endpoint is not per-tool and carries no __token, got {url}"
        );
    }

    #[test]
    fn test_swaig_defaults_honors_web_hook_url_override() {
        // agent_base.py:975-979 — `set_web_hook_url` replaces the composed
        // default wholesale.
        let mut a = agent_with_secure_and_insecure_tools();
        a.set_web_hook_url("https://override.example.com/swaig");
        let doc = a.render_swml(&HashMap::new());

        assert_eq!(
            rendered_swaig_defaults(&doc).expect("SWAIG.defaults")["web_hook_url"],
            "https://override.example.com/swaig"
        );
    }

    #[test]
    fn test_no_swaig_defaults_when_no_functions() {
        // The reference emits `defaults` only INSIDE `if functions:`.
        let mut a = AgentBase::new(default_options());
        a.set_prompt_text("no tools");
        let doc = a.render_swml(&HashMap::new());
        assert!(
            rendered_swaig_defaults(&doc).is_none(),
            "no functions means no SWAIG.defaults block"
        );
    }

    #[test]
    fn test_insecure_tool_gets_webhook_when_swaig_query_params_exist() {
        // The reference's guard is `token OR _swaig_query_params` — with query
        // params configured, even an insecure tool gets a local URL (carrying the
        // params, and still no __token).
        let mut a = agent_with_secure_and_insecure_tools();
        a.add_swaig_query_params(HashMap::from([("tenant".to_string(), "acme".to_string())]));
        let doc = a.render_swml(&HashMap::new());

        let url = rendered_webhook_url(&doc, "insecure_tool")
            .expect("query params must produce a local web_hook_url even when insecure");
        assert!(
            url.contains("tenant=acme"),
            "the SWAIG query params must ride on the URL, got {url}"
        );
        assert!(
            !url.contains("__token="),
            "an insecure tool must carry NO __token, got {url}"
        );
    }

    #[test]
    fn test_render_token_validates_for_its_own_call() {
        use base64::Engine as _;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;

        // The token the render minted must verify against the call_id it was
        // minted for — i.e. it is a real SessionManager token, not a placeholder.
        let a = agent_with_secure_and_insecure_tools();
        let doc = a.render_swml(&HashMap::new());
        let url = rendered_webhook_url(&doc, "secure_tool").expect("web_hook_url");
        let token = url
            .split("__token=")
            .nth(1)
            .expect("__token present")
            .split('&')
            .next()
            .expect("token value");

        // The token payload is `{call_id}.{function}.{expiry}.{nonce}.{hmac}`
        // base64url-encoded; recover the render's call_id and confirm the pair
        // validates (i.e. it is a real SessionManager token, not a placeholder).
        let payload = String::from_utf8(
            URL_SAFE_NO_PAD
                .decode(token)
                .expect("token is base64url-encoded"),
        )
        .expect("token payload is utf-8");
        let call_id = payload.split('.').next().expect("token carries a call_id");
        assert!(!call_id.is_empty(), "the render must mint a call_id");
        assert!(
            a.validate_tool_token("secure_tool", token, call_id),
            "the render-minted token must validate for its own call_id"
        );
    }

    // ── SWAIG dispatch token validation ──────────────────────────────────
    //
    // Parity: signalwire-python `agent_base.py:1414-1444` — a bad token on a
    // SECURE function refuses execution with a spoken message; an INSECURE
    // function is dispatched regardless.

    fn swaig_body(function: &str, token: Option<&str>) -> String {
        let mut body = json!({
            "function": function,
            "call_id": "call_dispatch",
            "argument": {"parsed": [{}]},
        });
        if let Some(t) = token {
            body["query_params"] = json!({"__token": t});
        }
        body.to_string()
    }

    #[test]
    fn test_swaig_dispatch_rejects_bad_token_on_secure_function() {
        let a = agent_with_secure_and_insecure_tools();
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            Some(&swaig_body("secure_tool", Some("garbage_token"))),
        );
        assert_eq!(status, 200, "the reference refuses in-band, not via HTTP");
        assert!(
            body.contains("security token for this function is invalid"),
            "expected the token-refusal response, got {body}"
        );
    }

    #[test]
    fn test_swaig_dispatch_accepts_valid_token_on_secure_function() {
        let a = agent_with_secure_and_insecure_tools();
        let token = a.create_tool_token("secure_tool", "call_dispatch");
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            Some(&swaig_body("secure_tool", Some(&token))),
        );
        assert_eq!(status, 200);
        assert!(
            !body.contains("security token for this function is invalid"),
            "a valid token must dispatch to the handler, got {body}"
        );
        assert!(
            body.contains("ok"),
            "expected the handler's response: {body}"
        );
    }

    #[test]
    fn test_swaig_dispatch_ignores_bad_token_on_insecure_function() {
        // The reference only refuses when the function is SECURE.
        let a = agent_with_secure_and_insecure_tools();
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            Some(&swaig_body("insecure_tool", Some("garbage_token"))),
        );
        assert_eq!(status, 200);
        assert!(
            body.contains("ok"),
            "an insecure function dispatches even with a bad token, got {body}"
        );
    }

    #[test]
    fn test_swaig_dispatch_reads_nested_call_call_id() {
        // The reference falls back to `body["call"]["call_id"]` when the flat
        // `call_id` is absent (agent_base.py:1744-1747). A token minted for that
        // call must validate through the nested path too.
        let a = agent_with_secure_and_insecure_tools();
        let token = a.create_tool_token("secure_tool", "call_nested");
        let body = json!({
            "function": "secure_tool",
            "call": {"call_id": "call_nested"},
            "argument": {"parsed": [{}]},
            "query_params": {"__token": token},
        });
        let (status, _, out) =
            a.handle_request("POST", "/swaig", &authed_headers(), Some(&body.to_string()));
        assert_eq!(status, 200);
        assert!(
            !out.contains("security token for this function is invalid"),
            "the nested call.call_id must be read, so this valid token dispatches: {out}"
        );
        assert!(out.contains("ok"), "expected the handler's response: {out}");
    }

    #[test]
    fn test_swaig_dispatch_refuses_absent_token_on_secure_function() {
        // An ABSENT token is refused exactly like a forged one. Omitting the
        // credential must never be weaker than presenting a wrong one, or
        // `secure` would be a flag that permits anonymous calls.
        let a = agent_with_secure_and_insecure_tools();
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            Some(&swaig_body("secure_tool", None)),
        );
        assert_eq!(status, 200, "the refusal is in-band, not an HTTP error");
        assert!(
            body.contains("security token for this function is invalid"),
            "a secure tool must fail CLOSED with no token, got {body}"
        );
    }

    #[test]
    fn test_swaig_dispatch_runs_insecure_function_without_token() {
        // The counterweight to the test above: an insecure tool runs ungated.
        // A fix that refuses everything is not a fix.
        let a = agent_with_secure_and_insecure_tools();
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig",
            &authed_headers(),
            Some(&swaig_body("insecure_tool", None)),
        );
        assert_eq!(status, 200);
        assert!(
            body.contains("ok"),
            "an insecure tool dispatches with no token at all, got {body}"
        );
    }

    #[test]
    fn test_swaig_dispatch_refuses_when_call_id_absent() {
        // A token can only be checked against a call_id; with none there is
        // nothing to check it against, so it counts as unvalidated rather than
        // as a bypass.
        let a = agent_with_secure_and_insecure_tools();
        let token = a.create_tool_token("secure_tool", "call_dispatch");
        let body = json!({
            "function": "secure_tool",
            "argument": {"parsed": [{}]},
            "query_params": {"__token": token},
        });
        let (status, _, out) =
            a.handle_request("POST", "/swaig", &authed_headers(), Some(&body.to_string()));
        assert_eq!(status, 200);
        assert!(
            out.contains("security token for this function is invalid"),
            "a missing call_id must not be a bypass, got {out}"
        );
    }

    #[test]
    fn test_swaig_dispatch_runs_insecure_function_without_call_id() {
        let a = agent_with_secure_and_insecure_tools();
        let body = json!({
            "function": "insecure_tool",
            "argument": {"parsed": [{}]},
        });
        let (status, _, out) =
            a.handle_request("POST", "/swaig", &authed_headers(), Some(&body.to_string()));
        assert_eq!(status, 200);
        assert!(
            out.contains("ok"),
            "an insecure tool runs with no call_id and no token, got {out}"
        );
    }

    // ── The credential rides the request's own query string ──────────────
    //
    // The built-in server hands `handle_request` `tiny_http`'s `request.url()`,
    // which retains `?a=b`. Before this was split off, `/swaig?__token=…`
    // missed the route entirely (404) and the query was never parsed, so the
    // token could not arrive over the real HTTP transport at all.

    #[test]
    fn test_swaig_route_matches_when_path_carries_a_query_string() {
        let a = agent_with_secure_and_insecure_tools();
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig?foo=bar",
            &authed_headers(),
            Some(&swaig_body("insecure_tool", None)),
        );
        assert_eq!(status, 200, "a query string must not break routing: {body}");
        assert!(
            body.contains("ok"),
            "expected the handler's response: {body}"
        );
    }

    #[test]
    fn test_swaig_token_read_from_the_request_query_string() {
        let a = agent_with_secure_and_insecure_tools();
        let token = a.create_tool_token("secure_tool", "call_dispatch");
        let path = format!("/swaig?__token={token}");
        let (status, _, body) = a.handle_request(
            "POST",
            &path,
            &authed_headers(),
            Some(&swaig_body("secure_tool", None)),
        );
        assert_eq!(status, 200);
        assert!(
            !body.contains("security token for this function is invalid"),
            "a valid token on the query string must dispatch, got {body}"
        );
        assert!(
            body.contains("ok"),
            "expected the handler's response: {body}"
        );
    }

    #[test]
    fn test_swaig_forged_token_on_the_query_string_is_refused() {
        let a = agent_with_secure_and_insecure_tools();
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig?__token=garbage_token",
            &authed_headers(),
            Some(&swaig_body("secure_tool", None)),
        );
        assert_eq!(status, 200);
        assert!(
            body.contains("security token for this function is invalid"),
            "a forged query-string token must be refused, got {body}"
        );
    }

    #[test]
    fn test_query_string_parsing_decodes_escapes_and_keeps_body_precedence() {
        let parsed = parse_query_string("?a=1&b=hello+world&c=%2Ffoo&d&a=2");
        assert_eq!(parsed.get("a").and_then(Value::as_str), Some("1"));
        assert_eq!(parsed.get("b").and_then(Value::as_str), Some("hello world"));
        assert_eq!(parsed.get("c").and_then(Value::as_str), Some("/foo"));
        assert_eq!(parsed.get("d").and_then(Value::as_str), Some(""));

        // A body-supplied query_params entry wins over the transport's, so the
        // existing dispatch shape keeps working unchanged.
        let a = agent_with_secure_and_insecure_tools();
        let token = a.create_tool_token("secure_tool", "call_dispatch");
        let (status, _, body) = a.handle_request(
            "POST",
            "/swaig?__token=garbage_token",
            &authed_headers(),
            Some(&swaig_body("secure_tool", Some(&token))),
        );
        assert_eq!(status, 200);
        assert!(
            body.contains("ok"),
            "the body's query_params must take precedence, got {body}"
        );
    }
}
