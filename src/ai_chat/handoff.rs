//! Moving one conversation between voice and text.
//!
//! [`ChatGateway`] lets a browser hold a text conversation. This module is the
//! other half a browser client needs: the three routes it calls to move that
//! conversation to a phone call and back, and to type into a live call. The
//! SignalWire address widget calls `{gateway-url}/handoff`, `/escalate` and
//! `/say` against the URL that points at a `ChatGateway`, so mount both
//! routers at the same prefix:
//!
//! ```no_run
//! # use signalwire::agent::{AgentBase, AgentOptions};
//! # use signalwire::ai_chat::{ChatGateway, ChatGatewayOptions, HandoffRouter, HandoffRouterOptions};
//! # let mut agent = AgentBase::new(AgentOptions::new("support"));
//! let gateway = ChatGateway::new(ChatGatewayOptions::new("https://example.com/swml"))?;
//! let handoff = HandoffRouter::new(HandoffRouterOptions::new(gateway.clone()));
//! agent.mount(gateway.router(), Some("/chat"), None);
//! agent.mount(handoff.router(), Some("/chat"), None);
//! # Ok::<(), signalwire::ai_chat::AIChatError>(())
//! ```
//!
//! **Mechanism vs policy.** This type owns the wire contract only — the routes,
//! the nonce, the ordering guarantee and the spend guards. Where a leg's
//! transcript is written, what a resumed greeting says, how much history to
//! carry: all of that is the application's, injected as callbacks.
//!
//! **The nonce.** A browser cannot be trusted to name a call (a page-supplied
//! `call_id` would let anyone inject speech into a stranger's call). So the
//! application puts a random `handoff_nonce` in the user variables of one dial,
//! [`register`](HandoffRouter::register)s it against that call's ids, and the
//! browser presents it later. The first registration stands. Redemption is
//! single use; typing is repeatable up to `max_messages_per_call` until the
//! nonce is redeemed or `nonce_ttl` passes. An unknown nonce is answered exactly
//! like an expired or redeemed one, so it cannot probe whether a call is live.
//!
//! **The ordering guarantee.** A medium never starts until the one it replaces
//! has finished and its record is durable: `/handoff` ends the call and waits
//! for `capture_leg` before minting a handle; `/escalate` ends the chat leg and
//! waits before returning.
//!
//! **Deployment.** The nonce registry lives in this process; a redemption must
//! reach the replica that served the dial (one replica, sticky routing, or a
//! shared `registry`). Registration, redemption and the typing count are atomic
//! within one router.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::gateway::{
    ChatGateway, GatewayRejection, MAX_MESSAGE_BYTES, MAX_REQUEST_BODY_BYTES, header, json_body,
    read_json_body, rejected, utf8_len,
};
use crate::agent::agent_base::{MountHandler, MountResponse};

/// Seconds a nonce stays usable after its first registration.
pub const DEFAULT_NONCE_TTL: i64 = 3600;

/// Typed messages allowed into one call.
pub const DEFAULT_MAX_MESSAGES_PER_CALL: i64 = 200;

/// Seconds to wait for `capture_leg` (a ceiling — capture is normally
/// sub-second).
pub const DEFAULT_CAPTURE_TIMEOUT: f64 = 8.0;

/// Seconds on this process's monotonic clock (the `issued_at` timebase).
fn monotonic() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// What a nonce is a capability for.
///
/// `redeemed` marks a nonce `/handoff` has exchanged for a handle; the entry is
/// kept until its TTL passes, so the nonce can be neither redeemed nor
/// registered again.
#[derive(Debug, Clone, PartialEq)]
pub struct NonceEntry {
    /// The conversation the nonce continues.
    pub conversation_id: String,
    /// The call the nonce was dialed on (from the platform's request).
    pub call_id: Option<String>,
    /// When it was registered, in seconds on the process monotonic clock.
    pub issued_at: f64,
    /// Typed messages delivered into the call so far.
    pub messages: i64,
    /// Whether `/handoff` has redeemed it.
    pub redeemed: bool,
}

