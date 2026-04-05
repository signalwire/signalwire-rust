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
    let search_tool = DataMap::new("search_knowledge")
        .description("Search the Datasphere knowledge base")
        .parameter("query", "string", "Search query", true)
        .parameter("max_results", "integer", "Maximum results", false)
        .webhook(
            "POST",
            "https://${env.SIGNALWIRE_SPACE}/api/datasphere/documents/search",
            json!({
                "query": "${args.query}",
                "limit": "${args.max_results}"
            }),
            json!({
                "Authorization": "Basic ${env.DATASPHERE_AUTH}",
                "Content-Type": "application/json"
            }),
        )
        .output(FunctionResult::with_response(
            "Found ${response.total} results. Top result: ${response.results[0].text}",
        ))
        .build();

    agent.define_datamap_tool(search_tool);

    agent.run();
}
