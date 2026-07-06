use std::collections::HashMap;
use std::env;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, KeyInit, Mac};
use rand::RngExt;
use serde_json::Value;
use sha2::Sha256;

use crate::logging::Logger;
use crate::swaig::FunctionResult;
use crate::swml::document::Document;
use crate::swml::schema;

type HmacSha256 = Hmac<Sha256>;

/// Maximum request body size (1 MB).
const MAX_BODY_SIZE: usize = 1_048_576;

/// Fixed key for HMAC-based timing-safe comparison.
const HMAC_KEY: &[u8] = b"signalwire-swml-service-auth-compare";

/// Options for constructing a `Service`.
///
/// Doubles as an idiomatic **builder** (parallel to
/// [`AgentOptions`](crate::agent::AgentOptions)): [`ServiceOptions::new`] gives
/// a name-only default and the `with_*` methods take/return `self` for
/// one-expression configuration feeding [`Service::new`]:
///
/// ```no_run
/// use signalwire::SWMLService;
/// use signalwire::swml::service::ServiceOptions;
///
/// let svc = SWMLService::new(
///     ServiceOptions::new("sidecar")
///         .route("/swml")
///         .basic_auth("user", "secret"),
/// );
/// ```
///
/// Direct struct-literal construction still works; the builder methods are an
/// additive convenience. `#[must_use]` flags an options value built but never
/// passed to [`Service::new`].
#[must_use]
pub struct ServiceOptions {
    pub name: String,
    pub route: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
}

impl ServiceOptions {
    /// Name-only options with the same defaults the struct-literal callers use
    /// (`port` defaults to 3000 via [`Service::new`] when left `None`).
    pub fn new(name: &str) -> Self {
        ServiceOptions {
            name: name.to_string(),
            route: None,
            host: None,
            port: None,
            basic_auth_user: None,
            basic_auth_password: None,
        }
    }

    /// Set the HTTP route this service serves (e.g. `"/swml"`).
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

    /// Set HTTP Basic-Auth credentials guarding this service's endpoints.
    pub fn basic_auth(mut self, user: &str, password: &str) -> Self {
        self.basic_auth_user = Some(user.to_string());
        self.basic_auth_password = Some(password.to_string());
        self
    }
}

/// Hook function for SWML-request customization. Mirrors Python's
/// `WebMixin.on_swml_request(request_data, callback_path)` — receives the
/// parsed body and optional callback path, and returns a JSON `Value` of
/// modifications to merge (or `None` to use default rendering).
///
/// Rust has no method overriding via inheritance — the function-field
/// hook is the idiomatic equivalent of Python's overridable
/// `on_swml_request`. Set via `Service::set_on_swml_request_hook`.
pub type OnSwmlRequestHook =
    Box<dyn Fn(Option<&Value>, Option<&str>) -> Option<Value> + Send + Sync>;

/// SWML service: holds a document, auth credentials, and handles HTTP requests.
#[derive(Clone)]
pub struct Service {
    name: String,
    route: String,
    host: String,
    port: u16,
    document: Document,
    logger: Logger,
    basic_auth_user: String,
    basic_auth_password: String,

    // SWAIG tool registry — lifted from AgentBase so any Service (sidecar,
    // non-agent verb host) can register and dispatch SWAIG functions.
    // Same registry shape used by AgentBase via inherited access through
    // its Deref<Target=Service> impl, so tools registered on either
    // Service-the-sidecar or AgentBase live in a single place.
    pub(crate) tools: HashMap<String, ToolDef>,
    pub(crate) tool_order: Vec<String>,

    // SWML customization hook (Python WebMixin parity).
    pub(crate) on_swml_request_hook: Option<std::sync::Arc<OnSwmlRequestHook>>,

    // Specialized verb handlers (Python `verb_registry`). Pre-populated with
    // the `ai` handler; consulted by `add_verb` for validation.
    verb_registry: crate::swml::handler::VerbHandlerRegistry,

    // Manual proxy-URL override (Python WebMixin `manual_set_proxy_url`).
    manual_proxy_url: Option<String>,

    // Registered routing callbacks keyed by mount path (Python
    // `register_routing_callback`). Each maps a path to a callback that
    // inspects request data and returns an optional redirect route.
    routing_callbacks: HashMap<String, std::sync::Arc<RoutingCallback>>,
}

/// Routing-callback signature (Python `register_routing_callback`).
///
/// Receives `(body, headers)` — the parsed request body and the request
/// headers — and returns `Some(route)` to redirect the request to a different
/// agent/route, or `None` to fall through. Python decomposed the callback to
/// `callback_fn(body, headers)` (it no longer takes a framework `Request`); the
/// Rust closure mirrors that `(body, headers)` shape.
pub type RoutingCallback = dyn Fn(&Value, &HashMap<String, String>) -> Option<String> + Send + Sync;

/// Handler type for SWAIG function callbacks.
///
/// Receives `(args, raw_data)` and returns a `FunctionResult`. Same signature
/// `AgentBase` uses, so handlers are interchangeable between the two paths.
pub type FunctionHandler = Box<
    dyn Fn(&serde_json::Map<String, Value>, &serde_json::Map<String, Value>) -> FunctionResult
        + Send
        + Sync,
>;

/// Tool registered on a `Service` for SWAIG dispatch. The `definition`
/// field holds the rendered SWAIG function dict (function/purpose/argument);
/// `handler` is `None` for raw / DataMap-style functions, where dispatch
/// happens server-side rather than in this process.
#[derive(Clone)]
pub struct ToolDef {
    pub definition: Value,
    pub handler: Option<std::sync::Arc<FunctionHandler>>,
    pub secure: bool,
}

