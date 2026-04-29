// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Simple Dynamic Agent — same output as the static version, but configured per-request.
//!
//! Demonstrates the dynamic configuration pattern: no static setup in the constructor;
//! everything happens in the callback, called fresh for every request.

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;
use std::sync::Arc;

fn main() {
    let mut opts = AgentOptions::new("Simple Customer Service Agent (Dynamic)");
    opts.auto_answer = true;
    opts.record_call = true;

    let mut agent = AgentBase::new(opts);

    // Dynamic configuration — called for every inbound request
    agent.set_dynamic_config_callback(Box::new(
        |_query_params, _body_params, _headers, agent| {
            // Voice
            agent.add_language("English", "en-US", "inworld.Mark");

            // AI params
            agent.set_params(json!({
                "ai_model": "gpt-4.1-nano",
                "end_of_speech_timeout": 500,
                "attention_timeout": 15000,
                "background_file_volume": -20,
            }));

            // Hints
            agent.add_hints(vec!["SignalWire", "SWML", "API", "webhook", "SIP"]);

            // Global data
            agent.set_global_data(json!({
                "agent_type": "customer_service",
                "service_level": "standard",
            }));

            // Prompt
            agent.prompt_add_section(
                "Role",
                "You are a friendly and professional customer service representative.",
                vec![],
            );
            agent.prompt_add_section("Instructions", "", vec![
                "Greet the customer warmly",
                "Be helpful and concise",
                "If you cannot help, offer to connect with a human",
            ]);
        },
    ));

    println!("Dynamic agent running at http://localhost:3000/");
    agent.run();
}
