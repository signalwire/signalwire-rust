// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! DataMap Demo — server-side API tools without webhooks.
//!
//! Shows:
//! 1. Simple API call (weather)
//! 2. Expression-based pattern matching
//! 3. Regular SWAIG tool for comparison

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "datamap-demo".to_string(),
        route: Some("/datamap-demo".to_string()),
        ..AgentOptions::new("datamap-demo")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a demo assistant that shows DataMap and regular tool capabilities.",
        vec![],
    );

    // Regular SWAIG tool for comparison
    agent.define_tool(
        "echo_test",
        "A simple echo function for testing",
        json!({
            "message": {"type": "string", "description": "Message to echo back"},
            "repeat": {"type": "integer", "description": "Number of times to repeat"}
        }),
        Box::new(|args, _raw| {
            let msg = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
            let repeat = args.get("repeat").and_then(|v| v.as_u64()).unwrap_or(1);
            let output = (0..repeat).map(|_| msg).collect::<Vec<_>>().join(" ");
            FunctionResult::with_response(&output)
        }),
        false,
    );

    // DataMap tool: weather API (no webhook needed)
    let weather = DataMap::new("get_weather")
        .description("Get the current weather for a city")
        .parameter("city", "string", "City name", true)
        .webhook(
            "GET",
            "https://api.weatherapi.com/v1/current.json",
            json!({"key": "demo", "q": "${args.city}"}),
            json!({}),
        )
        .output(FunctionResult::with_response(
            "The weather in ${args.city} is ${response.current.condition.text}, \
             temperature ${response.current.temp_f}F.",
        ))
        .build();
    agent.define_datamap_tool(weather);

    // DataMap tool: expression-based command processor
    let commands = DataMap::new("process_command")
        .description("Process a user command")
        .parameter("command", "string", "Command to process", true)
        .expression(
            "${args.command}",
            r"^start",
            FunctionResult::with_response("Starting the process."),
        )
        .expression(
            "${args.command}",
            r"^stop",
            FunctionResult::with_response("Stopping the process."),
        )
        .expression_with_nomatch(
            "${args.command}",
            r"^status",
            FunctionResult::with_response("Current status: running."),
            FunctionResult::with_response("Unknown command. Try start, stop, or status."),
        )
        .build();
    agent.define_datamap_tool(commands);

    agent.run();
}
