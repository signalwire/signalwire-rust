//! MCP Gateway skill — bridge MCP (Model Context Protocol) servers with SWAIG
//! functions.
//!
//! Mirrors Python's `signalwire.skills.mcp_gateway.MCPGatewaySkill`. This is the
//! **CLIENT** half of the MCP integration: it connects to an ALREADY-RUNNING MCP
//! Gateway service over HTTP, authenticates (bearer token OR HTTP basic auth),
//! enumerates the gateway's services + tools, and registers each remote MCP tool
//! as a local SWAIG function whose handler proxies the call back through the
//! gateway.
//!
//! ## Server half is Python-only (deliberate)
//!
//! Python additionally ships a standalone gateway SERVER (`signalwire/mcp_gateway/`)
//! that spawns and sandboxes MCP subprocesses and exposes them over HTTP — that
//! server, its subprocess/sandbox machinery, and the `mcp-gateway` CLI are NOT
//! ported to Rust (nor to any other SDK). Rust ships only the client skill that
//! talks to a running gateway. See `PORT_PHILOSOPHY_RUST.md` (the `mcp_gateway`
//! standalone server is listed in `PORT_OMISSIONS.md`).
//!
//! ## TLS verification (`verify_ssl`)
//!
//! `verify_ssl` is a config param with a SECURE default (`true` = verify ON). It
//! is threaded into the ureq TLS config: `disable_verification(!verify_ssl)` — so
//! the default path verifies the gateway's certificate, and only an explicit
//! `verify_ssl = false` opt-out (self-signed-cert environments) disables it. This
//! mirrors Python's `verify=self.verify_ssl` (`skill.py`).

use base64::Engine as _;
use serde_json::{Map, Value, json};
use ureq::tls::TlsConfig;

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Bridge MCP servers with SWAIG functions (client to a running MCP Gateway).
pub struct McpGateway {
    sp: SkillParams,
    /// Bearer token, when token auth is configured (else basic auth is used).
    auth_token: Option<String>,
    /// `(user, password)` for HTTP basic auth, when no bearer token is set.
    basic_auth: Option<(String, String)>,
    /// Gateway base URL (trailing slash stripped) — set in `setup`.
    gateway_url: String,
    /// Configured services (each a `{ "name": ..., "tools": ... }` object).
    services: Vec<Value>,
    tool_prefix: String,
    session_timeout: i64,
    retry_attempts: i64,
    request_timeout: i64,
    /// SECURE default `true` — TLS verification ON unless explicitly opted out.
    verify_ssl: bool,
}

impl McpGateway {
    pub fn new(params: Map<String, Value>) -> Self {
        McpGateway {
            sp: SkillParams::new(params),
            auth_token: None,
            basic_auth: None,
            gateway_url: String::new(),
            services: Vec::new(),
            tool_prefix: "mcp_".to_string(),
            session_timeout: 300,
            retry_attempts: 3,
            request_timeout: 30,
            verify_ssl: true,
        }
    }

