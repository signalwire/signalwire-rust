// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Rust test helper for the porting-sdk `mock_signalwire` HTTP server.
//!
//! Mirrors the Go pilot at
//! `signalwire-go/pkg/rest/internal/mocktest/mocktest.go` and the Python
//! conftest fixtures (`signalwire_client` + `mock`).
//!
//! The server's lifetime is per-process: the first [`client`] / [`harness`]
//! call probes `http://127.0.0.1:<port>/__mock__/health` and either confirms
//! a running server or starts one as a detached subprocess. Each test must
//! call [`journal_reset`] (or [`scenario_reset`]) before relying on
//! [`journal_last`].
//!
//! The default port is 8771 (the Rust slot in the parallel rollout —
//! TS=8766, Java=8767, PHP=8768, Ruby=8769, Perl=8770, Rust=8771,
//! C++=8772). Override with `MOCK_SIGNALWIRE_PORT`.
//!
//! All functions panic on error. Tests using these helpers should run
//! single-threaded (`cargo test -- --test-threads=1`) because the mock
//! journal is shared global state.

#![allow(dead_code)]
// Helper signatures take `Value` / owned args by value to mirror the
// cross-port mock-test helper contract (the Python `mock_relay` /
// `mock_signalwire` helpers and the Go pilot pass payloads by value). Keeping
// the by-value shape keeps these helpers 1:1 with their sibling ports.
#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;
use signalwire::rest::RestClient;

/// Global mutex serializing every mocktest-driven test. The mock journal
/// is process-global state, so two parallel tests would otherwise see
/// each other's recorded entries. Tests that derive from this helper
/// implicitly hold the mutex via `reset_all` (or `journal_reset`) for
/// the full duration of their assertions.
static SERIALIZE: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_journal() -> MutexGuard<'static, ()> {
    SERIALIZE
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Default port for the Rust slot in the parallel parallel-port lineup.
pub const DEFAULT_PORT: u16 = 8771;

/// Maximum wait for the spawned mock server to answer `/__mock__/health`.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// One-shot health probe / spawn lock.
static SERVER: OnceLock<HarnessHandle> = OnceLock::new();

/// The current mock-server harness — its base URL + port.
#[derive(Clone, Debug)]
pub struct HarnessHandle {
    pub url: String,
    pub port: u16,
}

/// A single recorded request as journaled by the mock server.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub method: String,
    pub path: String,
    pub query_params: HashMap<String, Vec<String>>,
    pub headers: HashMap<String, String>,
    pub body: Value,
    pub matched_route: Option<String>,
    pub response_status: Option<i64>,
}

impl JournalEntry {
    /// Try to interpret `body` as a JSON object and return it.
    pub fn body_object(&self) -> Option<&serde_json::Map<String, Value>> {
        self.body.as_object()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return a [`RestClient`] configured to hit the local mock server, with
/// throwaway credentials matching the Python `signalwire_client` fixture.
///
/// The mock server is probed/spawned on first call and reused thereafter.
/// Call [`journal_reset`] before each test to clear shared journal state.
pub fn client() -> RestClient {
    let h = harness();
    // Use with_base_url so the http:// prefix and explicit host:port survive
    // the constructor's https:// resolution.
    RestClient::with_base_url("test_proj", "test_tok", &h.url)
        .expect("RestClient::with_base_url")
}

/// Return the singleton harness, spawning the mock server if necessary.
///
/// Panics with a descriptive message if the server can't be reached or
/// started — tests that need an early skip should check
/// `SERVER.get().is_some()` first, but in practice the audit pipeline
/// guarantees the mock is always available.
pub fn harness() -> HarnessHandle {
    SERVER
        .get_or_init(|| ensure_server().expect("mocktest: failed to ensure mock server"))
        .clone()
}

/// Return every journal entry recorded since the last reset, in arrival order.
pub fn journal_all() -> Vec<JournalEntry> {
    let h = harness();
    let url = format!("{}/__mock__/journal", h.url);
    let mut resp = ureq::get(&url)
        .call()
        .unwrap_or_else(|e| panic!("mocktest: GET /__mock__/journal: {e}"));
    let body: Value = resp
        .body_mut()
        .read_json()
        .unwrap_or_else(|e| panic!("mocktest: decode journal: {e}"));
    decode_journal(&body)
}

/// Return the most-recent journal entry. Panics if the journal is empty
/// — every test that reaches this point should have produced an entry.
pub fn journal_last() -> JournalEntry {
    let entries = journal_all();
    assert!(
        !entries.is_empty(),
        "mocktest: journal is empty - the SDK call did not reach the mock server"
    );
    entries.into_iter().last().unwrap()
}

/// Clear the mock journal so the next assertion starts from a clean slate.
pub fn journal_reset() {
    let h = harness();
    let url = format!("{}/__mock__/journal/reset", h.url);
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("mocktest: POST /__mock__/journal/reset: {e}"));
}

/// Clear staged scenarios.
pub fn scenario_reset() {
    let h = harness();
    let url = format!("{}/__mock__/scenarios/reset", h.url);
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("mocktest: POST /__mock__/scenarios/reset: {e}"));
}

/// Reset both journal and scenarios — typical fixture entry point.
pub fn reset_all() {
    journal_reset();
    scenario_reset();
}

/// RAII guard that holds the global serialization mutex for the duration
/// of a single test. Tests should bind the return value to a `let _g`
/// at the top of the function so that no two tests in the same binary
/// race on the shared journal.
pub struct TestGuard {
    _inner: MutexGuard<'static, ()>,
}

