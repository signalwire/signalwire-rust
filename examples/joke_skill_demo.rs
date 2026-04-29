// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Joke Skill Demo — using the modular skills system with DataMap.
//!
//! Compare with joke_agent.rs to see the benefits of the skills system.
//!
//! Usage: API_NINJAS_KEY=your_key cargo run --example joke_skill_demo

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
        name: "Joke Skill Demo Agent".to_string(),
        route: Some("/joke-skill-demo".to_string()),
        ..AgentOptions::new("joke-skill-demo")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Personality",
        "You are a cheerful comedian who loves sharing jokes and making people laugh.",
        vec![],
    );
    agent.prompt_add_section("Goal", "Entertain users with great jokes and spread joy.", vec![]);
    agent.prompt_add_section("Instructions", "", vec![
        "When users ask for jokes, use your joke functions",
        "Be enthusiastic and fun in your responses",
        "You can tell both regular jokes and dad jokes",
    ]);

    // One-liner skill integration (compare with raw data_map in joke_agent.rs)
    agent.add_skill("joke", json!({"api_key": api_key}));

    println!("Joke Skill Demo running at http://localhost:3000/joke-skill-demo");
    agent.run();
}