    /// Build a ureq agent whose TLS verification is controlled by `verify_ssl`.
    ///
    /// SECURE default: when `verify_ssl` is `true` (the default) the gateway's
    /// certificate IS verified. Only an explicit `verify_ssl = false` disables
    /// it — `disable_verification(!verify_ssl)` — for self-signed-cert gateways.
    /// This is the Rust wiring of Python's `verify=self.verify_ssl`.
    fn http_agent(&self) -> ureq::Agent {
        let tls = TlsConfig::builder()
            .disable_verification(!self.verify_ssl)
            .build();
        ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(
                u64::try_from(self.request_timeout).unwrap_or(30),
            )))
            .http_status_as_error(false)
            .tls_config(tls)
            .build()
            .into()
    }

    /// Apply this skill's auth to a `WithoutBody` (GET/DELETE) request builder.
    fn auth_get(
        &self,
        rb: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    ) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
        match auth_header_value(self.auth_token.as_deref(), self.basic_auth.as_ref()) {
            Some(hv) => rb.header("Authorization", &hv),
            None => rb,
        }
    }

    /// Register a single MCP tool (from a service's tool list) as a SWAIG
    /// function. Mirrors Python `_register_mcp_tool`: names it
    /// `{tool_prefix}{service}_{tool}`, converts the MCP `inputSchema` into SWAIG
    /// parameters, and wires a handler that proxies the call through the gateway.
    fn register_mcp_tool(&self, agent: &mut AgentBase, service_name: &str, tool_def: &Value) {
        let Some(tool_name) = tool_def.get("name").and_then(|v| v.as_str()) else {
            return;
        };
        let swaig_name = format!("{}{}_{}", self.tool_prefix, service_name, tool_name);

        // Convert the MCP inputSchema.properties → SWAIG parameters, carrying
        // through enum/default and the required-argument list.
        let input_schema = tool_def.get("inputSchema").cloned().unwrap_or(json!({}));
        let properties = input_schema
            .get("properties")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let required: Vec<String> = input_schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut swaig_props = Map::new();
        for (prop_name, prop_def) in &properties {
            let mut param = Map::new();
            param.insert(
                "type".to_string(),
                json!(
                    prop_def
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("string")
                ),
            );
            param.insert(
                "description".to_string(),
                json!(
                    prop_def
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                ),
            );
            if let Some(en) = prop_def.get("enum") {
                param.insert("enum".to_string(), en.clone());
            }
            if let Some(def) = prop_def.get("default")
                && !required.contains(prop_name)
            {
                param.insert("default".to_string(), def.clone());
            }
            swaig_props.insert(prop_name.clone(), Value::Object(param));
        }

        let mut argument = Map::new();
        argument.insert("type".to_string(), json!("object"));
        argument.insert("properties".to_string(), Value::Object(swaig_props));
        if !required.is_empty() {
            argument.insert("required".to_string(), json!(required));
        }

        // Snapshot everything the handler needs (it must be 'static + Send/Sync).
        let gateway_url = self.gateway_url.clone();
        let service = service_name.to_string();
        let tool = tool_name.to_string();
        let auth_token = self.auth_token.clone();
        let basic_auth = self.basic_auth.clone();
        let verify_ssl = self.verify_ssl;
        let session_timeout = self.session_timeout;
        let retry_attempts = self.retry_attempts;
        let request_timeout = self.request_timeout;
        let agent_name = agent.get_name();

        let description = format!(
            "[{service}] {}",
            tool_def
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(&tool)
        );

        agent.define_tool(
            &swaig_name,
            &description,
            Value::Object(argument),
            Box::new(move |args, raw_data| {
                call_mcp_tool(
                    &gateway_url,
                    &service,
                    &tool,
                    auth_token.as_deref(),
                    basic_auth.as_ref(),
                    verify_ssl,
                    session_timeout,
                    retry_attempts,
                    request_timeout,
                    &agent_name,
                    args,
                    raw_data,
                )
            }),
            true,
        );
    }
}