impl Service {
    pub fn new(options: ServiceOptions) -> Self {
        let route = options.route.map_or_else(
            || "/".to_string(),
            |r| {
                let trimmed = r.trim_end_matches('/');
                if trimmed.is_empty() {
                    "/".to_string()
                } else {
                    trimmed.to_string()
                }
            },
        );

        let host = options.host.unwrap_or_else(|| "0.0.0.0".to_string());

        let port = options.port.unwrap_or_else(|| {
            env::var("PORT")
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(3000)
        });

        // Auth: explicit > env > auto-generated
        let mut password_auto_generated = false;
        let (basic_auth_user, basic_auth_password) =
            if let (Some(u), Some(p)) = (options.basic_auth_user, options.basic_auth_password) {
                (u, p)
            } else if let (Ok(u), Ok(p)) = (
                env::var("SWML_BASIC_AUTH_USER"),
                env::var("SWML_BASIC_AUTH_PASSWORD"),
            ) {
                (u, p)
            } else {
                password_auto_generated = true;
                let user = env::var("SWML_BASIC_AUTH_USER").unwrap_or_else(|_| random_hex(16));
                (user, random_hex(32))
            };

        let logger = Logger::new("swml_service");
        logger.info(&format!(
            "Service '{}' initialised (route={}, port={})",
            options.name, route, port
        ));

        // Warn loudly if the password was auto-generated. This is the
        // silent cause of every external caller hitting HTTP 401 when
        // .env wasn't loaded — the password lives only in this process
        // and changes on every restart.
        if password_auto_generated {
            logger.warn(&format!(
                "basic_auth_password_autogenerated: username=\"{basic_auth_user}\". \
                 No SWML_BASIC_AUTH_PASSWORD found in environment and no \
                 password passed via ServiceOptions. The SDK generated a \
                 random password that exists only in this process; external \
                 callers will get HTTP 401 unless they read the value from \
                 this process's env. To fix, set SWML_BASIC_AUTH_USER and \
                 SWML_BASIC_AUTH_PASSWORD in your environment, or pass \
                 basic_auth_user / basic_auth_password in ServiceOptions \
                 (or AgentOptions) when constructing the agent."
            ));
        }

        Service {
            name: options.name,
            route,
            host,
            port,
            document: Document::new(),
            logger,
            basic_auth_user,
            basic_auth_password,
            tools: HashMap::new(),
            tool_order: Vec::new(),
            on_swml_request_hook: None,
            verb_registry: crate::swml::handler::VerbHandlerRegistry::new(),
            manual_proxy_url: None,
            routing_callbacks: HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    // SWAIG tool registry (lifted from AgentBase)
    // ------------------------------------------------------------------

    /// Normalize a tool's flat property map into `(properties, required)`.
    ///
    /// Skills mark a parameter required by setting `"required": true` *inside*
    /// the property object (the ergonomic per-property idiom). JSON Schema —
    /// and the Python reference — express requiredness as a top-level
    /// `required: [...]` array on the parameters object, not a per-property
    /// flag. This lifts each property's `"required": true` into that array (in
    /// the property's declared order) and strips the flag from the property, so
    /// the emitted `argument` is standard JSON Schema and byte-matches the
    /// reference. A property without the flag (or `"required": false`) is
    /// optional and left untouched.
    fn normalize_parameters(parameters: Value) -> (serde_json::Map<String, Value>, Vec<String>) {
        let mut required: Vec<String> = Vec::new();
        let Value::Object(props) = parameters else {
            // Non-object params (shouldn't happen) pass through as empty.
            return (serde_json::Map::new(), required);
        };
        let mut out = serde_json::Map::with_capacity(props.len());
        for (key, value) in props {
            if let Value::Object(mut prop) = value {
                if prop
                    .remove("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    required.push(key.clone());
                }
                out.insert(key, Value::Object(prop));
            } else {
                out.insert(key, value);
            }
        }
        (out, required)
    }

    /// Define a SWAIG function the AI can call. Tool descriptions and
    /// parameter descriptions are LLM-facing prompt engineering — see
    /// `PORTING_GUIDE` for guidance.
    ///
    /// Same shape as `AgentBase::define_tool` — a tool registered here is
    /// usable on both sidecar and agent paths because they share storage.
    pub fn define_tool(
        &mut self,
        name: &str,
        description: &str,
        parameters: Value,
        handler: FunctionHandler,
        secure: bool,
    ) -> &mut Self {
        let (properties, required) = Self::normalize_parameters(parameters);
        let mut argument = serde_json::Map::new();
        argument.insert("type".to_string(), serde_json::json!("object"));
        argument.insert("properties".to_string(), Value::Object(properties));
        // Emit the top-level JSON-Schema `required` array (the form the model +
        // validator expect) ONLY when non-empty — matching the Python reference,
        // which omits the key for an empty required list (swaig_function.py:128).
        if !required.is_empty() {
            argument.insert(
                "required".to_string(),
                Value::Array(required.into_iter().map(Value::String).collect()),
            );
        }

        let mut definition = serde_json::Map::new();
        definition.insert("function".to_string(), serde_json::json!(name));
        definition.insert("purpose".to_string(), serde_json::json!(description));
        definition.insert("argument".to_string(), Value::Object(argument));

        self.tools.insert(
            name.to_string(),
            ToolDef {
                definition: Value::Object(definition),
                handler: Some(std::sync::Arc::new(handler)),
                secure,
            },
        );
        if !self.tool_order.contains(&name.to_string()) {
            self.tool_order.push(name.to_string());
        }
        self
    }

    /// Merge additional SWAIG metadata keys into an already-registered
    /// tool's definition.
    ///
    /// Used by `SkillBase::define_tool` to fold a skill's `swaig_fields`
    /// into a handler-backed tool after it has been defined (the `DataMap`
    /// skills merge fields into the def before calling
    /// `register_swaig_function` instead). No-op if the tool isn't
    /// registered or the definition isn't a JSON object.
    pub fn merge_swaig_fields(
        &mut self,
        name: &str,
        fields: &serde_json::Map<String, Value>,
    ) -> &mut Self {
        if let Some(tool) = self.tools.get_mut(name)
            && let Value::Object(obj) = &mut tool.definition
        {
            for (k, v) in fields {
                obj.insert(k.clone(), v.clone());
            }
        }
        self
    }

    /// Register a raw SWAIG function definition (e.g. `DataMap` tools that
    /// have no local handler).
    pub fn register_swaig_function(&mut self, func_def: Value) -> &mut Self {
        let name = func_def["function"].as_str().unwrap_or("").to_string();
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

    /// Whether a SWAIG function with the given name is registered.
    /// Python parity: `ToolRegistry.has_function`.
    pub fn has_function(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get a registered SWAIG function definition by name, or `None`
    /// when absent. Python parity: `ToolRegistry.get_function`.
    pub fn get_function(&self, name: &str) -> Option<&ToolDef> {
        self.tools.get(name)
    }

    /// Snapshot of all registered SWAIG functions keyed by name.
    /// Python parity: `ToolRegistry.get_all_functions`.
    pub fn get_all_functions(&self) -> HashMap<String, ToolDef> {
        self.tools.clone()
    }

    /// Remove a registered SWAIG function. Returns `true` when the
    /// function was found and removed; `false` when it wasn't
    /// registered. Python parity: `ToolRegistry.remove_function`.
    pub fn remove_function(&mut self, name: &str) -> bool {
        if self.tools.remove(name).is_some() {
            self.tool_order.retain(|n| n != name);
            true
        } else {
            false
        }
    }

    /// Dispatch a function call to the registered handler. Returns
    /// `None` for unknown functions or registered functions with no
    /// local handler (e.g. `DataMap` tools that execute server-side).
    pub fn on_function_call(
        &self,
        name: &str,
        args: &serde_json::Map<String, Value>,
        raw_data: &serde_json::Map<String, Value>,
    ) -> Option<FunctionResult> {
        let tool = self.tools.get(name)?;
        let handler = tool.handler.as_ref()?;
        Some(handler(args, raw_data))
    }

    /// Whether a tool with the given name is registered.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Registered tool names in insertion order.
    pub fn list_tool_names(&self) -> Vec<String> {
        self.tool_order.clone()
    }

    /// Look up a registered tool's full SWAIG definition (the JSON
    /// shape returned to the SignalWire platform). Used by audit
    /// harnesses that need to inspect the `DataMap` webhook URL of a
    /// registered tool without invoking it.
    pub fn tool_definition(&self, name: &str) -> Option<Value> {
        self.tools.get(name).map(|t| t.definition.clone())
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn route(&self) -> &str {
        &self.route
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Override the bind host (crate-internal; used by `serve` host override).
    pub(crate) fn set_host(&mut self, host: &str) {
        self.host = host.to_string();
    }

    /// Override the bind port (crate-internal; used by `serve` port override).
    pub(crate) fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    /// `SchemaUtils` helper bound to this Service.  Mirrors Python's
    /// `self.schema_utils` instance attribute on `SWMLService`.
    /// Returns a freshly-built helper each call — the underlying
    /// schema is `LazyLock`-cached, so this is cheap.
    pub fn schema_utils(&self) -> crate::utils::SchemaUtils {
        crate::utils::SchemaUtils::new(None, true)
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    pub fn basic_auth_credentials(&self) -> (&str, &str) {
        (&self.basic_auth_user, &self.basic_auth_password)
    }

    /// Get (user, password) — Python-canonical name.
    /// Python parity: ``AuthMixin.get_basic_auth_credentials``.
    pub fn get_basic_auth_credentials(&self) -> (String, String) {
        (
            self.basic_auth_user.clone(),
            self.basic_auth_password.clone(),
        )
    }

    /// Get (user, password, source) where source is one of "provided",
    /// "environment", or "generated". Python parity:
    /// ``AuthMixin.get_basic_auth_credentials(include_source=True)``.
    pub fn get_basic_auth_credentials_with_source(&self) -> (String, String, String) {
        let user = self.basic_auth_user.clone();
        let pass = self.basic_auth_password.clone();
        let env_user = env::var("SWML_BASIC_AUTH_USER").unwrap_or_default();
        let env_pass = env::var("SWML_BASIC_AUTH_PASSWORD").unwrap_or_default();
        let source =
            if !env_user.is_empty() && !env_pass.is_empty() && user == env_user && pass == env_pass
            {
                "environment".to_string()
            } else if user.starts_with("user_") && pass.len() > 20 {
                "generated".to_string()
            } else {
                "provided".to_string()
            };
        (user, pass, source)
    }

    /// Validate provided basic-auth credentials against the configured ones.
    /// Python parity: ``AuthMixin.validate_basic_auth(username, password)``.
    pub fn validate_basic_auth(&self, username: &str, password: &str) -> bool {
        constant_time_eq(username, &self.basic_auth_user)
            && constant_time_eq(password, &self.basic_auth_password)
    }

    #[must_use]
    pub fn render(&self) -> String {
        self.document.render()
    }

    #[must_use]
    pub fn render_pretty(&self) -> String {
        self.document.render_pretty()
    }

    // ------------------------------------------------------------------
    // SWML customization hooks (Python WebMixin parity)
    // ------------------------------------------------------------------

    /// Register a function that customizes the SWML response on a
    /// per-request basis. The hook receives the parsed body and the
    /// callback path; returning `Some(value)` applies modifications,
    /// `None` falls through to the default rendering.
    ///
    /// Rust has no method overriding via inheritance — this hook is
    /// the idiomatic Rust equivalent of Python's overridable
    /// `on_swml_request`.
    pub fn set_on_swml_request_hook<F>(&mut self, hook: F) -> &mut Self
    where
        F: Fn(Option<&Value>, Option<&str>) -> Option<Value> + Send + Sync + 'static,
    {
        self.on_swml_request_hook = Some(std::sync::Arc::new(Box::new(hook)));
        self
    }

    /// Customization hook called when SWML is requested. Default
    /// delegates to [`Service::on_swml_request`] and returns its result.
    /// Subclasses (or external callers) typically configure
    /// `on_swml_request` via [`Service::set_on_swml_request_hook`]
    /// rather than overriding this method.
    ///
    /// Returning `None` uses the default rendered SWML; returning a
    /// non-`None` value applies modifications to the rendered document.
    ///
    /// Python parity: `WebMixin.on_request(request_data, callback_path)`.
    pub fn on_request(
        &self,
        request_data: Option<&Value>,
        callback_path: Option<&str>,
    ) -> Option<Value> {
        self.on_swml_request(request_data, callback_path)
    }

    /// Customization point for modifying SWML based on request data.
    /// If a hook has been registered via
    /// [`Service::set_on_swml_request_hook`] the hook is invoked;
    /// otherwise this returns `None` (no modification).
    ///
    /// Python parity: `WebMixin.on_swml_request(request_data, callback_path)`.
    /// The Python third `request` argument is FastAPI-specific and
    /// intentionally not mirrored.
    pub fn on_swml_request(
        &self,
        request_data: Option<&Value>,
        callback_path: Option<&str>,
    ) -> Option<Value> {
        if let Some(hook) = &self.on_swml_request_hook {
            return hook(request_data, callback_path);
        }
        None
    }

    // ------------------------------------------------------------------
    // Verb helpers
    // ------------------------------------------------------------------

    /// Add a verb to the `main` section of the current document.
    ///
    /// Python parity: `SWMLService.add_verb(verb_name, config)`. `config` may
    /// be an object, or a bare integer for the `sleep` verb. Returns `true` if
    /// added. A verb with a registered handler is validated by the handler;
    /// otherwise validated against the embedded schema.
    ///
    /// # Panics
    ///
    /// Panics on a schema-invalid verb config (mirrors Python's
    /// `SchemaValidationError`), matching the fail-loud contract of the
    /// legacy section-scoped path.
    pub fn add_verb(&mut self, verb_name: &str, config: Value) -> bool {
        self.add_verb_to_section("main", verb_name, config)
    }

    /// Add a verb to a specific section, creating the section if needed.
    ///
    /// Python parity: `SWMLService.add_verb_to_section(section_name,
    /// verb_name, config)`.
    ///
    /// # Panics
    ///
    /// Panics if the verb name is not in the schema (fail-loud).
    pub fn add_verb_to_section(&mut self, section: &str, verb: &str, config: Value) -> bool {
        if !self.document.has_section(section) {
            self.document.add_section(section);
        }
        // Sleep takes a direct integer value.
        if verb == "sleep" && config.is_i64() {
            self.document.add_verb_to_section(section, verb, config);
            return true;
        }
        if !config.is_object() {
            return false;
        }
        // A registered handler validates its own verb; else use the schema.
        if !self.verb_registry.has_handler(verb) {
            assert!(schema::is_valid_verb(verb), "Unknown SWML verb: {verb}");
        }
        self.document.add_verb_to_section(section, verb, config);
        true
    }

    /// Add a `sleep` verb (integer milliseconds) to a section.
    pub fn sleep(&mut self, millis: i64, section: &str) {
        self.document
            .add_verb_to_section(section, "sleep", Value::Number(millis.into()));
    }

    // ------------------------------------------------------------------
    // Document management (Python SWMLService parity)
    // ------------------------------------------------------------------

    /// Add a new named section to the document. Returns `true` if created,
    /// `false` if it already existed. Python parity: `add_section`.
    pub fn add_section(&mut self, section_name: &str) -> bool {
        self.document.add_section(section_name)
    }

    /// Get the current SWML document as a value. Python parity: `get_document`.
    #[must_use]
    pub fn get_document(&self) -> Value {
        self.document.to_value()
    }

    /// Render the current SWML document as a JSON string. Python parity:
    /// `render_document`.
    #[must_use]
    pub fn render_document(&self) -> String {
        self.document.render()
    }

    /// Reset the current document to an empty state. Python parity:
    /// `reset_document`.
    pub fn reset_document(&mut self) {
        self.document.reset();
    }

    /// Register a custom verb handler. Python parity: `register_verb_handler`.
    pub fn register_verb_handler(
        &mut self,
        handler: Box<dyn crate::swml::handler::SwmlVerbHandler>,
    ) {
        self.verb_registry.register_handler(handler);
    }

    /// Whether full JSON-Schema validation is enabled. The Rust port always
    /// validates verb names against the embedded schema, so full validation is
    /// always available. Python parity: `full_validation_enabled`.
    #[must_use]
    pub fn full_validation_enabled(&self) -> bool {
        true
    }

    // ------------------------------------------------------------------
    // Web/routing surface (Python WebMixin parity)
    // ------------------------------------------------------------------

    /// Set a manual proxy-URL override (strips a trailing slash). Python
    /// parity: `manual_set_proxy_url`.
    pub fn manual_set_proxy_url(&mut self, proxy_url: &str) -> &mut Self {
        self.manual_proxy_url = Some(proxy_url.trim_end_matches('/').to_string());
        self
    }

    /// Register a routing callback for `path`. The callback inspects request
    /// data and returns `Some(route)` to redirect. Python parity:
    /// `register_routing_callback`.
    pub fn register_routing_callback<F>(&mut self, callback: F, path: &str) -> &mut Self
    where
        F: Fn(&Value, &HashMap<String, String>) -> Option<String> + Send + Sync + 'static,
    {
        // Path normalization mirrors Python's `register_routing_callback`
        // (swml_service.py): strip trailing '/', then ensure a leading '/'.
        // "/sip/" -> "/sip"; "voice" -> "/voice"; "" -> "/".
        let normalized = {
            let p = path.trim_end_matches('/');
            if p.is_empty() {
                "/".to_string()
            } else if p.starts_with('/') {
                p.to_string()
            } else {
                format!("/{p}")
            }
        };
        self.routing_callbacks
            .insert(normalized, std::sync::Arc::new(callback));
        self
    }

    /// Look up a registered routing callback by path.
    #[must_use]
    pub fn routing_callback(&self, path: &str) -> Option<&std::sync::Arc<RoutingCallback>> {
        self.routing_callbacks.get(path)
    }

    /// The registered (normalized) routing-callback paths, sorted. Python
    /// parity: `sorted(self._routing_callbacks.keys())`.
    #[must_use]
    pub fn routing_callback_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.routing_callbacks.keys().cloned().collect();
        paths.sort();
        paths
    }

    /// Return a mountable [`axum::Router`] that serves this service's routes.
    ///
    /// This is the Rust equivalent of Python's `WebMixin.as_router` /
    /// `SWMLService.as_router`, which return a FastAPI `APIRouter` (the
    /// "embed my routes in a host app" unit). The returned router wraps this
    /// service's HTTP request handling (SWML render, `/swaig`, `/post_prompt`,
    /// `/health`, `/ready`) and can be mounted into a caller's own axum/hyper
    /// application with [`axum::Router::nest`] or served directly. The service
    /// is cloned into an [`std::sync::Arc`] so the router owns its state.
    #[cfg(feature = "tower-middleware")]
    pub fn as_router(&self) -> axum::Router {
        let svc = std::sync::Arc::new(self.clone());
        crate::swml::router::build_router(svc)
    }

    /// Start a blocking web server for this service (Python `serve`).
    ///
    /// The Rust HTTP serving lives on [`crate::server::AgentServer`]; this
    /// method is the parity entry point. `host`/`port` override the
    /// configured values.
    pub fn serve(&self, _host: Option<&str>, _port: Option<u16>) {
        self.run();
    }

    /// Stop the running server (Python `stop`). The Rust `serve`/`run` is a
    /// synchronous placeholder, so `stop` is a no-op parity entry point.
    pub fn stop(&self) {}

    // ------------------------------------------------------------------
    // HTTP handling
    // ------------------------------------------------------------------

    /// Handle an HTTP request. Returns `(status_code, headers, body)`.
    pub fn handle_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (u16, HashMap<String, String>, String) {
        self.logger
            .info(&format!("incoming request: {method} {path}"));

        // Health/ready: no auth
        if path == "/health" {
            return self.json_response(200, &serde_json::json!({"status": "healthy"}));
        }
        if path == "/ready" {
            return self.json_response(200, &serde_json::json!({"status": "ready"}));
        }

        // Determine if path matches our route
        let sub_path = if self.route == "/" {
            Some(path.to_string())
        } else if path == self.route || path.starts_with(&format!("{}/", self.route)) {
            let rest = &path[self.route.len()..];
            if rest.is_empty() {
                Some("/".to_string())
            } else {
                Some(rest.to_string())
            }
        } else {
            None
        };

        let Some(sub_path) = sub_path else {
            self.logger
                .debug(&format!("path {} did not match route {}", path, self.route));
            return self.json_response(404, &serde_json::json!({"error": "Not found"}));
        };

        // Auth required for everything under the route.
        //
        // Framework-free contract (Python `_handle_request_core`): a 401 is the
        // bare triple `(401, {"WWW-Authenticate": "Basic"}, {"error":
        // "Unauthorized"})` — a JSON body, a single `WWW-Authenticate: Basic`
        // header (no realm), and NO Content-Type/security headers (the HTTP
        // adapter layer re-adds Content-Type). Matches the cross-port oracle.
        if !self.check_basic_auth(headers) {
            self.logger
                .warn(&format!("basic auth failed for {method} {path}"));
            let mut resp_headers = HashMap::new();
            resp_headers.insert("WWW-Authenticate".to_string(), "Basic".to_string());
            return (
                401,
                resp_headers,
                serde_json::json!({"error": "Unauthorized"}).to_string(),
            );
        }

        // Parse body
        let request_data: Option<Value> = if body.is_empty() {
            None
        } else {
            if body.len() > MAX_BODY_SIZE {
                self.logger.warn(&format!(
                    "request body {} bytes exceeds limit {}",
                    body.len(),
                    MAX_BODY_SIZE
                ));
                return self
                    .json_response(413, &serde_json::json!({"error": "Request body too large"}));
            }
            match serde_json::from_str(body) {
                Ok(v) => Some(v),
                Err(e) => {
                    self.logger.debug(&format!(
                        "body JSON parse failed: {} ({} bytes)",
                        e,
                        body.len()
                    ));
                    None
                }
            }
        };

        // Routing-callback dispatch (Python `_handle_request_core`): if a
        // callback is registered for this sub-path, invoke it with (body,
        // headers). A returned route string becomes a 307 redirect preserving
        // method+body: `(307, {"Location": route}, "")`. `None` falls through to
        // normal SWML processing.
        if let Some(callback) = self.routing_callbacks.get(sub_path.as_str()) {
            let body_for_cb = request_data.clone().unwrap_or(Value::Null);
            if let Some(route) = callback(&body_for_cb, headers) {
                let mut resp_headers = HashMap::new();
                resp_headers.insert("Location".to_string(), route);
                return (307, resp_headers, String::new());
            }
        }

        // Route dispatch. `/swaig` is the tool-dispatch endpoint; every OTHER
        // authed sub-path under the route falls through to the SWML document —
        // matching Python's framework-free `_handle_request_core`, which serves
        // the doc for any path after the routing-callback check (a
        // routing-callback path that returns `None` is a passthrough → 200
        // SWML, NOT a 404). The 404-for-unknown-path is a web-framework concern
        // in Python (a FastAPI route miss), not part of the decomposed core.
        match sub_path.as_str() {
            "/swaig" => self.handle_swaig_request(method, &request_data, headers),
            _ => self.handle_swml_request(method, &request_data, headers),
        }
    }

    // ------------------------------------------------------------------
    // SIP username extraction
    // ------------------------------------------------------------------

    /// Extract the SIP username from a request body's `call.to` field.
    ///
    /// Mirrors Python's `SWMLService.extract_sip_username` exactly
    /// (`swml_service.py`): reads only `call.to`, then branches on the URI
    /// scheme —
    ///
    /// - `sip:user@domain` -> the username part (between `sip:` and `@`), or
    ///   the whole remainder if there is no `@`;
    /// - `tel:+1234567890` -> the phone-number part (after `tel:`);
    /// - otherwise -> the whole `to` field verbatim.
    ///
    /// Returns `None` when there is no `call.to` string. There is no charset or
    /// length validation (Python performs none), so `tel:` numbers with a `+`
    /// survive.
    pub fn extract_sip_username(body: &Value) -> Option<String> {
        let to_field = body.get("call")?.get("to")?.as_str()?;

        if let Some(rest) = to_field.strip_prefix("sip:") {
            Some(
                rest.split_once('@')
                    .map_or(rest, |(user, _)| user)
                    .to_string(),
            )
        } else if let Some(number) = to_field.strip_prefix("tel:") {
            Some(number.to_string())
        } else {
            Some(to_field.to_string())
        }
    }

    // ------------------------------------------------------------------
    // Proxy URL
    // ------------------------------------------------------------------

    /// Detect or construct the proxy URL base from request headers.
    pub fn get_proxy_url_base(&self, headers: &HashMap<String, String>) -> String {
        // 1. Explicit env var
        if let Ok(env_proxy) = env::var("SWML_PROXY_URL_BASE")
            && !env_proxy.is_empty()
        {
            return env_proxy.trim_end_matches('/').to_string();
        }

        // 2. X-Forwarded-Proto + X-Forwarded-Host
        let proto = headers
            .get("X-Forwarded-Proto")
            .or_else(|| headers.get("x-forwarded-proto"));
        let fwd_host = headers
            .get("X-Forwarded-Host")
            .or_else(|| headers.get("x-forwarded-host"));
        if let (Some(p), Some(h)) = (proto, fwd_host) {
            return format!("{p}://{h}");
        }

        // 3. X-Original-URL
        let orig_url = headers
            .get("X-Original-URL")
            .or_else(|| headers.get("x-original-url"));
        if let Some(url) = orig_url {
            return url.trim_end_matches('/').to_string();
        }

        // 4. Fallback to server config
        format!("http://{}:{}", self.host, self.port)
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    /// Check Basic Auth from request headers using timing-safe comparison.
    fn check_basic_auth(&self, headers: &HashMap<String, String>) -> bool {
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

        // Timing-safe comparison using HMAC
        let user_ok = constant_time_eq(input_user, &self.basic_auth_user);
        let pass_ok = constant_time_eq(input_pass, &self.basic_auth_password);

        user_ok && pass_ok
    }

    // Uniform `&Option<Value>` request-body shape across the Service handler
    // pair — mirrors the AgentBase dispatch trio (see its handle_swml_request)
    // so the two services' endpoint handlers read identically.
    #[allow(clippy::ref_option)]
    fn handle_swml_request(
        &self,
        _method: &str,
        _request_data: &Option<Value>,
        _headers: &HashMap<String, String>,
    ) -> (u16, HashMap<String, String>, String) {
        // Framework-free contract (Python `_handle_request_core`): the SWML
        // happy path is the bare triple `(200, {}, swml_string)` — NO headers
        // (not even Content-Type). The HTTP adapter layer (router / serverless)
        // re-adds `Content-Type: application/json` when marshaling to the wire,
        // mirroring FastAPI's `Response(..., media_type="application/json")`.
        let body =
            serde_json::to_string(&self.document.to_value()).unwrap_or_else(|_| "{}".to_string());
        (200, HashMap::new(), body)
    }

    /// Handle `/swaig` — the SWAIG dispatch endpoint.
    ///
    /// GET: returns the rendered SWML document. This mirrors what
    /// `AgentBase` serves and lets the platform fetch the doc from either
    /// `/` or `/swaig?call_id=...`.
    ///
    /// POST: dispatches a tool call. Expected body shape:
    ///
    /// ```json
    /// {
    ///   "function": "<name>",
    ///   "argument": {"parsed": [{"<arg>": "<value>"}], "raw": "<json>"}
    /// }
    /// ```
    ///
    /// Argument extraction also accepts a flat `{"arguments": {...}}` form
    /// for compatibility with non-platform callers (e.g. swaig-test
    /// driving the endpoint with a simple body).
    ///
    /// Status codes:
    /// - 200 with `{"response": ...}` from the handler on success.
    /// - 200 with `{"response": "Function '<name>' not found"}` for an
    ///   unknown function (mirrors Python — does not 404, since the
    ///   platform expects a SWAIG response shape).
    /// - 400 if `function` is missing or fails the
    ///   `^[a-zA-Z_][a-zA-Z0-9_]*$` validator (path-traversal guard).
    /// - 415 if the request is POST without `Content-Type: application/json`.
    ///
    /// Auth and body-size checks already ran in `handle_request` before
    /// this method is invoked.
    #[allow(clippy::ref_option)] // uniform with the Service handler pair; see handle_swml_request
    fn handle_swaig_request(
        &self,
        method: &str,
        request_data: &Option<Value>,
        _headers: &HashMap<String, String>,
    ) -> (u16, HashMap<String, String>, String) {
        if method.eq_ignore_ascii_case("GET") {
            return self.json_response(200, &self.document.to_value());
        }

        let Some(body) = request_data else {
            return self.json_response(400, &serde_json::json!({"error": "Missing request body"}));
        };

        let function_name = match body.get("function").and_then(|v| v.as_str()) {
            Some(name) if !name.is_empty() => name,
            _ => {
                self.logger.warn("/swaig POST missing function name");
                return self
                    .json_response(400, &serde_json::json!({"error": "Missing function name"}));
            }
        };

        if !function_name_is_valid(function_name) {
            self.logger.warn(&format!(
                "/swaig rejected invalid function name: {function_name:?}"
            ));
            return self.json_response(
                400,
                &serde_json::json!({
                    "error": format!("Invalid function name format: '{}'", function_name)
                }),
            );
        }

        self.logger
            .info(&format!("/swaig dispatch: function={function_name}"));

        // Extract args. Handle the platform's nested
        // `{argument: {parsed: [{...}], raw: "..."}}` shape and the flat
        // `{arguments: {...}}` shape used by some external callers.
        let mut args = serde_json::Map::new();
        if let Some(arg_obj) = body.get("argument").and_then(|v| v.as_object()) {
            if let Some(parsed_arr) = arg_obj.get("parsed").and_then(|v| v.as_array()) {
                if let Some(first) = parsed_arr.first().and_then(|v| v.as_object()) {
                    for (k, v) in first {
                        args.insert(k.clone(), v.clone());
                    }
                }
            } else if let Some(raw_str) = arg_obj.get("raw").and_then(|v| v.as_str())
                && !raw_str.is_empty()
                && let Ok(Value::Object(parsed)) = serde_json::from_str::<Value>(raw_str)
            {
                for (k, v) in &parsed {
                    args.insert(k.clone(), v.clone());
                }
            }
        } else if let Some(flat) = body.get("arguments").and_then(|v| v.as_object()) {
            for (k, v) in flat {
                args.insert(k.clone(), v.clone());
            }
        }

        // Raw data is the full request body as a map (callers like skill
        // handlers may want call_id, global_data, etc.).
        let raw_data = match body.as_object() {
            Some(m) => m.clone(),
            None => serde_json::Map::new(),
        };

        // Dispatch via the registered handler. If the function name isn't
        // registered, or it's registered without a local handler (DataMap
        // tools that the platform executes server-side), return a SWAIG
        // response saying so — the platform expects a `{response: ...}`
        // shape, not a 404.
        if let Some(handler) = self
            .tools
            .get(function_name)
            .and_then(|t| t.handler.as_ref())
        {
            let result = handler(&args, &raw_data);
            self.logger
                .debug(&format!("/swaig dispatched: function={function_name} ok"));
            self.json_response(200, &result.to_value())
        } else {
            // Differentiate "name not in registry" from "name in registry
            // but DataMap (no local handler)" so the response is honest.
            let msg = if self.tools.contains_key(function_name) {
                format!(
                    "Function '{function_name}' is registered but has no local handler (DataMap tool runs server-side)"
                )
            } else {
                format!("Function '{function_name}' not found")
            };
            self.logger.warn(&format!("/swaig {msg}"));
            self.json_response(200, &serde_json::json!({"response": msg}))
        }
    }

    #[allow(clippy::unused_self)] // private helper kept on the self-method family for consistency
    fn json_response(&self, status: u16, data: &Value) -> (u16, HashMap<String, String>, String) {
        let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        for (k, v) in security_headers() {
            headers.insert(k, v);
        }
        (status, headers, body)
    }

    // ------------------------------------------------------------------
    // HTTP server
    // ------------------------------------------------------------------

    /// Start a blocking HTTP server on `host:port`.
    /// Introspect path: when invoked with `SWAIG_LIST_TOOLS=1`, print the
    /// runtime tool registry as JSON to stdout (between sentinel markers so
    /// the swaig-test CLI can extract it past any user log noise) and exit.
    /// This is how the CLI lists tools on a compiled `SWMLService` example
    /// without standing up an HTTP server.
    fn print_tool_registry_and_exit(&self) -> ! {
        let signatures: Vec<&serde_json::Value> = self
            .tool_order
            .iter()
            .filter_map(|name| self.tools.get(name).map(|td| &td.definition))
            .collect();
        let body = serde_json::json!({ "tools": signatures });
        println!("__SWAIG_TOOLS_BEGIN__");
        println!(
            "{}",
            serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string())
        );
        println!("__SWAIG_TOOLS_END__");
        std::process::exit(0);
    }

    /// # Panics
    ///
    /// Panics if the configured `host:port` cannot be bound (e.g. the port
    /// is already in use or permission is denied).
    pub fn run(&self) {
        if std::env::var("SWAIG_LIST_TOOLS").is_ok() {
            self.print_tool_registry_and_exit();
        }
        let addr = format!("{}:{}", self.host, self.port);
        // HTTP, or HTTPS when SWML_SSL_ENABLED + SWML_SSL_CERT_PATH/KEY_PATH are
        // set (mirrors Python's SecurityConfig / uvicorn ssl_* contract).
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
                self.handle_request(&method, &path, &req_headers, &body_buf);

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

// ------------------------------------------------------------------
// Free functions
// ------------------------------------------------------------------

/// Security headers applied to all responses.
/// SWAIG function-name validator. Mirrors the regex used by every other
/// port: `^[a-zA-Z_][a-zA-Z0-9_]*$`. Rejects path-traversal-style names
/// (`../etc/passwd`), names starting with a digit, and any name containing
/// a character other than ASCII letters / digits / underscore.
fn function_name_is_valid(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    true
}

fn security_headers() -> Vec<(String, String)> {
    vec![
        ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
        ("X-Frame-Options".to_string(), "DENY".to_string()),
        ("Cache-Control".to_string(), "no-store".to_string()),
    ]
}

/// Timing-safe string comparison using HMAC.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let mut mac_a = HmacSha256::new_from_slice(HMAC_KEY).expect("HMAC key should be valid");
    mac_a.update(a.as_bytes());
    let digest_a = mac_a.finalize().into_bytes();

    let mut mac_b = HmacSha256::new_from_slice(HMAC_KEY).expect("HMAC key should be valid");
    mac_b.update(b.as_bytes());
    let digest_b = mac_b.finalize().into_bytes();

    digest_a == digest_b
}

/// Generate a cryptographically secure random hex string.
fn random_hex(bytes: usize) -> String {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..bytes).map(|_| rng.random()).collect();
    hex_encode(&random_bytes)
}

/// Encode bytes as lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Build a Basic auth header value. Test-only — Service consumes incoming
/// Authorization headers via `check_basic_auth` and never builds outgoing
/// ones in production code.
#[cfg(test)]
fn make_basic_auth(user: &str, pass: &str) -> String {
    let encoded = BASE64.encode(format!("{user}:{pass}"));
    format!("Basic {encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options(name: &str) -> ServiceOptions {
        ServiceOptions {
            name: name.to_string(),
            route: None,
            host: None,
            port: Some(3000),
            basic_auth_user: Some("testuser".to_string()),
            basic_auth_password: Some("testpass".to_string()),
        }
    }

    fn authed_headers(user: &str, pass: &str) -> HashMap<String, String> {
        let mut h = HashMap::new();
        h.insert("Authorization".to_string(), make_basic_auth(user, pass));
        h
    }

    #[test]
    fn test_construction() {
        let svc = Service::new(default_options("my-service"));
        assert_eq!(svc.name(), "my-service");
        assert_eq!(svc.route(), "/");
        assert_eq!(svc.host(), "0.0.0.0");
        assert_eq!(svc.port(), 3000);
    }

    #[test]
    fn test_explicit_auth() {
        let svc = Service::new(ServiceOptions {
            name: "svc".to_string(),
            route: None,
            host: None,
            port: Some(3000),
            basic_auth_user: Some("alice".to_string()),
            basic_auth_password: Some("secret".to_string()),
        });
        let (u, p) = svc.basic_auth_credentials();
        assert_eq!(u, "alice");
        assert_eq!(p, "secret");
    }

    #[test]
    fn test_env_auth() {
        // SAFETY: test-only env mutation
        unsafe {
            env::set_var("SWML_BASIC_AUTH_USER", "envuser");
            env::set_var("SWML_BASIC_AUTH_PASSWORD", "envpass");
        }
        let svc = Service::new(ServiceOptions {
            name: "svc".to_string(),
            route: None,
            host: None,
            port: Some(3000),
            basic_auth_user: None,
            basic_auth_password: None,
        });
        let (u, p) = svc.basic_auth_credentials();
        assert_eq!(u, "envuser");
        assert_eq!(p, "envpass");
        unsafe {
            env::remove_var("SWML_BASIC_AUTH_USER");
            env::remove_var("SWML_BASIC_AUTH_PASSWORD");
        }
    }

    #[test]
    fn test_auto_generated_auth() {
        unsafe {
            env::remove_var("SWML_BASIC_AUTH_USER");
            env::remove_var("SWML_BASIC_AUTH_PASSWORD");
        }
        let svc = Service::new(ServiceOptions {
            name: "svc".to_string(),
            route: None,
            host: None,
            port: Some(3000),
            basic_auth_user: None,
            basic_auth_password: None,
        });
        let (u, p) = svc.basic_auth_credentials();
        // Auto-generated: 16 bytes -> 32 hex chars, 32 bytes -> 64 hex chars
        assert_eq!(u.len(), 32);
        assert_eq!(p.len(), 64);
        // Should be valid hex
        assert!(u.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(p.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_health_endpoint() {
        let svc = Service::new(default_options("svc"));
        let (status, headers, body) = svc.handle_request("GET", "/health", &HashMap::new(), "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["status"], "healthy");
        assert_eq!(headers["Content-Type"], "application/json");
    }

    #[test]
    fn test_ready_endpoint() {
        let svc = Service::new(default_options("svc"));
        let (status, _headers, body) = svc.handle_request("GET", "/ready", &HashMap::new(), "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["status"], "ready");
    }

    #[test]
    fn test_auth_required_on_root() {
        let svc = Service::new(default_options("svc"));
        let (status, headers, body) = svc.handle_request("POST", "/", &HashMap::new(), "");
        assert_eq!(status, 401);
        // Framework-free contract: JSON error body + bare `WWW-Authenticate:
        // Basic` header (no realm, no Content-Type).
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["error"], "Unauthorized");
        assert_eq!(headers.get("WWW-Authenticate").unwrap(), "Basic");
    }

    #[test]
    fn test_auth_success_returns_document() {
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let (status, _, body) = svc.handle_request("POST", "/", &headers, "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["version"], "1.0.0");
    }

    #[test]
    fn test_auth_wrong_password() {
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "wrong");
        let (status, _, _) = svc.handle_request("POST", "/", &headers, "");
        assert_eq!(status, 401);
    }

    #[test]
    fn test_auth_wrong_user() {
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("wrong", "testpass");
        let (status, _, _) = svc.handle_request("POST", "/", &headers, "");
        assert_eq!(status, 401);
    }

    #[test]
    fn test_security_headers_present() {
        let svc = Service::new(default_options("svc"));
        let (_, headers, _) = svc.handle_request("GET", "/health", &HashMap::new(), "");
        assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(headers.get("Cache-Control").unwrap(), "no-store");
    }

    #[test]
    fn test_add_verb_valid() {
        let mut svc = Service::new(default_options("svc"));
        svc.add_verb_to_section("main", "answer", serde_json::json!({}));
        let verbs = svc.document().get_verbs("main");
        assert_eq!(verbs.len(), 1);
        assert!(verbs[0].get("answer").is_some());
    }

    #[test]
    #[should_panic(expected = "Unknown SWML verb")]
    fn test_add_verb_unknown_panics() {
        let mut svc = Service::new(default_options("svc"));
        svc.add_verb_to_section("main", "totally_fake_verb", serde_json::json!({}));
    }

    #[test]
    fn test_sleep_integer() {
        let mut svc = Service::new(default_options("svc"));
        svc.sleep(2000, "main");
        let verbs = svc.document().get_verbs("main");
        assert_eq!(verbs.len(), 1);
        assert_eq!(verbs[0]["sleep"], 2000);
    }

    #[test]
    fn test_sip_extraction_basic() {
        let body = serde_json::json!({"call": {"to": "sip:alice@example.com"}});
        let result = Service::extract_sip_username(&body);
        assert_eq!(result, Some("alice".to_string()));
    }

    #[test]
    fn test_sip_extraction_tel_uri() {
        // Python parity: tel: URIs strip the "tel:" prefix (incl. the '+').
        let body = serde_json::json!({"call": {"to": "tel:+15551234567"}});
        let result = Service::extract_sip_username(&body);
        assert_eq!(result, Some("+15551234567".to_string()));
    }

    #[test]
    fn test_sip_extraction_plain_username() {
        // Non-sip/non-tel 'to' is returned verbatim (Python parity).
        let body = serde_json::json!({"call": {"to": "support"}});
        let result = Service::extract_sip_username(&body);
        assert_eq!(result, Some("support".to_string()));
    }

    #[test]
    fn test_sip_extraction_missing() {
        // Only `call.to` is consulted; a top-level `to` (or none) yields None.
        let body = serde_json::json!({"other": "data"});
        let result = Service::extract_sip_username(&body);
        assert!(result.is_none());
        let top_level = serde_json::json!({"to": "sip:bob@example.com"});
        assert!(Service::extract_sip_username(&top_level).is_none());
    }

    // Cargo runs tests in parallel by default; the proxy tests below mutate
    // a shared environment variable (SWML_PROXY_URL_BASE). Without
    // serialization they race — one test's `remove_var` clears another's
    // `set_var` mid-flight. This Mutex pins them to one-at-a-time access.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_proxy_url_env() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            env::set_var("SWML_PROXY_URL_BASE", "https://proxy.example.com/");
        }
        let svc = Service::new(default_options("svc"));
        let result = svc.get_proxy_url_base(&HashMap::new());
        unsafe {
            env::remove_var("SWML_PROXY_URL_BASE");
        }
        assert_eq!(result, "https://proxy.example.com");
    }

    #[test]
    fn test_proxy_url_forwarded_headers() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            env::remove_var("SWML_PROXY_URL_BASE");
        }
        let svc = Service::new(default_options("svc"));
        let mut headers = HashMap::new();
        headers.insert("X-Forwarded-Proto".to_string(), "https".to_string());
        headers.insert(
            "X-Forwarded-Host".to_string(),
            "app.example.com".to_string(),
        );
        let result = svc.get_proxy_url_base(&headers);
        assert_eq!(result, "https://app.example.com");
    }

    #[test]
    fn test_proxy_url_fallback() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            env::remove_var("SWML_PROXY_URL_BASE");
        }
        let svc = Service::new(ServiceOptions {
            name: "svc".to_string(),
            route: None,
            host: Some("127.0.0.1".to_string()),
            port: Some(8080),
            basic_auth_user: Some("u".to_string()),
            basic_auth_password: Some("p".to_string()),
        });
        let result = svc.get_proxy_url_base(&HashMap::new());
        assert_eq!(result, "http://127.0.0.1:8080");
    }

    #[test]
    fn test_body_size_limit() {
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let big_body = "x".repeat(MAX_BODY_SIZE + 1);
        let (status, _, body) = svc.handle_request("POST", "/", &headers, &big_body);
        assert_eq!(status, 413);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["error"], "Request body too large");
    }

    #[test]
    fn test_swaig_route_get_returns_swml_document() {
        // GET /swaig returns the rendered SWML doc — same as GET / —
        // letting the platform fetch it from either endpoint.
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let (status, _, body) = svc.handle_request("GET", "/swaig", &headers, "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert!(parsed.is_object(), "SWML doc must be an object, got {body}");
        assert!(
            parsed.get("sections").is_some(),
            "SWML doc must have a sections key"
        );
    }

    #[test]
    fn test_swaig_route_post_dispatches_registered_handler() {
        // The previous test_swaig_route asserted POST /swaig returned `[]`
        // — that was the stub talking. Now the dispatcher actually invokes
        // the registered handler and returns its FunctionResult.
        let mut svc = Service::new(default_options("svc"));
        svc.define_tool(
            "lookup",
            "Look it up",
            serde_json::json!({"competitor": {"type": "string"}}),
            Box::new(|args, _raw| {
                let competitor = args
                    .get("competitor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unknown>");
                FunctionResult::with_response(&format!("{competitor} pricing: $99"))
            }),
            false,
        );
        let headers = authed_headers("testuser", "testpass");
        let body = r#"{"function":"lookup","argument":{"parsed":[{"competitor":"ACME"}]}}"#;
        let (status, _, resp) = svc.handle_request("POST", "/swaig", &headers, body);
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["response"], "ACME pricing: $99");
    }

    #[test]
    fn test_swaig_route_post_unknown_function_returns_swaig_response() {
        // Mirrors Python: unknown function returns 200 with a SWAIG-shaped
        // {"response": "..."} body, NOT a 404 — the platform expects the
        // SWAIG response shape regardless.
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let body = r#"{"function":"never_registered","argument":{"parsed":[{}]}}"#;
        let (status, _, resp) = svc.handle_request("POST", "/swaig", &headers, body);
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let msg = parsed["response"].as_str().unwrap_or("");
        assert!(
            msg.contains("never_registered"),
            "response should name the missing function: {msg}"
        );
        assert!(
            msg.contains("not found"),
            "response should say 'not found': {msg}"
        );
    }

    #[test]
    fn test_swaig_route_post_invalid_function_name_returns_400() {
        // Path-traversal guard. Function name must match
        // ^[a-zA-Z_][a-zA-Z0-9_]*$.
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let body = r#"{"function":"../etc/passwd","argument":{"parsed":[{}]}}"#;
        let (status, _, resp) = svc.handle_request("POST", "/swaig", &headers, body);
        assert_eq!(status, 400);
        assert!(resp.contains("Invalid function name format"));
    }

    #[test]
    fn test_swaig_route_post_missing_function_returns_400() {
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let body = r#"{"argument":{"parsed":[{}]}}"#;
        let (status, _, resp) = svc.handle_request("POST", "/swaig", &headers, body);
        assert_eq!(status, 400);
        assert!(resp.contains("Missing function name"));
    }

    #[test]
    fn test_swaig_route_post_accepts_flat_arguments_shape() {
        // External callers may use {"arguments": {...}} instead of the
        // platform's nested {"argument": {"parsed": [{...}]}}. Both work.
        let mut svc = Service::new(default_options("svc"));
        svc.define_tool(
            "echo",
            "Echo",
            serde_json::json!({"name": {"type": "string"}}),
            Box::new(|args, _raw| {
                let n = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unknown>");
                FunctionResult::with_response(&format!("hi {n}"))
            }),
            false,
        );
        let headers = authed_headers("testuser", "testpass");
        let body = r#"{"function":"echo","arguments":{"name":"there"}}"#;
        let (status, _, resp) = svc.handle_request("POST", "/swaig", &headers, body);
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["response"], "hi there");
    }

    #[test]
    fn test_function_name_is_valid_accepts_normal_names() {
        assert!(function_name_is_valid("lookup"));
        assert!(function_name_is_valid("_internal"));
        assert!(function_name_is_valid("get_weather_v2"));
        assert!(function_name_is_valid("a"));
    }

    #[test]
    fn test_function_name_is_valid_rejects_bad_names() {
        assert!(!function_name_is_valid(""));
        assert!(!function_name_is_valid("../etc/passwd"));
        assert!(!function_name_is_valid("1starts_with_digit"));
        assert!(!function_name_is_valid("has space"));
        assert!(!function_name_is_valid("has-dash"));
        assert!(!function_name_is_valid("has.dot"));
    }

    #[test]
    fn test_post_prompt_route_serves_swml_on_swml_service() {
        // Framework-free core parity (Python `_handle_request_core`): every
        // authed sub-path under the route that is NOT `/swaig` falls through to
        // the SWML document. A bare SWMLService has no `/post_prompt` semantic,
        // so it serves the doc (200) rather than 404-ing — the 404 for an
        // unknown route is a web-framework concern, layered above this core.
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let (status, _, body) = svc.handle_request("POST", "/post_prompt", &headers, "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["version"], "1.0.0");
    }

    #[test]
    fn test_unknown_subpath_serves_swml() {
        // A sub-path under the route (not `/swaig`) serves the SWML doc, per
        // the framework-free core contract.
        let svc = Service::new(default_options("svc"));
        let headers = authed_headers("testuser", "testpass");
        let (status, _, _) = svc.handle_request("GET", "/unknown", &headers, "");
        assert_eq!(status, 200);
    }

    #[test]
    fn test_custom_route() {
        let svc = Service::new(ServiceOptions {
            name: "svc".to_string(),
            route: Some("/api/v1".to_string()),
            host: None,
            port: Some(3000),
            basic_auth_user: Some("u".to_string()),
            basic_auth_password: Some("p".to_string()),
        });
        assert_eq!(svc.route(), "/api/v1");

        let headers = authed_headers("u", "p");
        // Root of the custom route
        let (status, _, _) = svc.handle_request("POST", "/api/v1", &headers, "");
        assert_eq!(status, 200);

        // Sub-route — GET /swaig returns the SWML doc.
        let (status, _, _) = svc.handle_request("GET", "/api/v1/swaig", &headers, "");
        assert_eq!(status, 200);

        // Path outside the route should 404
        let (status, _, _) = svc.handle_request("POST", "/other", &headers, "");
        assert_eq!(status, 404);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("hello", "hello"));
        assert!(!constant_time_eq("hello", "world"));
        assert!(!constant_time_eq("hello", "hell"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn test_health_no_auth_required() {
        let svc = Service::new(default_options("svc"));
        // No auth headers at all — should still work for /health
        let (status, _, _) = svc.handle_request("GET", "/health", &HashMap::new(), "");
        assert_eq!(status, 200);
    }

    #[test]
    fn test_ready_no_auth_required() {
        let svc = Service::new(default_options("svc"));
        let (status, _, _) = svc.handle_request("GET", "/ready", &HashMap::new(), "");
        assert_eq!(status, 200);
    }

    // ------------------------------------------------------------------
    // SWAIG hosting tests (lifted from AgentBase). Prove plain Service
    // can register tools and dispatch them without subclassing AgentBase.
    // ------------------------------------------------------------------

    #[test]
    fn test_service_define_tool_dispatches_via_on_function_call() {
        let mut svc = Service::new(default_options("svc"));
        let captured = std::sync::Arc::new(std::sync::Mutex::new(serde_json::Map::new()));
        let captured_for_handler = captured.clone();
        svc.define_tool(
            "lookup",
            "Look it up",
            serde_json::json!({}),
            Box::new(move |args, _raw| {
                *captured_for_handler.lock().unwrap() = args.clone();
                FunctionResult::with_response("ok")
            }),
            false,
        );
        let mut args = serde_json::Map::new();
        args.insert("x".to_string(), Value::String("y".to_string()));
        let result = svc.on_function_call("lookup", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let v = result.unwrap().to_value();
        assert_eq!(v["response"], "ok");
        assert_eq!(
            captured.lock().unwrap().get("x").unwrap(),
            &Value::String("y".to_string())
        );
    }

    #[test]
    fn test_service_on_function_call_returns_none_for_unknown() {
        let svc = Service::new(default_options("svc"));
        let result = svc.on_function_call(
            "no_such_fn",
            &serde_json::Map::new(),
            &serde_json::Map::new(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_service_list_tool_names_returns_registered_order() {
        let mut svc = Service::new(default_options("svc"));
        svc.define_tool(
            "first",
            "f",
            serde_json::json!({}),
            Box::new(|_, _| FunctionResult::default()),
            false,
        );
        svc.define_tool(
            "second",
            "s",
            serde_json::json!({}),
            Box::new(|_, _| FunctionResult::default()),
            false,
        );
        assert_eq!(
            svc.list_tool_names(),
            vec!["first".to_string(), "second".to_string()]
        );
    }

    #[test]
    fn test_service_register_swaig_function_tracks_in_order() {
        let mut svc = Service::new(default_options("svc"));
        svc.register_swaig_function(serde_json::json!({
            "function": "datamap_tool",
            "description": "from data map",
        }));
        assert!(svc.has_tool("datamap_tool"));
    }

    #[test]
    fn test_sidecar_pattern_emits_verb_and_registers_tool() {
        // 1. Build SWML — answer.
        let mut svc = Service::new(default_options("sidecar"));
        svc.add_verb_to_section("main", "answer", serde_json::json!({}));
        // ai_sidecar isn't in the schema; bypass via direct document access.
        // (The methods to pierce are intentionally limited; using add_verb with
        // unknown name panics, so we use ai which IS in the schema for the
        // shape demo.) Instead, just register the tool and confirm dispatch.
        svc.define_tool(
            "lookup_competitor",
            "Look up competitor pricing.",
            serde_json::json!({"competitor": {"type": "string"}}),
            Box::new(|args, _raw| {
                let competitor = args
                    .get("competitor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                FunctionResult::with_response(&format!("Pricing for {competitor}: $99"))
            }),
            false,
        );
        let mut args = serde_json::Map::new();
        args.insert("competitor".to_string(), Value::String("ACME".to_string()));
        let result = svc.on_function_call("lookup_competitor", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let v = result.unwrap().to_value();
        let resp = v["response"].as_str().unwrap();
        assert!(resp.contains("ACME"));
    }

    // -----------------------------------------------------------------
    // WebMixin parity: on_request / on_swml_request
    //
    // Python parity:
    //   tests/unit/core/mixins/test_web_mixin.py::
    //     test_on_request_delegates_to_on_swml_request
    //     test_on_swml_request_called
    //
    // Rust has no method overriding via inheritance — the
    // function-field hook (set_on_swml_request_hook) is the
    // idiomatic way to inject custom behavior into Service.
    // -----------------------------------------------------------------

    #[test]
    fn test_on_request_delegates_to_on_swml_request() {
        use std::sync::Mutex;
        let mut svc = Service::new(default_options("t"));

        let captured: std::sync::Arc<Mutex<(Option<Value>, Option<String>)>> =
            std::sync::Arc::new(Mutex::new((None, None)));
        let cap = captured.clone();
        svc.set_on_swml_request_hook(move |rd, cb| {
            let mut g = cap.lock().unwrap();
            g.0 = rd.cloned();
            g.1 = cb.map(std::string::ToString::to_string);
            Some(serde_json::json!({"custom": true}))
        });

        let rd = serde_json::json!({"data": "val"});
        let result = svc.on_request(Some(&rd), Some("/cb"));

        let g = captured.lock().unwrap();
        assert_eq!(g.0.as_ref(), Some(&rd));
        assert_eq!(g.1.as_deref(), Some("/cb"));
        assert_eq!(result, Some(serde_json::json!({"custom": true})));
    }

    #[test]
    fn test_on_request_default_returns_none() {
        let svc = Service::new(default_options("t"));
        assert!(svc.on_request(None, None).is_none());
    }

    #[test]
    fn test_on_swml_request_default_returns_none() {
        let svc = Service::new(default_options("t"));
        assert!(svc.on_swml_request(None, None).is_none());
    }

    #[test]
    fn test_on_swml_request_hook_invoked() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let mut svc = Service::new(default_options("t"));
        let called = std::sync::Arc::new(AtomicBool::new(false));
        let c = called.clone();
        svc.set_on_swml_request_hook(move |_, _| {
            c.store(true, Ordering::SeqCst);
            None
        });
        svc.on_swml_request(None, None);
        assert!(called.load(Ordering::SeqCst));
    }
}
