// Copyright (c) 2026 SignalWire
//
// This file is part of the SignalWire SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Async client for the SignalWire AI Chat service.
//!
//! The client speaks the standard SignalWire front-door protocol: HTTP Basic
//! `project:api_token` with the space in the hostname —
//! `POST https://{space}.signalwire.com/api/ai/chat` — carrying a JSON-RPC 2.0
//! body whose params are pure payload (identity NEVER appears in the body; it
//! rides the Basic-auth header only).
//!
//! Async by nature: a [`AIChatClient::chat`] call awaits a full LLM round trip
//! (seconds, not milliseconds), and the typical consumers — bots, MCP servers —
//! run on async event loops where a blocking HTTP call would stall every other
//! conversation. This mirrors the async-first python reference
//! `signalwire.ai_chat.AIChatClient` (aiohttp). The AI Chat module is the one
//! async surface in this otherwise-synchronous crate: it is built on `tokio` +
//! `reqwest` because its contract (a long-lived streaming LLM turn on an event
//! loop) is fundamentally async, exactly as the reference is.
//!
//! # Streaming / liveness
//!
//! The service streams keepalive whitespace ahead of a slow response body (proxy
//! read-timeout protection, roughly every 10s), so liveness is byte-driven rather
//! than wall-clock: there is NO total-request timeout an idle-but-live turn could
//! trip — only a per-read idle timeout, mirroring the python reference's
//! `aiohttp.ClientTimeout(total=None, connect=10, sock_read=60)`. `reqwest`
//! exposes exactly this as [`reqwest::ClientBuilder::read_timeout`] (idle time
//! between reads) + [`reqwest::ClientBuilder::connect_timeout`]; a total
//! `.timeout()` cap is deliberately absent so a slow-but-live turn is never
//! severed. Leading whitespace is valid JSON, so the buffered parse is unaffected.
//!
//! # Example
//!
//! ```no_run
//! use signalwire::ai_chat::{AIChatClient, ChatOptions, CreateOptions};
//!
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! // env supplies creds (SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN)
//! let client = AIChatClient::builder().space("myspace").build()?;
//! client
//!     .create_conversation("conv-1", CreateOptions::new("http://cfg"))
//!     .await?;
//! let reply = client.chat("conv-1", "hello", ChatOptions::default()).await?;
//! println!("{}", reply.text);
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{Map, Value, json};

/// Default endpoint path appended to a `space`-derived base URL.
const DEFAULT_PATH: &str = "/api/ai/chat";

/// Bounded connect timeout (seconds). Mirrors the reference `connect=10`.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Idle read timeout (seconds): the max byte-silence tolerated on a single
/// request before the connection is treated as dead. Mirrors the reference
/// `sock_read=60`. The service heartbeats well inside this window, so a
/// live-but-slow turn never trips it; it bounds a truly dead connection only.
/// This is NOT a total-request cap — turn length is the server's business.
const DEFAULT_READ_IDLE_TIMEOUT_SECS: u64 = 60;

// ── Errors ───────────────────────────────────────────────────────────

/// The kind of an [`AIChatError`] — the typed error family for AI Chat failures.
///
/// Mirrors the python reference's exception hierarchy. Every variant carries the
/// JSON-RPC `code` and server `message` via the enclosing [`AIChatError`]; an
/// unmapped code falls to [`AIChatErrorKind::Api`]. [`AIChatErrorKind::Summary`]
/// rides the JSON-RPC *success* envelope (no code) and exists so a failed summary
/// can never masquerade as an empty string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIChatErrorKind {
    /// Missing/rejected identity (HTTP 401 / JSON-RPC -32009).
    Authentication,
    /// The conversation does not exist in this project (-32001).
    ConversationNotFound,
    /// Project or conversation rate limit hit (-32005 / -32006).
    RateLimit,
    /// Another message is being processed for this conversation (-32007).
    ChatInProgress,
    /// `summarize` reported generation failed via its `{error}` success-envelope
    /// branch (no JSON-RPC code).
    Summary,
    /// Any other service failure — an unmapped JSON-RPC error code, or a
    /// transport/decode failure. Carries the code (or `None`).
    Api,
}