/// Acquire the global serialization mutex and reset journal+scenarios.
/// Drop the returned guard at the end of the test (typically via the
/// implicit drop at the end of the function scope) to release it.
pub fn begin() -> TestGuard {
    let guard = lock_journal();
    journal_reset();
    scenario_reset();
    TestGuard { _inner: guard }
}

/// Stage a one-shot response override for the route identified by
/// `endpoint_id` (Spectral OperationId from the OpenAPI spec).
pub fn scenario_set(endpoint_id: &str, status: u16, body: Value) {
    let h = harness();
    let url = format!("{}/__mock__/scenarios/{endpoint_id}", h.url);
    let payload = serde_json::json!({"status": status, "response": body});
    let _ = ureq::post(&url)
        .send_json(&payload)
        .unwrap_or_else(|e| panic!("mocktest: POST scenario: {e}"));
}

// ---------------------------------------------------------------------------
// Server lifecycle
// ---------------------------------------------------------------------------

fn resolve_port() -> u16 {
    if let Ok(raw) = std::env::var("MOCK_SIGNALWIRE_PORT")
        && let Ok(p) = raw.parse::<u16>()
            && p != 0 {
                return p;
            }
    DEFAULT_PORT
}

fn ensure_server() -> Result<HarnessHandle, String> {
    let port = resolve_port();
    let url = format!("http://127.0.0.1:{port}");

    if probe_health(&url) {
        return Ok(HarnessHandle { url, port });
    }

    spawn_server(port)?;

    // Poll up to STARTUP_TIMEOUT.
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if probe_health(&url) {
            return Ok(HarnessHandle { url, port });
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    Err(format!(
        "`python -m mock_signalwire` did not become ready within {STARTUP_TIMEOUT:?} on port {port} \
         (clone porting-sdk next to signalwire-rust so tests can find \
         porting-sdk/test_harness/mock_signalwire/, or pip install the mock_signalwire package)"
    ))
}

fn probe_health(base_url: &str) -> bool {
    let url = format!("{base_url}/__mock__/health");
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
        Ok(v) => v.get("specs_loaded").is_some(),
        Err(_) => false,
    }
}

fn spawn_server(port: u16) -> Result<(), String> {
    // Detach via a new session/process group so the test binary's pipe-drain
    // logic doesn't block. We mirror Go's `setsid + Setpgid` approach.
    use std::os::unix::process::CommandExt;

    // Try to inject porting-sdk/test_harness/mock_signalwire/ into
    // PYTHONPATH so `python -m mock_signalwire` resolves without a prior
    // `pip install -e ...`. Adjacency contract: porting-sdk next to
    // signalwire-rust in ~/src/. When the walk fails (e.g. porting-sdk
    // is not adjacent), we still spawn — the child falls back to whatever
    // is on the system Python's sys.path, and the readiness probe surfaces
    // a clear timeout error if neither mode is available.
    let pkg_dir = discover_porting_sdk_package("mock_signalwire");

    let mut cmd = Command::new("python");
    cmd.arg("-m")
        .arg("mock_signalwire")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--log-level")
        .arg("error")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(pkg_dir) = pkg_dir {
        // Prepend the porting-sdk package dir to PYTHONPATH. Use the OS
        // path separator so this works on every platform Cargo runs on.
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

    // Detach to a new session.
    unsafe {
        cmd.pre_exec(|| {
            // setsid() -> own process group + session, no controlling terminal.
            if libc_setsid() == -1 {
                // Best effort; not fatal.
            }
            Ok(())
        });
    }

    cmd.spawn()
        .map_err(|e| format!("failed to spawn `python -m mock_signalwire`: {e} (set MOCK_SIGNALWIRE_PORT to use a pre-running instance)"))?;
    Ok(())
}

/// Walk up from this source file looking for an adjacent
/// `../porting-sdk/test_harness/<name>/<name>/__init__.py`. Returns the
/// absolute path to the directory containing the Python package (the value
/// to put on PYTHONPATH so that `python -m <name>` resolves), or `None`
/// when no adjacent porting-sdk is reachable.
///
/// The walk anchors at `CARGO_MANIFEST_DIR` (the crate root, injected by
/// Cargo at compile time). Tests run with that as their working directory
/// by default, so this is the canonical source-of-truth for "where this
/// repo lives on disk."
fn discover_porting_sdk_package(name: &str) -> Option<String> {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let mut dir: PathBuf = PathBuf::from(crate_root);
    loop {
        let parent = match dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return None,
        };
        let candidate = parent
            .join("porting-sdk")
            .join("test_harness")
            .join(name);
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

// libc setsid binding without pulling in the libc crate. Rust 2024
// edition requires `unsafe extern` for foreign blocks.
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
    let method = v
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let path = v
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let mut query_params: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(obj) = v.get("query_params").and_then(Value::as_object) {
        for (k, vv) in obj {
            let list = vv
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| x.as_str().unwrap_or("").to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            query_params.insert(k.clone(), list);
        }
    }

    let mut headers: HashMap<String, String> = HashMap::new();
    if let Some(obj) = v.get("headers").and_then(Value::as_object) {
        for (k, vv) in obj {
            headers.insert(k.clone(), vv.as_str().unwrap_or("").to_string());
        }
    }

    let body = v.get("body").cloned().unwrap_or(Value::Null);

    let matched_route = v
        .get("matched_route")
        .and_then(Value::as_str)
        .map(str::to_string);

    let response_status = v.get("response_status").and_then(Value::as_i64);

    JournalEntry {
        method,
        path,
        query_params,
        headers,
        body,
        matched_route,
        response_status,
    }
}