impl NonceEntry {
    /// A fresh, unredeemed entry issued now.
    #[must_use]
    pub fn new(conversation_id: &str, call_id: Option<&str>) -> Self {
        NonceEntry {
            conversation_id: conversation_id.to_string(),
            call_id: call_id.map(str::to_string),
            issued_at: monotonic(),
            messages: 0,
            redeemed: false,
        }
    }
}

/// `capture_leg(conversation_id, medium)`: end the leg and write its record;
/// return `true` once the record is durable.
pub type CaptureLeg = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;
/// `end_call(call_id)`: hang the call up server-side.
pub type EndCall = Arc<dyn Fn(&str) + Send + Sync>;
/// `send_message(call_id, text)`: inject typed text into the live call.
pub type SendMessage = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;
/// `next_conversation_id(conversation_id)`: the id for the NEW leg.
pub type NextConversationId = Arc<dyn Fn(&str) -> String + Send + Sync>;

/// Construction options for [`HandoffRouter`] (the reference's keyword
/// arguments). `gateway` is required.
#[derive(Clone)]
pub struct HandoffRouterOptions {
    /// The gateway that owns the conversations (mints handles, checks origins).
    pub gateway: ChatGateway,
    /// Ends a leg and confirms its record is durable. Omitted: no wait, and no
    /// ordering guarantee.
    pub capture_leg: Option<CaptureLeg>,
    /// Hangs a call up server-side.
    pub end_call: Option<EndCall>,
    /// Delivers `/say` text into a call. Omitted: typing disabled (404).
    pub send_message: Option<SendMessage>,
    /// Produces the next leg's conversation id (default appends `.N`).
    pub next_conversation_id: Option<NextConversationId>,
    /// Seconds a nonce stays usable (default [`DEFAULT_NONCE_TTL`]).
    pub nonce_ttl: Option<i64>,
    /// Typed-message ceiling per call (default [`DEFAULT_MAX_MESSAGES_PER_CALL`]).
    pub max_messages_per_call: Option<i64>,
    /// Seconds to wait for `capture_leg` (default [`DEFAULT_CAPTURE_TIMEOUT`]).
    pub capture_timeout: Option<f64>,
    /// A shared nonce table, for running more than one router or replica.
    pub registry: Option<Arc<Mutex<HashMap<String, NonceEntry>>>>,
}

impl HandoffRouterOptions {
    /// Options for a router beside `gateway`.
    #[must_use]
    pub fn new(gateway: ChatGateway) -> Self {
        HandoffRouterOptions {
            gateway,
            capture_leg: None,
            end_call: None,
            send_message: None,
            next_conversation_id: None,
            nonce_ttl: None,
            max_messages_per_call: None,
            capture_timeout: None,
            registry: None,
        }
    }

    /// Set the capture callback.
    #[must_use]
    pub fn capture_leg(mut self, f: CaptureLeg) -> Self {
        self.capture_leg = Some(f);
        self
    }

    /// Set the end-call callback.
    #[must_use]
    pub fn end_call(mut self, f: EndCall) -> Self {
        self.end_call = Some(f);
        self
    }

    /// Set the send-message callback (enables `/say`).
    #[must_use]
    pub fn send_message(mut self, f: SendMessage) -> Self {
        self.send_message = Some(f);
        self
    }

    /// Set the next-conversation-id policy.
    #[must_use]
    pub fn next_conversation_id(mut self, f: NextConversationId) -> Self {
        self.next_conversation_id = Some(f);
        self
    }

    /// Set the nonce TTL in seconds.
    #[must_use]
    pub fn nonce_ttl(mut self, seconds: i64) -> Self {
        self.nonce_ttl = Some(seconds);
        self
    }

    /// Set the per-call typed-message ceiling.
    #[must_use]
    pub fn max_messages_per_call(mut self, n: i64) -> Self {
        self.max_messages_per_call = Some(n);
        self
    }

