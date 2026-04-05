// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! InfoGatherer Example — collect answers to a set of questions using the prefab.

use signalwire::prefabs::InfoGathererAgent;

fn main() {
    let mut agent = InfoGathererAgent::new(
        "contact-form",
        "/contact",
        vec![
            serde_json::json!({"key_name": "name", "question_text": "What is your full name?"}),
            serde_json::json!({"key_name": "phone", "question_text": "What is your phone number?", "confirm": true}),
            serde_json::json!({"key_name": "age", "question_text": "What is your age?"}),
            serde_json::json!({"key_name": "reason", "question_text": "What are you contacting us about today?"}),
        ],
    );

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Introduction",
        "I'm here to help you fill out our contact form. \
         This information helps us better serve you.",
        vec![],
    );

    agent.set_post_prompt("Summarize the questions and answers in a concise manner.");

    let (user, pass) = agent.get_basic_auth_credentials();
    println!("InfoGatherer agent");
    println!("  URL: http://localhost:3000/contact");
    println!("  Auth: {user}:{pass}");
    agent.run();
}
