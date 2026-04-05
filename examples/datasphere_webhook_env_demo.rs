// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Datasphere Webhook Environment Demo — Datasphere via webhook with env config.
//!
//! Environment:
//!   DATASPHERE_WEBHOOK_URL — webhook URL for Datasphere search
//!   DATASPHERE_API_KEY     — API key

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;
use std::env;

fn main() {
    let webhook_url = env::var("DATASPHERE_WEBHOOK_URL")
        .unwrap_or_else(|_| "https://example.signalwire.com/api/datasphere/search".into());
    let api_key = env::var("DATASPHERE_API_KEY")
        .unwrap_or_else(|_| "your-api-key".into());

    let mut agent = AgentBase::new(AgentOptions {
        name: "datasphere-webhook-env".to_string(),
        route: Some("/datasphere-webhook".to_string()),
        ..AgentOptions::new("datasphere-webhook-env")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a knowledge assistant backed by Datasphere webhooks.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Use the search_knowledge function to find information",
        "Summarize search results clearly",
    ]);

    // Webhook-based SWAIG tool for Datasphere
    let url = webhook_url.clone();
    let key = api_key.clone();
    agent.define_tool(
        "search_knowledge",
        "Search the Datasphere knowledge base via webhook",
        json!({
            "query": {"type": "string", "description": "Search query"},
            "max_results": {"type": "integer", "description": "Maximum results (default 5)"}
        }),
        Box::new(move |args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let max = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5);
            // In production, this would make an HTTP request to the webhook URL
            FunctionResult::with_response(&format!(
                "Datasphere search for '{query}' (max {max} results) via {}: [simulated results]",
                url
            ))
        }),
        false,
    );

    println!("Datasphere webhook demo at http://localhost:3000/datasphere-webhook");
    println!("  Webhook URL: {webhook_url}");
    agent.run();
}
