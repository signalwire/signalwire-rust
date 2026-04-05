// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Simple Agent — POM prompts, SWAIG tools, multilingual support, LLM tuning.
//!
//! Demonstrates:
//! 1. Structured prompt via POM (Prompt Object Model)
//! 2. SWAIG tool with FunctionResult
//! 3. Multiple languages/voices
//! 4. LLM parameter tuning
//! 5. Speech recognition hints

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions::new("simple-agent"));

    // -- Languages --
    agent.add_language("English", "en-US", "rime.spore");
    agent.add_language("Spanish", "es-ES", "inworld.Sarah");

    // -- POM prompt --
    agent.prompt_add_section(
        "Role",
        "You are a friendly assistant who can tell the time and help with basic questions.",
        vec![],
    );

    agent.prompt_add_section("Instructions", "", vec![
        "Greet users warmly",
        "Use get_time when asked about the current time",
        "Be concise in responses",
        "If you don't know something, say so",
    ]);

    agent.prompt_add_subsection(
        "Role",
        "Tone",
        "Casual and friendly, like chatting with a knowledgeable friend.",
    );

    // -- Hints --
    agent.add_hints(vec!["SignalWire", "SWML", "SWAIG", "what time is it"]);

    // -- LLM params --
    agent.set_prompt_llm_params(json!({
        "temperature": 0.5,
        "top_p": 0.9,
        "barge_confidence": 0.5,
    }));

    // -- AI params --
    agent.set_params(json!({
        "end_of_speech_timeout": 500,
        "attention_timeout": 15000,
    }));

    // -- Tool --
    agent.define_tool(
        "get_time",
        "Get the current time",
        json!({}),
        Box::new(|_args, _raw| {
            let now = chrono::Local::now().format("%I:%M %p");
            FunctionResult::with_response(&format!("The current time is {now}."))
        }),
        false,
    );

    // -- Post prompt --
    agent.set_post_prompt("Summarize the conversation briefly.");

    // -- Run --
    let (user, pass) = agent.get_basic_auth_credentials();
    println!("Starting simple agent");
    println!("  URL: http://localhost:3000/");
    println!("  Auth: {user}:{pass}");
    agent.run();
}
