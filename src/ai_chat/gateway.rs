//! A browser-facing gateway for the SignalWire AI Chat service.
//!
//! A chat widget running in a page cannot hold a SignalWire API token: the token
//! carries the whole project, so putting it in JavaScript hands every visitor
//! the ability to run up billed turns. So the widget talks to a gateway mounted
//! in your own app, which holds the credential server-side and forwards on the
//! widget's behalf:
//!
//! ```text
//! browser ──(publishable key)──▶ your app ──(project:token)──▶ chat service
//! ```
//!
//! The browser learns the gateway's URL and a publishable key — not the
//! project, space or token, and not which agent config runs: the gateway
//! injects `config_url` itself, so a key only ever reaches the one script it was
//! issued for. Mount it beside an agent:
//!
//! ```no_run
//! use signalwire::agent::{AgentBase, AgentOptions};
//! use signalwire::ai_chat::{ChatGateway, ChatGatewayOptions};
//!
//! let gateway = ChatGateway::new(
//!     ChatGatewayOptions::new("https://my-agent.example.com/swml")
//!         .key("pk_live_...")
//!         .allowed_origins(vec!["https://shop.example.com".to_string()]),
//! )?;
//! let mut agent = AgentBase::new(AgentOptions::new("support"));
//! agent.mount(gateway.router(), Some("/chat"), None);
//! # Ok::<(), signalwire::ai_chat::AIChatError>(())
//! ```
//!
//! ## What a stolen key gets you
//!
//! Nothing to read beyond the visible transcript of a conversation whose handle
//! it holds: a conversation handle is signed by the gateway, so ids cannot be
//! guessed or enumerated. What it gets you is the ability to *talk*, which
//! costs the project money — so `max_new_conversations` and `max_turns` bound
//! the bill from both directions. The origin allowlist stops a key pasted into
//! someone else's page (a browser sends their origin and it is refused); it
//! does not stop curl. Treat it as leak containment, not access control.
//!
//! ## What the browser may volunteer
//!
//! Exactly one field is forwarded rather than overwritten: `user_meta_data`,
//! the page context a widget collects about itself. It reaches the agent's
//! config request as `params.user_meta_data` when a conversation is CREATED (the
//! `start` call, or whichever `chat` auto-creates). It is browser-authored: a
//! visitor's claim about themselves, never authority.
//!
//! ## Size limits
//!
//! Every field is sized by whoever holds the key, so each is bounded and
//! answered with `413` past its limit: the request body
//! ([`MAX_REQUEST_BODY_BYTES`]), a chat message ([`MAX_MESSAGE_BYTES`] of UTF-8)
//! and `user_meta_data` ([`MAX_USER_METADATA_BYTES`] serialized).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Map, Value, json};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use super::client::{AIChatClient, AIChatError, CreateOptions};
use crate::agent::agent_base::{MountHandler, MountResponse};

/// A handle outlives a page refresh but not a session left open overnight.
pub const DEFAULT_HANDLE_TTL: i64 = 24 * 60 * 60;

/// The chat service's own default conversation timeout, reported to the
/// browser when the gateway was given none (the service owns the behaviour).
pub const SERVICE_DEFAULT_CONVERSATION_TIMEOUT: i64 = 3600;

/// New conversations per window, per gateway.
pub const DEFAULT_MAX_NEW_CONVERSATIONS: i64 = 60;

/// Turns per conversation, ever.
pub const DEFAULT_MAX_TURNS: i64 = 200;

/// The window `max_new_conversations` counts over, in seconds.
pub const DEFAULT_WINDOW_SECONDS: i64 = 60;

/// The browser-volunteered `user_meta_data` bound, serialized.
pub const MAX_USER_METADATA_BYTES: usize = 8 * 1024;

/// One typed message, UTF-8 encoded: a chat turn here, and the text a
/// [`HandoffRouter`](super::handoff::HandoffRouter)'s `/say` injects.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024;

/// A whole request body, checked before it is parsed.
pub const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;

