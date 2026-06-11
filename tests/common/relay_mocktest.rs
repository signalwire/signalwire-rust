// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Rust test helper for the porting-sdk `mock_relay` WebSocket server.
//!
//! Mirrors the Python conftest fixture (`signalwire_relay_client` /
//! `mock_relay`) and the REST mocktest pattern at `tests/common/mocktest.rs`.
//! The mock server is probed/spawned on first call. The probe-or-spawn
//! lifecycle matches the REST mock harness exactly, but talks to a
//! WebSocket plane on `ws://127.0.0.1:<ws_port>` (default 8781) and an
//! HTTP control plane on port `ws_port + 1000` (default 9781).
//!
//! The Rust slot in the parallel rollout is `WS=8781 / HTTP=9781`. Override
//! either with `MOCK_RELAY_PORT` (WS) or `MOCK_RELAY_HTTP_PORT` (HTTP).
//!
//! All functions panic on error. Tests using these helpers should run
//! single-threaded (`cargo test -- --test-threads=1`) because the journal
//! and scenario queues are shared global state on the mock server.

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use signalwire::relay::Client as RelayClient;

/// Default WebSocket port for the Rust slot in the parallel rollout.
pub const DEFAULT_WS_PORT: u16 = 8781;

/// Default HTTP control-plane port (WS + 1000).
pub const DEFAULT_HTTP_PORT: u16 = 9781;

/// Maximum wait for the spawned mock server to answer `/__mock__/health`.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Global mutex serializing every mocktest-driven test. The mock journal,
/// scenario queues, and the env-var redirect for `relay::Client` are
/// process-global state, so two parallel tests would otherwise see each
/// other's recorded entries.
///
/// Note: this mutex is only effective WITHIN a single test binary. Cargo
/// runs each integration-test binary as its own process, and runs them
/// concurrently. The shared `mock_relay` server is the same instance for
/// all binaries, so we additionally hold a Unix file lock (see
/// [`acquire_cross_binary_lock`]) to serialize across binaries.
static SERIALIZE: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_journal() -> MutexGuard<'static, ()> {
    SERIALIZE
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Path of the cross-binary advisory file lock. Located in `/tmp` so each
/// test binary process (with its own copy of this module) refers to the
/// same inode and `flock` can serialize them.
const CROSS_BINARY_LOCK_PATH: &str = "/tmp/signalwire-rust-mock-relay.lock";

/// Cross-process advisory `flock` guard. Ensures only one test (across all
/// integration-test binaries) is touching the mock-relay server at a time
/// — the WebSocket session registry, journal, and broadcast plane are
/// shared state that two concurrent binaries would otherwise pollute.
///
/// Held for the entire test (acquired in [`begin`], released when the
/// returned [`TestGuard`] drops).
struct CrossBinaryLock {
    file: std::fs::File,
}

impl CrossBinaryLock {
    fn acquire() -> Self {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(CROSS_BINARY_LOCK_PATH)
            .unwrap_or_else(|e| {
                panic!(
                    "relay_mocktest: open {}: {}",
                    CROSS_BINARY_LOCK_PATH, e
                )
            });
        // LOCK_EX: exclusive lock. Blocks until acquired. Released on
        // close (i.e. when the File drops at the end of the test).
        let fd = file.as_raw_fd();
        // SAFETY: fd is valid for the lifetime of `file`.
        let rc = unsafe { libc_flock(fd, LOCK_EX) };
        if rc != 0 {
            panic!(
                "relay_mocktest: flock LOCK_EX on {}: errno={}",
                CROSS_BINARY_LOCK_PATH,
                std::io::Error::last_os_error()
            );
        }
        CrossBinaryLock { file }
    }
}

impl Drop for CrossBinaryLock {
    fn drop(&mut self) {
        // LOCK_UN: explicit unlock. Closing the file would also release
        // the lock, but explicit is safer in case the kernel reuses the
        // descriptor.
        let fd = self.file.as_raw_fd();
        // SAFETY: fd is valid until the File drops.
        let _ = unsafe { libc_flock(fd, LOCK_UN) };
    }
}

const LOCK_EX: i32 = 2;
const LOCK_UN: i32 = 8;

unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

unsafe fn libc_flock(fd: i32, op: i32) -> i32 {
    unsafe { flock(fd, op) }
}

