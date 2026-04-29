// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! swmlservice_swaig_standalone — proves that `signalwire::swml::Service`
//! can host SWAIG functions on its own `/swaig` endpoint with NO
//! `signalwire::agent::AgentBase` involved.
//!
//! This is the path you take when you want a SWAIG-callable HTTP service
//! that isn't an `<ai>` agent: the SWAIG verb is a generic LLM-tool
//! surface and `Service` is the host. `AgentBase` is just a `Service`
//! decorator (via `Deref<Target=Service>`) that *also* layers in
//! prompts, AI config, dynamic config, and token validation.
//!
//! Run:
//!     cargo run --example swmlservice_swaig_standalone
//!
//! Then exercise the endpoints (Basic auth user/pass come from
//! `SWML_BASIC_AUTH_USER` / `SWML_BASIC_AUTH_PASSWORD` env vars or the
//! auto-generated values logged at startup):
//!     curl -u user:pass http://localhost:3000/standalone
//!     curl -u user:pass http://localhost:3000/standalone/swaig \
//!         -H 'Content-Type: application/json' \
//!         -d '{"function":"lookup_competitor","argument":{"parsed":[{"competitor":"ACME"}]}}'
//!
//! Or drive it through the SDK CLI without standing up the server:
//!     swaig-test --url http://user:pass@localhost:3000/standalone --list-tools
//!     swaig-test --url http://user:pass@localhost:3000/standalone \
//!         --exec lookup_competitor --param competitor=ACME

use signalwire::swaig::FunctionResult;
use signalwire::swml::service::{Service, ServiceOptions};
use serde_json::json;

fn main() {
    let mut service = Service::new(ServiceOptions {
        name: "standalone-swaig".to_string(),
        route: Some("/standalone".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(3000),
        basic_auth_user: None,
        basic_auth_password: None,
    });

    // 1. Build a minimal SWML document. Any verbs are fine — the SWAIG
    //    HTTP surface is independent of what the document contains.
    service.add_verb("answer", "main", json!({}));
    service.add_verb("hangup", "main", json!({}));

    // 2. Register a SWAIG function. `define_tool` lives on `Service`,
    //    not just `AgentBase`. The handler receives parsed arguments
    //    plus the raw POST body.
    //
    //    Signature mirrored exactly from the canonical inline test
    //    `test_service_define_tool_dispatches_via_on_function_call` in
    //    `src/swml/service.rs`.
    service.define_tool(
        "lookup_competitor",
        "Look up competitor pricing by company name. Use this when the \
         user asks how a competitor's price compares to ours.",
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
                "{} pricing is $99/seat; we're $79/seat.",
                competitor
            ))
        }),
        false, // standalone services don't validate session tokens by default
    );

    println!("=== SWML document ===");
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
