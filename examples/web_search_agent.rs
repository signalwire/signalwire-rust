// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Web Search Agent — integrate web search into a voice agent.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "web-search-agent".to_string(),
        route: Some("/web-search".to_string()),
        ..AgentOptions::new("web-search")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a helpful assistant with web search capabilities.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Use the web_search function to find current information",
        "Summarize search results concisely for the caller",
        "Always cite the source when providing information",
    ]);

    agent.add_hints(vec!["search", "look up", "find", "Google"]);

    agent.define_tool(
        "web_search",
        "Search the web for current information",
        json!({
            "query": {"type": "string", "description": "Search query"},
            "num_results": {"type": "integer", "description": "Number of results (default 3)"}
        }),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            // In production, this would call a real search API
            FunctionResult::with_response(&format!(
                "Search results for '{query}':\n\
                 1. Example result about {query} - example.com\n\
                 2. Comprehensive guide to {query} - guide.com\n\
                 3. Latest news on {query} - news.com"
            ))
        }),
        false,
    );

    agent.run();
}