/// Block until the mock-relay session registry is empty, or `budget`
/// elapses. Used at the start of each test to ensure no stale session
/// from a previous binary's torn-down client is still registered (which
/// would receive broadcasts intended for *this* test's client).
fn wait_for_no_sessions(budget: Duration) {
    let h = harness();
    let url = format!("{}/__mock__/sessions", h.http_url);
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        let mut resp = match ureq::get(&url).call() {
            Ok(r) => r,
            Err(_) => return, // server unreachable — let later code panic with detail
        };
        let body: Value = match resp.body_mut().read_json::<Value>() {
            Ok(v) => v,
            Err(_) => return,
        };
        let sessions = body
            .get("sessions")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        if sessions == 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// One-shot health probe / spawn lock.
static SERVER: OnceLock<HarnessHandle> = OnceLock::new();

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

/// Convenience: build a connected `relay::Client` pointed at the mock.
///
/// Sets `SIGNALWIRE_RELAY_SCHEME=ws` and `SIGNALWIRE_RELAY_HOST=<host:port>`
/// so the Client's `connect()` method dials the mock instead of the real
/// WSS endpoint, and runs the connect handshake. Returns the connected
/// `Arc<Client>` ready for `.send_message(...)`, `.dial(...)`, etc. callers.
///
/// Each test should call [`begin`] first to acquire the global mutex and
/// reset journal/scenarios. The returned client should be `disconnect()`ed
/// (typically through a `let client = ...; defer-style block) before the
/// `TestGuard` drops — see the integration tests for the canonical pattern.
pub fn connected_client(contexts: &[&str]) -> Arc<RelayClient> {
    let h = harness();
    // Direct env-var manipulation — protected by the SERIALIZE mutex held
    // by the active `TestGuard`.
    unsafe {
        std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "ws");
        std::env::set_var("SIGNALWIRE_RELAY_HOST", &h.relay_host);
    }
    let client = Arc::new(RelayClient::new("test_proj", "test_tok", &h.relay_host));
    // Pre-populate contexts before connect so they go out on the connect frame.
    {
        let mut ctx = client.contexts.lock().unwrap();
        for c in contexts {
            ctx.push((*c).to_string());
        }
    }
    client.connect().expect("relay_mocktest: connect");
    client
}

/// Return every journal entry recorded since the last reset, in arrival order.
pub fn journal_all() -> Vec<JournalEntry> {
    let h = harness();
    let url = format!("{}/__mock__/journal", h.http_url);
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
            let want = match event_type {
                Some(t) => t,
                None => return true,
            };
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

/// Clear the mock journal so the next assertion starts from a clean slate.
pub fn journal_reset() {
    let h = harness();
    let url = format!("{}/__mock__/journal/reset", h.http_url);
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("relay_mocktest: POST journal/reset: {e}"));
}

/// Drain all queued scenarios.
pub fn scenario_reset() {
    let h = harness();
    let url = format!("{}/__mock__/scenarios/reset", h.http_url);
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("relay_mocktest: POST scenarios/reset: {e}"));
}

/// Reset both journal and scenarios — typical fixture entry point.
pub fn reset_all() {
    journal_reset();
    scenario_reset();
}

/// Queue scripted post-RPC events for a method (FIFO consume-once).
///
/// `events` is a list of `{"emit": {...}, "delay_ms": N, "event_type": "..."}`
/// entries; `event_type` defaults to a derivation from `method`
/// (`calling.play` → `calling.call.play`).
pub fn arm_method(method: &str, events: Value) {
    let h = harness();
    let url = format!("{}/__mock__/scenarios/{}", h.http_url, method);
    let _ = ureq::post(&url)
        .send_json(&events)
        .unwrap_or_else(|e| panic!("relay_mocktest: arm_method: {e}"));
}

/// Queue a dial-dance scenario.
pub fn arm_dial(payload: Value) {
    let h = harness();
    let url = format!("{}/__mock__/scenarios/dial", h.http_url);
    let _ = ureq::post(&url)
        .send_json(&payload)
        .unwrap_or_else(|e| panic!("relay_mocktest: arm_dial: {e}"));
}

/// Push a single frame to the connected SDK session(s).
pub fn push(frame: Value) {
    let h = harness();
    let url = format!("{}/__mock__/push", h.http_url);
    let body = json!({"frame": frame});
    let _ = ureq::post(&url)
        .send_json(&body)
        .unwrap_or_else(|e| panic!("relay_mocktest: push: {e}"));
}

