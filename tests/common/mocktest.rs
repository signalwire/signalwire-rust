// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Rust test helper for the porting-sdk `mock_signalwire` HTTP server.
//!
//! Mirrors the Go pilot at
//! `signalwire-go/pkg/rest/internal/mocktest/mocktest.go`, the TypeScript
//! port's `tests/rest/mocktest.ts`, and the Python conftest fixtures
//! (`signalwire_client` + `mock`).
//!
//! The server's lifetime is per-process: the first [`client`] / [`harness`]
//! call probes `http://127.0.0.1:<port>/__mock__/health` and either confirms
//! a running server or starts one as a detached subprocess.
//!
//! The default port is 8771 (the Rust slot in the parallel rollout —
//! TS=8766, Java=8767, PHP=8768, Ruby=8769, Perl=8770, Rust=8771,
//! C++=8772). Override with `MOCK_SIGNALWIRE_PORT`.
//!
//! ## Session isolation (parallel-safe)
//!
//! Tests run in parallel (cargo's default — one thread per test; each
//! integration-test binary is its own process). They share the one
//! `mock_signalwire` server, so global journal reads would race. REST is pure
//! request/response with no session handshake, so — exactly as the frozen
//! TypeScript design — the isolation **key is the `Authorization` header**:
//!
//! * [`client`] mints a **unique random project** (`test_proj_<12 hex>`) per
//!   test, so its `Authorization: Basic base64(project:token)` header is
//!   unique. The random suffix (not a counter) keeps it collision-free across
//!   threads AND separate processes hitting one shared mock. Project +
//!   auth-header are stashed in a thread-local (the per-test scope — cargo
//!   runs each test on its own thread).
//! * [`journal_all`] / [`journal_last`] filter the shared global journal
//!   **client-side** by that auth header, so a test only ever sees its own
//!   requests. No SDK change and no mock-server change.
//! * [`scenario_set`] scopes overrides **server-side** via
//!   `?session_id=<urlencoded auth header>` (the mock keys REST scenarios on
//!   the auth header), so a concurrent test can't consume another's override.
//! * [`reset_all`] / [`journal_reset`] are **no-ops when scoped**: a scoped
//!   test starts with zero entries in its auth-filtered view, and a global
//!   wipe would race a concurrent test.
//!
//! Tests that assert on the `AccountSid` embedded in a LAML path must read it
//! from [`project`] (or build it via [`account_path`]) rather than hard-coding
//! `test_proj`.
//!
//! All functions panic on error.

#![allow(dead_code)]
// Helper signatures take `Value` / owned args by value to mirror the
// cross-port mock-test helper contract (the Python `mock_relay` /
// `mock_signalwire` helpers and the Go pilot pass payloads by value). Keeping
// the by-value shape keeps these helpers 1:1 with their sibling ports.
#![allow(clippy::needless_pass_by_value)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;
use signalwire::rest::RestClient;

/// Throwaway token shared by every test client; only the project varies (so
/// only the project — and thus the Basic-auth header — is per-test unique).
const REST_TOKEN: &str = "test_tok";

