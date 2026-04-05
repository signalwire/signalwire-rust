// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Wikipedia Demo — search Wikipedia via DataMap.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "wikipedia-demo".to_string(),
        route: Some("/wikipedia".to_string()),
        ..AgentOptions::new("wikipedia-demo")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a knowledgeable assistant that can look up information on Wikipedia.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Use the search_wikipedia function to find information",
        "Summarize the results in a conversational way",
        "If the search returns no results, suggest alternative search terms",
    ]);

    // DataMap tool for Wikipedia API (no webhook needed)
    let wiki_tool = DataMap::new("search_wikipedia")
        .description("Search Wikipedia for information on a topic")
        .parameter("topic", "string", "Topic to search for", true)
        .webhook(
            "GET",
            "https://en.wikipedia.org/api/rest_v1/page/summary/${args.topic}",
            json!({}),
            json!({"Accept": "application/json"}),
        )
        .output(FunctionResult::with_response(
            "Wikipedia summary for '${args.topic}': ${response.extract}",
        ))
        .build();
    agent.define_datamap_tool(wiki_tool);

    agent.add_hints(vec!["Wikipedia", "look up", "what is", "tell me about"]);

    agent.run();
}
