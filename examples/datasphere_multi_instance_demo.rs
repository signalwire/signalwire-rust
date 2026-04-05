// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Datasphere Multi-Instance — multiple Datasphere document collections.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::server::AgentServer;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn product_kb_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "product-kb".to_string(),
        route: Some("/product-kb".to_string()),
        ..AgentOptions::new("product-kb")
    });

    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a product knowledge base assistant.",
        vec![],
    );

    agent.define_tool(
        "search_products",
        "Search the product knowledge base",
        json!({"query": {"type": "string", "description": "Product query"}}),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            FunctionResult::with_response(&format!(
                "Product KB results for '{query}': [simulated Datasphere results]"
            ))
        }),
        false,
    );

    agent
}

fn support_kb_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "support-kb".to_string(),
        route: Some("/support-kb".to_string()),
        ..AgentOptions::new("support-kb")
    });

    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a support knowledge base assistant for troubleshooting.",
        vec![],
    );

    agent.define_tool(
        "search_support",
        "Search the support knowledge base",
        json!({"query": {"type": "string", "description": "Support query"}}),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            FunctionResult::with_response(&format!(
                "Support KB results for '{query}': [simulated Datasphere results]"
            ))
        }),
        false,
    );

    agent
}

fn main() {
    let mut server = AgentServer::new("0.0.0.0", 3000);
    server.add_agent(product_kb_agent());
    server.add_agent(support_kb_agent());

    println!("Datasphere multi-instance:");
    println!("  Products: http://localhost:3000/product-kb");
    println!("  Support:  http://localhost:3000/support-kb");
    server.run();
}
