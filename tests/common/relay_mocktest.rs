// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Rust test helper for the porting-sdk `mock_relay` WebSocket server.
//!
//! Mirrors the Python conftest fixture (`signalwire_relay_client` /
//! `mock_relay`), the TypeScript port's `tests/relay/mocktest.ts`, and the
//! REST mocktest pattern at `tests/common/mocktest.rs`. The mock server is
//! probed/spawned on first call. It talks to a WebSocket plane on
//! `ws://127.0.0.1:<ws_port>` (default 8781) and an HTTP control plane on
//! port `ws_port + 1000` (default 9781).
//!
//! The Rust slot in the parallel rollout is `WS=8781 / HTTP=9781`. Override
//! either with `MOCK_RELAY_PORT` (WS) or `MOCK_RELAY_HTTP_PORT` (HTTP).
//!
//! ## Session isolation (parallel-safe)
//!
//! Tests run in parallel (cargo's default — one thread per test, and each
//! integration-test binary is its own process). They all share the one
//! `mock_relay` server, so global journal/scenario reads would race. We
//! isolate exactly as the frozen TypeScript design does: the key is the
//! server-assigned handshake `sessionid`.
//!
//! [`connected_client`] captures the connected client's `session_id` (the
//! `sessionid` the mock returned on the `signalwire.connect` handshake) and
//! stashes it in a **thread-local** scope. Because cargo runs each test on
//! its own thread, the thread-local IS the per-test session scope — the Rust
//! analogue of the TS port's per-call scoped `MockRelayHarness`. Every
//! control-plane helper then threads `?session_id=<id>` onto its request
//! (journal read/reset, scenario reset/arm/dial, push, `inbound_call`, and
//! per-op stamping for `scenario_play`), so a test only ever sees / disturbs
//! its own session's frames. A brand-new session starts with an empty
//! (scoped) journal, so no global reset is needed.
//!
//! Tests that build a `RelayClient` by hand call [`scope_to_client`] after
//! `connect()` to bind the thread-local scope to that client's session
//! (mirrors TS `sessionIdOf` + `mock.sessionId = ...`).
//!
//! All functions panic on error.

#![allow(dead_code)]
// Helper signatures take `Value` by value to mirror the cross-port mock-test
// helper contract (the Python `mock_relay` helpers pass frames/payloads by
// value). Keeping the by-value shape keeps these helpers 1:1 with siblings.
#![allow(clippy::needless_pass_by_value)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Once, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use signalwire::relay::Client as RelayClient;

/// Default WebSocket port for the Rust slot in the parallel rollout.
pub const DEFAULT_WS_PORT: u16 = 8781;

/// Default HTTP control-plane port (WS + 1000).
pub const DEFAULT_HTTP_PORT: u16 = 9781;

/// Maximum wait for the spawned mock server to answer `/__mock__/health`.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

