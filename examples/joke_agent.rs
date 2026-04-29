// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Joke Agent — raw data_map integration with API Ninjas joke API.
//!
//! Usage: API_NINJAS_KEY=your_key cargo run --example joke_agent

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;
use std::env;

fn main() {
    let api_key = env::var("API_NINJAS_KEY").unwrap_or_else(|_| {
        eprintln!("Error: API_NINJAS_KEY environment variable is required");
        eprintln!("Get your free API key from https://api.api-ninjas.com/");
        std::process::exit(1);
    });

    let mut agent = AgentBase::new(AgentOptions {
        name: "Joke Agent".to_string(),
        route: Some("/joke-agent".to_string()),
        ..AgentOptions::new("joke-agent")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section("Personality", "You are a funny assistant who loves to tell jokes.", vec![]);
    agent.prompt_add_section("Goal", "Make people laugh with great jokes.", vec![]);
    agent.prompt_add_section("Instructions", "", vec![
        "Use the get_joke function to tell jokes when asked",
        "You can tell either regular jokes or dad jokes",
        "Be enthusiastic about sharing humor",
    ]);

    // Raw data_map tool
    agent.register_swaig_function(json!({
        "function": "get_joke",
        "description": "Tell a joke",
        "data_map": {
            "webhooks": [{
                "url": format!("https://api.api-ninjas.com/v1/%{{args.type}}"),
                "headers": {"X-Api-Key": api_key},
                "output": {
                    "response": "Tell the user: %{array[0].joke}",
                    "action": [{
                        "SWML": {
                            "sections": {
                                "main": [{"set": {"dad_joke": "%{array[0].joke}"}}]
                            },
                            "version": "1.0.0"
                        }
                    }]
                }
            }]
        },
        "argument": {
            "type": "object",
            "properties": {
                "type": {
                    "type": "string",
                    "description": "Type of joke: jokes or dadjokes",
                    "enum": ["jokes", "dadjokes"]
                }
            }
        }
    }));

    agent.run();
}