impl SkillBase for McpGateway {
    fn name(&self) -> &'static str {
        "mcp_gateway"
    }

    fn description(&self) -> &'static str {
        "Bridge MCP servers with SWAIG functions"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn skill_state(&self) -> Option<&crate::skills::skill_base::SkillParams> {
        Some(&self.sp)
    }

    /// Python `REQUIRED_PACKAGES = ["requests"]`. Rust links its HTTP client
    /// (`ureq`) at build time, so this is purely declarative surface.
    fn required_packages(&self) -> Vec<String> {
        vec!["requests".to_string()]
    }

    /// JSON-Schema describing the accepted configuration parameters. Extends the
    /// shared base schema (`swaig_fields` / `skip_prompt` / `tool_name`) with the
    /// gateway connection + auth + behaviour params. Mirrors Python's
    /// `get_parameter_schema`.
    fn get_parameter_schema(&self) -> Value {
        let mut schema = crate::skills::skill_base::default_parameter_schema();
        if let Some(props) = schema.get_mut("properties").and_then(|v| v.as_object_mut()) {
            props.insert(
                "gateway_url".to_string(),
                json!({
                    "type": "string",
                    "description": "URL of the MCP Gateway service",
                    "required": true,
                }),
            );
            props.insert(
                "auth_token".to_string(),
                json!({
                    "type": "string",
                    "description": "Bearer token for authentication (alternative to basic auth)",
                    "required": false,
                    "hidden": true,
                    "env_var": "MCP_GATEWAY_AUTH_TOKEN",
                }),
            );
            props.insert("auth_user".to_string(), json!({
                "type": "string",
                "description": "Username for basic authentication (required if auth_token not provided)",
                "required": false,
                "env_var": "MCP_GATEWAY_AUTH_USER",
            }));
            props.insert("auth_password".to_string(), json!({
                "type": "string",
                "description": "Password for basic authentication (required if auth_token not provided)",
                "required": false,
                "hidden": true,
                "env_var": "MCP_GATEWAY_AUTH_PASSWORD",
            }));
            props.insert(
                "services".to_string(),
                json!({
                    "type": "array",
                    "description": "List of MCP services to connect to (empty for all available)",
                    "default": [],
                    "required": false,
                }),
            );
            props.insert(
                "session_timeout".to_string(),
                json!({
                    "type": "integer",
                    "description": "Session timeout in seconds",
                    "default": 300,
                    "required": false,
                }),
            );
            props.insert(
                "tool_prefix".to_string(),
                json!({
                    "type": "string",
                    "description": "Prefix for registered SWAIG function names",
                    "default": "mcp_",
                    "required": false,
                }),
            );
            props.insert(
                "retry_attempts".to_string(),
                json!({
                    "type": "integer",
                    "description": "Number of retry attempts for failed requests",
                    "default": 3,
                    "required": false,
                }),
            );
            props.insert(
                "request_timeout".to_string(),
                json!({
                    "type": "integer",
                    "description": "Request timeout in seconds",
                    "default": 30,
                    "required": false,
                }),
            );
            // SECURE default: TLS verification ON. `false` opts out for
            // self-signed-cert gateways (threaded to disable_verification).
            props.insert(
                "verify_ssl".to_string(),
                json!({
                    "type": "boolean",
                    "description": "Verify SSL certificates",
                    "default": true,
                    "required": false,
                }),
            );
        }
        schema
    }

    /// Setup + validate configuration. Mirrors Python `setup`: requires a bearer
    /// token OR (`gateway_url` + `auth_user` + `auth_password`) for basic auth; stores
    /// the config; validates the gateway `/health` endpoint. Returns `false` on
    /// missing params or an unreachable gateway.
    fn setup(&mut self) -> bool {
        // Resolve auth: bearer token (param or env) wins; else basic auth.
        let auth_token = self
            .sp
            .get_str("auth_token")
            .map(str::to_string)
            .or_else(|| std::env::var("MCP_GATEWAY_AUTH_TOKEN").ok())
            .filter(|s| !s.is_empty());

        let gateway_url = self.sp.get_str("gateway_url").unwrap_or("").to_string();

        if let Some(token) = auth_token {
            if gateway_url.is_empty() {
                return false;
            }
            self.auth_token = Some(token);
            self.basic_auth = None;
        } else {
            let user = self
                .sp
                .get_str("auth_user")
                .map(str::to_string)
                .or_else(|| std::env::var("MCP_GATEWAY_AUTH_USER").ok())
                .unwrap_or_default();
            let pass = self
                .sp
                .get_str("auth_password")
                .map(str::to_string)
                .or_else(|| std::env::var("MCP_GATEWAY_AUTH_PASSWORD").ok())
                .unwrap_or_default();
            if gateway_url.is_empty() || user.is_empty() || pass.is_empty() {
                return false;
            }
            self.auth_token = None;
            self.basic_auth = Some((user, pass));
        }

        self.gateway_url = gateway_url.trim_end_matches('/').to_string();
        self.services = self.sp.get_array("services");
        self.session_timeout = self.sp.get_i64("session_timeout", 300);
        self.tool_prefix = self.sp.get_str("tool_prefix").unwrap_or("mcp_").to_string();
        self.retry_attempts = self.sp.get_i64("retry_attempts", 3);
        self.request_timeout = self.sp.get_i64("request_timeout", 30);
        // SECURE default true (verify ON) — get_bool_or falls back to true.
        self.verify_ssl = self.sp.get_bool_or("verify_ssl", true);

        // Validate the gateway connection (GET /health, 2xx expected).
        let url = format!("{}/health", self.gateway_url);
        let agent = self.http_agent();
        match self.auth_get(agent.get(&url)).call() {
            Ok(resp) => (200..300).contains(&resp.status().as_u16()),
            Err(_) => false,
        }
    }

    /// Register SWAIG tools from the gateway's MCP services. Mirrors Python
    /// `register_tools`: when no services are configured, lists all available;
    /// for each service fetches its tools, applies the `tools` filter, and
    /// registers each as a SWAIG function.
    fn register_tools(&self, agent: &mut AgentBase) {
        let agent_http = self.http_agent();

        // Resolve the service list: configured, or all available from the gateway.
        let mut services = self.services.clone();
        if services.is_empty() {
            let url = format!("{}/services", self.gateway_url);
            if let Ok(resp) = self.auth_get(agent_http.get(&url)).call()
                && (200..300).contains(&resp.status().as_u16())
                && let Ok(body) = resp.into_body().read_to_string()
                && let Ok(Value::Array(names)) = serde_json::from_str::<Value>(&body)
            {
                services = names
                    .into_iter()
                    .filter_map(|n| n.as_str().map(|s| json!({ "name": s })))
                    .collect();
            }
        }

        for service_config in &services {
            let Some(service_name) = service_config.get("name").and_then(|v| v.as_str()) else {
                continue;
            };

            let url = format!("{}/services/{}/tools", self.gateway_url, service_name);
            let Ok(resp) = self.auth_get(agent_http.get(&url)).call() else {
                continue;
            };
            if !(200..300).contains(&resp.status().as_u16()) {
                continue;
            }
            let Ok(body) = resp.into_body().read_to_string() else {
                continue;
            };
            let Ok(tools_data) = serde_json::from_str::<Value>(&body) else {
                continue;
            };
            let mut tools = tools_data
                .get("tools")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            // Filter tools when a specific list is configured ("*" = all).
            if let Some(filter) = service_config.get("tools").and_then(|v| v.as_array()) {
                let allowed: Vec<&str> = filter.iter().filter_map(|v| v.as_str()).collect();
                tools.retain(|t| {
                    t.get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(|n| allowed.contains(&n))
                });
            }

            for tool in &tools {
                self.register_mcp_tool(agent, service_name, tool);
            }
        }
    }

    /// Speech-recognition hints. Mirrors Python `get_hints`: the literals
    /// `MCP`/`gateway` plus every configured service name.
    fn get_hints(&self) -> Vec<String> {
        let mut hints = vec!["MCP".to_string(), "gateway".to_string()];
        for service in &self.services {
            if let Some(name) = service.get("name").and_then(|v| v.as_str()) {
                hints.push(name.to_string());
            }
        }
        hints
    }

    /// Global data for `DataMap` variables. Mirrors Python `get_global_data`:
    /// the gateway URL, the (initially null) session id, and the service names.
    fn get_global_data(&self) -> Map<String, Value> {
        let service_names: Vec<Value> = self
            .services
            .iter()
            .map(|s| {
                s.get("name")
                    .cloned()
                    .unwrap_or_else(|| json!(s.as_str().unwrap_or("")))
            })
            .collect();
        let mut data = Map::new();
        data.insert("mcp_gateway_url".to_string(), json!(self.gateway_url));
        // Session id is established from the call at tool-call time (Python
        // initializes self.session_id = None); no session yet at config time.
        data.insert("mcp_session_id".to_string(), Value::Null);
        data.insert("mcp_services".to_string(), Value::Array(service_names));
        data
    }

    /// Prompt sections. Mirrors Python `get_prompt_sections`: a single "MCP
    /// Gateway Integration" section describing the connected services, emitted
    /// only when at least one service is configured (and `skip_prompt` is unset).
    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        let mut descriptions = Vec::new();
        for service in &self.services {
            if let Some(name) = service.get("name").and_then(|v| v.as_str()) {
                match service.get("tools") {
                    Some(Value::Array(tools)) => {
                        descriptions.push(format!("{name} ({} tools)", tools.len()));
                    }
                    // "*" or unset → all tools.
                    _ => descriptions.push(format!("{name} (all tools)")),
                }
            } else if let Some(s) = service.as_str() {
                descriptions.push(s.to_string());
            }
        }

        if descriptions.is_empty() {
            return Vec::new();
        }

        vec![json!({
            "title": "MCP Gateway Integration",
            "body": "You have access to external MCP (Model Context Protocol) services through a gateway.",
            "bullets": [
                format!("Connected to gateway at {}", self.gateway_url),
                format!("Available services: {}", descriptions.join(", ")),
                format!("Functions are prefixed with '{}' followed by service name", self.tool_prefix),
                "Each service maintains its own session state throughout the call".to_string(),
            ],
        })]
    }
}