impl AIChatErrorKind {
    /// The kind for a mapped JSON-RPC error `code`, else [`AIChatErrorKind::Api`].
    fn from_code(code: i64) -> Self {
        match code {
            -32001 => Self::ConversationNotFound,
            -32005 | -32006 => Self::RateLimit,
            -32007 => Self::ChatInProgress,
            -32009 => Self::Authentication,
            _ => Self::Api,
        }
    }

    /// A stable name for the kind (the "typed class name" the gate records).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Authentication => "AuthenticationError",
            Self::ConversationNotFound => "ConversationNotFoundError",
            Self::RateLimit => "RateLimitError",
            Self::ChatInProgress => "ChatInProgressError",
            Self::Summary => "SummaryError",
            Self::Api => "AIChatError",
        }
    }
}

/// A typed AI Chat service failure.
///
/// Callers match on [`AIChatError::kind`] to branch on the failure class, or read
/// [`AIChatError::code`] for the raw JSON-RPC code (`None` for a summary-failed or
/// transport failure). The base [`AIChatErrorKind::Api`] variant carries any
/// unmapped code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AIChatError {
    /// The typed failure class.
    pub kind: AIChatErrorKind,
    /// The JSON-RPC error code, or `None` when the failure rode the success
    /// envelope (summary-failed) or was a transport/decode error.
    pub code: Option<i64>,
    /// The server-provided (or client-synthesized) error message.
    pub message: String,
}

impl AIChatError {
    /// Build an error from a JSON-RPC error object: the `code` selects the kind
    /// ([`AIChatErrorKind::from_code`]), the message is carried through.
    fn from_jsonrpc(code: Option<i64>, message: String) -> Self {
        let kind = code.map_or(AIChatErrorKind::Api, AIChatErrorKind::from_code);
        Self {
            kind,
            code,
            message,
        }
    }

    /// The summary-failed error (the `summarize` `{error}` branch): no JSON-RPC
    /// code, [`AIChatErrorKind::Summary`] kind.
    fn summary(message: String) -> Self {
        Self {
            kind: AIChatErrorKind::Summary,
            code: None,
            message,
        }
    }

    /// A transport/decode failure (no JSON-RPC code): base [`AIChatErrorKind::Api`].
    fn transport(message: String) -> Self {
        Self {
            kind: AIChatErrorKind::Api,
            code: None,
            message,
        }
    }
}

impl std::fmt::Display for AIChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.code {
            Some(code) => write!(f, "[{code}] {}", self.message),
            None => write!(f, "[{}] {}", self.kind.name(), self.message),
        }
    }
}

impl std::error::Error for AIChatError {}

// ── Response models ──────────────────────────────────────────────────

/// Result of [`AIChatClient::create_conversation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationInfo {
    /// The conversation id (echoed back — the caller's own input).
    pub id: String,
    /// Lifecycle status the service reported (e.g. `"created"`).
    pub status: String,
    /// The opening assistant message, if the config produced one.
    pub initial_message: Option<String>,
}

/// Result of [`AIChatClient::chat`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatResponse {
    /// The assistant's reply text (the wire `response` field).
    pub text: String,
    /// The conversation id this reply belongs to.
    pub conversation_id: String,
    /// An optional structured event the turn emitted, else `None`.
    pub user_event: Option<Value>,
}

/// Result of [`AIChatClient::log`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatLog {
    /// Full message history (the wire `chat_log` field).
    pub messages: Vec<Value>,
    /// The call timeline (the wire `call_timeline` field).
    pub call_timeline: Vec<Value>,
}

// ── Per-turn options ─────────────────────────────────────────────────

/// Options for [`AIChatClient::create_conversation`]. `config_url` is required.
#[derive(Debug, Clone, Default)]
pub struct CreateOptions {
    /// Config URL locating the agent config (required).
    pub config_url: String,
    /// The opening user message to send with the create (wire `user_message`).
    pub user_message: Option<String>,
    /// Conversation inactivity timeout in seconds (wire `conversation_timeout`).
    pub timeout: Option<i64>,
    /// Reinitialize an existing conversation.
    pub reinit: bool,
    /// Arbitrary caller metadata (wire `user_meta_data`).
    pub user_metadata: Option<Value>,
}

impl CreateOptions {
    /// Start create options with the required `config_url`.
    #[must_use]
    pub fn new(config_url: impl Into<String>) -> Self {
        Self {
            config_url: config_url.into(),
            ..Self::default()
        }
    }

