// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Enhanced Dynamic Agent — adapts based on request parameters.
//!
//! Try:
//!   curl "http://localhost:3000?vip=true&customer_id=CUST123"
//!   curl "http://localhost:3000?department=sales&language=es"
//!   curl "http://localhost:3000?department=billing&vip=true"

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;
use std::sync::Arc;

fn main() {
    let mut opts = AgentOptions::new("Enhanced Dynamic Customer Service Agent");
    opts.auto_answer = true;
    opts.record_call = true;

    let mut agent = AgentBase::new(opts);

    agent.set_dynamic_config_callback(Box::new(
        |query_params, _body_params, _headers, agent| {
            let is_vip = query_params
                .get("vip")
                .map(|v| v.as_str() == Some("true"))
                .unwrap_or(false);
            let department = query_params
                .get("department")
                .and_then(|v| v.as_str())
                .unwrap_or("general");
            let customer_id = query_params
                .get("customer_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let language = query_params
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("en");

            // Voice selection based on VIP + language
            match language {
                "es" => {
                    if is_vip {
                        agent.add_language("Spanish", "es-ES", "inworld.Sarah");
                    } else {
                        agent.add_language("Spanish", "es-ES", "inworld.Mark");
                    }
                }
                _ => {
                    if is_vip {
                        agent.add_language("English", "en-US", "inworld.Sarah");
                    } else {
                        agent.add_language("English", "en-US", "inworld.Mark");
                    }
                }
            }

            // AI params — faster for VIP
            let timeout = if is_vip { 300 } else { 500 };
            agent.set_params(json!({
                "end_of_speech_timeout": timeout,
                "attention_timeout": 15000,
            }));

            // Department-specific prompt
            let dept_desc = match department {
                "sales" => "You are a persuasive sales specialist.",
                "support" => "You are a patient technical support expert.",
                "billing" => "You are a precise billing and accounts specialist.",
                _ => "You are a friendly general customer service representative.",
            };
            agent.prompt_add_section("Role", dept_desc, vec![]);

            // Personalisation
            if !customer_id.is_empty() {
                agent.prompt_add_section(
                    "Customer",
                    &format!("The customer ID is {customer_id}. Reference it when relevant."),
                    vec![],
                );
            }

            if is_vip {
                agent.prompt_add_section("VIP", "", vec![
                    "This is a VIP customer — provide premium service",
                    "Offer proactive help and exclusive options",
                ]);
            }

            agent.set_global_data(json!({
                "department": department,
                "is_vip": is_vip,
                "customer_id": customer_id,
            }));
        },
    ));

    println!("Enhanced dynamic agent at http://localhost:3000/");
    agent.run();
}