/// Hosts that never need listing, so local development works unconfigured.
const LOCAL_HOSTS: &[&str] = &["localhost", "127.0.0.1", "::1", "[::1]"];

/// The methods a browser may call.
const ALLOWED_METHODS: &[&str] = &["start", "chat", "log", "end"];

/// Roles a browser may see in a transcript.
const VISIBLE_ROLES: &[&str] = &["user", "assistant"];

type HmacSha256 = Hmac<Sha256>;

/// What [`ChatGateway::prepare`] builds: `(service method, params,
/// minted handle)` — the handle only on the call that created the conversation.
pub type PreparedCall = (String, Map<String, Value>, Option<String>);

/// A request the gateway refused, with the status the browser should see.
///
/// Deliberately coarse: the browser is told *that* it was refused and, at most,
/// which of a handful of buckets it fell into — anything finer would let a
/// caller map out the caps and the allowlist by probing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayRejection {
    /// HTTP status to send back (401 bad key, 403 origin/handle, 400 disallowed
    /// method, 413 over a size limit, 429 a cap was hit).
    pub status: u16,
    /// Short, fixed explanation naming only the bucket.
    pub reason: String,
}

impl GatewayRejection {
    /// Refuse a browser request with `status` and a fixed `reason`.
    #[must_use]
    pub fn new(status: u16, reason: &str) -> Self {
        GatewayRejection {
            status,
            reason: reason.to_string(),
        }
    }
}

impl std::fmt::Display for GatewayRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status, self.reason)
    }
}

impl std::error::Error for GatewayRejection {}

/// Construction options for [`ChatGateway`] (the reference's keyword
/// arguments). `config_url` is required; every other field has the
/// reference's default.
#[derive(Clone, Default)]
pub struct ChatGatewayOptions {
    /// The agent this key may talk to. Injected on every call — never accepted
    /// from the request.
    pub config_url: String,
    /// The publishable key the widget carries; from
    /// `SIGNALWIRE_CHAT_GATEWAY_KEY`, else generated.
    pub key: Option<String>,
    /// Origins permitted to use the key (localhost is always allowed).
    pub allowed_origins: Vec<String>,
    /// The AI Chat client to forward with; built from the environment when
    /// omitted.
    pub client: Option<Arc<AIChatClient>>,
    /// HMAC key for signing handles; from `SIGNALWIRE_CHAT_GATEWAY_SECRET`, else
    /// random per process (set it when running more than one replica).
    pub secret: Option<Vec<u8>>,
    /// Seconds a handle stays valid (default [`DEFAULT_HANDLE_TTL`]).
    pub handle_ttl: Option<i64>,
    /// Idle seconds before the service ends a conversation, sent on every
    /// create; `None` leaves the service default.
    pub conversation_timeout: Option<i64>,
    /// New conversations per window (default [`DEFAULT_MAX_NEW_CONVERSATIONS`]).
    pub max_new_conversations: Option<i64>,
    /// Turns per conversation (default [`DEFAULT_MAX_TURNS`]).
    pub max_turns: Option<i64>,
    /// The `max_new_conversations` window (default [`DEFAULT_WINDOW_SECONDS`]).
    pub window_seconds: Option<i64>,
}

impl ChatGatewayOptions {
    /// Options for a gateway scoped to `config_url`.
    #[must_use]
    pub fn new(config_url: impl Into<String>) -> Self {
        ChatGatewayOptions {
            config_url: config_url.into(),
            ..Self::default()
        }
    }

    /// Set the publishable key.
    #[must_use]
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set the allowed origins.
    #[must_use]
    pub fn allowed_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    /// Forward with this client.
    #[must_use]
    pub fn client(mut self, client: Arc<AIChatClient>) -> Self {
        self.client = Some(client);
        self
    }