    /// Set the opening user message (wire `user_message`).
    #[must_use]
    pub fn user_message(mut self, msg: impl Into<String>) -> Self {
        self.user_message = Some(msg.into());
        self
    }

    /// Set the conversation inactivity timeout in seconds.
    #[must_use]
    pub const fn timeout(mut self, seconds: i64) -> Self {
        self.timeout = Some(seconds);
        self
    }

    /// Reinitialize an existing conversation.
    #[must_use]
    pub const fn reinit(mut self, reinit: bool) -> Self {
        self.reinit = reinit;
        self
    }

    /// Set arbitrary caller metadata (wire `user_meta_data`).
    #[must_use]
    pub fn user_metadata(mut self, meta: Value) -> Self {
        self.user_metadata = Some(meta);
        self
    }
}

/// Options for [`AIChatClient::chat`].
#[derive(Debug, Clone)]
pub struct ChatOptions {
    /// Message role (`"user"` or `"system"`). Default `"user"`.
    pub role: String,
    /// Config URL locating the agent config (auto-creates the conversation on
    /// chat when the conversation does not exist yet).
    pub config_url: Option<String>,
    /// Conversation inactivity timeout in seconds (wire `conversation_timeout`).
    pub timeout: Option<i64>,
    /// Reinitialize an existing conversation (applies to the auto-create).
    pub reinit: bool,
    /// Arbitrary caller metadata (wire `user_meta_data`).
    pub user_metadata: Option<Value>,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            role: "user".to_string(),
            config_url: None,
            timeout: None,
            reinit: false,
            user_metadata: None,
        }
    }
}

impl ChatOptions {
    /// Set the message role (`"user"` or `"system"`).
    #[must_use]
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    /// Set the config URL (auto-creates the conversation if absent).
    #[must_use]
    pub fn config_url(mut self, url: impl Into<String>) -> Self {
        self.config_url = Some(url.into());
        self
    }

    /// Set the conversation inactivity timeout in seconds.
    #[must_use]
    pub const fn timeout(mut self, seconds: i64) -> Self {
        self.timeout = Some(seconds);
        self
    }

    /// Reinitialize an existing conversation.
    #[must_use]
    pub const fn reinit(mut self, reinit: bool) -> Self {
        self.reinit = reinit;
        self
    }

    /// Set arbitrary caller metadata (wire `user_meta_data`).
    #[must_use]
    pub fn user_metadata(mut self, meta: Value) -> Self {
        self.user_metadata = Some(meta);
        self
    }
}

/// Sampling / prompt options for [`AIChatClient::summarize`]. All optional; only
/// set fields are put on the wire (mirroring the reference's None-drop).
#[derive(Debug, Clone, Default)]
pub struct SummarizeOptions {
    /// Custom prompt steering the summary (wire `summary_prompt`).
    pub summary_prompt: Option<String>,
    /// Sampling temperature.
    pub temperature: Option<f64>,
    /// Nucleus-sampling top-p.
    pub top_p: Option<f64>,
    /// Frequency penalty.
    pub frequency_penalty: Option<f64>,
    /// Presence penalty.
    pub presence_penalty: Option<f64>,
    /// Max tokens for the summary.
    pub max_tokens: Option<i64>,
}

impl SummarizeOptions {
    /// Set a custom prompt steering the summary (wire `summary_prompt`).
    #[must_use]
    pub fn summary_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.summary_prompt = Some(prompt.into());
        self
    }

    /// Set the sampling temperature.
    #[must_use]
    pub const fn temperature(mut self, value: f64) -> Self {
        self.temperature = Some(value);
        self
    }

    /// Set the nucleus-sampling top-p.
    #[must_use]
    pub const fn top_p(mut self, value: f64) -> Self {
        self.top_p = Some(value);
        self
    }

    /// Set the frequency penalty.
    #[must_use]
    pub const fn frequency_penalty(mut self, value: f64) -> Self {
        self.frequency_penalty = Some(value);
        self
    }

    /// Set the presence penalty.
    #[must_use]
    pub const fn presence_penalty(mut self, value: f64) -> Self {
        self.presence_penalty = Some(value);
        self
    }

    /// Set the max tokens for the summary.
    #[must_use]
    pub const fn max_tokens(mut self, value: i64) -> Self {
        self.max_tokens = Some(value);
        self
    }
}