thread_local! {
    /// The current test's session scope — the handshake `sessionid` of the
    /// client this test connected. Set by [`connected_client`] /
    /// [`scope_to_client`]; read by every control-plane helper to thread
    /// `?session_id=<id>` onto its request. `None` => unscoped (legacy global
    /// view; only correct under serial execution). Thread-local because cargo
    /// runs each test on its own thread, so this is exactly the per-test scope.
    static SESSION_SCOPE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Current thread-local session scope, if any.
fn current_scope() -> Option<String> {
    SESSION_SCOPE.with(|s| s.borrow().clone())
}

/// `?session_id=<id>` suffix for control-plane calls when scoped, else "".
fn session_query() -> String {
    match current_scope() {
        Some(sid) if !sid.is_empty() => format!("?session_id={}", urlencode(&sid)),
        _ => String::new(),
    }
}

/// Minimal percent-encoding for a session id used as a query value. Session
/// ids are UUID hex (`[0-9a-f]`), so in practice no byte needs escaping; we
/// still encode defensively to stay correct if the mock ever changes the
/// id shape.
fn urlencode(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// One-shot health probe / spawn lock.
static SERVER: OnceLock<HarnessHandle> = OnceLock::new();

/// Set the test-redirect env vars (`SIGNALWIRE_RELAY_SCHEME=ws` +
/// `SIGNALWIRE_RELAY_HOST=<host:port>`) exactly once per process. These are
/// the public test hook `relay::Client::connect()` reads to dial the mock
/// instead of the real WSS endpoint (mirrors Python's audit fixture). Every
/// test sets them to the SAME values, so a one-time idempotent set is
/// parallel-safe — no per-test serialization required.
static REDIRECT_ENV: Once = Once::new();

fn ensure_redirect_env(relay_host: &str) {
    REDIRECT_ENV.call_once(|| {
        // SAFETY: run exactly once, before any test thread connects; the
        // value is a process-wide constant for the test run.
        unsafe {
            std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "ws");
            std::env::set_var("SIGNALWIRE_RELAY_HOST", relay_host);
        }
    });
}

/// Set the `SIGNALWIRE_RELAY_SCHEME=ws` + `SIGNALWIRE_RELAY_HOST=<mock>`
/// redirect env vars (idempotent, once per process). Tests that build a
/// `RelayClient` by hand (rather than via [`connected_client`]) call this so
/// their `connect()` dials the mock. Every test uses the same values, so this
/// is parallel-safe and the vars must NEVER be removed mid-run.
pub fn ensure_redirect() {
    let h = harness();
    ensure_redirect_env(&h.relay_host);
}

/// The current mock-relay harness — its WebSocket URL, HTTP control-plane
/// URL, and the host:port (sans scheme) tests pass to `RelayClient::new`.
#[derive(Clone, Debug)]
pub struct HarnessHandle {
    pub ws_url: String,
    pub http_url: String,
    pub relay_host: String,
    pub ws_port: u16,
    pub http_port: u16,
}

/// A single recorded WebSocket frame as journaled by the mock server.
///
/// Mirrors the Python `_RelayJournalEntry` shape but is decoupled from the
/// upstream type so this harness doesn't depend on upstream private modules.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub timestamp: f64,
    /// Either `"recv"` (SDK→server) or `"send"` (server→SDK).
    pub direction: String,
    /// Inner JSON-RPC method (e.g. `"signalwire.connect"`, `"calling.dial"`).
    pub method: String,
    pub request_id: String,
    /// Full JSON-RPC frame as recorded.
    pub frame: Value,
    pub connection_id: String,
    pub session_id: String,
}

impl JournalEntry {
    /// `frame.params` if present; otherwise [`Value::Null`].
    ///
    /// For flat-Blade frames (`{"method": "messaging.send", "params": {...}}`),
    /// this returns the inner method's parameters directly. For
    /// `signalwire.event` frames, it returns the outer envelope —
    /// callers wanting the event payload should use [`Self::event_params`].
    pub fn frame_params(&self) -> &Value {
        self.frame.get("params").unwrap_or(&Value::Null)
    }

    /// Inner parameters for the request / event regardless of whether
    /// the SDK used the flat-Blade form or the wrapped
    /// `signalwire.execute` / `signalwire.event` shape.
    ///
    /// * Flat Blade (`{"method":"messaging.send","params": <inner>}`) →
    ///   returns `<inner>`.
    /// * `signalwire.execute` wrap (`{"method":"signalwire.execute","params":{"params": <inner>}}`)
    ///   → returns `<inner>`.
    /// * `signalwire.event` (`{"method":"signalwire.event","params":{"params": <inner>}}`)
    ///   → returns `<inner>` (the inner event params).
    pub fn inner_params(&self) -> &Value {
        let p1 = self.frame.get("params").unwrap_or(&Value::Null);
        // If params has a `params` field, use it (wrapped shape). Otherwise
        // params is the inner payload itself.
        if let Some(inner) = p1.get("params") {
            inner
        } else {
            p1
        }
    }