    /// Set the capture timeout in seconds.
    #[must_use]
    pub fn capture_timeout(mut self, seconds: f64) -> Self {
        self.capture_timeout = Some(seconds);
        self
    }

    /// Use a shared nonce table.
    #[must_use]
    pub fn registry(mut self, registry: Arc<Mutex<HashMap<String, NonceEntry>>>) -> Self {
        self.registry = Some(registry);
        self
    }
}

/// The three routes a browser client needs beside a [`ChatGateway`].
#[derive(Clone)]
pub struct HandoffRouter {
    /// The gateway that owns the conversations.
    pub gateway: ChatGateway,
    /// Ends a leg and confirms its record.
    pub capture_leg: Option<CaptureLeg>,
    /// Hangs a call up server-side.
    pub end_call: Option<EndCall>,
    /// Delivers typed text into a call.
    pub send_message: Option<SendMessage>,
    /// Produces the next leg's conversation id.
    pub next_conversation_id: NextConversationId,
    /// Seconds a nonce stays usable.
    pub nonce_ttl: i64,
    /// Typed-message ceiling per call.
    pub max_messages_per_call: i64,
    /// Seconds to wait for `capture_leg`.
    pub capture_timeout: f64,
    nonces: Arc<Mutex<HashMap<String, NonceEntry>>>,
}

impl std::fmt::Debug for HandoffRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandoffRouter")
            .field("nonce_ttl", &self.nonce_ttl)
            .field("max_messages_per_call", &self.max_messages_per_call)
            .finish_non_exhaustive()
    }
}

/// `root` -> `root.1`; `root.2` -> `root.3` (`.` because the chat service
/// strips `~`, `_`/`-` occur inside generated ids, and `:` is the handle
/// delimiter).
fn default_next_id(conversation_id: &str) -> String {
    if let Some((root, tail)) = conversation_id.rsplit_once('.')
        && !root.is_empty()
        && !tail.is_empty()
        && tail.bytes().all(|b| b.is_ascii_digit())
        && let Ok(n) = tail.parse::<u64>()
    {
        return format!("{root}.{}", n + 1);
    }
    format!("{conversation_id}.1")
}

impl HandoffRouter {
    /// Configure the handoff.
    #[must_use]
    pub fn new(options: HandoffRouterOptions) -> Self {
        HandoffRouter {
            gateway: options.gateway,
            capture_leg: options.capture_leg,
            end_call: options.end_call,
            send_message: options.send_message,
            next_conversation_id: options
                .next_conversation_id
                .unwrap_or_else(|| Arc::new(default_next_id)),
            nonce_ttl: options.nonce_ttl.unwrap_or(DEFAULT_NONCE_TTL),
            max_messages_per_call: options
                .max_messages_per_call
                .unwrap_or(DEFAULT_MAX_MESSAGES_PER_CALL),
            capture_timeout: options.capture_timeout.unwrap_or(DEFAULT_CAPTURE_TIMEOUT),
            nonces: options
                .registry
                .unwrap_or_else(|| Arc::new(Mutex::new(HashMap::new()))),
        }
    }

    fn table(&self) -> std::sync::MutexGuard<'_, HashMap<String, NonceEntry>> {
        self.nonces
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Drop entries (redeemed ones included) whose TTL has passed.
    #[allow(clippy::cast_precision_loss)]
    fn prune(&self, table: &mut HashMap<String, NonceEntry>) {
        let cutoff = monotonic() - self.nonce_ttl as f64;
        table.retain(|_, e| e.issued_at >= cutoff);
    }

