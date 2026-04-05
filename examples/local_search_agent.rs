// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Local Search Agent — search local documents with graceful fallback.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "local-search-demo".to_string(),
        route: Some("/search-demo".to_string()),
        ..AgentOptions::new("local-search-demo")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Personality",
        "You are a helpful assistant with access to local document search.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Use the search_documents function to find relevant information",
        "Provide helpful answers based on the search results",
        "If no results are found, let the user know politely",
    ]);

    // Simulated search tool
    agent.define_tool(
        "search_documents",
        "Search the local document index for relevant information",
        json!({
            "query": {"type": "string", "description": "Search query"},
            "max_results": {"type": "integer", "description": "Maximum results to return"}
        }),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let max = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(3);

            // Simulated search results
            let results = vec![
                ("Getting Started Guide", "Installation steps and first configuration.", 0.95),
                ("API Reference", "Complete type and method reference.", 0.87),
                ("Troubleshooting FAQ", "Common issues and solutions.", 0.82),
            ];

            let relevant: Vec<_> = results
                .iter()
                .filter(|(title, _, _)| {
                    title.to_lowercase().contains(&query.to_lowercase())
                        || query.to_lowercase().contains("help")
                        || query.to_lowercase().contains("how")
                })
                .take(max as usize)
                .collect();

            if relevant.is_empty() {
                FunctionResult::with_response(&format!(
                    "No results found for '{query}'. Try a different search term."
                ))
            } else {
                let text = relevant
                    .iter()
                    .map(|(t, d, s)| format!("[{s:.2}] {t}: {d}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                FunctionResult::with_response(&format!("Search results:\n{text}"))
            }
        }),
        false,
    );

    agent.run();
}
