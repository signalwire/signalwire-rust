// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Simple Static Agent — all configuration at startup, same for every request.

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;

fn main() {
    let mut opts = AgentOptions::new("Simple Customer Service Agent");
    opts.auto_answer = true;
    opts.record_call = true;

    let mut agent = AgentBase::new(opts);

    // Voice and language
    agent.add_language("English", "en-US", "inworld.Mark");

    // AI parameters
    agent.set_params(json!({
        "ai_model": "gpt-4.1-nano",
        "end_of_speech_timeout": 500,
        "attention_timeout": 15000,
        "background_file_volume": -20,
    }));

    // Hints
    agent.add_hints(vec!["SignalWire", "SWML", "API", "webhook", "SIP"]);

    // Global data
    agent.set_global_data(json!({
        "agent_type": "customer_service",
        "service_level": "standard",
        "features_enabled": ["basic_conversation", "help_desk"],
    }));

    // Prompt
    agent.prompt_add_section(
        "Role",
        "You are a friendly and professional customer service representative for SignalWire.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Greet the customer warmly",
        "Be helpful and concise",
        "If you cannot help, offer to connect with a human",
    ]);

    let (user, pass) = agent.get_basic_auth_credentials();
    println!("Static agent at http://{user}:{pass}@localhost:3000/");
    agent.run();
}
