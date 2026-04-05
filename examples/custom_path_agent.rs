// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Custom Path Agent — agent at /chat with query-parameter personalization.
//!
//! Try:
//!   curl "http://localhost:3000/chat"
//!   curl "http://localhost:3000/chat?user_name=Alice&topic=AI"
//!   curl "http://localhost:3000/chat?mood=professional"

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;
use std::sync::Arc;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "Chat Assistant".to_string(),
        route: Some("/chat".to_string()),
        auto_answer: true,
        record_call: true,
        ..AgentOptions::new("chat")
    });

    agent.prompt_add_section(
        "Role",
        "You are a friendly chat assistant ready to help with any questions.",
        vec![],
    );

    agent.set_dynamic_config_callback(Arc::new(Box::new(
        |query_params, _body_params, _headers, agent| {
            let user_name = query_params
                .get("user_name")
                .and_then(|v| v.as_str())
                .unwrap_or("friend");
            let topic = query_params
                .get("topic")
                .and_then(|v| v.as_str())
                .unwrap_or("general conversation");
            let mood = query_params
                .get("mood")
                .and_then(|v| v.as_str())
                .unwrap_or("friendly");

            agent.add_language("English", "en-US", "inworld.Mark");

            agent.prompt_add_section(
                "Personalization",
                &format!(
                    "The user's name is {user_name}. They're interested in discussing {topic}."
                ),
                vec![],
            );

            let style_desc = match mood {
                "professional" => "Maintain a professional, business-appropriate tone.",
                "casual" => "Use a casual, relaxed conversational style.",
                _ => "Be friendly and approachable.",
            };
            agent.prompt_add_section("Communication Style", style_desc, vec![]);
        },
    )));

    println!("Chat agent at http://localhost:3000/chat");
    agent.run();
}
