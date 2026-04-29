// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! swmlservice_ai_sidecar — proves that `signalwire::swml::Service` can
//! emit the `ai_sidecar` verb, register SWAIG tools the sidecar's LLM
//! can call, and dispatch them end-to-end — without any
//! `signalwire::agent::AgentBase` code path.
//!
//! The `ai_sidecar` verb runs an AI listener alongside an in-progress
//! call (real-time copilot, transcription analyzer, compliance monitor,
//! etc.). It is NOT an agent — it does not own the call. So the right
//! host is `Service`, not `AgentBase`.
//!
//! `ai_sidecar` is not in the SDK's verb schema (`add_verb` would
//! panic), so we go through `document_mut().add_verb_to_section(...)`
//! to write the verb directly. This is the canonical pattern for
//! emitting platform-side verbs that pre-date the SDK's bundled schema
//! — same approach used by the inline regression test
//! `test_sidecar_pattern_emits_verb_and_registers_tool` in
//! `src/swml/service.rs`.
//!
//! What this serves:
//!     GET  /sales-sidecar          → SWML doc containing the ai_sidecar verb
//!     POST /sales-sidecar/swaig    → SWAIG tool dispatch (used by the sidecar's LLM)
//!
//! Run:
//!     cargo run --example swmlservice_ai_sidecar
//!
//! Drive the SWAIG path through the SDK CLI:
//!     swaig-test --url http://user:pass@localhost:3000/sales-sidecar --list-tools
//!     swaig-test --url http://user:pass@localhost:3000/sales-sidecar \
//!         --exec lookup_competitor --param competitor=ACME

use signalwire::swaig::FunctionResult;
use signalwire::swml::service::{Service, ServiceOptions};
use serde_json::json;

fn main() {
    // In production, set this to your externally reachable URL so the
    // sidecar's LLM can POST tool calls back to /swaig.
    let public_url = "https://your-host.example.com/sales-sidecar";

    let mut service = Service::new(ServiceOptions {
        name: "sales-sidecar".to_string(),
        route: Some("/sales-sidecar".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(3000),
        basic_auth_user: None,
        basic_auth_password: None,
    });

    // 1. Emit SWML — answer, ai_sidecar, hangup.
    //
    //    `add_verb` validates against the bundled schema and panics on
    //    unknown verbs. `ai_sidecar` post-dates that schema, so we
    //    route the verb through `document_mut().add_verb_to_section`,
    //    which accepts any verb dict. (Same approach as
    //    `test_sidecar_pattern_emits_verb_and_registers_tool`.)
    service.add_verb("answer", "main", json!({}));
    service.document_mut().add_verb_to_section(
        "main",
        "ai_sidecar",
        json!({
            // Required: prompt + lang.
            "prompt": "You are a real-time sales copilot. Listen to the \
                       call and surface competitor pricing comparisons \
                       when relevant.",
            "lang": "en-US",
            // Listen to both legs.
            "direction": ["remote-caller", "local-caller"],
            // Where the sidecar POSTs lifecycle/transcription events.
            // Optional — omit if you don't need an event sink.
            "url": format!("{}/events", public_url),
            // Where the sidecar's LLM POSTs SWAIG tool calls. The
            // SDK's /swaig route is what answers them. SWAIG must be
            // UPPERCASE in this verb's schema.
            "SWAIG": {
                "defaults": {
                    "web_hook_url": format!("{}/swaig", public_url)
                }
            }
        }),
    );
    service.add_verb("hangup", "main", json!({}));

    // 2. Register tools the sidecar's LLM can call. Same `define_tool`
    //    you'd use on `AgentBase` — it lives on `Service`. Dispatch
    //    pattern lifted from
    //    `test_service_define_tool_dispatches_via_on_function_call`.
    service.define_tool(
        "lookup_competitor",
        "Look up competitor pricing by company name. The sidecar should \
         call this whenever the caller mentions a competitor.",
        json!({
            "competitor": {
                "type": "string",
                "description": "The competitor's company name, e.g. 'ACME'."
            }
        }),
        Box::new(|args, _raw| {
            let competitor = args
                .get("competitor")
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>");
            FunctionResult::with_response(&format!(
                "Pricing for {}: $99/seat. Our equivalent plan is $79/seat \
                 with the same SLA.",
                competitor
            ))
        }),
        false,
    );

    // 3. (Optional) Mount an event sink for ai_sidecar lifecycle
    //    events at POST /sales-sidecar/events.
    //
    //    The Rust SDK's Service does not yet expose a
    //    `register_routing_callback` hook (the Python-side equivalent
    //    of mounting an arbitrary sub-route handler). Until that
    //    surface lands, sidecar events can be observed by pointing the
    //    `url` field above at any HTTP listener of your choice — for
    //    example a separate `Service` instance on a different port, a
    //    static logger, or your own tiny_http server. Documenting the
    //    gap explicitly here so the example doesn't ship a fake hook.

    println!("=== SWML document (with ai_sidecar verb) ===");
    println!("{}", service.render_pretty());
    println!();
    println!("=== Registered SWAIG tools ===");
    for name in service.list_tool_names() {
        println!("  - {}", name);
    }
    println!();
    let (user, pass) = service.basic_auth_credentials();
    println!(
        "Serving at http://localhost:{}{} (Basic auth: {}:{})",
        service.port(),
        service.route(),
        user,
        pass
    );

    service.run();
}
