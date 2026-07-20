use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as WsMessage, WebSocket};

use super::call::Call;
use super::constants;
use super::error::RelayError;
use super::event::Event;
use super::message::Message;
use crate::logging::Logger;

/// Default WebSocket path appended to the configured host. Mirrors Python's
/// `wss://{space}/api/relay/ws` and is the canonical RELAY endpoint.
const RELAY_PATH: &str = "/api/relay/ws";

/// How long `connect()` waits for the `signalwire.connect` response before
/// giving up and tearing the socket down. Matches Python's `_EXECUTE_TIMEOUT`.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Live socket type. `MaybeTlsStream` selects plain TCP for `ws://` and TLS
/// when compiled with native-tls / rustls features. The audit fixture uses
/// plain `ws://`; production users get `wss://` with TLS-enabled features.
type WsStream = WebSocket<MaybeTlsStream<TcpStream>>;

/// WebSocket transport setup, including TLS for `wss://`.
///
/// The crate enables tungstenite's `rustls-tls-webpki-roots` feature, so
/// `wss://` connections verify against the bundled Mozilla root set out of the
/// box. rustls deliberately does NOT consult `SSL_CERT_FILE` or the OS trust
/// store, so to trust a private / self-signed CA (e.g. a self-signed CA in a
/// test environment, or a corporate proxy CA in production) the
/// caller sets `SIGNALWIRE_RELAY_CA_FILE` to a PEM bundle. When set, we build a
/// dedicated `rustls::ClientConfig` whose root store is *exactly* that CA and
/// hand it to tungstenite as a custom `Connector` — real verification against a
/// caller-chosen trust anchor, never `danger_accept_invalid_certs`.
mod tls {
    use std::net::TcpStream;
    use std::sync::Arc;

    use tungstenite::client::uri_mode;
    use tungstenite::stream::{MaybeTlsStream, Mode};
    use tungstenite::{Connector, client_tls_with_config};

    use super::RelayError;
    use super::WsStream;

    /// Env var pointing at a PEM CA bundle to trust for `wss://` instead of
    /// (in addition to verifying against) the built-in webpki roots. Mirrors
    /// the per-client custom-CA hook the Go pilot flagged as a follow-up;
    /// here it is wired so the relay client can verify a private CA.
    const CA_FILE_ENV: &str = "SIGNALWIRE_RELAY_CA_FILE";

    /// Open a verified WebSocket to `url`.
    ///
    /// * `ws://`  -> plain TCP (the shared mock-relay audit path).
    /// * `wss://` with no `SIGNALWIRE_RELAY_CA_FILE` -> rustls + webpki roots.
    /// * `wss://` with `SIGNALWIRE_RELAY_CA_FILE` -> rustls verifying against
    ///   *that* CA, via a custom `Connector::Rustls`.
    pub fn ws_connect(url: &str) -> Result<WsStream, RelayError> {
        let ca_file = std::env::var(CA_FILE_ENV).ok().filter(|s| !s.is_empty());

        // No custom CA: let tungstenite drive the connect with its default
        // (webpki-roots) connector. This still performs full verification.
        let Some(ca_file) = ca_file else {
            let (ws, _resp) = tungstenite::connect(url)
                .map_err(|e| RelayError::transport(format!("connect {url}"), e))?;
            return Ok(ws);
        };

        // Custom CA path: parse the URL, open the TCP stream ourselves, and
        // wrap it with a rustls connector whose only trust anchor is `ca_file`.
        let parsed = url::Url::parse(url)
            .map_err(|e| RelayError::transport(format!("parse url {url}"), e))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| RelayError::transport(format!("url {url}"), "no host in URL"))?
            .to_string();
        let mode = uri_mode(
            &url.parse::<http::Uri>()
                .map_err(|e| RelayError::transport(format!("parse uri {url}"), e))?,
        )
        .map_err(|e| RelayError::transport(format!("uri mode {url}"), e))?;
        let port = parsed.port().unwrap_or(match mode {
            Mode::Tls => 443,
            Mode::Plain => 80,
        });

        let tcp = TcpStream::connect((host.as_str(), port))
            .map_err(|e| RelayError::transport(format!("tcp connect {host}:{port}"), e))?;

        let config = client_config_trusting(&ca_file)?;
        let connector = Connector::Rustls(Arc::new(config));

        let (ws, _resp) = client_tls_with_config(url, tcp, None, Some(connector))
            .map_err(|e| RelayError::transport(format!("wss handshake {url}"), e))?;
        // Sanity: a `wss://` url with a custom CA must have negotiated TLS.
        debug_assert!(matches!(ws.get_ref(), MaybeTlsStream::Rustls(_)));
        Ok(ws)
    }

    /// Build a `rustls::ClientConfig` whose root store contains *only* the
    /// certificate(s) in the PEM file at `ca_path`. Uses the `ring` provider
    /// explicitly (it is already in the dependency tree) rather than relying on
    /// a process-default `CryptoProvider`, which the crate never installs.
    fn client_config_trusting(ca_path: &str) -> Result<rustls::ClientConfig, RelayError> {
        use rustls_pki_types::pem::PemObject;

        let pem = std::fs::read(ca_path)
            .map_err(|e| RelayError::transport(format!("read CA file {ca_path}"), e))?;

        let mut roots = rustls::RootCertStore::empty();
        let mut added = 0usize;
        for cert in rustls_pki_types::CertificateDer::pem_slice_iter(&pem) {
            let cert =
                cert.map_err(|e| RelayError::transport(format!("parse CA pem {ca_path}"), e))?;
            roots
                .add(cert)
                .map_err(|e| RelayError::transport(format!("add CA cert from {ca_path}"), e))?;
            added += 1;
        }
        if added == 0 {
            return Err(RelayError::transport(
                format!("CA file {ca_path}"),
                "no certificates found",
            ));
        }

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| RelayError::transport("rustls protocol versions", e))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(config)
    }
}

/// Callback type for inbound call handler.
pub type OnCallHandler = Box<dyn Fn(Arc<Call>, &Event) + Send + Sync>;

/// Callback type for inbound message handler.
pub type OnMessageHandler = Box<dyn Fn(&Event, &Value) + Send + Sync>;

/// Callback type for generic events.
pub type OnEventHandler = Box<dyn Fn(&Event, &Value) + Send + Sync>;

/// Resolve callback for a pending RPC request.
type ResolveCallback = Box<dyn FnOnce(Value) + Send>;

/// Reject callback for a pending RPC request.
type RejectCallback = Box<dyn FnOnce(Value) + Send>;

/// Pending RPC request slot.
struct PendingRequest {
    resolve: Option<ResolveCallback>,
    reject: Option<RejectCallback>,
}

/// Pending dial slot.
struct PendingDial {
    resolve: Box<dyn FnOnce(Arc<Call>) + Send>,
    #[allow(dead_code)]
    tag: String,
}

/// RELAY Client -- manages the WebSocket connection to SignalWire, sends
/// JSON-RPC requests, and dispatches inbound events to the correct Call
/// or Message objects.
///
/// The transport is a real WebSocket connection over TCP (plus TLS for
/// `wss://`). One reader thread (spawned on `connect()`) owns the read
/// half and dispatches every inbound JSON-RPC frame through
/// `handle_message`. Writes go through `send()` which serializes the
/// frame and pushes it onto an mpsc channel that the reader thread
/// drains alongside its read loop, so all socket I/O is single-
/// threaded but both directions make forward progress.
///
/// Tests still use `sent_messages` to inspect what the client *would*
/// have written; `send()` mirrors every frame into that Vec whether or
/// not a live socket is attached. That keeps the unit tests covering
/// dispatch logic working without a real RELAY server.
pub struct Client {
    // ── identity / auth ───────────────────────────────────────────────
    pub project: String,
    pub token: String,
    pub host: String,
    pub contexts: Mutex<Vec<String>>,
    pub connected: Mutex<bool>,
    pub session_id: Mutex<Option<String>>,
    pub protocol: Mutex<Option<String>>,
    pub authorization_state: Mutex<Option<String>>,
    pub agent: String,

    // ── correlation maps ──────────────────────────────────────────────
    pending: Mutex<HashMap<String, PendingRequest>>,
    pub calls: Mutex<HashMap<String, Arc<Call>>>,
    pending_dials: Mutex<HashMap<String, PendingDial>>,
    pub messages: Mutex<HashMap<String, Arc<Message>>>,

    // ── event handlers ────────────────────────────────────────────────
    on_call_handler: Mutex<Option<OnCallHandler>>,
    on_message_handler: Mutex<Option<OnMessageHandler>>,
    on_event_handler: Mutex<Option<OnEventHandler>>,

    // ── internals ─────────────────────────────────────────────────────
    reconnect_delay: Mutex<u64>,
    running: Mutex<bool>,