    /// Set the handle-signing secret.
    #[must_use]
    pub fn secret(mut self, secret: impl Into<Vec<u8>>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Set the handle lifetime in seconds.
    #[must_use]
    pub fn handle_ttl(mut self, seconds: i64) -> Self {
        self.handle_ttl = Some(seconds);
        self
    }

    /// Set the conversation idle timeout in seconds.
    #[must_use]
    pub fn conversation_timeout(mut self, seconds: i64) -> Self {
        self.conversation_timeout = Some(seconds);
        self
    }

    /// Set the new-conversation cap per window.
    #[must_use]
    pub fn max_new_conversations(mut self, n: i64) -> Self {
        self.max_new_conversations = Some(n);
        self
    }

    /// Set the per-conversation turn cap.
    #[must_use]
    pub fn max_turns(mut self, n: i64) -> Self {
        self.max_turns = Some(n);
        self
    }

    /// Set the cap window in seconds.
    #[must_use]
    pub fn window_seconds(mut self, seconds: i64) -> Self {
        self.window_seconds = Some(seconds);
        self
    }
}

/// The process-wide runtime the gateway (and handoff router) drive the async
/// AI Chat client on from the synchronous request handlers.
pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("signalwire-ai-chat")
            .enable_all()
            .build()
            .expect("AI Chat gateway runtime")
    })
}

/// Server-side proxy that lets a browser chat without holding a token.
///
/// Counters live in this process. Behind several replicas each holds its own,
/// so the effective cap multiplies by replica count — set them with that in
/// mind, or put a shared limiter in front. Cloning shares the counters.
#[derive(Clone)]
pub struct ChatGateway {
    /// The agent config every call is scoped to.
    pub config_url: String,
    /// The publishable key a browser must present.
    pub key: String,
    /// Origins permitted besides localhost (trailing `/` stripped).
    pub allowed_origins: HashSet<String>,
    /// Seconds a handle stays valid.
    pub handle_ttl: i64,
    /// Idle seconds sent on every create, or `None` for the service default.
    pub conversation_timeout: Option<i64>,
    /// New conversations per window.
    pub max_new_conversations: i64,
    /// Turns per conversation.
    pub max_turns: i64,
    /// The new-conversation window, in seconds.
    pub window_seconds: i64,
    client: Arc<AIChatClient>,
    secret: Arc<Vec<u8>>,
    mints: Arc<Mutex<Vec<Instant>>>,
    turns: Arc<Mutex<HashMap<String, (i64, Instant)>>>,
}

impl std::fmt::Debug for ChatGateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatGateway")
            .field("config_url", &self.config_url)
            .field("allowed_origins", &self.allowed_origins)
            .finish_non_exhaustive()
    }
}

/// Random bytes, URL-safe base64 without padding (`secrets.token_urlsafe`).
fn token_urlsafe(n: usize) -> String {
    use rand::RngExt;
    let mut bytes = vec![0u8; n];
    rand::rng().fill(&mut bytes[..]);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Seconds since the Unix epoch.
fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Length of `text` in UTF-8 bytes.
pub(crate) fn utf8_len(text: &str) -> usize {
    text.len()
}

/// Unpadded URL-safe base64 decode, tolerating stripped or present padding.
fn unb64(text: &str) -> Result<Vec<u8>, base64::DecodeError> {
    URL_SAFE_NO_PAD.decode(text.trim_end_matches('='))
}

/// Parse a JSON request body, refusing one over `limit` bytes (413).
pub(crate) fn read_json_body(
    headers: &HashMap<String, String>,
    body: &str,
    limit: usize,
) -> Result<Option<Value>, GatewayRejection> {
    let declared = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok());
    if declared.is_some_and(|n| n > limit) || body.len() > limit {
        return Err(GatewayRejection::new(413, "request too large"));
    }
    Ok(serde_json::from_str(body).ok())
}

/// A case-insensitive request-header read.
pub(crate) fn header<'a>(headers: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// A JSON response body as a mount reader.
pub(crate) fn json_body(value: &Value) -> Box<dyn std::io::Read + Send> {
    Box::new(std::io::Cursor::new(value.to_string().into_bytes()))
}

/// A `{"error": reason}` mount response.
pub(crate) fn rejected(
    rej: &GatewayRejection,
    mut headers: HashMap<String, String>,
) -> MountResponse {
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    (
        rej.status,
        headers,
        json_body(&json!({"error": rej.reason})),
    )
}

