// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! FAQ Bot Agent — answer FAQs from a knowledge base.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;
use std::collections::HashMap;

fn build_faq_knowledge_base() -> HashMap<&'static str, &'static str> {
    let mut faqs = HashMap::new();
    faqs.insert(
        "What are your business hours?",
        "We are open Monday through Friday, 9 AM to 5 PM Eastern Time.",
    );
    faqs.insert(
        "How do I reset my password?",
        "Go to the login page, click 'Forgot Password', and follow the email instructions.",
    );
    faqs.insert(
        "What is your return policy?",
        "We accept returns within 30 days of purchase with a receipt.",
    );
    faqs.insert(
        "How do I contact support?",
        "Email support@example.com or call +1-555-123-4567 during business hours.",
    );
    faqs.insert(
        "Do you offer free shipping?",
        "Yes, free shipping on orders over $50 within the continental US.",
    );
    faqs
}

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "faq-bot".to_string(),
        route: Some("/faq".to_string()),
        ..AgentOptions::new("faq-bot")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Personality",
        "You are a helpful FAQ assistant for ACME Corporation.",
        vec![],
    );
    agent.prompt_add_section(
        "Goal",
        "Answer customer questions using only the provided FAQ knowledge base.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Only answer questions if the information is in the FAQ knowledge base.",
        "If you don't know the answer, politely say so and offer to help with something else.",
        "Be concise and direct in your responses.",
    ]);

    // Build knowledge base as a prompt section
    let faqs = build_faq_knowledge_base();
    let kb_text: String = faqs
        .iter()
        .map(|(q, a)| format!("Q: {q}\nA: {a}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    agent.prompt_add_section("Knowledge Base", &kb_text, vec![]);

    // Tool to search the FAQ
    let faq_data = build_faq_knowledge_base();
    agent.define_tool(
        "search_faq",
        "Search the FAQ knowledge base for an answer",
        json!({"query": {"type": "string", "description": "The question to search for"}}),
        Box::new(move |args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let query_lower = query.to_lowercase();
            for (q, a) in &faq_data {
                if q.to_lowercase().contains(&query_lower)
                    || query_lower.contains(&q.to_lowercase())
                {
                    return FunctionResult::with_response(a);
                }
            }
            FunctionResult::with_response(
                "I don't have that information in my FAQ database. \
                 Would you like to speak with a live agent?",
            )
        }),
        false,
    );

    agent.run();
}