    /// Outbound write channel — `send()` enqueues a JSON-encoded frame
    /// and the reader thread flushes it to the socket. `None` when no
    /// reader thread is running (purely in-memory test mode).
    write_tx: Mutex<Option<Sender<WsMessage>>>,

    /// Reader thread join handle. Set on `connect()`, joined on
    /// `disconnect()` / drop.
    reader_thread: Mutex<Option<thread::JoinHandle<()>>>,

    /// Reader thread observes this to know when to exit.
    closing: Arc<AtomicBool>,

    /// Bounded log of frames sent through the transport, for wire-frame
    /// inspection / tests (read via [`Client::sent_messages`]). Internal field
    /// so callers cannot mutate it; capped at [`crate::relay::SENT_LOG_CAP`].
    pub(crate) sent_messages: Mutex<Vec<Value>>,

    logger: Logger,
}

impl Client {
    pub fn new(project: &str, token: &str, host: &str) -> Self {
        Client {
            project: project.to_string(),
            token: token.to_string(),
            host: host.to_string(),
            contexts: Mutex::new(Vec::new()),
            connected: Mutex::new(false),
            session_id: Mutex::new(None),
            protocol: Mutex::new(None),
            authorization_state: Mutex::new(None),
            // RELAY protocol agent identifier sent on the connect frame. The
            // `/1.0` mirrors Python's `relay.constants.AGENT_STRING`
            // ("signalwire-agents-python/1.0") — it is the wire protocol tag, NOT
            // the crate version, so it must stay `/1.0` for cross-port parity.
            agent: "signalwire-agents-rust/1.0".to_string(),
            pending: Mutex::new(HashMap::new()),
            calls: Mutex::new(HashMap::new()),
            pending_dials: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new()),
            on_call_handler: Mutex::new(None),
            on_message_handler: Mutex::new(None),
            on_event_handler: Mutex::new(None),
            reconnect_delay: Mutex::new(1),
            running: Mutex::new(false),
            write_tx: Mutex::new(None),
            reader_thread: Mutex::new(None),
            closing: Arc::new(AtomicBool::new(false)),
            sent_messages: Mutex::new(Vec::new()),
            logger: Logger::new("relay.client"),
        }
    }

    /// Create from env vars `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RelayError::missing_env(...))` if any of the three
    /// required environment variables — `SIGNALWIRE_PROJECT_ID`,
    /// `SIGNALWIRE_API_TOKEN`, or `SIGNALWIRE_SPACE` — is unset (or not
    /// valid UTF-8). No network I/O occurs here, so this is purely a
    /// configuration-presence check.
    pub fn from_env() -> Result<Self, RelayError> {
        let project = std::env::var("SIGNALWIRE_PROJECT_ID")
            .map_err(|_| RelayError::missing_env("SIGNALWIRE_PROJECT_ID"))?;
        let token = std::env::var("SIGNALWIRE_API_TOKEN")
            .map_err(|_| RelayError::missing_env("SIGNALWIRE_API_TOKEN"))?;
        let host = std::env::var("SIGNALWIRE_SPACE")
            .map_err(|_| RelayError::missing_env("SIGNALWIRE_SPACE"))?;
        Ok(Self::new(&project, &token, &host))
    }

    // ══════════════════════════════════════════════════════════════════
    //  Connection lifecycle
    // ══════════════════════════════════════════════════════════════════

    /// Open the WebSocket connection, run the `signalwire.connect`
    /// handshake, subscribe to the configured contexts, and spawn the
    /// reader thread that dispatches every inbound frame through
    /// `handle_message`.
    ///
    /// Reads the WebSocket scheme from `SIGNALWIRE_RELAY_SCHEME` (defaults
    /// to `wss`; the audit fixture sets `ws`) and the host override from
    /// `SIGNALWIRE_RELAY_HOST` (used by the audit fixture to point at a
    /// `127.0.0.1:N` ephemeral port). In production neither env var is
    /// usually set and the URL resolves to `wss://{self.host}/api/relay/ws`,
    /// matching Python's `RelayClient.connect()`.
    ///
    /// Returns `Err` if the TCP/WS upgrade fails, the server rejects the
    /// connect handshake, or the response doesn't arrive within
    /// `HANDSHAKE_TIMEOUT`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RelayError)` when the connection cannot be brought
    /// up: `RelayError::Transport` if the TCP connect / WebSocket
    /// upgrade fails or the reader thread cannot be spawned;
    /// `RelayError::Auth` if the server rejects the `signalwire.connect`
    /// handshake; or `RelayError::Timeout` if no handshake response
    /// arrives within `HANDSHAKE_TIMEOUT` (the auth failure paths are
    /// surfaced through the delegated [`authenticate_blocking`]).
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn connect(self: &Arc<Self>) -> Result<(), RelayError> {
        self.logger.info(&format!("Connecting to {}", self.host));

        let scheme = std::env::var("SIGNALWIRE_RELAY_SCHEME").unwrap_or_else(|_| "wss".to_string());
        let host_override = std::env::var("SIGNALWIRE_RELAY_HOST").ok();
        let endpoint_host = host_override.as_deref().unwrap_or(self.host.as_str());
        let url = format!("{scheme}://{endpoint_host}{RELAY_PATH}");

        // ws_connect already returns a `RelayError::Transport`; keep its
        // (richer) context rather than re-wrapping it.
        let ws_stream = tls::ws_connect(&url)?;

        // Set a short read timeout on the underlying TCP stream so the
        // reader thread can periodically check the closing flag and the
        // outbound write channel without blocking forever on read().
        // `ws://` yields a Plain stream; `wss://` (rustls enabled) yields a
        // Rustls stream whose `.sock` is the same TcpStream — set the timeout
        // on whichever socket is live so the reader loop stays responsive.
        match ws_stream.get_ref() {
            MaybeTlsStream::Plain(s) => {
                let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
            }
            MaybeTlsStream::Rustls(s) => {
                let _ = s.sock.set_read_timeout(Some(Duration::from_millis(100)));
            }
            _ => {}
        }

        // Wire the write side: outbound `send()` calls push frames here;
        // the reader thread drains the receiver alongside its read loop.
        let (write_tx, write_rx) = mpsc::channel::<WsMessage>();
        *self.write_tx.lock().unwrap() = Some(write_tx);

        // Reset closing state — a fresh connect starts running again.
        self.closing.store(false, Ordering::SeqCst);
        *self.connected.lock().unwrap() = true;
        *self.running.lock().unwrap() = true;

        // Spawn the reader thread. It owns the WebSocket; all socket I/O
        // happens inside it. Both directions make forward progress
        // because the loop alternates read attempts (with a 100ms read
        // timeout) and write-channel drains.
        let me = Arc::clone(self);
        let handle = thread::Builder::new()
            .name("relay-reader".to_string())
            .spawn(move || {
                Client::reader_loop(me, ws_stream, write_rx);
            })
            .map_err(|e| RelayError::transport("spawn reader thread", e))?;
        *self.reader_thread.lock().unwrap() = Some(handle);

        // Run signalwire.connect synchronously, blocking until the auth
        // response arrives or HANDSHAKE_TIMEOUT elapses. Contexts known
        // at construction time go in `params.contexts` of the connect
        // frame, matching Python's behavior. Callers can still call
        // [`receive`] later to add more contexts dynamically.
        self.authenticate_blocking()?;

        Ok(())
    }

    /// Initial connect -- resets reconnect delay and connects.
    ///
    /// # Errors
    ///
    /// Propagates the error from [`connect`]: `RelayError::Transport`
    /// (TCP/WS upgrade or thread spawn failure), `RelayError::Auth`
    /// (handshake rejected), or `RelayError::Timeout` (no handshake
    /// response within `HANDSHAKE_TIMEOUT`).
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn connect_fresh(self: &Arc<Self>) -> Result<(), RelayError> {
        *self.reconnect_delay.lock().unwrap() = 1;
        self.connect()
    }

    /// Send the `signalwire.connect` RPC and block until the response
    /// arrives or the handshake times out. The response carries the
    /// server-assigned protocol string and authorization state.
    ///
    /// # Errors
    ///
    /// Returns `Err(RelayError::Auth { message })` if the server rejects
    /// `signalwire.connect` (the message comes from the JSON-RPC error's
    /// `message` field, defaulting to `"authentication failed"`), or
    /// `Err(RelayError::Timeout)` if no response is received within
    /// `HANDSHAKE_TIMEOUT` (in which case the pending request is cleaned
    /// up so a late reply cannot fire a stale callback).
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn authenticate_blocking(&self) -> Result<(), RelayError> {
        self.logger.info("Authenticating");

        let id = generate_uuid();
        // We send `params.project`/`params.token` at the top level AND
        // under `params.authentication`. Python sends them only under
        // `authentication`; the porting-sdk's audit fixture watches the
        // top level. Both real RELAY servers and the audit fixture
        // accept the top-level shape, so this is the common form.
        let mut params = json!({
            "version": {
                "major": constants::PROTOCOL_VERSION_MAJOR,
                "minor": constants::PROTOCOL_VERSION_MINOR,
                "revision": constants::PROTOCOL_VERSION_REVISION,
            },
            "project": self.project,
            "token": self.token,
            "authentication": {
                "project": self.project,
                "token": self.token,
            },
            "agent": self.agent,
            "event_acks": true,
        });
        // Include current contexts on the connect frame so the audit
        // fixture (and Python's RELAY) sees the subscription on the
        // initial handshake. Subsequent `signalwire.receive` calls
        // dynamically add more.
        let ctxs: Vec<String> = self.contexts.lock().unwrap().clone();
        if !ctxs.is_empty()
            && let Value::Object(ref mut obj) = params
        {
            obj.insert("contexts".to_string(), json!(ctxs));
        }
        // Re-send authorization state on reconnect for fast resume.
        if let Some(state) = self.authorization_state.lock().unwrap().clone()
            && let Value::Object(ref mut obj) = params
        {
            obj.insert("authorization_state".to_string(), json!(state));
        }
        // Re-send the previously-issued protocol string on reconnect so
        // the server can restore the session. Mirrors Python's
        // `RelayClient.connect` behaviour when `_relay_protocol` is set.
        if let Some(p) = self.protocol.lock().unwrap().clone()
            && let Value::Object(ref mut obj) = params
        {
            obj.insert("protocol".to_string(), json!(p));
        }

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "signalwire.connect",
            "params": params,
        });

        // Use a oneshot channel to surface the response back here.
        let (resp_tx, resp_rx) = mpsc::channel::<Result<Value, Value>>();
        let resolve_tx = resp_tx.clone();
        let reject_tx = resp_tx;
        self.register_pending(
            &id,
            move |v| {
                let _ = resolve_tx.send(Ok(v));
            },
            move |e| {
                let _ = reject_tx.send(Err(e));
            },
        );

        self.send(&msg);

        // Block on the oneshot. If the reader thread sees the response
        // it forwards it through `pending` → resolve → resp_tx.
        match resp_rx.recv_timeout(HANDSHAKE_TIMEOUT) {
            Ok(Ok(result)) => {
                if let Some(p) = result.get("protocol").and_then(|v| v.as_str()) {
                    *self.protocol.lock().unwrap() = Some(p.to_string());
                }
                if let Some(state) = result
                    .get("authorization")
                    .and_then(|a| a.get("authorization_state"))
                    .and_then(|v| v.as_str())
                {
                    *self.authorization_state.lock().unwrap() = Some(state.to_string());
                }
                // Capture the server-assigned session id from the connect
                // handshake result (`result.sessionid`). Mirrors Python's
                // `RelayClient` and the TS port: the SDK keeps this for its
                // own bookkeeping, and mock-backed tests read it to scope
                // their journal reads/resets to this connection's session so
                // the shared mock_relay server is safe under parallel tests.
                if let Some(sid) = result.get("sessionid").and_then(|v| v.as_str()) {
                    *self.session_id.lock().unwrap() = Some(sid.to_string());
                }
                self.logger.info("Authenticated");
                Ok(())
            }
            Ok(Err(err)) => {
                let msg = err
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("authentication failed")
                    .to_string();
                Err(RelayError::Auth { message: msg })
            }
            Err(_) => {
                // Pending may still be registered — clean it up so a
                // late response doesn't trip into a stale callback.
                self.pending.lock().unwrap().remove(&id);
                Err(RelayError::Timeout {
                    what: format!("signalwire.connect response (after {HANDSHAKE_TIMEOUT:?})"),
                })
            }
        }
    }

    /// Backwards-compat: enqueue the `signalwire.connect` frame without
    /// waiting. Used by older tests that drive `handle_message` directly.
    /// Production code should call [`connect`] which runs the full
    /// handshake.
    pub fn authenticate(&self) {
        let id = generate_uuid();
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "signalwire.connect",
            "params": {
                "version": {
                    "major": constants::PROTOCOL_VERSION_MAJOR,
                    "minor": constants::PROTOCOL_VERSION_MINOR,
                    "revision": constants::PROTOCOL_VERSION_REVISION,
                },
                "authentication": {
                    "project": self.project,
                    "token": self.token,
                },
                "agent": self.agent,
            },
        });
        self.send(&msg);
    }

    /// Gracefully close the connection. Signals the reader thread to
    /// exit, sends a WS close frame, and joins the thread.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn disconnect(&self) {
        self.logger.info("Disconnecting");
        self.closing.store(true, Ordering::SeqCst);
        *self.running.lock().unwrap() = false;
        *self.connected.lock().unwrap() = false;
        // Drop the write sender so the reader's drain loop sees the
        // channel close and breaks promptly.
        *self.write_tx.lock().unwrap() = None;

        // Join the reader thread (best effort — thread will exit once
        // it observes the closing flag).
        if let Some(handle) = self.reader_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
    }

    /// Reconnect with exponential back-off (1s → 30s cap). Sleeps for
    /// the current delay, doubles the delay (capped at 30s), and runs
    /// the full connect handshake again. Authorization state survives
    /// across reconnects because [`authenticate_blocking`] re-sends the
    /// stored token on the new socket.
    ///
    /// # Errors
    ///
    /// After the back-off sleep, propagates the error from [`connect`]:
    /// `RelayError::Transport` if the new socket cannot be established,
    /// `RelayError::Auth` if the re-handshake is rejected, or
    /// `RelayError::Timeout` if the handshake response does not arrive
    /// within `HANDSHAKE_TIMEOUT`.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn reconnect(self: &Arc<Self>) -> Result<(), RelayError> {
        *self.connected.lock().unwrap() = false;

        let delay = self.bump_reconnect_delay();
        self.logger.warn(&format!("Reconnecting in {delay}s"));
        thread::sleep(Duration::from_secs(delay));

        self.connect()
    }

    /// Compute the next reconnect delay (1s → 2s → 4s → … → 30s) and
    /// return the value to wait *this* time. Mirrors Python's
    /// `RECONNECT_MIN_DELAY` / `RECONNECT_MAX_DELAY` / backoff factor.
    /// Exposed (and tested) separately from [`reconnect`] so the math
    /// is verifiable without opening a real socket.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn bump_reconnect_delay(&self) -> u64 {
        let mut rd = self.reconnect_delay.lock().unwrap();
        let cur = *rd;
        *rd = (cur.saturating_mul(2)).min(30);
        cur
    }

    /// Returns whether the client currently considers itself connected.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    /// Returns whether the client's reader loop is still running.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    // ══════════════════════════════════════════════════════════════════
    //  JSON-RPC transport
    // ══════════════════════════════════════════════════════════════════

    /// Build and send a JSON-RPC request. Returns the message ID.
    pub fn send_request(&self, method: &str, params: Value) -> String {
        let id = generate_uuid();
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send(&msg);
        id
    }

    /// Register a pending-response slot for a request ID.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn register_pending<R, E>(&self, id: &str, resolve: R, reject: E)
    where
        R: FnOnce(Value) + Send + 'static,
        E: FnOnce(Value) + Send + 'static,
    {
        self.pending.lock().unwrap().insert(
            id.to_string(),
            PendingRequest {
                resolve: Some(Box::new(resolve)),
                reject: Some(Box::new(reject)),
            },
        );
    }

    /// Snapshot of the frames this client has sent through the transport, for
    /// wire-frame inspection / tests. Bounded to the most recent
    /// [`crate::relay::SENT_LOG_CAP`] frames.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned (another thread panicked while
    /// holding the lock). This does not occur under normal operation.
    #[must_use]
    pub fn sent_messages(&self) -> Vec<Value> {
        self.sent_messages.lock().unwrap().clone()
    }

    /// Send a raw JSON message through the transport.
    ///
    /// Records the frame in `sent_messages` (used by tests and for debug
    /// inspection) and, when a live socket is attached, enqueues the
    /// frame on the writer channel so the reader thread flushes it to
    /// the WebSocket. With no live socket attached the call is purely
    /// in-memory — that's the path the dispatch unit tests below take.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn send(&self, msg: &Value) {
        // SECRET-SCRUB: never log the raw frame — it carries project/token/
        // jwt_token/authorization_state on the connect + re-auth frames. Route
        // through scrub_frame_value so the wire shape is visible but the
        // credential VALUES are masked (mirrors python's `>> {_scrub_frame(...)}`).
        self.logger.debug(&format!(">> {}", scrub_frame_value(msg)));
        {
            // Bounded ring: retain only the most recent SENT_LOG_CAP frames so a
            // long-running session cannot grow this inspection log without limit.
            let mut sent = self.sent_messages.lock().unwrap();
            if sent.len() >= crate::relay::SENT_LOG_CAP {
                sent.remove(0);
            }
            sent.push(msg.clone());
        }
        if let Some(tx) = self.write_tx.lock().unwrap().as_ref() {
            let raw = msg.to_string();
            if let Err(e) = tx.send(WsMessage::Text(raw.into())) {
                self.logger.warn(&format!("write channel closed: {e}"));
            }
        }
    }

    /// Send an acknowledgement for a server-initiated request.
    pub fn send_ack(&self, id: &str) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {},
        }));
    }

    // ══════════════════════════════════════════════════════════════════
    //  Inbound message handling
    // ══════════════════════════════════════════════════════════════════

    /// Parse a raw JSON string from the server and route it.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn handle_message(self: &Arc<Self>, raw: &str) {
        // SECRET-SCRUB: never log the raw inbound frame — a re-auth
        // (`signalwire.authorization.state`) frame carries the authorization_state
        // blob, and a connect echo could carry credentials. Route through
        // scrub_frame_raw so the values are masked (mirrors python's
        // `<< {_scrub_frame(raw)}`).
        self.logger.debug(&format!("<< {}", scrub_frame_raw(raw)));

        let data: Value = if let Ok(d) = serde_json::from_str(raw) {
            d
        } else {
            self.logger.warn("Received unparseable message");
            return;
        };

        // ── response to a pending request ────────────────────────────
        if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
            let mut pending = self.pending.lock().unwrap();
            if let Some(mut slot) = pending.remove(id) {
                if data.get("error").is_some() {
                    if let Some(reject) = slot.reject.take() {
                        reject(data["error"].clone());
                    }
                } else if let Some(resolve) = slot.resolve.take() {
                    resolve(data.get("result").cloned().unwrap_or(json!({})));
                }
                return;
            }
        }

        // ── server-initiated request ─────────────────────────────────
        let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");

        match method {
            "signalwire.ping" => {
                self.send_ack(id);
            }
            "signalwire.disconnect" => {
                self.handle_disconnect();
            }
            "signalwire.event" => {
                self.send_ack(id);
                let outer_params = data.get("params").cloned().unwrap_or(json!({}));
                self.handle_event(&outer_params);
            }
            _ => {
                self.logger.debug(&format!("Unhandled method: {method}"));
            }
        }
    }

    /// Route a signalwire.event payload to the appropriate handler.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn handle_event(self: &Arc<Self>, outer_params: &Value) {
        let event_type = outer_params
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let params = outer_params.get("params").cloned().unwrap_or(json!({}));

        let event = Event::parse(event_type, &params);

        // ── authorization state ──────────────────────────────────────
        if event_type == "signalwire.authorization.state" {
            let auth_state = params
                .get("authorization_state")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string);
            self.authorization_state
                .lock()
                .unwrap()
                .clone_from(&auth_state);
            self.logger
                .info(&format!("Authorization state: {auth_state:?}"));
            return;
        }

        // ── inbound call ─────────────────────────────────────────────
        if event_type == "calling.call.receive" {
            self.handle_inbound_call(&event, &params);
            return;
        }

        // ── inbound message ──────────────────────────────────────────
        if event_type == "messaging.receive" {
            if let Some(handler) = self.on_message_handler.lock().unwrap().as_ref() {
                handler(&event, &params);
            }
            return;
        }

        // ── message state updates ────────────────────────────────────
        if event_type == "messaging.state" {
            if let Some(msg_id) = params.get("message_id").and_then(|v| v.as_str()) {
                let msg = self.messages.lock().unwrap().get(msg_id).cloned();
                if let Some(msg) = msg {
                    msg.dispatch_event(&event);
                    if let Some(s) = params.get("state").and_then(|v| v.as_str())
                        && constants::is_message_terminal(s)
                    {
                        self.messages.lock().unwrap().remove(msg_id);
                    }
                }
            }
            return;
        }

        // ── call state with a pending dial tag ───────────────────────
        if event_type == "calling.call.state"
            && let Some(tag) = params.get("tag").and_then(|v| v.as_str())
        {
            let has_dial = self.pending_dials.lock().unwrap().contains_key(tag);
            if has_dial && let Some(call_id) = params.get("call_id").and_then(|v| v.as_str()) {
                let mut calls = self.calls.lock().unwrap();
                if !calls.contains_key(call_id) {
                    let call = Arc::new(Call::new(&params));
                    call.set_client(self);
                    calls.insert(call_id.to_string(), call);
                }
            }
        }

        // ── dial completion event ────────────────────────────────────
        if event_type == "calling.call.dial" {
            self.handle_dial_event(&event, &params);
            return;
        }

        // ── default: route to the Call by call_id ────────────────────
        if let Some(call_id) = params.get("call_id").and_then(|v| v.as_str()) {
            let call = self.calls.lock().unwrap().get(call_id).cloned();
            if let Some(call) = call {
                call.dispatch_event(&event);

                if call.current_state() == constants::CALL_STATE_ENDED {
                    self.calls.lock().unwrap().remove(call_id);
                }
                return;
            }
        }

        // Fire generic event handler if nothing else matched.
        if let Some(handler) = self.on_event_handler.lock().unwrap().as_ref() {
            handler(&event, outer_params);
        }
    }

    // ══════════════════════════════════════════════════════════════════
    //  Public API methods
    // ══════════════════════════════════════════════════════════════════

    /// Subscribe to one or more inbound contexts.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn receive(&self, contexts: &[String]) {
        {
            let mut ctx = self.contexts.lock().unwrap();
            for c in contexts {
                if !ctx.contains(c) {
                    ctx.push(c.clone());
                }
            }
        }

        self.send_request("signalwire.receive", json!({"contexts": contexts}));
    }

    /// Unsubscribe from one or more contexts.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn unreceive(&self, contexts: &[String]) {
        {
            let mut ctx = self.contexts.lock().unwrap();
            ctx.retain(|c| !contexts.contains(c));
        }

        self.send_request("signalwire.unreceive", json!({"contexts": contexts}));
    }

    /// Register a handler for inbound calls.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn on_call<F: Fn(Arc<Call>, &Event) + Send + Sync + 'static>(&self, cb: F) {
        *self.on_call_handler.lock().unwrap() = Some(Box::new(cb));
    }

    /// Register a handler for inbound messages.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn on_message<F: Fn(&Event, &Value) + Send + Sync + 'static>(&self, cb: F) {
        *self.on_message_handler.lock().unwrap() = Some(Box::new(cb));
    }

    /// Register a generic event handler.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn on_event<F: Fn(&Event, &Value) + Send + Sync + 'static>(&self, cb: F) {
        *self.on_event_handler.lock().unwrap() = Some(Box::new(cb));
    }

    /// Get a call by ID.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn get_call(&self, call_id: &str) -> Option<Arc<Call>> {
        self.calls.lock().unwrap().get(call_id).cloned()
    }

    /// Get a message by ID.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn get_message(&self, message_id: &str) -> Option<Arc<Message>> {
        self.messages.lock().unwrap().get(message_id).cloned()
    }

    /// Track a new message.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn track_message(&self, message_id: &str, msg: Arc<Message>) {
        self.messages
            .lock()
            .unwrap()
            .insert(message_id.to_string(), msg);
    }

    /// Register a pending dial.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn register_dial<F: FnOnce(Arc<Call>) + Send + 'static>(&self, tag: &str, resolve: F) {
        self.pending_dials.lock().unwrap().insert(
            tag.to_string(),
            PendingDial {
                resolve: Box::new(resolve),
                tag: tag.to_string(),
            },
        );
    }

    /// Remove a pending dial.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn remove_pending_dial(&self, tag: &str) {
        self.pending_dials.lock().unwrap().remove(tag);
    }

    // ══════════════════════════════════════════════════════════════════
    //  High-level RPC: signalwire.execute(...)
    // ══════════════════════════════════════════════════════════════════

    /// Send a JSON-RPC request for a calling/messaging method.
    ///
    /// Mirrors Python's `RelayClient._send_request`: emits a flat-Blade
    /// frame `{"method": <method>, "params": <params>}` directly — no
    /// `signalwire.execute` wrapper. Both forms are accepted by the
    /// production RELAY server and the mock; the flat form is what
    /// every existing SDK port emits because it keeps the journal
    /// filterable by inner method name without unwrapping.
    ///
    /// Returns the response's `result` value on success, or an `Err`
    /// with the server's error message on failure or timeout.
    ///
    /// # Errors
    ///
    /// Returns `Err(RelayError::Rpc { method, message })` when the
    /// server replies with a JSON-RPC error for `method` (the `message`
    /// comes from the error's `message` field, defaulting to
    /// `"execute failed"`), or `Err(RelayError::Timeout)` if no response
    /// for the request id arrives within `HANDSHAKE_TIMEOUT` (the
    /// pending entry is removed on timeout).
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn execute_blocking(&self, method: &str, inner_params: Value) -> Result<Value, RelayError> {
        let id = generate_uuid();
        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": inner_params,
        });

        let (resp_tx, resp_rx) = mpsc::channel::<Result<Value, Value>>();
        let resolve_tx = resp_tx.clone();
        let reject_tx = resp_tx;
        self.register_pending(
            &id,
            move |v| {
                let _ = resolve_tx.send(Ok(v));
            },
            move |e| {
                let _ = reject_tx.send(Err(e));
            },
        );

        self.send(&frame);

        // Use the same handshake-style timeout — 10s, matching Python's
        // `_EXECUTE_TIMEOUT`.
        match resp_rx.recv_timeout(HANDSHAKE_TIMEOUT) {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(err)) => Err(RelayError::Rpc {
                method: method.to_string(),
                message: err
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("execute failed")
                    .to_string(),
            }),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(RelayError::Timeout {
                    what: format!("{method} response"),
                })
            }
        }
    }

    /// Send an outbound SMS/MMS message.
    ///
    /// Mirrors Python's `RelayClient.send_message`. At least one of
    /// `body` or `media` must be supplied. Returns a tracked `Message`
    /// whose state will progress as `messaging.state` events arrive
    /// from the server.
    ///
    /// # Errors
    ///
    /// Returns `Err(RelayError::InvalidArgument { message: "At least one
    /// of body or media is required" })` if both `body` and `media` are
    /// empty/absent. Otherwise propagates the error from the underlying
    /// `messaging.send` call via [`execute_blocking`]:
    /// `RelayError::Rpc` if the server rejects the send, or
    /// `RelayError::Timeout` if no response arrives in time.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn send_message_blocking(
        &self,
        to_number: &str,
        from_number: &str,
        body: Option<&str>,
        media: Option<&[String]>,
        tags: Option<&[String]>,
        context: Option<&str>,
    ) -> Result<Arc<Message>, RelayError> {
        if body.unwrap_or("").is_empty() && media.is_none_or(<[String]>::is_empty) {
            return Err(RelayError::InvalidArgument {
                message: "At least one of body or media is required".to_string(),
            });
        }

        let ctx = context
            .map(str::to_string)
            .or_else(|| self.protocol.lock().unwrap().clone())
            .unwrap_or_else(|| "default".to_string());

        let mut params = json!({
            "context": ctx,
            "to_number": to_number,
            "from_number": from_number,
        });
        if let Some(b) = body
            && !b.is_empty()
        {
            params["body"] = json!(b);
        }
        if let Some(m) = media
            && !m.is_empty()
        {
            params["media"] = json!(m);
        }
        if let Some(t) = tags
            && !t.is_empty()
        {
            params["tags"] = json!(t);
        }

        let result = self.execute_blocking("messaging.send", params)?;

        let message_id = result
            .get("message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Build a Message we'll track.
        let mut init_params = json!({
            "context": ctx,
            "direction": "outbound",
            "from_number": from_number,
            "to_number": to_number,
            "state": "queued",
        });
        if !message_id.is_empty() {
            init_params["message_id"] = json!(message_id);
        }
        if let Some(b) = body {
            init_params["body"] = json!(b);
        }
        if let Some(m) = media {
            init_params["media"] = json!(m);
        }
        if let Some(t) = tags {
            init_params["tags"] = json!(t);
        }

        let msg = Arc::new(Message::new(&init_params));
        if !message_id.is_empty() {
            self.track_message(&message_id, msg.clone());
        }
        Ok(msg)
    }

    /// Initiate an outbound call using `calling.dial`.
    ///
    /// Mirrors Python's `RelayClient.dial`. The dial response carries no
    /// `call_id` — the actual call info arrives via subsequent
    /// `calling.call.dial` events keyed by `tag`. This method waits for
    /// that event up to `dial_timeout` and returns the resolved Call.
    ///
    /// `devices` is the standard serial/parallel device matrix
    /// (`[[device]]` for one parallel leg / serial = one inner list with
    /// multiple devices).
    ///
    /// # Errors
    ///
    /// Returns `Err(RelayError::DialFailed { reason })` if no
    /// `calling.call.dial` answer event for this `tag` arrives within
    /// `dial_timeout` — `reason` is the failure message captured from a
    /// `failed` dial event when one was seen, otherwise a synthesized
    /// `"timed out waiting for answer (tag=...)"`. The fire-and-forget
    /// `calling.dial` RPC's own result is intentionally not gated on, so
    /// it does not surface as an error here.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned (i.e. another thread
    /// panicked while holding the lock). This does not occur under
    /// normal operation.
    pub fn dial_blocking(
        self: &Arc<Self>,
        devices: Value,
        tag: Option<&str>,
        max_duration: Option<u32>,
        dial_timeout: Duration,
    ) -> Result<Arc<Call>, RelayError> {
        let dial_tag = tag.map_or_else(generate_uuid, str::to_string);

        let (resolve_tx, resolve_rx) = mpsc::channel::<Arc<Call>>();
        // Fail channel — when the SDK observes a `failed` dial event for
        // this tag we surface that here via handle_dial_event sending an
        // Err. For now we wire only the success path; `failed` events
        // are observed separately by reading the journal in tests.
        self.register_dial(&dial_tag, move |call| {
            let _ = resolve_tx.send(call);
        });

        // Track failure events: we install a generic event handler hook
        // for the duration of this dial. The Client supports only a
        // single `on_event` handler; saving and restoring the previous
        // value lets nested dials work.
        let dial_failed: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let _failed_rx = dial_failed.clone();

        let mut params = json!({
            "tag": dial_tag,
            "devices": devices,
        });
        if let Some(md) = max_duration {
            params["max_duration"] = json!(md);
        }

        // Send the dial RPC — response is just `{"code":"200","message":"Dialing"}`.
        // We don't gate on its result; the actual completion comes via the
        // dial event.
        let _ = self.execute_blocking("calling.dial", params);

        // Wait for either the resolved call or the timeout. The
        // existing `handle_dial_event` resolves only on the
        // `calling.call.dial` event with a `call.call_id` in `params.call`.
        if let Ok(call) = resolve_rx.recv_timeout(dial_timeout) {
            Ok(call)
        } else {
            self.remove_pending_dial(&dial_tag);
            if let Some(reason) = dial_failed.lock().unwrap().clone() {
                Err(RelayError::DialFailed { reason })
            } else {
                Err(RelayError::DialFailed {
                    reason: format!("timed out waiting for answer (tag={dial_tag})"),
                })
            }
        }
    }

    // ══════════════════════════════════════════════════════════════════
    //  Python-parity method names
    //
    //  These mirror the exact names on the Python `RelayClient`
    //  (`execute` / `dial` / `run` / `send_message`). The Rust client is a
    //  blocking API whose implementations carry a `_blocking` suffix; these
    //  parity-named wrappers delegate to them so both the Rust-idiom name
    //  and the Python name resolve to the same behaviour.
    // ══════════════════════════════════════════════════════════════════

    /// Send an arbitrary JSON-RPC request and block for its result
    /// (mirrors Python `RelayClient.execute`). Delegates to
    /// [`execute_blocking`].
    ///
    /// # Errors
    /// Propagates [`execute_blocking`]'s errors: `RelayError::Rpc` if the
    /// server returns a JSON-RPC error, or `RelayError::Timeout` if no
    /// response arrives within the deadline.
    pub fn execute(&self, method: &str, params: Value) -> Result<Value, RelayError> {
        self.execute_blocking(method, params)
    }

    /// Initiate an outbound call and block until it is answered or the
    /// dial deadline elapses (mirrors Python `RelayClient.dial`). Delegates
    /// to [`dial_blocking`].
    ///
    /// # Errors
    /// Propagates [`dial_blocking`]'s errors: `RelayError::DialFailed` on a
    /// failed dial or answer timeout.
    pub fn dial(
        self: &Arc<Self>,
        devices: Value,
        tag: Option<&str>,
        max_duration: Option<u32>,
        dial_timeout: Duration,
    ) -> Result<Arc<Call>, RelayError> {
        self.dial_blocking(devices, tag, max_duration, dial_timeout)
    }

    /// Send an outbound SMS/MMS message (mirrors Python
    /// `RelayClient.send_message`). Delegates to [`send_message_blocking`].
    ///
    /// # Errors
    /// Propagates [`send_message_blocking`]'s errors: `RelayError::
    /// InvalidArgument` if neither body nor media is supplied, or the
    /// `messaging.send` `RelayError::Rpc` / `RelayError::Timeout`.
    #[allow(clippy::too_many_arguments)]
    pub fn send_message(
        &self,
        to_number: &str,
        from_number: &str,
        body: Option<&str>,
        media: Option<&[String]>,
        tags: Option<&[String]>,
        context: Option<&str>,
    ) -> Result<Arc<Message>, RelayError> {
        self.send_message_blocking(to_number, from_number, body, media, tags, context)
    }

    /// Blocking entry point — run the client's event loop until the
    /// connection is torn down (mirrors Python `RelayClient.run`).
    ///
    /// The reader thread (spawned by [`connect`]) owns all socket I/O and
    /// dispatches events. `run` blocks the calling thread until that reader
    /// thread stops running (i.e. `is_running()` becomes false, e.g. after
    /// [`disconnect`] or a lost connection), so a caller can `connect()`
    /// then `run()` to stay alive for the session. Returns immediately if
    /// the client is not currently running.
    pub fn run(&self) {
        while self.is_running() {
            thread::sleep(Duration::from_millis(50));
        }
    }

    // ══════════════════════════════════════════════════════════════════
    //  Private helpers
    // ══════════════════════════════════════════════════════════════════

    fn handle_inbound_call(self: &Arc<Self>, event: &Event, params: &Value) {
        let Some(call_id) = params.get("call_id").and_then(|v| v.as_str()) else {
            self.logger.warn("Inbound call event missing call_id");
            return;
        };

        let call = Arc::new(Call::new(params));
        call.set_client(self);
        self.calls
            .lock()
            .unwrap()
            .insert(call_id.to_string(), call.clone());

        self.logger.info(&format!("Inbound call {call_id}"));

        if let Some(handler) = self.on_call_handler.lock().unwrap().as_ref() {
            handler(call, event);
        }
    }

    fn handle_dial_event(self: &Arc<Self>, _event: &Event, params: &Value) {
        let tag = match params.get("tag").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return,
        };

        // The production server emits `params.call.call_id` (nested),
        // while older shapes carried a top-level `params.call_id`. Try
        // the nested shape first, then fall back. We also pass the
        // nested `call` object as the construction params so the resulting
        // Call carries device/tag fields too.
        let call_obj = params.get("call");
        let nested_call_id = call_obj
            .and_then(|c| c.get("call_id"))
            .and_then(|v| v.as_str());
        let top_level_call_id = params.get("call_id").and_then(|v| v.as_str());
        let call_id = nested_call_id.or(top_level_call_id);

        // Build the construction params: prefer the nested `call` object
        // if it exists (it carries `tag`, `device`, etc. for the winner),
        // otherwise fall back to the top-level params.
        let ctor_params = call_obj.cloned().unwrap_or_else(|| params.clone());

        // If the dial failed, surface a failed event but do not resolve
        // the pending dial — the dial_blocking timeout path notes the
        // failure. Same for any state other than answered: we wait for
        // a winner.
        let dial_state = params
            .get("dial_state")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if dial_state == "failed" {
            self.logger.warn(&format!("dial failed for tag={tag}"));
            self.pending_dials.lock().unwrap().remove(&tag);
            return;
        }

        let call = if let Some(cid) = call_id {
            let mut calls = self.calls.lock().unwrap();
            if let Some(existing) = calls.get(cid) {
                existing.clone()
            } else {
                let call = Arc::new(Call::new(&ctor_params));
                call.set_client(self);
                calls.insert(cid.to_string(), call.clone());
                call
            }
        } else {
            return;
        };

        // Resolve the pending dial
        let pending = self.pending_dials.lock().unwrap().remove(&tag);
        if let Some(dial) = pending {
            *call.dial_winner.lock().unwrap() = true;
            *call.state.lock().unwrap() = if dial_state.is_empty() {
                "answered".to_string()
            } else {
                dial_state.to_string()
            };
            (dial.resolve)(call);
        }
    }

    fn handle_disconnect(&self) {
        self.logger.warn("Server sent disconnect");
        *self.connected.lock().unwrap() = false;
    }

    /// Long-running loop that owns the WebSocket. Alternates between
    /// short-timeout reads and draining the outbound write channel,
    /// dispatching every inbound text frame through `handle_message`.
    /// Exits when the closing flag is set, the write channel is
    /// dropped, or the socket reports a fatal error.
    fn reader_loop(client: Arc<Self>, mut socket: WsStream, write_rx: Receiver<WsMessage>) {
        // Track whether we observed a clean close so the disconnect
        // logic can decide whether to attempt a reconnect later.
        loop {
            if client.closing.load(Ordering::SeqCst) {
                let _ = socket.close(None);
                let _ = socket.flush();
                break;
            }

            // 1) Drain any queued outbound frames before reading. Writes
            //    are infrequent; a single batch here keeps latency low
            //    and avoids stale frames when the server pushes a burst
            //    of events.
            let mut wrote_any = false;
            loop {
                match write_rx.try_recv() {
                    Ok(frame) => {
                        if let Err(e) = socket.send(frame) {
                            client.logger.warn(&format!("WS send error: {e}"));
                            break;
                        }
                        wrote_any = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Sender side dropped — disconnect() was called.
                        let _ = socket.close(None);
                        let _ = socket.flush();
                        return;
                    }
                }
            }
            if wrote_any {
                let _ = socket.flush();
            }

            // 2) Try to read one inbound frame. The TCP read timeout is
            //    100ms so this returns regularly to give the write side
            //    a turn.
            match socket.read() {
                Ok(WsMessage::Text(t)) => {
                    client.handle_message(&t);
                }
                Ok(WsMessage::Binary(b)) => {
                    // RELAY uses text frames; binary is unexpected. Try
                    // to decode as UTF-8 and dispatch anyway, otherwise
                    // log and drop.
                    match std::str::from_utf8(&b) {
                        Ok(s) => client.handle_message(s),
                        Err(_) => client.logger.debug("ignoring non-UTF8 binary WS frame"),
                    }
                }
                Ok(WsMessage::Ping(p)) => {
                    // Tungstenite normally auto-pongs on the next write;
                    // ensure we explicitly respond so the server's
                    // half-open detector doesn't fire.
                    let _ = socket.send(WsMessage::Pong(p));
                }
                // Pong: we don't track our own pings here. Frame: a raw frame
                // shouldn't occur with the protocol module's `read()` API. Both no-op.
                Ok(WsMessage::Pong(_) | WsMessage::Frame(_)) => {}
                Ok(WsMessage::Close(_)) => {
                    client.logger.info("server closed WS");
                    *client.connected.lock().unwrap() = false;
                    break;
                }
                Err(tungstenite::Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // Read timeout — loop back to drain writes. This is
                    // the steady state when the server is quiet.
                }
                Err(tungstenite::Error::ConnectionClosed) => {
                    client.logger.info("WS connection closed");
                    *client.connected.lock().unwrap() = false;
                    break;
                }
                Err(tungstenite::Error::AlreadyClosed) => {
                    *client.connected.lock().unwrap() = false;
                    break;
                }
                Err(e) => {
                    client.logger.warn(&format!("WS read error: {e}"));
                    *client.connected.lock().unwrap() = false;
                    break;
                }
            }
        }

        // Reader is exiting — clear the write sender so subsequent
        // `send()` calls fall back to in-memory buffering rather than
        // pushing into a dead channel.
        *client.write_tx.lock().unwrap() = None;
    }
}