    /// For `signalwire.event` frames, the inner event payload (i.e.
    /// `frame.params.params`). Returns null for non-event frames.
    pub fn event_params(&self) -> &Value {
        if self.frame.get("method").and_then(Value::as_str) != Some("signalwire.event") {
            return &Value::Null;
        }
        self.frame
            .get("params")
            .and_then(|p| p.get("params"))
            .unwrap_or(&Value::Null)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the singleton harness, spawning the mock server if necessary.
pub fn harness() -> HarnessHandle {
    SERVER
        .get_or_init(|| ensure_server().expect("relay_mocktest: failed to ensure mock_relay"))
        .clone()
}

/// Bind the current thread's session scope to a connected client's session.
///
/// Reads the `sessionid` the client captured from its `signalwire.connect`
/// handshake (`client.session_id`) and stashes it in the thread-local scope
/// so subsequent journal reads/resets and scenario/push helpers target only
/// this client's session. Mirrors the TS port's `sessionIdOf(client)` +
/// `mock.sessionId = ...`. Panics if the client has no session id (i.e. it
/// was never connected, or the mock didn't return a `sessionid`).
pub fn scope_to_client(client: &Arc<RelayClient>) {
    let sid = client
        .session_id
        .lock()
        .unwrap()
        .clone()
        .expect("relay_mocktest: client has no session_id - was it connected?");
    set_scope(Some(sid));
}

/// Set (or clear) the current thread's session scope directly.
pub fn set_scope(sid: Option<String>) {
    SESSION_SCOPE.with(|s| *s.borrow_mut() = sid);
}

/// The current thread's session scope, if any (the active client's `sessionid`).
pub fn scope() -> Option<String> {
    current_scope()
}

/// Convenience: build a connected `relay::Client` pointed at the mock and
/// scope this test's thread to its session.
///
/// Sets `SIGNALWIRE_RELAY_SCHEME=ws` and `SIGNALWIRE_RELAY_HOST=<host:port>`
/// (once per process; every test uses the same values) so the Client's
/// `connect()` dials the mock instead of the real WSS endpoint, runs the
/// connect handshake, then binds the thread-local session scope to the new
/// connection. Returns the connected `Arc<Client>` ready for
/// `.send_message(...)`, `.dial(...)`, etc.
///
/// The returned client should be `disconnect()`ed before the test ends.
pub fn connected_client(contexts: &[&str]) -> Arc<RelayClient> {
    let h = harness();
    ensure_redirect_env(&h.relay_host);
    let client = Arc::new(RelayClient::new("test_proj", "test_tok", &h.relay_host));
    // Pre-populate contexts before connect so they go out on the connect frame.
    {
        let mut ctx = client.contexts.lock().unwrap();
        for c in contexts {
            ctx.push((*c).to_string());
        }
    }
    client.connect().expect("relay_mocktest: connect");
    scope_to_client(&client);
    client
}

/// Return every journal entry recorded for this test's session since connect,
/// in arrival order (scoped to the thread-local session when set; global
/// otherwise).
pub fn journal_all() -> Vec<JournalEntry> {
    let h = harness();
    let url = format!("{}/__mock__/journal{}", h.http_url, session_query());
    let mut resp = ureq::get(&url)
        .call()
        .unwrap_or_else(|e| panic!("relay_mocktest: GET /__mock__/journal: {e}"));
    let body: Value = resp
        .body_mut()
        .read_json()
        .unwrap_or_else(|e| panic!("relay_mocktest: decode journal: {e}"));
    decode_journal(&body)
}

/// Return inbound (SDK→server) journal entries, optionally by method.
pub fn journal_recv(method: Option<&str>) -> Vec<JournalEntry> {
    journal_all()
        .into_iter()
        .filter(|e| e.direction == "recv")
        .filter(|e| match method {
            Some(m) => e.method == m,
            None => true,
        })
        .collect()
}

/// Return outbound (server→SDK) journal entries, optionally by inner
/// `event_type` (under `frame.params.event_type` for `signalwire.event`
/// frames).
pub fn journal_send(event_type: Option<&str>) -> Vec<JournalEntry> {
    journal_all()
        .into_iter()
        .filter(|e| e.direction == "send")
        .filter(|e| {
            let Some(want) = event_type else { return true };
            // Only signalwire.event carries an event_type.
            if e.frame.get("method").and_then(Value::as_str) != Some("signalwire.event") {
                return false;
            }
            e.frame
                .get("params")
                .and_then(|p| p.get("event_type"))
                .and_then(Value::as_str)
                == Some(want)
        })
        .collect()
}

/// Return the most-recent journal entry. Panics if the journal is empty.
pub fn journal_last() -> JournalEntry {
    let entries = journal_all();
    entries
        .into_iter()
        .last()
        .expect("relay_mocktest: journal is empty - the SDK call did not reach the mock server")
}

/// Clear this session's journal entries (scoped when the thread-local session
/// is set; global otherwise).
pub fn journal_reset() {
    let h = harness();
    let url = format!("{}/__mock__/journal/reset{}", h.http_url, session_query());
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("relay_mocktest: POST journal/reset: {e}"));
}

/// Drain this session's queued scenarios (scoped when set; global otherwise).
pub fn scenario_reset() {
    let h = harness();
    let url = format!("{}/__mock__/scenarios/reset{}", h.http_url, session_query());
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("relay_mocktest: POST scenarios/reset: {e}"));
}

