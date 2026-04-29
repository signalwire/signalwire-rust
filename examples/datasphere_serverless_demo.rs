// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Datasphere Serverless Demo — Datasphere search via DataMap (no webhook needed).

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "datasphere-serverless".to_string(),
        route: Some("/datasphere".to_string()),
        ..AgentOptions::new("datasphere-serverless")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a knowledge assistant backed by SignalWire Datasphere.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Use the search_knowledge function to find relevant documents",
        "Summarize results clearly for the caller",
        "Cite the document source when available",
    ]);

    // Datasphere search via DataMap
    let mut search_tool = DataMap::new("search_knowledge");
    search_tool
        .description("Search the Datasphere knowledge base")
        .parameter("query", "string", "Search query", true, vec![])
        .parameter("max_results", "integer", "Maximum results", false, vec![])
        .webhook(
            "POST",
            "https://${env.SIGNALWIRE_SPACE}/api/datasphere/documents/search",
            json!({
                "Authorization": "Basic ${env.DATASPHERE_AUTH}",
                "Content-Type": "application/json"
            }),
            "",
            false,
            vec![],
        )
        .body(json!({
            "query": "${args.query}",
            "limit": "${args.max_results}"
        }))
        .output(FunctionResult::with_response(
            "Found ${response.total} results. Top result: ${response.results[0].text}",
        ).to_value());

    agent.register_swaig_function(search_tool.to_swaig_function());

    agent.run();
}
