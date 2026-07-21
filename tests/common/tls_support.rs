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
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Dedicated TLS-mode ports, disjoint from the plain-mock default slots
/// (`mock_signalwire` 8771; `mock_relay` WS 8781 / HTTP 9781).
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
            joined.push(pythonpath_separator());
            joined.push(ev);
            joined
        }
        _ => std::ffi::OsString::from(&pkg_dir),
    };
    cmd.env("PYTHONPATH", new_pp);

    // Detach into its own session so the test binary's exit doesn't orphan it
    // weirdly; the Drop handler kills it explicitly (unix only — windows is a
    // no-op, see detach_process_group).
    detach_process_group(&mut cmd);
    cmd.spawn().ok()
}

#[cfg(unix)]
fn pythonpath_separator() -> &'static str {
    ":"
}

#[cfg(windows)]
fn pythonpath_separator() -> &'static str {
    ";"
}

// Detach the spawned TLS mock into its own process group (unix) or spawn it plainly
// (windows). The Windows CI leg compiles this test crate, so the unix-only
// pre_exec/setsid path must be cfg-gated. On Windows the TlsMockProc Drop handler's
// explicit `.kill()` is the cleanup; no Job Object is needed as the mock forks no
// grandchildren.
#[cfg(unix)]
fn detach_process_group(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: setsid() is async-signal-safe and the closure does nothing else.
    unsafe {
        cmd.pre_exec(|| {
            let _ = libc_setsid();
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn detach_process_group(_cmd: &mut Command) {}

// libc setsid binding (unix only), without pulling in the libc crate.
#[cfg(unix)]
unsafe extern "C" {
    fn setsid() -> i32;
}

#[cfg(unix)]
fn libc_setsid() -> i32 {
    unsafe { setsid() }
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
            && v.get(expect_key).is_some()
        {
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
            && v.get(expect_key).is_some()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

// ---------------------------------------------------------------------------
// Cross-binary advisory lock for the TLS mock-relay instance
// ---------------------------------------------------------------------------

/// Path to the cross-binary lock file. Derived from the OS temp dir so it is
/// portable (POSIX `/tmp`, Windows `%TEMP%`) rather than a hardcoded `/tmp`.
fn relay_tls_lock_path() -> PathBuf {
    std::env::temp_dir().join("signalwire-rust-mock-relay-tls.lock")
}

/// Exclusive cross-binary advisory lock guarding the dedicated TLS mock-relay
/// instance so two concurrently-run test binaries can't both drive the same WSS
/// mock session registry. Uses the OS's whole-file advisory lock: `flock` on unix,
/// `LockFileEx` on Windows.
pub struct RelayTlsLock {
    file: std::fs::File,
}

impl RelayTlsLock {
    pub fn acquire() -> Self {
        let path = relay_tls_lock_path();
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .unwrap_or_else(|e| panic!("tls_support: open {}: {e}", path.display()));
        lock_exclusive(&file);
        RelayTlsLock { file }
    }
}

impl Drop for RelayTlsLock {
    fn drop(&mut self) {
        unlock(&self.file);
    }
}

// --- unix: flock(LOCK_EX / LOCK_UN) ----------------------------------------
#[cfg(unix)]
fn lock_exclusive(file: &std::fs::File) {
    use std::os::unix::io::AsRawFd;
    const LOCK_EX: i32 = 2;
    let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX) };
    assert!(
        rc == 0,
        "tls_support: flock LOCK_EX: {}",
        std::io::Error::last_os_error()
    );
}

#[cfg(unix)]
fn unlock(file: &std::fs::File) {
    use std::os::unix::io::AsRawFd;
    const LOCK_UN: i32 = 8;
    let _ = unsafe { flock(file.as_raw_fd(), LOCK_UN) };
}

#[cfg(unix)]
unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

// --- windows: LockFileEx / UnlockFileEx ------------------------------------
// Whole-file exclusive lock over a large byte range, the Win32 analog of flock.
#[cfg(windows)]
fn lock_exclusive(file: &std::fs::File) {
    use std::os::windows::io::AsRawHandle;
    const LOCKFILE_EXCLUSIVE_LOCK: u32 = 0x0000_0002;
    let mut overlapped = [0u8; OVERLAPPED_SIZE];
    let ok = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK,
            0,
            u32::MAX,
            u32::MAX,
            overlapped.as_mut_ptr(),
        )
    };
    assert!(
        ok != 0,
        "tls_support: LockFileEx: {}",
        std::io::Error::last_os_error()
    );
}

#[cfg(windows)]
fn unlock(file: &std::fs::File) {
    use std::os::windows::io::AsRawHandle;
    let mut overlapped = [0u8; OVERLAPPED_SIZE];
    let _ = unsafe {
        UnlockFileEx(
            file.as_raw_handle(),
            0,
            u32::MAX,
            u32::MAX,
            overlapped.as_mut_ptr(),
        )
    };
}

// A zeroed OVERLAPPED is all LockFileEx needs for a blocking whole-file lock (no
// offset, no completion event). Its size on x64 Windows is 32 bytes; we pass a
// zeroed buffer of that size rather than pulling in the winapi/windows crate.
#[cfg(windows)]
const OVERLAPPED_SIZE: usize = 32;

#[cfg(windows)]
unsafe extern "system" {
    fn LockFileEx(
        handle: std::os::windows::raw::HANDLE,
        flags: u32,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut u8,
    ) -> i32;
    fn UnlockFileEx(
        handle: std::os::windows::raw::HANDLE,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut u8,
    ) -> i32;
}
