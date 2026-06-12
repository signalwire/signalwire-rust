// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Shared test-only TLS support for the three "verified HTTPS + WSS"
//! capability tests (`tls_wss_relay.rs`, `tls_https_rest.rs`,
//! `tls_https_server.rs`).
//!
//! Provides:
//!   * [`certs_dir`] — locate `porting-sdk/test_harness/tls`, run the
//!     idempotent `gen_certs.sh`, and return the `certs/` dir (`ca.crt`,
//!     `server.crt`, `server.key`).
//!   * [`spawn_tls_mock_relay`] / [`spawn_tls_mock_signalwire`] — start the
//!     shared mock servers in `--tls` mode on *dedicated* ports so the plain
//!     mocks the rest of the suite uses on the default ports are untouched.
//!   * [`ca_trusting_agent`] — a `ureq::Agent` that trusts the test CA, for
//!     reading the mocks' HTTPS `/__mock__/` control plane.
//!   * [`RelayTlsLock`] — a cross-binary `flock` so the WSS test serializes
//!     against any other binary that might touch the TLS mock-relay instance.
//!
//! The CA the mocks present is the throwaway self-signed CA produced by
//! `gen_certs.sh`; tests trust it explicitly (the SDK's `SIGNALWIRE_*_CA_FILE`
//! / `SWML_SSL_*` hooks) — never `danger_accept_invalid_certs`.

#![allow(dead_code)]

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Dedicated TLS-mode ports, disjoint from the plain-mock default slots
/// (mock_signalwire 8771; mock_relay WS 8781 / HTTP 9781).
pub const TLS_RELAY_WS_PORT: u16 = 18781;
pub const TLS_RELAY_HTTP_PORT: u16 = 19781;
pub const TLS_SIGNALWIRE_PORT: u16 = 18771;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(40);

// ---------------------------------------------------------------------------
// Cert discovery
// ---------------------------------------------------------------------------