/// Reset both journal and scenarios for this session — typical fixture entry
/// point. When scoped, this clears only this session's state; a brand-new
/// session already starts empty, so calling it is harmless either way.
pub fn reset_all() {
    journal_reset();
    scenario_reset();
}

/// Queue scripted post-RPC events for a method (FIFO consume-once), scoped to
/// this session.
///
/// `events` is a list of `{"emit": {...}, "delay_ms": N, "event_type": "..."}`
/// entries; `event_type` defaults to a derivation from `method`
/// (`calling.play` → `calling.call.play`).
pub fn arm_method(method: &str, events: Value) {
    let h = harness();
    let url = format!(
        "{}/__mock__/scenarios/{}{}",
        h.http_url,
        method,
        session_query()
    );
    let _ = ureq::post(&url)
        .send_json(&events)
        .unwrap_or_else(|e| panic!("relay_mocktest: arm_method: {e}"));
}

/// Queue a dial-dance scenario, scoped to this session.
pub fn arm_dial(payload: Value) {
    let h = harness();
    let url = format!("{}/__mock__/scenarios/dial{}", h.http_url, session_query());
    let _ = ureq::post(&url)
        .send_json(&payload)
        .unwrap_or_else(|e| panic!("relay_mocktest: arm_dial: {e}"));
}

/// Push a single frame to this test's session (so a parallel test's client
/// never receives it). An unscoped harness broadcasts (legacy behavior).
pub fn push(frame: Value) {
    let h = harness();
    let url = format!("{}/__mock__/push{}", h.http_url, session_query());
    let body = json!({"frame": frame});
    let _ = ureq::post(&url)
        .send_json(&body)
        .unwrap_or_else(|e| panic!("relay_mocktest: push: {e}"));
}

/// Convenience wrapper around `/__mock__/inbound_call`. Spec is the same
/// as the Python `mock_relay.inbound_call(...)` helper. Targets this test's
/// session by default (stamps `session_id` into the body unless the caller
/// already supplied one), so the inbound-call sequence is delivered only to
/// this test's client.
pub fn inbound_call(payload: Value) {
    let h = harness();
    let mut body = payload;
    if let (Some(sid), Some(obj)) = (current_scope(), body.as_object_mut())
        && !sid.is_empty()
        && !obj.contains_key("session_id")
    {
        obj.insert("session_id".to_string(), Value::String(sid));
    }
    let url = format!("{}/__mock__/inbound_call", h.http_url);
    let _ = ureq::post(&url)
        .send_json(&body)
        .unwrap_or_else(|e| panic!("relay_mocktest: inbound_call: {e}"));
}

/// Run a scripted timeline (`scenario_play`). When this test is session-scoped,
/// every `push` / `expect_recv` op is stamped with this session id (unless it
/// already carries one), so the timeline targets only this test's client and
/// `expect_recv` matches only this session's frames — making it parallel-safe.
pub fn scenario_play(ops: Value) -> Value {
    let h = harness();
    let scoped = match current_scope() {
        Some(sid) if !sid.is_empty() => scope_ops(ops, &sid),
        _ => ops,
    };
    let url = format!("{}/__mock__/scenario_play", h.http_url);
    let mut resp = ureq::post(&url)
        .send_json(&scoped)
        .unwrap_or_else(|e| panic!("relay_mocktest: scenario_play: {e}"));
    resp.body_mut()
        .read_json::<Value>()
        .unwrap_or_else(|e| panic!("relay_mocktest: decode scenario_play: {e}"))
}

/// Stamp `session_id` into each timeline op's `push` / `expect_recv` spec when
/// the op doesn't already specify one. Leaves `sleep` ops untouched. Mirrors
/// the TS port's `scopeOp`.
fn scope_ops(ops: Value, sid: &str) -> Value {
    let Some(arr) = ops.as_array() else {
        return ops;
    };
    let stamped: Vec<Value> = arr
        .iter()
        .map(|op| {
            let mut op = op.clone();
            if let Some(obj) = op.as_object_mut() {
                for key in ["push", "expect_recv"] {
                    if let Some(spec) = obj.get_mut(key)
                        && let Some(spec_obj) = spec.as_object_mut()
                        && !spec_obj.contains_key("session_id")
                    {
                        spec_obj.insert("session_id".to_string(), Value::String(sid.to_string()));
                    }
                }
            }
            op
        })
        .collect();
    Value::Array(stamped)
}