/// Compute the `Authorization` header value for the configured auth: a bearer
/// token (`Bearer <token>`) when set, else HTTP basic (`Basic <base64(user:pass)>`),
/// else `None`. Shared by the GET/DELETE (`auth_get`) and POST (`call_mcp_tool`)
/// paths so the two auth modes are wired identically.
fn auth_header_value(
    auth_token: Option<&str>,
    basic_auth: Option<&(String, String)>,
) -> Option<String> {
    if let Some(token) = auth_token {
        Some(format!("Bearer {token}"))
    } else if let Some((user, pass)) = basic_auth {
        let enc = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"));
        Some(format!("Basic {enc}"))
    } else {
        None
    }
}

/// Call an MCP tool through the gateway (the SWAIG-handler body). Mirrors Python
/// `_call_mcp_tool`: derives the session id from `global_data.mcp_call_id` or the
/// top-level `call_id`, POSTs `{tool, arguments, session_id, timeout, metadata}`
/// to `/services/{service}/call`, retries on 5xx up to `retry_attempts`, and
/// returns the gateway's `result` (or an error string) as a `FunctionResult`.
#[allow(clippy::too_many_arguments)]
fn call_mcp_tool(
    gateway_url: &str,
    service_name: &str,
    tool_name: &str,
    auth_token: Option<&str>,
    basic_auth: Option<&(String, String)>,
    verify_ssl: bool,
    session_timeout: i64,
    retry_attempts: i64,
    request_timeout: i64,
    agent_name: &str,
    args: &Map<String, Value>,
    raw_data: &Map<String, Value>,
) -> FunctionResult {
    // Session id: prefer global_data.mcp_call_id, else the top-level call_id.
    let session_id = raw_data
        .get("global_data")
        .and_then(|g| g.get("mcp_call_id"))
        .and_then(|v| v.as_str())
        .or_else(|| raw_data.get("call_id").and_then(|v| v.as_str()))
        .unwrap_or("unknown")
        .to_string();

    let request_data = json!({
        "tool": tool_name,
        "arguments": Value::Object(args.clone()),
        "session_id": session_id,
        "timeout": session_timeout,
        "metadata": {
            "agent_id": agent_name,
            "timestamp": raw_data.get("timestamp").cloned().unwrap_or(Value::Null),
            "call_id": raw_data.get("call_id").cloned().unwrap_or(Value::Null),
        },
    });

    let tls = TlsConfig::builder()
        .disable_verification(!verify_ssl)
        .build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(
            u64::try_from(request_timeout).unwrap_or(30),
        )))
        .http_status_as_error(false)
        .tls_config(tls)
        .build()
        .into();

    let url = format!("{gateway_url}/services/{service_name}/call");
    let mut last_error = String::from("no attempt made");

    for _attempt in 0..retry_attempts.max(1) {
        let mut rb = agent.post(&url);
        if let Some(hv) = auth_header_value(auth_token, basic_auth) {
            rb = rb.header("Authorization", &hv);
        }

        match rb.send_json(&request_data) {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.into_body().read_to_string().unwrap_or_default();
                if status == 200 {
                    let result_text = serde_json::from_str::<Value>(&body)
                        .ok()
                        .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from))
                        .unwrap_or_else(|| "No response".to_string());
                    let mut r = FunctionResult::new();
                    r.set_response(&result_text);
                    return r;
                }
                // Non-200: extract an error message; retry only on 5xx.
                last_error = serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                    .unwrap_or_else(|| format!("HTTP {status}"));
                if status < 500 {
                    break; // client error — don't retry
                }
            }
            Err(e) => {
                last_error = e.to_string();
                // Transport error — retry (matches Python's timeout/conn retry).
            }
        }
    }

    let mut r = FunctionResult::new();
    r.set_response(&format!(
        "Failed to call {service_name}.{tool_name}: {last_error}"
    ));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill_with(params: Value) -> McpGateway {
        McpGateway::new(params.as_object().cloned().unwrap_or_default())
    }

    #[test]
    fn test_mcp_gateway_metadata() {
        let skill = skill_with(json!({}));
        assert_eq!(skill.name(), "mcp_gateway");
        assert_eq!(
            skill.description(),
            "Bridge MCP servers with SWAIG functions"
        );
        assert!(skill.supports_multiple_instances());
        assert_eq!(skill.required_packages(), vec!["requests".to_string()]);
    }

    #[test]
    fn test_verify_ssl_default_true_and_schema() {
        // Default (unset) → verify_ssl true (SECURE).
        let skill = skill_with(json!({}));
        assert!(skill.sp.get_bool_or("verify_ssl", true));

        // Schema exposes verify_ssl with default true.
        let schema = skill.get_parameter_schema();
        let vs = &schema["properties"]["verify_ssl"];
        assert_eq!(vs["type"], json!("boolean"));
        assert_eq!(vs["default"], json!(true));
        // The gateway_url + auth params are present too.
        assert!(schema["properties"].get("gateway_url").is_some());
        assert!(schema["properties"].get("auth_token").is_some());
        assert!(schema["properties"].get("auth_user").is_some());
    }

    #[test]
    fn test_verify_ssl_false_builds_accept_invalid_agent() {
        // verify_ssl=false must be threaded to disable_verification(true); the
        // agent builds without panicking (the accept-invalid-cert TLS path).
        let mut skill = skill_with(json!({"verify_ssl": false}));
        skill.verify_ssl = false;
        let _agent = skill.http_agent(); // builds → true path exercised
        assert!(!skill.verify_ssl);

        // And the secure default path builds too.
        let mut secure = skill_with(json!({}));
        secure.verify_ssl = true;
        let _secure_agent = secure.http_agent();
        assert!(secure.verify_ssl);
    }

    #[test]
    fn test_setup_requires_gateway_url() {
        // No gateway_url + no auth → setup fails (missing basic-auth params).
        let mut skill = skill_with(json!({}));
        assert!(!skill.setup());

        // Token auth but no gateway_url → still fails.
        let mut skill2 = skill_with(json!({"auth_token": "t"}));
        assert!(!skill2.setup());
    }

    #[test]
    fn test_get_hints_includes_services() {
        let mut skill = skill_with(json!({}));
        skill.services = vec![json!({"name": "filesystem"}), json!({"name": "github"})];
        let hints = skill.get_hints();
        assert!(hints.contains(&"MCP".to_string()));
        assert!(hints.contains(&"gateway".to_string()));
        assert!(hints.contains(&"filesystem".to_string()));
        assert!(hints.contains(&"github".to_string()));
    }

    #[test]
    fn test_get_global_data_shape() {
        let mut skill = skill_with(json!({}));
        skill.gateway_url = "https://gw.example.com".to_string();
        skill.services = vec![json!({"name": "fs"})];
        let gd = skill.get_global_data();
        assert_eq!(gd["mcp_gateway_url"], json!("https://gw.example.com"));
        assert_eq!(gd["mcp_session_id"], Value::Null);
        assert_eq!(gd["mcp_services"], json!(["fs"]));
    }

    #[test]
    fn test_get_prompt_sections_empty_without_services() {
        let skill = skill_with(json!({}));
        assert!(skill.get_prompt_sections().is_empty());
    }

    #[test]
    fn test_get_prompt_sections_with_services() {
        let mut skill = skill_with(json!({}));
        skill.gateway_url = "https://gw".to_string();
        skill.services = vec![json!({"name": "fs", "tools": ["read", "write"]})];
        let sections = skill.get_prompt_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0]["title"], json!("MCP Gateway Integration"));
        let bullets = sections[0]["bullets"].as_array().unwrap();
        assert!(
            bullets
                .iter()
                .any(|b| b.as_str().unwrap().contains("fs (2 tools)"))
        );
    }

    #[test]
    fn test_skip_prompt_suppresses_sections() {
        let mut skill = skill_with(json!({"skip_prompt": true}));
        skill.services = vec![json!({"name": "fs"})];
        assert!(skill.get_prompt_sections().is_empty());
    }

    /// Spin a tiny local HTTP "gateway" on 127.0.0.1:0 that serves the MCP
    /// Gateway routes the skill hits (`/health`, `/services`,
    /// `/services/{name}/tools`, `/services/{name}/call`). Returns the base URL
    /// and a join handle. The server answers a fixed number of requests then
    /// exits so the test never hangs. This is the HTTP mock for the
    /// register-tools + call flow (no external network).
    fn spawn_mock_gateway(max_requests: usize) -> (String, std::thread::JoinHandle<()>) {
        use std::time::Duration;
        let server = tiny_http::Server::http("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = server.server_addr().to_ip().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");
        let handle = std::thread::spawn(move || {
            for _ in 0..max_requests {
                let Ok(Some(req)) = server.recv_timeout(Duration::from_millis(1500)) else {
                    break;
                };
                let url = req.url().to_string();
                let (status, body): (u16, String) = if url.starts_with("/health") {
                    (200, "{\"ok\":true}".to_string())
                } else if url == "/services" {
                    (200, "[\"filesystem\"]".to_string())
                } else if url == "/services/filesystem/tools" {
                    (
                        200,
                        json!({
                            "tools": [{
                                "name": "read_file",
                                "description": "Read a file",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "path": {"type": "string", "description": "the path"}
                                    },
                                    "required": ["path"]
                                }
                            }]
                        })
                        .to_string(),
                    )
                } else if url.ends_with("/call") {
                    (200, json!({"result": "file contents"}).to_string())
                } else {
                    (404, "{}".to_string())
                };
                let resp = tiny_http::Response::from_string(body)
                    .with_status_code(status)
                    .with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    );
                let _ = req.respond(resp);
            }
        });
        (base, handle)
    }

    #[test]
    fn test_setup_connects_to_live_gateway() {
        // A reachable gateway `/health` → setup succeeds (token auth path).
        let (base, handle) = spawn_mock_gateway(1);
        let mut skill = skill_with(json!({
            "gateway_url": base,
            "auth_token": "secret-token",
        }));
        assert!(
            skill.setup(),
            "setup should succeed against a healthy gateway"
        );
        assert_eq!(skill.auth_token.as_deref(), Some("secret-token"));
        assert!(skill.basic_auth.is_none());
        handle.join().unwrap();
    }

    #[test]
    fn test_register_tools_registers_gateway_tools_as_swaig_functions() {
        use crate::agent::{AgentBase, AgentOptions};

        // health + /services + /services/filesystem/tools = 3 requests.
        let (base, handle) = spawn_mock_gateway(3);

        let mut skill = skill_with(json!({
            "gateway_url": base,
            "auth_token": "t",
            "verify_ssl": true,
        }));
        assert!(skill.setup());

        let mut agent = AgentBase::new(AgentOptions::new("mcp-agent"));
        skill.register_tools(&mut agent);

        // The gateway's `filesystem/read_file` tool is registered as a SWAIG
        // function named `{prefix}{service}_{tool}` = `mcp_filesystem_read_file`.
        assert!(
            agent.get_function("mcp_filesystem_read_file").is_some(),
            "expected the gateway tool to be registered as a SWAIG function"
        );
        let def = &agent
            .get_function("mcp_filesystem_read_file")
            .unwrap()
            .definition;
        // The MCP inputSchema was converted into SWAIG argument properties +
        // the required list carried through.
        assert_eq!(
            def["argument"]["properties"]["path"]["type"],
            json!("string")
        );
        assert_eq!(def["argument"]["required"], json!(["path"]));
        assert!(def["purpose"].as_str().unwrap().contains("[filesystem]"));

        handle.join().unwrap();
    }

    #[test]
    fn test_register_tools_with_verify_ssl_false_still_registers() {
        use crate::agent::{AgentBase, AgentOptions};

        // Same flow but with verify_ssl=false (the disable_verification path);
        // the plain-HTTP mock is reachable either way — this exercises that the
        // accept-invalid TLS agent builds and drives the register flow.
        let (base, handle) = spawn_mock_gateway(3);
        let mut skill = skill_with(json!({
            "gateway_url": base,
            "auth_token": "t",
            "verify_ssl": false,
        }));
        assert!(skill.setup());
        assert!(!skill.verify_ssl);

        let mut agent = AgentBase::new(AgentOptions::new("mcp-agent"));
        skill.register_tools(&mut agent);
        assert!(agent.get_function("mcp_filesystem_read_file").is_some());
        handle.join().unwrap();
    }
}
