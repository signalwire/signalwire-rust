// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Multi-Endpoint Agent — SWML + web UI + API endpoints on one server.
//!
//! Endpoints:
//!   /swml   — Voice AI SWML endpoint
//!   /       — Web UI (hello world)
//!   /api    — JSON API

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "multi-endpoint".to_string(),
        route: Some("/swml".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(8080),
        ..AgentOptions::new("multi-endpoint")
    });

    // Voice AI configuration
    agent.add_language("English", "en-US", "inworld.Mark");
    agent.prompt_add_section("Role", "You are a helpful voice assistant.", vec![]);
    agent.prompt_add_section("Instructions", "", vec![
        "Greet callers warmly",
        "Be concise in your responses",
        "Use the available functions when appropriate",
    ]);

    agent.define_tool(
        "get_time",
        "Get the current time",
        json!({}),
        Box::new(|_args, _raw| {
            let now = chrono::Local::now().format("%I:%M %p");
            FunctionResult::with_response(&format!("The current time is {now}."))
        }),
        false,
    );

    // Note: Additional non-SWML endpoints (/api, / web UI) would be added
    // by overriding get_app() or using a custom router alongside the agent.
    // The agent's SWML is served at /swml, SWAIG at /swml/swaig.

    println!("Multi-endpoint agent:");
    println!("  SWML:  http://localhost:8080/swml");
    println!("  SWAIG: http://localhost:8080/swml/swaig");
    agent.run();
}
