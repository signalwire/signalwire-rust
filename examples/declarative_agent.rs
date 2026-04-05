// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Declarative Agent — prompt defined as structured data, not procedural calls.
//!
//! Demonstrates building a complete agent from a static prompt definition,
//! then adding tools programmatically.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

/// Prompt sections defined declaratively.
fn prompt_sections() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    vec![
        (
            "Personality",
            "You are a friendly and helpful AI assistant who responds in a casual, conversational tone.",
            vec![],
        ),
        (
            "Goal",
            "Help users with their questions about time and weather.",
            vec![],
        ),
        (
            "Instructions",
            "",
            vec![
                "Be concise and direct in your responses.",
                "If you don't know something, say so clearly.",
                "Use the get_time function when asked about the current time.",
                "Use the get_weather function when asked about the weather.",
            ],
        ),
    ]
}

fn main() {
    let mut agent = AgentBase::new(AgentOptions::new("declarative-agent"));

    agent.add_language("English", "en-US", "rime.spore");

    // Apply declarative prompt sections
    for (title, body, bullets) in prompt_sections() {
        agent.prompt_add_section(title, body, bullets);
    }

    // Tools
    agent.define_tool(
        "get_time",
        "Get the current time",
        json!({}),
        Box::new(|_args, _raw| {
            let now = chrono::Local::now().format("%I:%M %p on %A, %B %e, %Y");
            FunctionResult::with_response(&format!("The current time is {now}."))
        }),
        false,
    );

    agent.define_tool(
        "get_weather",
        "Get the weather for a location",
        json!({
            "location": {"type": "string", "description": "City name"}
        }),
        Box::new(|args, _raw| {
            let location = args
                .get("location")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown location");
            FunctionResult::with_response(&format!(
                "It is currently 72F and sunny in {location}."
            ))
        }),
        false,
    );

    agent.run();
}
