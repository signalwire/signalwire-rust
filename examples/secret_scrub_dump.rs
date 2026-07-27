// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `secret_scrub_dump` — the Rust port's SECRET-SCRUB-LIVE Layer-D dump program
//! for the cross-port behavioral differ
//! (porting-sdk `scripts/diff_port_secret_scrub.py`).
//!
//! The differ builds the golden per-sentinel classification by driving the python
//! reference through a real RELAY connect + an inbound
//! `signalwire.authorization.state` re-auth frame at `SIGNALWIRE_LOG_LEVEL=debug`
//! with the fixture sentinels (project=`PJ-TESTLEAK` / token=`PT-TESTLEAK` /
//! `authorization_state`=`AENC-TESTLEAK`) and observing that NONE appear in the
//! captured debug output — python scrubs, so every golden is `{leaked: false}`.
//! This program does the identical drive against the Rust SDK, captures the SDK's
//! OWN debug output, and emits per-sentinel `{"<id>": {"leaked": <bool>}}` on
//! stdout.
//!
//! A port that logged the raw frame verbatim would surface the sentinel in its
//! captured output → `leaked: true` → the differ reds. With
//! `relay::client::scrub_frame_value` / `scrub_frame_raw` masking the credential
//! and `authorization_state` VALUES at both frame-log sites, every sentinel comes
//! back `leaked: false`.
//!
//! ## Two-phase design (why)
//!
//! The SDK `Logger` writes with `eprintln!` — straight to the process's real fd 2.
//! An in-process capture would have to dup2 fd 2 (a libc dependency this crate
//! does not carry). So this program runs the actual drive in a CHILD invocation of
//! ITSELF (`SW_SS_CHILD=1`) whose stderr is a pipe. The PARENT reads that pipe
//! (the SDK's real debug output), classifies each sentinel, forwards the bytes on
//! to the real stderr (so the differ's own subprocess-stderr capture sees exactly
//! what we classified), and prints the JSON to stdout. The child prints nothing to
//! stdout. Same shape as the php port's `bin/secret-scrub-dump`.
//!
//! The child inherits `MOCK_RELAY_PORT` / `MOCK_RELAY_HTTP_PORT` from the parent,
//! which resolves the mock ONCE — so parent and child share a single `mock_relay`
//! instance rather than each spawning one.
//!
//! Protocol: stdout = ONE JSON object mapping corpus id -> `{leaked: bool}`.
//!
//! Run from the repo root: `cargo run --quiet --example secret_scrub_dump`.

#[path = "../tests/common/mod.rs"]
mod common;

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};
use signalwire::relay::Client;

use common::relay_mocktest;

// Fixture sentinels — byte-identical to porting-sdk/scripts/secret_scrub_corpus.py.
const SS_PROJECT: &str = "PJ-TESTLEAK";
const SS_TOKEN: &str = "PT-TESTLEAK";
const SS_AUTHORIZATION_STATE: &str = "AENC-TESTLEAK";

/// corpus id => the sentinel string that must NOT appear in the debug log.
const SS_CORPUS: [(&str, &str); 3] = [
    ("project", SS_PROJECT),
    ("token", SS_TOKEN),
    ("authorization_state", SS_AUTHORIZATION_STATE),
];

/// How long to pump the read loop waiting for the pushed re-auth frame to be
/// received and logged. Bounded — this must never hang.
const PUMP_BUDGET: Duration = Duration::from_millis(1500);

fn main() {
    if std::env::var_os("SW_SS_CHILD").is_some() {
        child_drive();
    } else {
        parent();
    }
}

/// PARENT: resolve the mock once, spawn the child drive with stderr piped, read
/// the SDK's real debug output, classify each sentinel, forward the bytes to the
/// real stderr, and print the JSON classification to stdout.
fn parent() {
    // Resolve (spawning if needed) the mock ONCE here, then hand its ports to the
    // child so both processes talk to the same instance. The mock's parent-death
    // watchdog reaps it when this process exits.
    let h = relay_mocktest::harness();

    let exe = std::env::current_exe().expect("secret_scrub_dump: current_exe");
    let child = Command::new(exe)
        .env("SW_SS_CHILD", "1")
        .env("SIGNALWIRE_LOG_LEVEL", "debug")
        .env("SIGNALWIRE_LOG_MODE", "default")
        .env("MOCK_RELAY_PORT", h.ws_port.to_string())
        .env("MOCK_RELAY_HTTP_PORT", h.http_port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            eprintln!("secret_scrub_dump: failed to spawn child drive: {e}");
            std::process::exit(1);
        }
    };

    let mut child_out = String::new();
    let mut child_err = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut child_out);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut child_err);
    }
    let _ = child.wait();

    // Forward the child's output so the differ's own subprocess capture sees the
    // identical bytes we classified.
    eprint!("{child_err}");
    eprint!("{child_out}");

    // Classify off the SDK's real debug output. The child's stdout is folded in
    // too, so a port that logged to stdout instead could not hide a leak here.
    let log = format!("{child_err}{child_out}");
    let mut out = Map::new();
    for (id, sentinel) in SS_CORPUS {
        out.insert(id.to_string(), json!({"leaked": log.contains(sentinel)}));
    }
    println!("{}", Value::Object(out));
}

/// CHILD: drive a real RELAY connect with the sentinel credentials (so the
/// outbound `signalwire.connect` frame — the `>>` log site — actually carries
/// them) plus an inbound `signalwire.authorization.state` event carrying the
/// sentinel blob (the `<<` log site's payload), at debug level. Everything the SDK
/// emits goes to stderr; stdout stays empty. The parent classifies from our
/// stderr.
fn child_drive() {
    let _g = relay_mocktest::begin();
    // Point `connect()` at the mock (MOCK_RELAY_PORT/_HTTP_PORT inherited from the
    // parent resolve to the parent's already-running instance).
    relay_mocktest::ensure_redirect();
    let h = relay_mocktest::harness();

    let client = Arc::new(Client::new(SS_PROJECT, SS_TOKEN, &h.relay_host));
    {
        let mut ctx = client.contexts.lock().unwrap();
        ctx.push("default".to_string());
    }
    if let Err(e) = client.connect() {
        eprintln!("secret_scrub_dump: connect failed: {e}");
        std::process::exit(1);
    }
    relay_mocktest::scope_to_client(&client);

    // Push an inbound re-auth blob and pump the read loop so the frame is
    // received AND logged through the `<<` site.
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "method": "signalwire.event",
        "params": {
            "event_type": "signalwire.authorization.state",
            "params": {"authorization_state": SS_AUTHORIZATION_STATE},
        },
    }));

    // Wait until the SDK has actually absorbed the pushed blob (the read loop
    // stores it on `authorization_state`), bounded so this can never hang. A
    // fixed sleep would let a slow machine classify a frame that was never
    // logged — the vacuity this gate exists to prevent.
    let deadline = Instant::now() + PUMP_BUDGET;
    while Instant::now() < deadline {
        if client.authorization_state.lock().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if client.authorization_state.lock().unwrap().is_none() {
        eprintln!(
            "secret_scrub_dump: the pushed authorization.state frame was never \
             absorbed within {PUMP_BUDGET:?} — the << log site was not driven, so \
             a `leaked: false` would be VACUOUS"
        );
        client.disconnect();
        std::process::exit(1);
    }

    client.disconnect();
}
