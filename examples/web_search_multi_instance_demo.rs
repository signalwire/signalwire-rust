// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Web Search Multi-Instance — multiple search agents with different configurations.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::server::AgentServer;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn news_search_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "news-search".to_string(),
        route: Some("/news-search".to_string()),
        ..AgentOptions::new("news-search")
    });
    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a news search assistant focused on current events.",
        vec![],
    );
    agent.define_tool(
        "search_news",
        "Search for current news articles",
        json!({"topic": {"type": "string", "description": "News topic"}}),
        Box::new(|args, _raw| {
            let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("general");
            FunctionResult::with_response(&format!("Latest news on {topic}: [simulated results]"))
        }),
        false,
    );
    agent
}

fn tech_search_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "tech-search".to_string(),
        route: Some("/tech-search".to_string()),
        ..AgentOptions::new("tech-search")
    });
    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a technical documentation search assistant.",
        vec![],
    );
    agent.define_tool(
        "search_docs",
        "Search technical documentation",
        json!({"query": {"type": "string", "description": "Technical query"}}),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            FunctionResult::with_response(&format!("Documentation results for '{query}': [simulated]"))
        }),
        false,
    );
    agent
}

fn main() {
    let mut server = AgentServer::new("0.0.0.0", 3000);
    server.add_agent(news_search_agent());
    server.add_agent(tech_search_agent());

    println!("Web search multi-instance:");
    println!("  News: http://localhost:3000/news-search");
    println!("  Tech: http://localhost:3000/tech-search");
    server.run();
}