    /// Record what a nonce is a capability for. Call it from the per-call
    /// config callback of the dial that carried the nonce, reading `call_id`
    /// from the platform's request — never from anything the browser supplied.
    ///
    /// The first registration stands: re-registering a nonce already in the
    /// table changes nothing (a mismatch is logged), until its `nonce_ttl`
    /// passes.
    pub fn register(&self, nonce: &str, conversation_id: &str, call_id: Option<&str>) {
        if nonce.is_empty() {
            return;
        }
        let mut table = self.table();
        self.prune(&mut table);
        if let Some(existing) = table.get(nonce) {
            if existing.redeemed
                || existing.conversation_id != conversation_id
                || existing.call_id.as_deref() != call_id
            {
                log::warn!(
                    "handoff_nonce_already_registered conversation_id={} redeemed={}",
                    existing.conversation_id,
                    existing.redeemed
                );
            }
            return;
        }
        table.insert(nonce.to_string(), NonceEntry::new(conversation_id, call_id));
        log::info!("handoff_nonce_registered conversation_id={conversation_id}");
    }

    /// The live entry for `nonce` (unknown, expired and redeemed are all `None`).
    fn lookup(&self, table: &mut HashMap<String, NonceEntry>, nonce: &str) -> Option<NonceEntry> {
        if nonce.is_empty() {
            return None;
        }
        self.prune(table);
        table.get(nonce).filter(|e| !e.redeemed).cloned()
    }