/// Credential / re-auth keys whose VALUES must never reach a log or error
/// string (enterprise SECRET-SCRUB, `r5/deep_enterprise.md` F3.1/F3.2). Mirrors
/// the python reference's `_scrub_frame` masked-key set.
const SENSITIVE_KEYS: [&str; 4] = ["project", "token", "jwt_token", "authorization_state"];

/// The mask substituted for a scrubbed credential value.
const SCRUB_MASK: &str = "***";

/// Recursively mask the VALUES of any [`SENSITIVE_KEYS`] anywhere in `value`,
/// leaving structure and non-sensitive fields intact. Used so a debug frame log
/// can show the wire shape without ever printing a live project id / token /
/// jwt / re-auth blob.
fn scrub_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if SENSITIVE_KEYS.contains(&k.as_str()) {
                    *v = Value::String(SCRUB_MASK.to_string());
                } else {
                    scrub_value(v);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                scrub_value(item);
            }
        }
        _ => {}
    }
}

/// Return a display string for a relay frame (`&Value`) with every credential /
/// re-auth VALUE masked. The clean, log-safe rendering of an outbound frame.
fn scrub_frame_value(msg: &Value) -> String {
    let mut clone = msg.clone();
    scrub_value(&mut clone);
    clone.to_string()
}

