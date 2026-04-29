// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Dynamic InfoGatherer — question sets chosen per-request via query parameter.
//!
//! Try:
//!   curl "http://localhost:3000/contact"              (default questions)
//!   curl "http://localhost:3000/contact?set=support"  (support questions)
//!   curl "http://localhost:3000/contact?set=medical"  (medical intake)
//!   curl "http://localhost:3000/contact?set=onboarding" (employee onboarding)

use signalwire::prefabs::InfoGathererAgent;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

fn question_sets() -> HashMap<&'static str, Vec<serde_json::Value>> {
    let mut sets = HashMap::new();
    sets.insert("default", vec![
        json!({"key_name": "name", "question_text": "What is your full name?"}),
        json!({"key_name": "phone", "question_text": "What is your phone number?", "confirm": true}),
        json!({"key_name": "reason", "question_text": "How can I help you today?"}),
    ]);
    sets.insert("support", vec![
        json!({"key_name": "customer_name", "question_text": "What is your name?"}),
        json!({"key_name": "account_number", "question_text": "What is your account number?", "confirm": true}),
        json!({"key_name": "issue", "question_text": "What issue are you experiencing?"}),
        json!({"key_name": "priority", "question_text": "How urgent is this? (Low, Medium, High)"}),
    ]);
    sets.insert("medical", vec![
        json!({"key_name": "patient_name", "question_text": "What is the patient's full name?"}),
        json!({"key_name": "symptoms", "question_text": "What symptoms are you experiencing?", "confirm": true}),
        json!({"key_name": "duration", "question_text": "How long have you had these symptoms?"}),
        json!({"key_name": "medications", "question_text": "Are you currently taking any medications?"}),
    ]);
    sets.insert("onboarding", vec![
        json!({"key_name": "full_name", "question_text": "What is your full name?"}),
        json!({"key_name": "email", "question_text": "What is your email address?", "confirm": true}),
        json!({"key_name": "company", "question_text": "What company do you work for?"}),
        json!({"key_name": "department", "question_text": "What department will you be in?"}),
        json!({"key_name": "start_date", "question_text": "What is your start date?"}),
    ]);
    sets
}

fn main() {
    let sets = Arc::new(question_sets());

    // Construct with the default question set, then swap it in per-request
    // via set_dynamic_config_callback.
    let mut agent = InfoGathererAgent::new(
        "dynamic-contact-form",
        sets["default"].clone(),
        Some("/contact"),
    );

    agent.agent_mut().add_language("English", "en-US", "inworld.Mark");

    let sets_for_callback = sets.clone();
    agent.agent_mut().set_dynamic_config_callback(Box::new(
        move |query_params, _body, _headers, agent| {
            let set_name = query_params
                .get("set")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            println!("Dynamic config: set={set_name}");
            let qs = sets_for_callback
                .get(set_name)
                .cloned()
                .unwrap_or_else(|| sets_for_callback["default"].clone());
            agent.set_global_data(json!({
                "questions": qs,
                "question_index": 0,
                "answers": [],
            }));
        },
    ));

    agent.agent().run();
}