thread_local! {
    /// The current test's isolation scope: (project, auth_header). Set by
    /// [`client`]; read by the journal filter, the scenario scoper, and the
    /// scoped-reset no-op. `None` => unscoped (legacy global view; only
    /// correct under serial execution). Thread-local because cargo runs each
    /// test on its own thread, so this is exactly the per-test scope.
    static SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct Scope {
    project: String,
    auth_header: String,
}

fn current_scope() -> Option<Scope> {
    SCOPE.with(|s| s.borrow().clone())
}

fn set_scope(scope: Option<Scope>) {
    SCOPE.with(|s| *s.borrow_mut() = scope);
}

/// The current test's unique random project (`test_proj_<hex>`). Tests that
/// assert on the `AccountSid` in a LAML path read it from here instead of
/// hard-coding `test_proj`. Falls back to `"test_proj"` when unscoped.
pub fn project() -> String {
    current_scope().map_or_else(|| "test_proj".to_string(), |s| s.project)
}

/// Build a LAML account path for the current test's project:
/// `/api/laml/2010-04-01/Accounts/<project>/<suffix>`. `suffix` is appended
/// verbatim (no leading slash needed). Mirrors the TS port's
/// `mock.project`-based path assertions.
pub fn account_path(suffix: &str) -> String {
    let base = format!("/api/laml/2010-04-01/Accounts/{}", project());
    if suffix.is_empty() {
        base
    } else {
        format!("{base}/{suffix}")
    }
}

/// Default port for the Rust slot in the parallel-port lineup.
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

/// Return a [`RestClient`] configured to hit the local mock server, and scope
/// this test's thread to the client's unique random project / auth header.
///
/// Each call mints a fresh `test_proj_<12 hex>` project, so the client's
/// `Authorization` header is unique and the journal is filterable per client.
/// The random suffix keeps it collision-free across threads/processes hitting
/// the one shared mock. The mock server is probed/spawned on first call and
/// reused thereafter.
///
/// Tests asserting on a LAML `AccountSid` path must use [`project`] /
/// [`account_path`] rather than hard-coding `test_proj`.
pub fn client() -> RestClient {
    let h = harness();
    let project = format!("test_proj_{}", random_hex12());
    let auth_header = format!("Basic {}", BASE64.encode(format!("{project}:{REST_TOKEN}")));
    set_scope(Some(Scope {
        project: project.clone(),
        auth_header,
    }));
    // Use with_base_url so the http:// prefix and explicit host:port survive
    // the constructor's https:// resolution.
    RestClient::with_base_url(&project, REST_TOKEN, &h.url).expect("RestClient::with_base_url")
}

/// 12 hex chars of process-and-thread-unique randomness for the per-test
/// project suffix. Uses `rand` (already a workspace dependency) seeded from
/// the OS entropy source, so two concurrent workers/processes can't collide.
fn random_hex12() -> String {
    use rand::RngExt;
    use std::fmt::Write as _;
    let mut rng = rand::rng();
    (0..6).fold(String::with_capacity(12), |mut acc, _| {
        let _ = write!(acc, "{:02x}", rng.random::<u8>());
        acc
    })
}

/// Return the singleton harness, spawning the mock server if necessary.
///
/// Panics with a descriptive message if the server can't be reached or
/// started.
pub fn harness() -> HarnessHandle {
    SERVER
        .get_or_init(|| ensure_server().expect("mocktest: failed to ensure mock server"))
        .clone()
}

/// Fetch the raw global journal (every client's requests, arrival order).
fn raw_journal() -> Vec<JournalEntry> {
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

/// Return this test's recorded requests in arrival order. Scoped to the
/// thread-local `auth_header` when set (so a parallel test never sees another
/// test's requests); unscoped views see the whole journal.
pub fn journal_all() -> Vec<JournalEntry> {
    let entries = raw_journal();
    match current_scope() {
        Some(scope) => entries
            .into_iter()
            .filter(|e| {
                e.headers.get("authorization").map(String::as_str) == Some(&scope.auth_header)
            })
            .collect(),
        None => entries,
    }
}

/// Return the most-recent journal entry for THIS test's client. Panics if the
/// journal is empty — every test that reaches this point should have produced
/// an entry.
pub fn journal_last() -> JournalEntry {
    let entries = journal_all();
    assert!(
        !entries.is_empty(),
        "mocktest: journal is empty - the SDK call did not reach the mock server"
    );
    entries.into_iter().last().unwrap()
}

/// Clear the mock journal. A **scoped** test leaves the shared journal alone
/// (it only ever reads its own entries, identified by auth header, so there
/// is nothing to clear and a global wipe would race a concurrent test). An
/// unscoped caller does the legacy global reset.
pub fn journal_reset() {
    if current_scope().is_some() {
        return;
    }
    let h = harness();
    let url = format!("{}/__mock__/journal/reset", h.url);
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("mocktest: POST /__mock__/journal/reset: {e}"));
}

/// Clear staged scenarios. Scoped tests leave the shared scenario store alone
/// (their scenarios are keyed by their unique auth header and a scoped test
/// starts with none); unscoped callers do the legacy global reset.
pub fn scenario_reset() {
    if current_scope().is_some() {
        return;
    }
    let h = harness();
    let url = format!("{}/__mock__/scenarios/reset", h.url);
    let _ = ureq::post(&url)
        .send_empty()
        .unwrap_or_else(|e| panic!("mocktest: POST /__mock__/scenarios/reset: {e}"));
}

/// Reset both journal and scenarios — typical fixture entry point. A no-op
/// when scoped (see [`journal_reset`] / [`scenario_reset`]).
pub fn reset_all() {
    journal_reset();
    scenario_reset();
}

/// RAII guard returned by [`begin`]. Clears this thread's scope on drop so a
/// same-thread follow-up test starts unscoped until it calls [`client`] again.
pub struct TestGuard {
    _private: (),
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        set_scope(None);
    }
}