/// RAII guard returned by [`begin`]. Clears this thread's session scope on
/// drop so a same-thread follow-up test (rare) starts unscoped until it calls
/// [`connected_client`] / [`scope_to_client`] again.
pub struct TestGuard {
    _private: (),
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        set_scope(None);
    }
}

/// Per-test entry point. Resets this thread's session scope so a test starts
/// clean, then ensures the mock server is up. No global lock and no global
/// journal/scenario reset: session isolation (set by [`connected_client`])
/// makes the shared mock safe under parallel tests, and a brand-new session
/// starts with an empty scoped journal. Bind the result to
/// `let _g = relay_mocktest::begin();` at the top of the test.
pub fn begin() -> TestGuard {
    set_scope(None);
    let _ = harness();
    TestGuard { _private: () }
}

// ---------------------------------------------------------------------------
// Server lifecycle
// ---------------------------------------------------------------------------

/// Bind an ephemeral loopback port, read the OS-assigned number, and release
/// it — the standard "pick a free port" dance. The port is momentarily free
/// after the listener drops; the mock re-binds it immediately after. Two
/// independent calls yield two independent free ports (WS + HTTP).
///
/// Returns `None` only if the OS refuses an ephemeral bind (never in practice),
/// in which case the caller falls back to the fixed slot default.
fn pick_free_port() -> Option<u16> {
    std::net::TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
}

/// Resolve the WS port. `MOCK_RELAY_PORT` (the load-bearing escape hatch) wins
/// so a gate can pre-spawn ONE shared mock and point every test binary at it.
/// When unset, pick a FREE ephemeral port per test binary — never the fixed
/// `DEFAULT_WS_PORT`. Under `cargo test --tests`, each mock-backed suite is a
/// SEPARATE process running in parallel; a hardcoded port would have every
/// binary race to bind the same 8781, the losers' mock dying on bind and their
/// tests failing with "Unable to connect". Free-port-per-binary removes the
/// collision (CLAUDE.md: always pick a free mock port, never a hardcoded one).
fn resolve_ws_port() -> u16 {
    if let Ok(raw) = std::env::var("MOCK_RELAY_PORT")
        && let Ok(p) = raw.parse::<u16>()
        && p != 0
    {
        return p;
    }
    pick_free_port().unwrap_or(DEFAULT_WS_PORT)
}

/// Resolve the HTTP control-plane port. `MOCK_RELAY_HTTP_PORT` wins (the escape
/// hatch — RELAY needs WS + HTTP as two INDEPENDENT ports, so the shared-mock
/// gate exports both). When unset, pick a SECOND independent free ephemeral
/// port — NOT `ws_port + 1000`, which could itself already be taken by another
/// binary's WS mock and reintroduce the very collision we're removing.
fn resolve_http_port(_ws_port: u16) -> u16 {
    if let Ok(raw) = std::env::var("MOCK_RELAY_HTTP_PORT")
        && let Ok(p) = raw.parse::<u16>()
        && p != 0
    {
        return p;
    }
    pick_free_port().unwrap_or(DEFAULT_HTTP_PORT)
}

/// Pick two DISTINCT free ephemeral ports at once. Calling `pick_free_port()`
/// twice can return the SAME port: each call drops its listener before the next
/// binds, so the OS is free to re-hand the just-released number. RELAY needs WS
/// and HTTP on two independent ports, and a collision makes `mock_relay` fail to
/// bind its second server → "did not become ready within 30s" (seen with
/// ws_port == http_port). Hold BOTH listeners simultaneously so the OS must
/// assign two different ports, then release them for the mock to re-bind.
fn pick_two_free_ports() -> Option<(u16, u16)> {
    let a = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    let b = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    let pa = a.local_addr().ok()?.port();
    let pb = b.local_addr().ok()?.port();
    // Both listeners are still alive here, so pa != pb is guaranteed; they drop
    // at end of scope, freeing both ports for the mock.
    Some((pa, pb))
}