/// Walk up from the crate root to `../porting-sdk/test_harness/tls`, run the
/// idempotent `gen_certs.sh`, and return the absolute `certs/` directory.
/// Returns `None` when porting-sdk is not adjacent (the caller should skip).
pub fn certs_dir() -> Option<PathBuf> {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let mut dir = PathBuf::from(crate_root);
    loop {
        let parent = dir.parent()?.to_path_buf();
        let tls_dir = parent.join("porting-sdk").join("test_harness").join("tls");
        if tls_dir.join("gen_certs.sh").is_file() {
            // Idempotent: regenerates only when the leaf cert is missing/expiring.
            let status = Command::new("bash")
                .arg(tls_dir.join("gen_certs.sh"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            match status {
                Ok(s) if s.success() => return Some(tls_dir.join("certs")),
                _ => return None,
            }
        }
        if parent == dir {
            return None;
        }
        dir = parent;
    }
}

/// Path to the test CA bundle (`certs/ca.crt`).
pub fn ca_file() -> Option<PathBuf> {
    certs_dir().map(|d| d.join("ca.crt"))
}

// ---------------------------------------------------------------------------
// CA-trusting HTTPS client (for reading the mocks' HTTPS control plane)
// ---------------------------------------------------------------------------

/// Build a `ureq::Agent` whose only trust anchor is the test CA, so it can read
/// the mocks' `/__mock__/` endpoints over HTTPS. This is the same mechanism the
/// SDK's REST client uses (ureq `TlsConfig` + `RootCerts::Specific`) — genuine
/// verification, just with the test CA added.
pub fn ca_trusting_agent(ca_path: &PathBuf) -> ureq::Agent {
    let pem = std::fs::read(ca_path).expect("read ca.crt");
    let cert = ureq::tls::Certificate::from_pem(&pem).expect("parse ca.crt");
    let root_certs = ureq::tls::RootCerts::new_with_certs(&[cert]);
    let tls_config = ureq::tls::TlsConfig::builder()
        .root_certs(root_certs)
        .build();
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .http_status_as_error(false)
        .tls_config(tls_config)
        .build()
        .into()
}

// ---------------------------------------------------------------------------
// Mock spawning (--tls mode, dedicated ports)
// ---------------------------------------------------------------------------

/// Locate the dir to prepend to PYTHONPATH so `python -m <name>` resolves.
fn discover_harness_pkg(name: &str) -> Option<String> {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let mut dir = PathBuf::from(crate_root);
    loop {
        let parent = dir.parent()?.to_path_buf();
        let candidate = parent.join("porting-sdk").join("test_harness").join(name);
        if candidate.join(name).join("__init__.py").is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        if parent == dir {
            return None;
        }
        dir = parent;
    }
}

/// A spawned `--tls` mock subprocess. Killed on drop so each test binary
/// cleans up its dedicated TLS mock instance.
pub struct TlsMockProc {
    child: std::process::Child,
}

impl Drop for TlsMockProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_with_pythonpath(name: &str, args: &[String]) -> Option<std::process::Child> {
    use std::os::unix::process::CommandExt;
    let pkg_dir = discover_harness_pkg(name)?;

    let mut cmd = Command::new("python");
    cmd.arg("-m").arg(name);
    for a in args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let existing = std::env::var_os("PYTHONPATH");
    let new_pp: std::ffi::OsString = match existing {
        Some(ev) if !ev.is_empty() => {
            let mut joined = std::ffi::OsString::from(&pkg_dir);
            joined.push(":");
            joined.push(ev);
            joined
        }
        _ => std::ffi::OsString::from(&pkg_dir),
    };
    cmd.env("PYTHONPATH", new_pp);

    // Detach into its own session so the test binary's exit doesn't orphan it
    // weirdly; the Drop handler kills it explicitly.
    unsafe {
        cmd.pre_exec(|| {
            libc_setsid();
            Ok(())
        });
    }
    cmd.spawn().ok()
}

/// Spawn `python -m mock_relay --tls` on the dedicated TLS WS/HTTP ports.
/// Waits for the (plain-HTTP) control plane `/__mock__/health`. Returns the
/// running process handle + http control-plane URL, or `None` if unavailable.
pub fn spawn_tls_mock_relay() -> Option<(TlsMockProc, String)> {
    let args = vec![
        "--host".into(),
        "127.0.0.1".into(),
        "--ws-port".into(),
        TLS_RELAY_WS_PORT.to_string(),
        "--http-port".into(),
        TLS_RELAY_HTTP_PORT.to_string(),
        "--tls".into(),
        "--log-level".into(),
        "error".into(),
    ];
    let child = spawn_with_pythonpath("mock_relay", &args)?;
    let proc = TlsMockProc { child };
    // mock_relay keeps its control plane on plain HTTP even in --tls mode.
    let http_url = format!("http://127.0.0.1:{TLS_RELAY_HTTP_PORT}");
    if wait_health_plain(&http_url, "schemas_loaded") {
        Some((proc, http_url))
    } else {
        None
    }
}

/// Spawn `python -m mock_signalwire --tls` on the dedicated TLS port. Waits for
/// the HTTPS `/__mock__/health` using a CA-trusting agent. Returns the running
/// process + the `https://` base URL, or `None` if unavailable.
pub fn spawn_tls_mock_signalwire(ca_path: &PathBuf) -> Option<(TlsMockProc, String)> {
    let args = vec![
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        TLS_SIGNALWIRE_PORT.to_string(),
        "--tls".into(),
        "--log-level".into(),
        "error".into(),
    ];
    let child = spawn_with_pythonpath("mock_signalwire", &args)?;
    let proc = TlsMockProc { child };
    let base_url = format!("https://127.0.0.1:{TLS_SIGNALWIRE_PORT}");
    let agent = ca_trusting_agent(ca_path);
    if wait_health_https(&agent, &base_url, "specs_loaded") {
        Some((proc, base_url))
    } else {
        None
    }
}

fn wait_health_plain(http_url: &str, expect_key: &str) -> bool {
    let url = format!("{http_url}/__mock__/health");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into();
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(mut resp) = agent.get(&url).call()
            && resp.status().as_u16() == 200
                && let Ok(v) = resp.body_mut().read_json::<Value>()
                    && v.get(expect_key).is_some() {
                        return true;
                    }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

fn wait_health_https(agent: &ureq::Agent, base_url: &str, expect_key: &str) -> bool {
    let url = format!("{base_url}/__mock__/health");
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(mut resp) = agent.get(&url).call()
            && resp.status().as_u16() == 200
                && let Ok(v) = resp.body_mut().read_json::<Value>()
                    && v.get(expect_key).is_some() {
                        return true;
                    }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

// ---------------------------------------------------------------------------
// Cross-binary flock for the TLS mock-relay instance
// ---------------------------------------------------------------------------

const RELAY_TLS_LOCK_PATH: &str = "/tmp/signalwire-rust-mock-relay-tls.lock";

/// Exclusive cross-binary advisory lock guarding the dedicated TLS mock-relay
/// instance — mirrors the plain `relay_mocktest` lock so two concurrently-run
/// test binaries can't both drive the same WSS mock session registry.
pub struct RelayTlsLock {
    file: std::fs::File,
}

impl RelayTlsLock {
    pub fn acquire() -> Self {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(RELAY_TLS_LOCK_PATH)
            .unwrap_or_else(|e| panic!("tls_support: open {RELAY_TLS_LOCK_PATH}: {e}"));
        let fd = file.as_raw_fd();
        let rc = unsafe { flock(fd, LOCK_EX) };
        assert!(
            rc == 0,
            "tls_support: flock LOCK_EX on {RELAY_TLS_LOCK_PATH}: {}",
            std::io::Error::last_os_error()
        );
        RelayTlsLock { file }
    }
}

impl Drop for RelayTlsLock {
    fn drop(&mut self) {
        let fd = self.file.as_raw_fd();
        let _ = unsafe { flock(fd, LOCK_UN) };
    }
}

const LOCK_EX: i32 = 2;
const LOCK_UN: i32 = 8;

unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
    fn setsid() -> i32;
}

fn libc_setsid() -> i32 {
    unsafe { setsid() }
}