    /// Run the application's capture, bounded by `capture_timeout`. Never fails.
    fn capture(&self, conversation_id: &str, medium: &str) -> bool {
        let Some(capture) = self.capture_leg.clone() else {
            return false;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let (c, m) = (conversation_id.to_string(), medium.to_string());
        std::thread::spawn(move || {
            let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| capture(&c, &m)))
                .unwrap_or(false);
            let _ = tx.send(ok);
        });
        let timeout = Duration::from_secs_f64(self.capture_timeout.max(0.0));
        if let Ok(ok) = rx.recv_timeout(timeout) {
            ok
        } else {
            log::warn!(
                "handoff_capture_timeout conversation_id={conversation_id} medium={medium} \
                 (starting the next medium without this leg's record)"
            );
            false
        }
    }

    /// Exchange a nonce for a chat handle. Single use: ends the call, waits
    /// for its record, and only then mints a handle for a new leg of the same
    /// conversation. `None` for an unknown, expired or already redeemed nonce
    /// — deliberately indistinguishable.
    #[must_use]
    pub fn redeem(&self, nonce: &str) -> Option<String> {
        let entry = {
            let mut table = self.table();
            let mut entry = self.lookup(&mut table, nonce)?;
            // Consumed even if what follows fails: a nonce is one attempt.
            entry.redeemed = true;
            table.insert(nonce.to_string(), entry.clone());
            entry
        };
        if let (Some(call_id), Some(end_call)) = (entry.call_id.as_deref(), self.end_call.as_ref())
        {
            let end_call = Arc::clone(end_call);
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| end_call(call_id))).is_err()
            {
                log::warn!("handoff_end_call_failed");
            }
        }
        self.capture(&entry.conversation_id, "voice");
        let next = (self.next_conversation_id)(&entry.conversation_id);
        let handle = self.gateway.mint_handle(Some(&next));
        log::info!("handoff_redeemed conversation_id={}", entry.conversation_id);
        Some(handle)
    }

    /// End a chat leg and wait for its record, before a call is placed. `false`
    /// for a handle that does not verify.
    #[must_use]
    pub fn escalate(&self, handle: &str) -> bool {
        let Ok(conversation_id) = self.gateway.read_handle(handle) else {
            return false;
        };
        self.capture(&conversation_id, "chat");
        log::info!("handoff_escalated conversation_id={conversation_id}");
        true
    }

    /// Deliver typed text into the live call the nonce names. Does NOT consume
    /// the nonce: typing repeats until it is redeemed or `nonce_ttl` passes, up
    /// to `max_messages_per_call`. Addressed by nonce only; no other request
    /// field (notably `global_data`) is forwarded. Text over
    /// [`MAX_MESSAGE_BYTES`] is refused.
    #[must_use]
    pub fn say(&self, nonce: &str, text: &str) -> bool {
        let Some(send) = self.send_message.clone() else {
            return false;
        };
        let cleaned = text.trim();
        if cleaned.is_empty() || utf8_len(cleaned) > MAX_MESSAGE_BYTES {
            return false;
        }
        let entry = {
            let mut table = self.table();
            let Some(mut entry) = self.lookup(&mut table, nonce) else {
                return false;
            };
            if entry.call_id.is_none() {
                return false;
            }
            if entry.messages >= self.max_messages_per_call {
                log::warn!("handoff_say_cap_reached");
                return false;
            }
            // Take the slot before delivering, so overlapping requests can't
            // all pass the cap.
            entry.messages += 1;
            table.insert(nonce.to_string(), entry.clone());
            entry
        };
        let call_id = entry.call_id.clone().unwrap_or_default();
        let delivered =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| send(&call_id, cleaned)));
        if delivered.is_err() {
            log::error!("handoff_say_failed");
            // Not delivered: give the slot back if the table still holds this
            // registration.
            let mut table = self.table();
            if let Some(current) = table.get_mut(nonce)
                && current.messages > 0
                && current.conversation_id == entry.conversation_id
                && current.call_id == entry.call_id
                && (current.issued_at - entry.issued_at).abs() < f64::EPSILON
            {
                current.messages -= 1;
            }
            return false;
        }
        true
    }

    /// The three routes as a mountable router (`/handoff`, `/escalate`,
    /// `/say`). Mount at the SAME prefix as the gateway's router. Every route
    /// answers 413 for a body over the gateway's [`MAX_REQUEST_BODY_BYTES`], and
    /// `/say` for text over [`MAX_MESSAGE_BYTES`]; both are checked before the
    /// nonce is looked up.
    #[must_use]
    pub fn router(&self) -> MountHandler {
        let router = self.clone();
        Box::new(move |method, path, headers, body| {
            if !method.eq_ignore_ascii_case("POST")
                || !matches!(path, "/handoff" | "/escalate" | "/say")
            {
                return None;
            }
            Some(router.route(path, headers, body))
        })
    }

    fn route(&self, path: &str, headers: &HashMap<String, String>, body: &str) -> MountResponse {
        let respond = |status: u16, payload: Value| -> MountResponse {
            let mut h = HashMap::new();
            h.insert("Content-Type".to_string(), "application/json".to_string());
            (status, h, json_body(&payload))
        };
        if self
            .gateway
            .check_origin(header(headers, "origin"))
            .is_err()
        {
            return respond(403, json!({"error": "origin not allowed"}));
        }
        let data = match read_json_body(headers, body, MAX_REQUEST_BODY_BYTES) {
            Ok(Some(Value::Object(m))) => m,
            Ok(_) => serde_json::Map::new(),
            Err(rej) => return rejected(&rej, HashMap::new()),
        };
        let not_found = || respond(404, json!({"error": "not found"}));
        match path {
            "/handoff" => {
                let Some(nonce) = data.get("nonce").and_then(Value::as_str) else {
                    return not_found();
                };
                // Same answer for unknown, expired and already-redeemed.
                self.redeem(nonce)
                    .map_or_else(not_found, |handle| respond(200, json!({"handle": handle})))
            }
            "/escalate" => {
                let Some(handle) = data
                    .get("handle")
                    .and_then(Value::as_str)
                    .filter(|h| !h.is_empty())
                else {
                    return respond(400, json!({"error": "bad request"}));
                };
                if self.escalate(handle) {
                    respond(200, json!({"ok": true}))
                } else {
                    not_found()
                }
            }
            _ => {
                let nonce = data.get("nonce").and_then(Value::as_str);
                let text = match data.get("text") {
                    None => Some(""),
                    Some(v) => v.as_str(),
                };
                let (Some(nonce), Some(text)) = (nonce, text) else {
                    return not_found();
                };
                if utf8_len(text) > MAX_MESSAGE_BYTES {
                    return rejected(
                        &GatewayRejection::new(413, "message too large"),
                        HashMap::new(),
                    );
                }
                if self.say(nonce, text) {
                    respond(200, json!({"ok": true}))
                } else {
                    not_found()
                }
            }
        }
    }
}