// ── Builder ──────────────────────────────────────────────────────────

/// Builder for [`AIChatClient`] (constructor options mirroring the reference).
#[derive(Debug, Clone, Default)]
pub struct AIChatClientBuilder {
    project: Option<String>,
    token: Option<String>,
    space: Option<String>,
    url: Option<String>,
    read_idle_timeout_secs: Option<u64>,
}

impl AIChatClientBuilder {
    /// Project id (Basic-auth username). Falls back to `SIGNALWIRE_PROJECT_ID`.
    #[must_use]
    pub fn project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// API token (Basic-auth password). Falls back to `SIGNALWIRE_API_TOKEN`.
    #[must_use]
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Space name; builds `https://{space}.signalwire.com/api/ai/chat`. Falls
    /// back to `SIGNALWIRE_SPACE`.
    #[must_use]
    pub fn space(mut self, space: impl Into<String>) -> Self {
        self.space = Some(space.into());
        self
    }

    /// Fully-qualified endpoint URL, used verbatim (highest precedence).
    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Idle read timeout in seconds (byte-silence, NOT total turn length).
    /// Default 60; `0` disables it.
    #[must_use]
    pub const fn read_idle_timeout_secs(mut self, seconds: u64) -> Self {
        self.read_idle_timeout_secs = Some(seconds);
        self
    }

    /// Build the client, resolving creds/URL from args then the environment.
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] ([`AIChatErrorKind::Api`]) when no project resolves
    /// (arg or `SIGNALWIRE_PROJECT_ID`), when no URL can be resolved (no `url`,
    /// no `space`/`SIGNALWIRE_SPACE`), or when the HTTP client fails to build.
    pub fn build(self) -> Result<AIChatClient, AIChatError> {
        let project = self
            .project
            .or_else(|| std::env::var("SIGNALWIRE_PROJECT_ID").ok())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AIChatError::transport(
                    "project is required. Provide it via .project() or set the \
                     SIGNALWIRE_PROJECT_ID environment variable."
                        .to_string(),
                )
            })?;
        let token = self
            .token
            .or_else(|| std::env::var("SIGNALWIRE_API_TOKEN").ok())
            .unwrap_or_default();
        let space = self
            .space
            .or_else(|| std::env::var("SIGNALWIRE_SPACE").ok());

        let url = AIChatClient::resolve_url(self.url.as_deref(), space.as_deref())?;

        let read_idle = self
            .read_idle_timeout_secs
            .unwrap_or(DEFAULT_READ_IDLE_TIMEOUT_SECS);

        // total=None (no wall-clock cap on a live turn); connect bounded;
        // read-idle detects a dead connection between byte reads.
        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
            .user_agent(user_agent());
        if read_idle > 0 {
            builder = builder.read_timeout(Duration::from_secs(read_idle));
        }
        let http = builder
            .build()
            .map_err(|e| AIChatError::transport(format!("HTTP client build failed: {e}")))?;

        let auth = base64::engine::general_purpose::STANDARD.encode(format!("{project}:{token}"));
        Ok(AIChatClient {
            url,
            auth_header: format!("Basic {auth}"),
            project,
            http,
            request_counter: AtomicU64::new(0),
        })
    }
}

// ── Client ───────────────────────────────────────────────────────────

/// The `signalwire-agents` User-Agent this client advertises (matches the REST
/// client's convention: `signalwire-agents-rust-ai-chat/<version>`).
fn user_agent() -> String {
    concat!("signalwire-agents-rust-ai-chat/", env!("CARGO_PKG_VERSION")).to_string()
}

/// Async client for the SignalWire AI Chat service.
///
/// Construct via [`AIChatClient::builder`]. Every method POSTs one JSON-RPC call
/// and returns `Result<T, AIChatError>`.
#[derive(Debug)]
pub struct AIChatClient {
    /// Fully-qualified endpoint URL requests are POSTed to.
    url: String,
    /// The pre-computed `Basic <base64>` Authorization header value.
    auth_header: String,
    /// The project id (Basic-auth username); retained for observability/tests.
    project: String,
    /// The shared reqwest client (connection pooling, timeouts).
    http: reqwest::Client,
    /// Monotonic JSON-RPC request id counter.
    request_counter: AtomicU64,
}