/// Per-test entry point. Resets this thread's scope so a test starts clean,
/// then ensures the mock server is up. No global lock and no global
/// journal/scenario reset: the auth-header scope (set by [`client`]) makes the
/// shared mock safe under parallel tests, and a scoped test starts with an
/// empty (auth-filtered) view. Bind the result to
/// `let _g = mocktest::begin();` at the top of the test.
pub fn begin() -> TestGuard {
    set_scope(None);
    let _ = harness();
    TestGuard { _private: () }
}

/// Stage a one-shot response override for the route identified by
/// `endpoint_id` (Spectral `OperationId` from the `OpenAPI` spec). Scoped to
/// THIS test's auth header (REST's session key) so a concurrent test can't
/// consume it; unscoped callers stage a shared override.
pub fn scenario_set(endpoint_id: &str, status: u16, body: Value) {
    let h = harness();
    let q = current_scope().map_or_else(String::new, |s| {
        format!("?session_id={}", urlencode(&s.auth_header))
    });
    let url = format!("{}/__mock__/scenarios/{endpoint_id}{q}", h.url);
    let payload = serde_json::json!({"status": status, "response": body});
    let _ = ureq::post(&url)
        .send_json(&payload)
        .unwrap_or_else(|e| panic!("mocktest: POST scenario: {e}"));
}

/// Percent-encode a query value (the auth header carries `+` / `/` / `=` from
/// base64, which must be escaped in a query string).
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

// ---------------------------------------------------------------------------
// Server lifecycle
// ---------------------------------------------------------------------------

/// Bind an ephemeral loopback port, read the OS-assigned number, and release
/// it — the standard "pick a free port" dance. The mock re-binds it immediately
/// after in [`spawn_server`].
fn pick_free_port() -> Option<u16> {
    std::net::TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
}

/// Resolve the mock port. `MOCK_SIGNALWIRE_PORT` (the load-bearing escape hatch)
/// wins so a gate can pre-spawn ONE shared mock and point every test binary at
/// it. When unset, pick a FREE ephemeral port per test binary — never the fixed
/// `DEFAULT_PORT`. Under `cargo test --tests`, each mock-backed suite is a
/// SEPARATE process running in parallel; a hardcoded port has every binary race
/// to bind the same 8771, the losers' mock dying on bind and their tests
/// failing. Free-port-per-binary removes the collision (CLAUDE.md: always pick a
/// free mock port, never a hardcoded one).
fn resolve_port() -> u16 {
    if let Ok(raw) = std::env::var("MOCK_SIGNALWIRE_PORT")
        && let Ok(p) = raw.parse::<u16>()
        && p != 0
    {
        return p;
    }
    pick_free_port().unwrap_or(DEFAULT_PORT)
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
/// Cargo at compile time).
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
    let Some(arr) = value.as_array() else {
        return Vec::new();
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