/// Streams a reqwest body to a synchronous reader, chunk by chunk.
struct ChannelReader {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    pending: std::io::Cursor<Vec<u8>>,
}

impl std::io::Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let n = self.pending.read(buf)?;
            if n > 0 {
                return Ok(n);
            }
            match self.rx.recv() {
                Ok(chunk) => self.pending = std::io::Cursor::new(chunk),
                Err(_) => return Ok(0),
            }
        }
    }
}

impl ChatGateway {
    /// Build a gateway that fronts one agent for browser traffic.
    ///
    /// # Errors
    ///
    /// Returns [`AIChatError`] when `config_url` is empty (it is what a key is
    /// scoped to) or, with no `client` given, when the AI Chat client cannot be
    /// built from the environment.
    pub fn new(options: ChatGatewayOptions) -> Result<Self, AIChatError> {
        if options.config_url.is_empty() {
            return Err(AIChatError::transport(
                "config_url is required — it is what a key is scoped to.".to_string(),
            ));
        }
        let client = match options.client {
            Some(c) => c,
            None => Arc::new(AIChatClient::builder().build()?),
        };
        let key = options
            .key
            .filter(|k| !k.is_empty())
            .or_else(|| {
                std::env::var("SIGNALWIRE_CHAT_GATEWAY_KEY")
                    .ok()
                    .filter(|k| !k.is_empty())
            })
            .unwrap_or_else(|| format!("pk_{}", token_urlsafe(24)));
        let secret = options
            .secret
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::env::var("SIGNALWIRE_CHAT_GATEWAY_SECRET")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(String::into_bytes)
            })
            .unwrap_or_else(|| {
                use rand::RngExt;
                let mut b = vec![0u8; 32];
                rand::rng().fill(&mut b[..]);
                b
            });
        Ok(ChatGateway {
            config_url: options.config_url,
            key,
            allowed_origins: options
                .allowed_origins
                .iter()
                .map(|o| o.trim_end_matches('/').to_string())
                .collect(),
            handle_ttl: options.handle_ttl.unwrap_or(DEFAULT_HANDLE_TTL),
            conversation_timeout: options.conversation_timeout,
            max_new_conversations: options
                .max_new_conversations
                .unwrap_or(DEFAULT_MAX_NEW_CONVERSATIONS),
            max_turns: options.max_turns.unwrap_or(DEFAULT_MAX_TURNS),
            window_seconds: options.window_seconds.unwrap_or(DEFAULT_WINDOW_SECONDS),
            client,
            secret: Arc::new(secret),
            mints: Arc::new(Mutex::new(Vec::new())),
            turns: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Epoch SECONDS of the newest message, or `None` if nothing is dated.
    ///
    /// Bootstraps a browser's idle clock across a reload. The service stamps
    /// messages in microseconds; this converts. Every role counts: the service's
    /// idle clock runs off any write.
    #[must_use]
    pub fn last_activity(messages: Option<&[Value]>) -> Option<f64> {
        #[allow(clippy::cast_precision_loss)]
        messages
            .unwrap_or_default()
            .iter()
            .filter_map(|m| m.get("timestamp").and_then(Value::as_i64))
            .filter(|ts| *ts > 0)
            .max()
            .map(|ts| ts as f64 / 1_000_000.0)
    }

    /// Idle seconds a conversation actually gets: the configured timeout, else
    /// the service default — never null, so a widget can always warn.
    #[must_use]
    pub fn effective_timeout(&self) -> i64 {
        self.conversation_timeout
            .filter(|t| *t != 0)
            .unwrap_or(SERVICE_DEFAULT_CONVERSATION_TIMEOUT)
    }

    /// Release the upstream client. The Rust client pools connections and is
    /// freed when its last handle drops, so this is a no-op kept for the
    /// lifecycle contract.
    pub fn close(&self) {
        self.client.close();
    }

    // ── Handles ──────────────────────────────────────────────────────

    /// Issue a signed handle for a conversation (a fresh id when `None`).
    ///
    /// The browser never names a conversation: signing means a caller can only
    /// present handles this gateway issued.
    ///
    /// # Panics
    ///
    /// Never in practice: HMAC-SHA256 accepts a key of any length.
    #[must_use]
    pub fn mint_handle(&self, conversation_id: Option<&str>) -> String {
        let conversation_id = conversation_id
            .filter(|c| !c.is_empty())
            .map_or_else(|| format!("chat-{}", token_urlsafe(18)), str::to_string);
        let expires = now_epoch() + self.handle_ttl;
        let payload = format!("{conversation_id}:{expires}");
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key");
        mac.update(payload.as_bytes());
        let sig = mac.finalize().into_bytes();
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(sig)
        )
    }

    /// Return the conversation id inside a handle: signature first, expiry
    /// second, both before the id is trusted.
    ///
    /// # Errors
    ///
    /// [`GatewayRejection`] 400 `malformed handle`, 403 `invalid handle` (bad
    /// signature) or 403 `expired handle`.
    ///
    /// # Panics
    ///
    /// Never in practice: HMAC-SHA256 accepts a key of any length.
    pub fn read_handle(&self, handle: &str) -> Result<String, GatewayRejection> {
        let malformed = || GatewayRejection::new(400, "malformed handle");
        let (raw, sig) = handle.split_once('.').ok_or_else(malformed)?;
        let payload = unb64(raw).map_err(|_| malformed())?;
        let given = unb64(sig).map_err(|_| malformed())?;
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key");
        mac.update(&payload);
        let expected = mac.finalize().into_bytes();
        if !bool::from(given.as_slice().ct_eq(expected.as_slice())) {
            return Err(GatewayRejection::new(403, "invalid handle"));
        }
        let text = String::from_utf8(payload).map_err(|_| malformed())?;
        let (conversation_id, expires) = text.rsplit_once(':').ok_or_else(malformed)?;
        let expires: i64 = expires.parse().map_err(|_| malformed())?;
        if now_epoch() > expires {
            return Err(GatewayRejection::new(403, "expired handle"));
        }
        Ok(conversation_id.to_string())
    }

    // ── Guards ───────────────────────────────────────────────────────

    /// Localhost always; anything else must be listed. A missing `Origin` is
    /// allowed (a non-browser caller — refusing it would stop no attacker).
    ///
    /// # Errors
    ///
    /// [`GatewayRejection`] 403 `origin not allowed`.
    pub fn check_origin(&self, origin: Option<&str>) -> Result<(), GatewayRejection> {
        let Some(origin) = origin else {
            return Ok(());
        };
        let host = url::Url::parse(origin)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_default();
        if LOCAL_HOSTS.contains(&host.as_str()) || host.ends_with(".localhost") {
            return Ok(());
        }
        if self.allowed_origins.contains(origin.trim_end_matches('/')) {
            return Ok(());
        }
        Err(GatewayRejection::new(403, "origin not allowed"))
    }

    /// Verify the publishable key, in constant time.
    ///
    /// # Errors
    ///
    /// [`GatewayRejection`] 401 `bad key` when missing or wrong.
    pub fn check_key(&self, presented: Option<&str>) -> Result<(), GatewayRejection> {
        match presented {
            Some(p) if !p.is_empty() && bool::from(p.as_bytes().ct_eq(self.key.as_bytes())) => {
                Ok(())
            }
            _ => Err(GatewayRejection::new(401, "bad key")),
        }
    }

    /// The transcript a browser may redraw: user and assistant turns with text,
    /// reduced to role, content and (epoch-seconds) timestamp — never the system
    /// prompt, tool calls or ids.
    #[must_use]
    pub fn visible_messages(messages: Option<&[Value]>) -> Vec<Value> {
        let mut out = Vec::new();
        for msg in messages.unwrap_or_default() {
            let Some(role) = msg.get("role").and_then(Value::as_str) else {
                continue;
            };
            let Some(content) = msg.get("content").and_then(Value::as_str) else {
                continue;
            };
            if !VISIBLE_ROLES.contains(&role) || content.trim().is_empty() {
                continue;
            }
            let mut entry = Map::new();
            entry.insert("role".to_string(), json!(role));
            entry.insert("content".to_string(), json!(content));
            if let Some(ts) = msg
                .get("timestamp")
                .and_then(Value::as_i64)
                .filter(|t| *t > 0)
            {
                #[allow(clippy::cast_precision_loss)]
                entry.insert("timestamp".to_string(), json!(ts as f64 / 1_000_000.0));
            }
            out.push(Value::Object(entry));
        }
        out
    }

    fn charge_mint(&self) -> Result<(), GatewayRejection> {
        let now = Instant::now();
        let window =
            std::time::Duration::from_secs(u64::try_from(self.window_seconds).unwrap_or(0));
        let mut mints = self
            .mints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        mints.retain(|t| now.duration_since(*t) < window);
        if i64::try_from(mints.len()).unwrap_or(i64::MAX) >= self.max_new_conversations {
            return Err(GatewayRejection::new(429, "too many new conversations"));
        }
        mints.push(now);
        Ok(())
    }

    fn charge_turn(&self, conversation_id: &str) -> Result<(), GatewayRejection> {
        let now = Instant::now();
        let ttl = std::time::Duration::from_secs(u64::try_from(self.handle_ttl).unwrap_or(0));
        let mut turns = self
            .turns
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // A handle cannot outlive its TTL, so older entries can never be charged again.
        turns.retain(|_, (_, at)| now.duration_since(*at) < ttl);
        let count = turns.get(conversation_id).map_or(0, |(c, _)| *c);
        if count >= self.max_turns {
            return Err(GatewayRejection::new(
                429,
                "conversation turn limit reached",
            ));
        }
        turns.insert(conversation_id.to_string(), (count + 1, now));
        Ok(())
    }

    // ── The proxied call ─────────────────────────────────────────────

    /// Validate the page context a browser volunteered (`user_meta_data`), or
    /// `None` when absent / null / empty.
    ///
    /// # Errors
    ///
    /// [`GatewayRejection`] 400 when it is not an object, 413 when it exceeds
    /// [`MAX_USER_METADATA_BYTES`] serialized.
    pub fn read_user_metadata(
        &self,
        body: &Map<String, Value>,
    ) -> Result<Option<Map<String, Value>>, GatewayRejection> {
        match body.get("user_meta_data") {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Object(raw)) if raw.is_empty() => Ok(None),
            Some(Value::Object(raw)) => {
                if Value::Object(raw.clone()).to_string().len() > MAX_USER_METADATA_BYTES {
                    return Err(GatewayRejection::new(413, "user_meta_data too large"));
                }
                Ok(Some(raw.clone()))
            }
            Some(_) => Err(GatewayRejection::new(
                400,
                "user_meta_data must be an object",
            )),
        }
    }

    /// Validate a browser request and build the upstream JSON-RPC call:
    /// `(method, params, minted_handle)` — the handle is set only on the call
    /// that created the conversation.
    ///
    /// Everything the browser could use to widen its own access is rejected or
    /// overwritten: the method must be allowed, the conversation comes from a
    /// signed handle, and `config_url` is the gateway's. A chat message over
    /// [`MAX_MESSAGE_BYTES`] is refused before a conversation is minted or a
    /// turn charged.
    ///
    /// # Errors
    ///
    /// [`GatewayRejection`] for a bad key/origin/handle, a disallowed method,
    /// an over-size message or metadata, or a hit cap.
    pub fn prepare(
        &self,
        body: &Map<String, Value>,
        origin: Option<&str>,
        key: Option<&str>,
    ) -> Result<PreparedCall, GatewayRejection> {
        self.check_key(key)?;
        self.check_origin(origin)?;

        let method = match body.get("method") {
            None => "chat",
            Some(Value::String(m)) => m.as_str(),
            Some(_) => "",
        };
        if !ALLOWED_METHODS.contains(&method) {
            return Err(GatewayRejection::new(400, "method not allowed"));
        }
        // Read before minting so a malformed bag costs the caller nothing.
        let user_metadata = self.read_user_metadata(body)?;
        let message = body.get("message").and_then(Value::as_str);
        if method == "chat" && message.is_some_and(|m| utf8_len(m) > MAX_MESSAGE_BYTES) {
            return Err(GatewayRejection::new(413, "message too large"));
        }

        let handle = body
            .get("handle")
            .and_then(Value::as_str)
            .filter(|h| !h.is_empty());
        let mut minted = None;
        let conversation_id = if let Some(handle) = handle {
            self.read_handle(handle)?
        } else if method == "end" || method == "log" {
            return Err(GatewayRejection::new(
                400,
                &format!("{method} requires a handle"),
            ));
        } else {
            self.charge_mint()?;
            let h = self.mint_handle(None);
            let id = self.read_handle(&h)?;
            minted = Some(h);
            id
        };

        let mut params = Map::new();
        params.insert("id".to_string(), json!(conversation_id));
        match method {
            "end" => return Ok(("end_conversation".to_string(), params, None)),
            // Scoped to the conversation named INSIDE the signed handle.
            "log" => return Ok(("chat_log".to_string(), params, None)),
            "start" => {
                // Opens the conversation with no user message: the agent speaks first.
                params.insert("config_url".to_string(), json!(self.config_url));
                if let Some(t) = self.conversation_timeout.filter(|t| *t != 0) {
                    params.insert("conversation_timeout".to_string(), json!(t));
                }
                if let Some(meta) = user_metadata {
                    params.insert("user_meta_data".to_string(), Value::Object(meta));
                }
                return Ok(("create_conversation".to_string(), params, minted));
            }
            _ => {}
        }

        let Some(message) = message.filter(|m| !m.trim().is_empty()) else {
            return Err(GatewayRejection::new(400, "message is required"));
        };
        self.charge_turn(&conversation_id)?;
        // config_url on every chat so the service auto-creates on the first one.
        params.insert("message".to_string(), json!(message));
        params.insert("config_url".to_string(), json!(self.config_url));
        if let Some(t) = self.conversation_timeout.filter(|t| *t != 0) {
            params.insert("conversation_timeout".to_string(), json!(t));
        }
        // On every chat: any chat may be the one that creates the conversation.
        if let Some(meta) = user_metadata {
            params.insert("user_meta_data".to_string(), Value::Object(meta));
        }
        Ok(("chat".to_string(), params, minted))
    }

    /// CORS headers for an allowed origin, none otherwise.
    fn cors(&self, origin: Option<&str>) -> HashMap<String, String> {
        let mut h = HashMap::new();
        if let Some(origin) = origin
            && self.check_origin(Some(origin)).is_ok()
        {
            h.insert(
                "Access-Control-Allow-Origin".to_string(),
                origin.to_string(),
            );
            h.insert(
                "Access-Control-Expose-Headers".to_string(),
                "X-Chat-Handle".to_string(),
            );
            h.insert("Vary".to_string(), "Origin".to_string());
        }
        h
    }

    /// The gateway as a mountable router (see
    /// [`AgentBase::mount`](crate::agent::AgentBase::mount)).
    ///
    /// `POST /` takes `{"method": "start"|"chat"|"log"|"end", "handle"?,
    /// "message"?, "user_meta_data"?}` with the key in `Authorization: Bearer`.
    /// A chat streams the service's JSON-RPC response body through
    /// **unbuffered** (the service pads slow turns with keepalive whitespace so
    /// proxies do not sever the connection; buffering would swallow it). A newly
    /// minted handle rides back in the `X-Chat-Handle` header. `OPTIONS /`
    /// answers CORS preflight. A body over [`MAX_REQUEST_BODY_BYTES`] is
    /// answered with 413 without being parsed.
    #[must_use]
    pub fn router(&self) -> MountHandler {
        let gw = self.clone();
        Box::new(move |method, path, headers, body| {
            if path != "/" {
                return None;
            }
            let origin = header(headers, "origin");
            let cors = gw.cors(origin);
            if method.eq_ignore_ascii_case("OPTIONS") {
                let mut h = cors;
                if !h.is_empty() {
                    h.insert(
                        "Access-Control-Allow-Headers".to_string(),
                        "Authorization, Content-Type".to_string(),
                    );
                    h.insert(
                        "Access-Control-Allow-Methods".to_string(),
                        "POST, OPTIONS".to_string(),
                    );
                    h.insert("Access-Control-Max-Age".to_string(), "600".to_string());
                }
                return Some((204, h, Box::new(std::io::empty())));
            }
            if !method.eq_ignore_ascii_case("POST") {
                return None;
            }
            Some(gw.proxy(headers, body, cors))
        })
    }

    fn proxy(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
        cors: HashMap<String, String>,
    ) -> MountResponse {
        let origin = header(headers, "origin");
        let auth = header(headers, "authorization").unwrap_or("");
        let key = auth
            .get(..7)
            .filter(|p| p.eq_ignore_ascii_case("bearer "))
            .map(|_| &auth[7..]);
        let prepared = read_json_body(headers, body, MAX_REQUEST_BODY_BYTES).and_then(|parsed| {
            let Some(Value::Object(map)) = parsed else {
                return Err(GatewayRejection::new(400, "body must be an object"));
            };
            self.prepare(&map, origin, key)
        });
        let (method, params, minted) = match prepared {
            Ok(p) => p,
            Err(rej) => return rejected(&rej, cors),
        };
        let mut out_headers = cors;
        out_headers.insert("Content-Type".to_string(), "application/json".to_string());
        let id = params
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let upstream_error = |e: &AIChatError, h: HashMap<String, String>| {
            rejected(
                &GatewayRejection::new(502, &format!("upstream: {}", e.message)),
                h,
            )
        };
        match method.as_str() {
            "end_conversation" => match runtime().block_on(self.client.end(&id)) {
                Ok(_) => (200, out_headers, json_body(&json!({"status": "ended"}))),
                Err(e) => upstream_error(&e, out_headers),
            },
            "create_conversation" => {
                let mut opts = CreateOptions::new(self.config_url.clone());
                opts.timeout = params.get("conversation_timeout").and_then(Value::as_i64);
                opts.user_metadata = params.get("user_meta_data").cloned();
                match runtime().block_on(self.client.create_conversation(&id, opts)) {
                    Ok(info) => {
                        if let Some(h) = minted {
                            out_headers.insert("X-Chat-Handle".to_string(), h);
                        }
                        let payload = json!({
                            "greeting": info.initial_message,
                            "status": info.status,
                            "timeout": self.effective_timeout(),
                        });
                        (200, out_headers, json_body(&payload))
                    }
                    Err(e) => upstream_error(&e, out_headers),
                }
            }
            "chat_log" => match runtime().block_on(self.client.log(&id)) {
                Ok(log) => {
                    let payload = json!({
                        "messages": Self::visible_messages(Some(&log.messages)),
                        "timeout": self.effective_timeout(),
                        "last_activity": Self::last_activity(Some(&log.messages)),
                    });
                    (200, out_headers, json_body(&payload))
                }
                Err(e) => upstream_error(&e, out_headers),
            },
            _ => {
                if let Some(h) = minted {
                    out_headers.insert("X-Chat-Handle".to_string(), h);
                }
                let client = Arc::clone(&self.client);
                let rt = runtime();
                match rt.block_on(client.raw_post(&method, params)) {
                    Ok(mut resp) => {
                        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
                        rt.spawn(async move {
                            while let Ok(Some(chunk)) = resp.chunk().await {
                                if tx.send(chunk.to_vec()).is_err() {
                                    break;
                                }
                            }
                        });
                        let reader = ChannelReader {
                            rx,
                            pending: std::io::Cursor::new(Vec::new()),
                        };
                        (200, out_headers, Box::new(reader))
                    }
                    Err(e) => upstream_error(&e, out_headers),
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod tests;