impl AIChatClient {
    /// Start building a client. `project` is required (arg or
    /// `SIGNALWIRE_PROJECT_ID`); either `url` or `space`/`SIGNALWIRE_SPACE` must
    /// resolve a target.
    #[must_use]
    pub fn builder() -> AIChatClientBuilder {
        AIChatClientBuilder::default()
    }

    /// The endpoint URL this client POSTs to.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The project id (Basic-auth username).
    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }

    /// Resolve the endpoint URL: explicit `url` wins; else build from `space`.
    fn resolve_url(url: Option<&str>, space: Option<&str>) -> Result<String, AIChatError> {
        if let Some(url) = url.filter(|u| !u.is_empty()) {
            return Ok(url.to_string());
        }
        if let Some(space) = space.filter(|s| !s.is_empty()) {
            return Ok(format!("https://{space}.signalwire.com{DEFAULT_PATH}"));
        }
        Err(AIChatError::transport(
            "No service URL: provide .url() or .space() / SIGNALWIRE_SPACE.".to_string(),
        ))
    }

    // ── Wire ─────────────────────────────────────────────────────────

    /// POST one JSON-RPC call and return its decoded `result` object.
    ///
    /// Success/failure is decided by the JSON-RPC BODY, not the HTTP status: the
    /// service's keepalive heartbeat commits `200` before the turn's outcome is
    /// known, so a slow error can arrive as `200 + {"error": …}`. Never gate on
    /// the HTTP status here (mirrors the python reference).
    async fn request(
        &self,
        method: &str,
        params: Map<String, Value>,
    ) -> Result<Map<String, Value>, AIChatError> {
        let id = self.request_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": Value::Object(params),
            "id": format!("req-{id}"),
        });

        let resp = self
            .http
            .post(&self.url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AIChatError::transport(format!("request failed: {e}")))?;

        let status = resp.status();
        // Buffer the whole body then parse. Leading keepalive whitespace is valid
        // JSON, so a plain parse handles it — no need to strip.
        let text = resp
            .text()
            .await
            .map_err(|e| AIChatError::transport(format!("reading body failed: {e}")))?;
        let body: Value = serde_json::from_str(&text).map_err(|_| {
            AIChatError::from_jsonrpc(
                Some(i64::from(status.as_u16())),
                format!("non-JSON response (HTTP {})", status.as_u16()),
            )
        })?;

        if let Some(error) = body.get("error").filter(|e| !e.is_null()) {
            let code = error.get("code").and_then(Value::as_i64);
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            return Err(AIChatError::from_jsonrpc(code, message));
        }

        match body.get("result") {
            Some(Value::Object(map)) => Ok(map.clone()),
            _ => Ok(Map::new()),
        }
    }

    // ── API methods ──────────────────────────────────────────────────

    /// Create a conversation (or, with `reinit`, reinitialize an existing one)
    /// and optionally send its opening user message.
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] on a JSON-RPC error or transport failure.
    pub async fn create_conversation(
        &self,
        conversation_id: &str,
        options: CreateOptions,
    ) -> Result<ConversationInfo, AIChatError> {
        let mut params = Map::new();
        params.insert("id".to_string(), json!(conversation_id));
        params.insert("config_url".to_string(), json!(options.config_url));
        if let Some(msg) = options.user_message.filter(|m| !m.is_empty()) {
            params.insert("user_message".to_string(), json!(msg));
        }
        if let Some(t) = options.timeout.filter(|t| *t != 0) {
            params.insert("conversation_timeout".to_string(), json!(t));
        }
        if let Some(meta) = options.user_metadata {
            params.insert("user_meta_data".to_string(), meta);
        }
        if options.reinit {
            params.insert("reinit".to_string(), json!(true));
        }

        let result = self.request("create_conversation", params).await?;
        Ok(ConversationInfo {
            id: conversation_id.to_string(),
            status: result
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("created")
                .to_string(),
            initial_message: result
                .get("initial_message")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    /// Send a message and await a full LLM round trip.
    ///
    /// Passing `config_url` (via [`ChatOptions::config_url`]) auto-creates the
    /// conversation if it doesn't exist yet; `timeout` and `reinit` apply to that
    /// auto-create. Expect seconds — the turn awaits the model.
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] on a JSON-RPC error or transport failure.
    pub async fn chat(
        &self,
        conversation_id: &str,
        message: &str,
        options: ChatOptions,
    ) -> Result<ChatResponse, AIChatError> {
        let mut params = Map::new();
        params.insert("id".to_string(), json!(conversation_id));
        params.insert("message".to_string(), json!(message));
        params.insert("role".to_string(), json!(options.role));
        if let Some(url) = options.config_url.filter(|u| !u.is_empty()) {
            params.insert("config_url".to_string(), json!(url));
        }
        if let Some(meta) = options.user_metadata {
            params.insert("user_meta_data".to_string(), meta);
        }
        if let Some(t) = options.timeout.filter(|t| *t != 0) {
            params.insert("conversation_timeout".to_string(), json!(t));
        }
        if options.reinit {
            params.insert("reinit".to_string(), json!(true));
        }

        let result = self.request("chat", params).await?;
        Ok(ChatResponse {
            text: result
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            conversation_id: conversation_id.to_string(),
            user_event: result.get("user_event").filter(|v| v.is_object()).cloned(),
        })
    }

    /// End a conversation (triggers server-side post-processing / archival).
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] on a JSON-RPC error or transport failure.
    pub async fn end(&self, conversation_id: &str) -> Result<bool, AIChatError> {
        let mut params = Map::new();
        params.insert("id".to_string(), json!(conversation_id));
        let result = self.request("end_conversation", params).await?;
        Ok(result.get("status").and_then(Value::as_str) == Some("ended"))
    }

    /// Permanently delete a conversation and its data. Idempotent.
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] on a JSON-RPC error or transport failure.
    pub async fn delete(&self, conversation_id: &str) -> Result<bool, AIChatError> {
        let mut params = Map::new();
        params.insert("id".to_string(), json!(conversation_id));
        let result = self.request("delete", params).await?;
        Ok(result.get("status").and_then(Value::as_str) == Some("deleted"))
    }

    /// Return the full message history plus the call timeline.
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] on a JSON-RPC error or transport failure.
    pub async fn log(&self, conversation_id: &str) -> Result<ChatLog, AIChatError> {
        let mut params = Map::new();
        params.insert("id".to_string(), json!(conversation_id));
        let result = self.request("chat_log", params).await?;
        Ok(ChatLog {
            messages: result
                .get("chat_log")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            call_timeline: result
                .get("call_timeline")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        })
    }

    /// Return an AI summary of the conversation (rate limited server-side).
    ///
    /// The service returns EXACTLY ONE of `{summary}` or `{error}` — BOTH on the
    /// success envelope — so a failed generation surfaces as
    /// `Err(AIChatError { kind: Summary, .. })`, never as an empty string.
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] ([`AIChatErrorKind::Summary`]) when the service
    /// reports summary generation failed, or the base error on a JSON-RPC error /
    /// transport failure.
    pub async fn summarize(
        &self,
        conversation_id: &str,
        options: SummarizeOptions,
    ) -> Result<String, AIChatError> {
        let mut params = Map::new();
        params.insert("id".to_string(), json!(conversation_id));
        if let Some(p) = options.summary_prompt.filter(|p| !p.is_empty()) {
            params.insert("summary_prompt".to_string(), json!(p));
        }
        // Sampling params: only set fields ride the wire (reference None-drop).
        let mut sampling: BTreeMap<&str, Value> = BTreeMap::new();
        if let Some(v) = options.temperature {
            sampling.insert("temperature", json!(v));
        }
        if let Some(v) = options.top_p {
            sampling.insert("top_p", json!(v));
        }
        if let Some(v) = options.frequency_penalty {
            sampling.insert("frequency_penalty", json!(v));
        }
        if let Some(v) = options.presence_penalty {
            sampling.insert("presence_penalty", json!(v));
        }
        if let Some(v) = options.max_tokens {
            sampling.insert("max_tokens", json!(v));
        }
        for (k, v) in sampling {
            params.insert(k.to_string(), v);
        }

        let result = self.request("summarize", params).await?;
        if result.contains_key("error") && !result.contains_key("summary") {
            let message = match result.get("error") {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            return Err(AIChatError::summary(message));
        }
        Ok(match result.get("summary") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        })
    }
}

#[cfg(test)]
mod tests;
