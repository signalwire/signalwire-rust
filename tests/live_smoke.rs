//! Real-server smoke test (plan §6.5) — the ONLY check that exercises the SDK
//! against the live SignalWire platform, catching mock↔production drift the
//! spec-sourcing pipeline hasn't closed.
//!
//! OFF by default: every test here is `#[ignore]`d, so `cargo test` (local + the
//! per-PR CI) never runs them. They run only when BOTH:
//!
//! * `SWSDK_LIVE_TESTS=1` is set (the opt-in convention), AND
//! * real credentials are present (`SIGNALWIRE_PROJECT_ID` /
//!   `SIGNALWIRE_API_TOKEN` / `SIGNALWIRE_SPACE`).
//!
//! The dedicated nightly workflow (`.github/workflows/live-smoke.yml`) sets both
//! and runs `cargo test --test live_smoke -- --ignored`. When creds are absent
//! each test SKIPS cleanly (returns early with a printed note) rather than
//! failing — a credentialed-skip, not a red.

use std::collections::HashMap;

use serde_json::json;
use signalwire::rest::RestClient;
use signalwire::swml::service::{Service, ServiceOptions};

/// True only when the live lane is explicitly opted into.
fn live_enabled() -> bool {
    std::env::var("SWSDK_LIVE_TESTS").as_deref() == Ok("1")
}

/// Return the three creds, or `None` (→ clean skip) if any is missing/empty.
fn creds() -> Option<(String, String, String)> {
    let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    Some((
        get("SIGNALWIRE_PROJECT_ID")?,
        get("SIGNALWIRE_API_TOKEN")?,
        get("SIGNALWIRE_SPACE")?,
    ))
}

/// Guard shared by every live test: skip cleanly unless opted-in AND credentialed.
/// Returns the built client, or `None` to signal "skip".
fn live_client() -> Option<RestClient> {
    if !live_enabled() {
        eprintln!("live_smoke: SWSDK_LIVE_TESTS!=1 — skipping (opt-in only)");
        return None;
    }
    let Some((project, token, space)) = creds() else {
        eprintln!("live_smoke: credentials absent — skipping (credentialed-skip, not a failure)");
        return None;
    };
    match RestClient::new(&project, &token, &space) {
        Ok(c) => Some(c),
        Err(e) => panic!("live_smoke: RestClient::new failed with real creds: {e}"),
    }
}

/// Auth + one REST list against the live platform.
#[test]
#[ignore = "live: opt-in via SWSDK_LIVE_TESTS=1 + real credentials"]
fn live_rest_list() {
    let Some(client) = live_client() else { return };
    // A cheap, universally-available list: phone numbers. A successful call
    // proves auth + a real REST round trip. An error here is a genuine failure
    // (creds are present by construction).
    let page = client
        .phone_numbers()
        .list(&HashMap::new(), None)
        .expect("live REST phone_numbers.list() must succeed with valid creds");
    // The platform returns a JSON object/array envelope; assert we got JSON back.
    assert!(
        page.is_object() || page.is_array(),
        "expected a JSON list envelope, got: {page}"
    );
}

/// One SWML render — a pure local render, but exercised in the live lane so the
/// smoke covers the full SWML → JSON path alongside the wire calls.
#[test]
#[ignore = "live: opt-in via SWSDK_LIVE_TESTS=1 + real credentials"]
fn live_swml_render() {
    if !live_enabled() {
        eprintln!("live_smoke: SWSDK_LIVE_TESTS!=1 — skipping");
        return;
    }
    let mut svc = Service::new(ServiceOptions::new("live-smoke"));
    svc.document_mut()
        .add_verb("answer", json!({ "max_duration": 3600 }));
    let rendered = svc.render();
    assert!(
        rendered.contains("answer"),
        "SWML render must contain the answer verb: {rendered}"
    );
}