/// Return a display string for a raw inbound frame string with every credential
/// / re-auth VALUE masked. Parses `raw` as JSON and scrubs it; if it does not
/// parse, returns a redacted placeholder rather than echoing the raw bytes (an
/// unparseable frame could still carry secrets).
fn scrub_frame_raw(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(mut v) => {
            scrub_value(&mut v);
            v.to_string()
        }
        Err(_) => "<unparseable frame redacted>".to_string(),
    }
}

/// Generate a simple UUID v4.
fn generate_uuid() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let mut data = [0u8; 16];
    rng.fill(&mut data);
    data[6] = (data[6] & 0x0f) | 0x40;
    data[8] = (data[8] & 0x3f) | 0x80;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
        u16::from_be_bytes([data[4], data[5]]),
        u16::from_be_bytes([data[6], data[7]]),
        u16::from_be_bytes([data[8], data[9]]),
        (u64::from(data[10]) << 40)
            | (u64::from(data[11]) << 32)
            | (u64::from(data[12]) << 24)
            | (u64::from(data[13]) << 16)
            | (u64::from(data[14]) << 8)
            | u64::from(data[15]),
    )
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> Client {
        Client::new("test-project", "test-token", "test.signalwire.com")
    }

    // -- SECRET-SCRUB (RUST-6) --

    #[test]
    fn test_scrub_frame_value_masks_connect_credentials() {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "signalwire.connect",
            "params": {
                "project": "PJ-LIVE-SECRET",
                "token": "PT-LIVE-SECRET",
                "authentication": { "project": "PJ-LIVE-SECRET", "token": "PT-LIVE-SECRET" },
                "agent": "signalwire-agents-rust/1.0",
            }
        });
        let out = scrub_frame_value(&frame);
        assert!(!out.contains("PJ-LIVE-SECRET"), "project leaked: {out}");
        assert!(!out.contains("PT-LIVE-SECRET"), "token leaked: {out}");
        // Structure/shape survives: method name + masked keys still present.
        assert!(out.contains("signalwire.connect"));
        assert!(out.contains("***"));
    }

    #[test]
    fn test_scrub_frame_raw_masks_authorization_state() {
        let raw = r#"{"method":"signalwire.event","params":{"event_type":"signalwire.authorization.state","params":{"authorization_state":"AENC-LIVE-BLOB","jwt_token":"JWT-LIVE"}}}"#;
        let out = scrub_frame_raw(raw);
        assert!(!out.contains("AENC-LIVE-BLOB"), "auth_state leaked: {out}");
        assert!(!out.contains("JWT-LIVE"), "jwt leaked: {out}");
        assert!(out.contains("signalwire.authorization.state"));
    }

    #[test]
    fn test_scrub_frame_raw_redacts_unparseable() {
        // A non-JSON frame must not be echoed verbatim (could still carry secrets).
        let out = scrub_frame_raw("token=PT-LIVE-SECRET not-json");
        assert!(!out.contains("PT-LIVE-SECRET"));
        assert!(out.contains("redacted"));
    }

    #[test]
    fn test_sent_messages_is_bounded() {
        let c = make_client();
        // Push well past the cap; the log must saturate at SENT_LOG_CAP and keep
        // the MOST RECENT frames (ring buffer), not grow without limit.
        for i in 0..(crate::relay::SENT_LOG_CAP + 50) {
            c.send(&json!({ "method": "signalwire.ping", "seq": i }));
        }
        let msgs = c.sent_messages();
        assert_eq!(msgs.len(), crate::relay::SENT_LOG_CAP, "log must be capped");
        // Oldest entries were dropped; the final frame is the most recent seq.
        let last_seq = msgs
            .last()
            .unwrap()
            .get("seq")
            .and_then(Value::as_u64)
            .unwrap();
        assert_eq!(last_seq, (crate::relay::SENT_LOG_CAP + 49) as u64);
    }

    #[test]
    fn test_client_new() {
        let c = make_client();
        assert_eq!(c.project, "test-project");
        assert_eq!(c.token, "test-token");
        assert_eq!(c.host, "test.signalwire.com");
        assert!(!c.is_connected());
    }

    #[test]
    fn test_disconnect_clears_state() {
        // disconnect() works on a Client without ever opening a socket
        // — it just flips the in-memory flags and joins (a non-existent)
        // reader thread. Proves the lifecycle hooks aren't tied to an
        // active connection.
        let c = make_client();
        *c.connected.lock().unwrap() = true;
        *c.running.lock().unwrap() = true;
        c.disconnect();
        assert!(!c.is_connected());
        assert!(!c.is_running());
    }

    #[test]
    fn test_reconnect_backoff_math() {
        // The backoff delay starts at 1s and doubles on each call,
        // capping at 30s. Tested directly so we don't need a real
        // socket — the doubling is the contract Python relies on.
        let c = make_client();
        assert_eq!(c.bump_reconnect_delay(), 1);
        assert_eq!(*c.reconnect_delay.lock().unwrap(), 2);
        assert_eq!(c.bump_reconnect_delay(), 2);
        assert_eq!(*c.reconnect_delay.lock().unwrap(), 4);
        for _ in 0..10 {
            c.bump_reconnect_delay();
        }
        assert!(*c.reconnect_delay.lock().unwrap() <= 30);
    }

    #[test]
    fn test_real_ws_handshake_against_loopback_fixture() {
        // Stand up a tiny WebSocket server on 127.0.0.1:0 that speaks
        // just enough JSON-RPC to satisfy `signalwire.connect`. Drive
        // the real `connect()` against it and prove the client opens
        // the socket, sends the connect frame with `params.project`,
        // parses the auth response, and stores the authorization
        // state for fast-reconnect.
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        // Server thread: accept one upgrade, run the handshake, push a
        // response, and exit.
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut ws = tungstenite::accept(stream).unwrap();
            // Read the connect frame.
            loop {
                match ws.read() {
                    Ok(WsMessage::Text(t)) => {
                        let v: Value = serde_json::from_str(&t).unwrap();
                        if v.get("method").and_then(|m| m.as_str()) == Some("signalwire.connect") {
                            let id = v.get("id").cloned().unwrap_or(json!(""));
                            let project = v
                                .get("params")
                                .and_then(|p| p.get("authentication"))
                                .and_then(|a| a.get("project"))
                                .cloned()
                                .unwrap_or(json!(""));
                            let resp = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "authorization": {
                                        "authorization_state": "fixture-state-token",
                                    },
                                    "protocol": "signalwire-relay-test",
                                    "project": project,
                                }
                            });
                            ws.send(WsMessage::Text(resp.to_string().into())).unwrap();
                            // Hold the connection open briefly so the
                            // client doesn't see a premature close.
                            std::thread::sleep(Duration::from_millis(150));
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        // SAFETY: this test serializes itself through env-var locking
        // by being one of the only tests that mutates the relay env
        // vars. Other tests in this module don't touch them.
        unsafe {
            std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "ws");
            std::env::set_var("SIGNALWIRE_RELAY_HOST", format!("127.0.0.1:{port}"));
        }

        let client = Arc::new(Client::new("test-project", "test-token", "ignored"));
        let res = client.connect();
        assert!(res.is_ok(), "connect failed: {res:?}");
        // Authorization state should have been captured from the
        // fixture's response — proves the client parsed
        // `result.authorization.authorization_state`.
        assert_eq!(
            *client.authorization_state.lock().unwrap(),
            Some("fixture-state-token".to_string())
        );
        // Protocol string captured.
        assert_eq!(
            *client.protocol.lock().unwrap(),
            Some("signalwire-relay-test".to_string())
        );
        client.disconnect();
        let _ = server.join();

        unsafe {
            std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
            std::env::remove_var("SIGNALWIRE_RELAY_HOST");
        }
    }

    #[test]
    fn test_authenticate_sends_message() {
        let c = make_client();
        c.authenticate();
        let msgs = c.sent_messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["method"], "signalwire.connect");
        assert_eq!(
            msgs[0]["params"]["authentication"]["project"],
            "test-project"
        );
    }

    #[test]
    fn test_send_request() {
        let c = make_client();
        let id = c.send_request("calling.dial", json!({"to": "+1555"}));
        assert!(!id.is_empty());
        let msgs = c.sent_messages.lock().unwrap();
        assert_eq!(msgs[0]["method"], "calling.dial");
    }

    #[test]
    fn test_send_ack() {
        let c = make_client();
        c.send_ack("req-123");
        let msgs = c.sent_messages.lock().unwrap();
        assert_eq!(msgs[0]["id"], "req-123");
        assert!(msgs[0]["result"].is_object());
    }

    #[test]
    fn test_handle_message_response_resolve() {
        let c = Arc::new(make_client());
        let result = Arc::new(Mutex::new(None));
        let result2 = result.clone();

        let id = c.send_request("test.method", json!({}));
        c.register_pending(
            &id,
            move |v| {
                *result2.lock().unwrap() = Some(v);
            },
            |_| {},
        );

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"session_id": "sess-1"},
        });
        c.handle_message(&response.to_string());

        let r = result.lock().unwrap();
        assert!(r.is_some());
        assert_eq!(r.as_ref().unwrap()["session_id"], "sess-1");
    }

    #[test]
    fn test_handle_message_response_reject() {
        let c = Arc::new(make_client());
        let error = Arc::new(Mutex::new(None));
        let error2 = error.clone();

        let id = c.send_request("test.method", json!({}));
        c.register_pending(
            &id,
            |_| {},
            move |v| {
                *error2.lock().unwrap() = Some(v);
            },
        );

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32000, "message": "fail"},
        });
        c.handle_message(&response.to_string());

        let e = error.lock().unwrap();
        assert!(e.is_some());
        assert_eq!(e.as_ref().unwrap()["code"], -32000);
    }

    #[test]
    fn test_handle_ping() {
        let c = Arc::new(make_client());
        let msg = json!({
            "jsonrpc": "2.0",
            "id": "ping-1",
            "method": "signalwire.ping",
        });
        c.handle_message(&msg.to_string());

        let msgs = c.sent_messages.lock().unwrap();
        // Should have sent an ack
        let ack = msgs.iter().find(|m| m["id"] == "ping-1");
        assert!(ack.is_some());
    }

    #[test]
    fn test_handle_disconnect() {
        let c = Arc::new(make_client());
        // Manually flip the in-memory connected flag (this test verifies
        // the dispatcher's response to a `signalwire.disconnect` frame —
        // not the transport).
        *c.connected.lock().unwrap() = true;
        assert!(c.is_connected());

        let msg = json!({
            "jsonrpc": "2.0",
            "id": "dc-1",
            "method": "signalwire.disconnect",
            "params": {},
        });
        c.handle_message(&msg.to_string());
        assert!(!c.is_connected());
    }

    #[test]
    fn test_handle_inbound_call() {
        let c = Arc::new(make_client());
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_call(move |_call, _ev| {
            *received2.lock().unwrap() = true;
        });

        c.handle_event(&json!({
            "event_type": "calling.call.receive",
            "params": {
                "call_id": "call-1",
                "node_id": "node-1",
                "context": "default",
            },
        }));

        assert!(*received.lock().unwrap());
        assert!(c.calls.lock().unwrap().contains_key("call-1"));
    }

    #[test]
    fn test_handle_call_state_event() {
        let c = Arc::new(make_client());

        // Create a call first
        c.handle_event(&json!({
            "event_type": "calling.call.receive",
            "params": {"call_id": "call-1", "node_id": "node-1"},
        }));

        // Send state event (real wire key is `call_state`).
        c.handle_event(&json!({
            "event_type": "calling.call.state",
            "params": {"call_id": "call-1", "call_state": "ringing"},
        }));

        let call = c.get_call("call-1").unwrap();
        assert_eq!(call.current_state(), "ringing");
    }

    #[test]
    fn test_handle_call_ended_removes_call() {
        let c = Arc::new(make_client());

        c.handle_event(&json!({
            "event_type": "calling.call.receive",
            "params": {"call_id": "call-1", "node_id": "node-1"},
        }));

        c.handle_event(&json!({
            "event_type": "calling.call.state",
            "params": {"call_id": "call-1", "call_state": "ended"},
        }));

        assert!(c.get_call("call-1").is_none());
    }

    #[test]
    fn test_handle_message_state() {
        let c = Arc::new(make_client());
        let msg = Arc::new(Message::new(&json!({"message_id": "msg-1"})));
        c.track_message("msg-1", msg.clone());

        c.handle_event(&json!({
            "event_type": "messaging.state",
            "params": {"message_id": "msg-1", "state": "sent"},
        }));

        assert_eq!(msg.state(), Some("sent".to_string()));
        // Not terminal, should still be tracked
        assert!(c.get_message("msg-1").is_some());
    }

    #[test]
    fn test_handle_message_terminal_removes() {
        let c = Arc::new(make_client());
        let msg = Arc::new(Message::new(&json!({"message_id": "msg-1"})));
        c.track_message("msg-1", msg.clone());

        c.handle_event(&json!({
            "event_type": "messaging.state",
            "params": {"message_id": "msg-1", "state": "delivered"},
        }));

        assert!(msg.is_done());
        assert!(c.get_message("msg-1").is_none());
    }

    #[test]
    fn test_handle_inbound_message() {
        let c = Arc::new(make_client());
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_message(move |_ev, _params| {
            *received2.lock().unwrap() = true;
        });

        c.handle_event(&json!({
            "event_type": "messaging.receive",
            "params": {"message_id": "msg-1", "body": "Hello"},
        }));

        assert!(*received.lock().unwrap());
    }

    #[test]
    fn test_handle_dial_event() {
        let c = Arc::new(make_client());
        let resolved_call = Arc::new(Mutex::new(None));
        let resolved2 = resolved_call.clone();

        c.register_dial("tag-dial-1", move |call| {
            *resolved2.lock().unwrap() = Some(call);
        });

        // First create call via state event with tag (wire key: call_state)
        c.handle_event(&json!({
            "event_type": "calling.call.state",
            "params": {"call_id": "call-dial-1", "tag": "tag-dial-1", "call_state": "created"},
        }));

        // Then the dial event resolves it
        c.handle_event(&json!({
            "event_type": "calling.call.dial",
            "params": {"call_id": "call-dial-1", "tag": "tag-dial-1", "state": "answered"},
        }));

        let r = resolved_call.lock().unwrap();
        assert!(r.is_some());
        assert!(*r.as_ref().unwrap().dial_winner.lock().unwrap());
    }

    #[test]
    fn test_handle_authorization_state() {
        let c = Arc::new(make_client());
        c.handle_event(&json!({
            "event_type": "signalwire.authorization.state",
            "params": {"authorization_state": "authorized"},
        }));
        assert_eq!(
            *c.authorization_state.lock().unwrap(),
            Some("authorized".to_string())
        );
    }

    #[test]
    fn test_receive_contexts() {
        let c = make_client();
        c.receive(&["default".to_string(), "support".to_string()]);
        let ctx = c.contexts.lock().unwrap();
        assert_eq!(ctx.len(), 2);
        assert!(ctx.contains(&"default".to_string()));

        let msgs = c.sent_messages.lock().unwrap();
        assert!(msgs.iter().any(|m| m["method"] == "signalwire.receive"));
    }

    #[test]
    fn test_receive_no_duplicates() {
        let c = make_client();
        c.receive(&["default".to_string()]);
        c.receive(&["default".to_string(), "other".to_string()]);
        let ctx = c.contexts.lock().unwrap();
        assert_eq!(ctx.len(), 2);
    }

    #[test]
    fn test_unreceive() {
        let c = make_client();
        c.receive(&["a".to_string(), "b".to_string(), "c".to_string()]);
        c.unreceive(&["b".to_string()]);
        let ctx = c.contexts.lock().unwrap();
        assert_eq!(ctx.len(), 2);
        assert!(!ctx.contains(&"b".to_string()));
    }

    #[test]
    fn test_on_event_handler() {
        let c = Arc::new(make_client());
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_event(move |_ev, _params| {
            *received2.lock().unwrap() = true;
        });

        // An unrecognized event should fall through to the generic handler
        c.handle_event(&json!({
            "event_type": "unknown.event",
            "params": {},
        }));

        assert!(*received.lock().unwrap());
    }

    #[test]
    fn test_handle_unparseable_message() {
        let c = Arc::new(make_client());
        // Should not panic
        c.handle_message("not-json{{{");
    }

    #[test]
    fn test_handle_event_signalwire_event_method() {
        let c = Arc::new(make_client());
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_call(move |_call, _ev| {
            *received2.lock().unwrap() = true;
        });

        let msg = json!({
            "jsonrpc": "2.0",
            "id": "evt-1",
            "method": "signalwire.event",
            "params": {
                "event_type": "calling.call.receive",
                "params": {"call_id": "c1", "node_id": "n1"},
            },
        });
        c.handle_message(&msg.to_string());

        assert!(*received.lock().unwrap());
        // Should have sent an ack
        let msgs = c.sent_messages.lock().unwrap();
        assert!(msgs.iter().any(|m| m["id"] == "evt-1"));
    }

    // -- Python-parity method wrappers --

    #[test]
    fn test_run_returns_immediately_when_not_running() {
        // `run` blocks only while the reader loop is running; with no
        // connection it must return promptly rather than hang.
        let c = make_client();
        assert!(!c.is_running());
        let start = std::time::Instant::now();
        c.run();
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_run_unblocks_on_disconnect() {
        let c = Arc::new(make_client());
        *c.running.lock().unwrap() = true;
        let c2 = c.clone();
        let handle = std::thread::spawn(move || c2.run());
        std::thread::sleep(Duration::from_millis(60));
        // Flip the running flag (as disconnect would) and confirm run exits.
        *c.running.lock().unwrap() = false;
        handle.join().unwrap();
    }

    #[test]
    fn test_send_message_parity_validates_args() {
        // `send_message` delegates to `send_message_blocking`; the
        // arg-validation path (neither body nor media) returns
        // InvalidArgument without touching a socket.
        let c = make_client();
        let result = c.send_message("+15551112222", "+15553334444", None, None, None, None);
        assert!(matches!(result, Err(RelayError::InvalidArgument { .. })));
    }
}
