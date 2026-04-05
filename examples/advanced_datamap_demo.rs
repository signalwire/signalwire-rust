// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Advanced DataMap — expressions, webhooks, auth headers, fallback chains.

use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    // Expression demo with test values and patterns
    let command_processor = DataMap::new("command_processor")
        .description("Process user commands with pattern matching")
        .parameter("command", "string", "User command to process", true)
        .parameter("target", "string", "Optional target", false)
        .expression(
            "${args.command}",
            r"^start",
            FunctionResult::with_response("Starting process: ${args.target}"),
        )
        .expression(
            "${args.command}",
            r"^stop",
            FunctionResult::with_response("Stopping process: ${args.target}"),
        )
        .expression_with_nomatch(
            "${args.command}",
            r"^status",
            FunctionResult::with_response("Checking status of: ${args.target}"),
            FunctionResult::with_response("Unknown command: ${args.command}. Try start, stop, or status."),
        )
        .build();

    println!("Command processor DataMap:");
    println!("{}", serde_json::to_string_pretty(&command_processor).unwrap());

    // Advanced webhook with auth headers
    let api_tool = DataMap::new("advanced_api_tool")
        .description("API tool with advanced webhook features")
        .parameter("action", "string", "Action to perform", true)
        .parameter("data", "string", "Data to send", false)
        .webhook(
            "POST",
            "https://api.example.com/advanced",
            json!({"action": "${args.action}", "data": "${args.data}"}),
            json!({
                "Authorization": "Bearer ${env.API_TOKEN}",
                "User-Agent": "SignalWire-Agent/1.0"
            }),
        )
        .webhook_expression(
            "${response.status}",
            "^success$",
            FunctionResult::with_response("Operation completed successfully."),
        )
        .webhook_expression(
            "${response.error_code}",
            "^(404|500)$",
            FunctionResult::with_response("API Error: ${response.error_message}"),
        )
        .build();

    println!("\nAdvanced API tool DataMap:");
    println!("{}", serde_json::to_string_pretty(&api_tool).unwrap());

    // Knowledge search with auth headers and foreach
    let search_tool = DataMap::new("knowledge_search")
        .description("Search the knowledge base")
        .parameter("query", "string", "Search query", true)
        .webhook(
            "POST",
            "https://api.example.com/search",
            json!({"query": "${args.query}", "limit": 5}),
            json!({
                "Authorization": "Bearer ${env.KB_API_KEY}",
                "Content-Type": "application/json"
            }),
        )
        .foreach(
            "response.results",
            "item",
            FunctionResult::with_response("Found: ${item.title} — ${item.snippet}"),
        )
        .build();

    println!("\nKnowledge search DataMap:");
    println!("{}", serde_json::to_string_pretty(&search_tool).unwrap());
}
