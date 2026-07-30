// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Advanced `DataMap` — expressions, webhooks, auth headers, fallback chains.

use serde_json::json;
use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;

fn main() {
    // Expression demo with test values and patterns
    let mut command_processor = DataMap::new("command_processor");
    command_processor
        .description("Process user commands with pattern matching")
        .parameter(
            "command",
            "string",
            "User command to process",
            Some(true),
            None,
        )
        .parameter("target", "string", "Optional target", None, None)
        .expression(
            "${args.command}",
            r"^start",
            FunctionResult::with_response("Starting process: ${args.target}").to_value(),
            None,
        )
        .expression(
            "${args.command}",
            r"^stop",
            FunctionResult::with_response("Stopping process: ${args.target}").to_value(),
            None,
        )
        .expression(
            "${args.command}",
            r"^status",
            FunctionResult::with_response("Checking status of: ${args.target}").to_value(),
            Some(
                FunctionResult::with_response(
                    "Unknown command: ${args.command}. Try start, stop, or status.",
                )
                .to_value(),
            ),
        );

    println!("Command processor DataMap:");
    println!(
        "{}",
        serde_json::to_string_pretty(&command_processor.to_swaig_function()).unwrap()
    );

    // Advanced webhook with auth headers
    let mut api_tool = DataMap::new("advanced_api_tool");
    api_tool
        .description("API tool with advanced webhook features")
        .parameter("action", "string", "Action to perform", Some(true), None)
        .parameter("data", "string", "Data to send", None, None)
        .webhook(
            "POST",
            "https://api.example.com/advanced",
            Some(json!({
                "Authorization": "Bearer ${env.API_TOKEN}",
                "User-Agent": "SignalWire-Agent/1.0"
            })),
            None,
            None,
            None)
        .params(json!({"action": "${args.action}", "data": "${args.data}"}))
        .webhook_expressions(vec![
            json!({
                "string": "${response.status}",
                "pattern": "^success$",
                "output": FunctionResult::with_response("Operation completed successfully.").to_value(),
            }),
            json!({
                "string": "${response.error_code}",
                "pattern": "^(404|500)$",
                "output": FunctionResult::with_response("API Error: ${response.error_message}").to_value(),
            }),
        ]);

    println!("\nAdvanced API tool DataMap:");
    println!(
        "{}",
        serde_json::to_string_pretty(&api_tool.to_swaig_function()).unwrap()
    );

    // Knowledge search with auth headers and foreach
    let mut search_tool = DataMap::new("knowledge_search");
    search_tool
        .description("Search the knowledge base")
        .parameter("query", "string", "Search query", Some(true), None)
        .webhook(
            "POST",
            "https://api.example.com/search",
            Some(json!({
                "Authorization": "Bearer ${env.KB_API_KEY}",
                "Content-Type": "application/json"
            })),
            None,
            None,
            None,
        )
        .params(json!({"query": "${args.query}", "limit": 5}))
        .for_each(json!({
            "input_key": "results",
            "output_key": "items",
        }))
        .output(FunctionResult::with_response("Found: ${item.title} — ${item.snippet}").to_value());

    println!("\nKnowledge search DataMap:");
    println!(
        "{}",
        serde_json::to_string_pretty(&search_tool.to_swaig_function()).unwrap()
    );
}