fn ensure_server() -> Result<HarnessHandle, String> {
    // If both env overrides are unset, pick the WS+HTTP pair together so they
    // can never collide. If either is set (shared-mock gate), honor the
    // per-port resolvers (the escape hatch is load-bearing).
    let (ws_port, http_port) = if std::env::var_os("MOCK_RELAY_PORT").is_none()
        && std::env::var_os("MOCK_RELAY_HTTP_PORT").is_none()
        && let Some(pair) = pick_two_free_ports()
    {
        pair
    } else {
        let ws = resolve_ws_port();
        (ws, resolve_http_port(ws))
    };
    let ws_url = format!("ws://127.0.0.1:{ws_port}");
    let http_url = format!("http://127.0.0.1:{http_port}");
    let relay_host = format!("127.0.0.1:{ws_port}");

    if probe_health(&http_url) {
        return Ok(HarnessHandle {
            ws_url,
            http_url,
            relay_host,
            ws_port,
            http_port,
        });
    }

    spawn_server(ws_port, http_port)?;

    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if probe_health(&http_url) {
            return Ok(HarnessHandle {
                ws_url,
                http_url,
                relay_host,
                ws_port,
                http_port,
            });
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    Err(format!(
        "`python -m mock_relay` did not become ready within {STARTUP_TIMEOUT:?} \
         on ws_port={ws_port} http_port={http_port} \
         (clone porting-sdk next to signalwire-rust so tests can find \
         porting-sdk/test_harness/mock_relay/, or pip install the mock_relay package)"
    ))
}

fn probe_health(http_url: &str) -> bool {
    let url = format!("{http_url}/__mock__/health");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into();
    let Ok(mut resp) = agent.get(&url).call() else {
        return false;
    };
    if resp.status().as_u16() != 200 {
        return false;
    }
    let Ok(body) = resp.body_mut().read_to_string() else {
        return false;
    };
    let parsed: serde_json::Result<Value> = serde_json::from_str(&body);
    match parsed {
        Ok(v) => v.get("schemas_loaded").is_some(),
        Err(_) => false,
    }
}

fn spawn_server(ws_port: u16, http_port: u16) -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let pkg_dir = discover_porting_sdk_package("mock_relay");

    let mut cmd = Command::new("python");
    cmd.arg("-m")
        .arg("mock_relay")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--ws-port")
        .arg(ws_port.to_string())
        .arg("--http-port")
        .arg(http_port.to_string())
        .arg("--log-level")
        .arg("error")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(pkg_dir) = pkg_dir {
        let existing = std::env::var_os("PYTHONPATH");
        let new_pp: std::ffi::OsString = match existing {
            Some(ev) if !ev.is_empty() => {
                let mut joined = std::ffi::OsString::from(&pkg_dir);
                joined.push(separator());
                joined.push(ev);
                joined
            }
            _ => std::ffi::OsString::from(&pkg_dir),
        };
        cmd.env("PYTHONPATH", new_pp);
    }

    unsafe {
        cmd.pre_exec(|| {
            if libc_setsid() == -1 {
                // Best effort.
            }
            Ok(())
        });
    }

    cmd.spawn()
        .map_err(|e| format!("failed to spawn `python -m mock_relay`: {e} (set MOCK_RELAY_PORT/MOCK_RELAY_HTTP_PORT to use a pre-running instance)"))?;
    Ok(())
}

fn discover_porting_sdk_package(name: &str) -> Option<String> {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let mut dir: PathBuf = PathBuf::from(crate_root);
    loop {
        let parent = dir.parent()?.to_path_buf();
        let candidate = parent.join("porting-sdk").join("test_harness").join(name);
        let init = candidate.join(name).join("__init__.py");
        if init.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        if parent == dir {
            return None;
        }
        dir = parent;
    }
}

#[cfg(unix)]
fn separator() -> &'static str {
    ":"
}

#[cfg(windows)]
fn separator() -> &'static str {
    ";"
}

unsafe extern "C" {
    fn setsid() -> i32;
}

fn libc_setsid() -> i32 {
    unsafe { setsid() }
}

// ---------------------------------------------------------------------------
// Decode helpers
// ---------------------------------------------------------------------------

fn decode_journal(value: &Value) -> Vec<JournalEntry> {
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter().map(decode_entry).collect()
}

fn decode_entry(v: &Value) -> JournalEntry {
    let timestamp = v.get("timestamp").and_then(Value::as_f64).unwrap_or(0.0);
    let direction = v
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let method = v
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let request_id = v
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let frame = v.get("frame").cloned().unwrap_or(Value::Null);
    let connection_id = v
        .get("connection_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let session_id = v
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    JournalEntry {
        timestamp,
        direction,
        method,
        request_id,
        frame,
        connection_id,
        session_id,
    }
}

// Suppress the "unused" warning for HashMap import: we re-export it for
// callers that build their own decoders.
#[allow(dead_code)]
fn _hashmap_anchor() -> HashMap<String, Vec<String>> {
    HashMap::new()
}
