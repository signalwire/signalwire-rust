// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Datasphere Serverless Environment Demo — config from environment variables.
//!
//! Environment:
//!   DATASPHERE_SPACE    — SignalWire space URL
//!   DATASPHERE_TOKEN    — API token for Datasphere
//!   DATASPHERE_DOC_ID   — Document collection ID

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;
use serde_json::json;
use std::env;

fn main() {
    let space = env::var("DATASPHERE_SPACE")
        .unwrap_or_else(|_| "example.signalwire.com".into());
    let token = env::var("DATASPHERE_TOKEN")
        .unwrap_or_else(|_| "your-token".into());
    let doc_id = env::var("DATASPHERE_DOC_ID")
        .unwrap_or_else(|_| "default-collection".into());

    let mut agent = AgentBase::new(AgentOptions {
        name: "datasphere-env".to_string(),
        route: Some("/datasphere-env".to_string()),
        ..AgentOptions::new("datasphere-env")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a knowledge assistant using SignalWire Datasphere.",
        vec![],
    );

    let search_url = format!("https://{space}/api/datasphere/documents/search");
    let auth_header = format!("Bearer {token}");

    let mut search_tool = DataMap::new("search_docs");
    search_tool
        .description("Search Datasphere documents")
        .parameter("query", "string", "Search query", true, vec![])
        .webhook(
            "POST",
            &search_url,
            json!({"Authorization": auth_header, "Content-Type": "application/json"}),
            "",
            false,
            vec![],
        )
        .body(json!({"query": "${args.query}", "document_id": doc_id, "limit": 5}))
        .output(FunctionResult::with_response(
            "Results: ${response.results[0].text}",
        ).to_value());

    agent.register_swaig_function(search_tool.to_swaig_function());

    println!("Datasphere env demo at http://localhost:3000/datasphere-env");
    println!("  Space: {space}");
    println!("  Collection: {doc_id}");
    agent.run();
}