/// Convenience wrapper around `/__mock__/inbound_call`. Spec is the same
/// as the Python `mock_relay.inbound_call(...)` helper.
pub fn inbound_call(payload: Value) {
    let h = harness();
    let url = format!("{}/__mock__/inbound_call", h.http_url);
    let _ = ureq::post(&url)
        .send_json(&payload)
        .unwrap_or_else(|e| panic!("relay_mocktest: inbound_call: {e}"));
}

/// Run a scripted timeline (`scenario_play`).
pub fn scenario_play(ops: Value) -> Value {
    let h = harness();
    let url = format!("{}/__mock__/scenario_play", h.http_url);
    let mut resp = ureq::post(&url)
        .send_json(&ops)
        .unwrap_or_else(|e| panic!("relay_mocktest: scenario_play: {e}"));
    resp.body_mut()
        .read_json::<Value>()
        .unwrap_or_else(|e| panic!("relay_mocktest: decode scenario_play: {e}"))
}

/// RAII guard that holds the global serialization mutex for the duration
/// of a single test. Tests should bind the return value to `let _g`.
///
/// Field order matters: `_cross_binary` must be listed AFTER `_inner` so
/// that on drop the cross-binary file lock is released BEFORE the
/// in-process mutex (Rust drops fields in declaration order, so the
/// in-process mutex drops first — no, actually fields drop in REVERSE
/// declaration order; the last-declared drops first). We want the
/// in-process mutex to release LAST so a same-binary follow-up test
/// doesn't re-enter `begin` and race with the about-to-release file lock.
pub struct TestGuard {
    _cross_binary: CrossBinaryLock,
    _inner: MutexGuard<'static, ()>,
}

/// Acquire the global serialization mutex (in-process) and the cross-binary
/// file lock, ensure the mock-relay session registry has drained, and reset
/// journal+scenarios. The returned [`TestGuard`] must outlive the test
/// body — bind it to `let _g = relay_mocktest::begin();` at the top.
pub fn begin() -> TestGuard {
    // Acquire the in-process mutex first; it's cheap and guards same-binary
    // concurrency. Then acquire the cross-binary file lock to serialize
    // against tests in other binaries running concurrently against the
    // same shared mock_relay server.
    let inner = lock_journal();
    let cross = CrossBinaryLock::acquire();

    // Wait for any leftover sessions from a previous binary's tests to
    // drain — once the file lock is held, we know no other test is in
    // begin/disconnect, but the SDK in the previous binary might not have
    // closed its WebSocket cleanly yet. The mock server unregisters
    // sessions only when the WS read loop sees ConnectionClosed.
    wait_for_no_sessions(Duration::from_secs(2));

    journal_reset();
    scenario_reset();
    TestGuard {
        _cross_binary: cross,
        _inner: inner,
    }
}

// ---------------------------------------------------------------------------
// Server lifecycle
// ---------------------------------------------------------------------------

fn resolve_ws_port() -> u16 {
    if let Ok(raw) = std::env::var("MOCK_RELAY_PORT")
        && let Ok(p) = raw.parse::<u16>()
            && p != 0 {
                return p;
            }
    DEFAULT_WS_PORT
}

fn resolve_http_port(ws_port: u16) -> u16 {
    if let Ok(raw) = std::env::var("MOCK_RELAY_HTTP_PORT")
        && let Ok(p) = raw.parse::<u16>()
            && p != 0 {
                return p;
            }
    // Default convention: WS_PORT + 1000.
    ws_port.saturating_add(1000)
}

fn ensure_server() -> Result<HarnessHandle, String> {
    let ws_port = resolve_ws_port();
    let http_port = resolve_http_port(ws_port);
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
    let mut resp = match agent.get(&url).call() {
        Ok(r) => r,
        Err(_) => return false,
    };
    if resp.status().as_u16() != 200 {
        return false;
    }
    let body = match resp.body_mut().read_to_string() {
        Ok(b) => b,
        Err(_) => return false,
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
        let parent = match dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return None,
        };
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
    let arr = match value.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter().map(decode_entry).collect()
}

fn decode_entry(v: &Value) -> JournalEntry {
    let timestamp = v
        .get("timestamp")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
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
