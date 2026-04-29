// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Comprehensive Dynamic Agent — multi-tenant routing, A/B testing, industry-specific config.
//!
//! Try:
//!   curl "http://localhost:3000/dynamic?tier=premium&industry=healthcare&voice=inworld.Sarah"
//!   curl "http://localhost:3000/dynamic?tier=enterprise&industry=finance&test_group=B"

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;
use std::sync::Arc;

fn main() {
    let mut opts = AgentOptions::new("Comprehensive Dynamic Agent");
    opts.route = Some("/dynamic".to_string());
    opts.auto_answer = true;
    opts.record_call = true;

    let mut agent = AgentBase::new(opts);

    agent.set_dynamic_config_callback(Box::new(
        |query_params, _body_params, _headers, agent| {
            let tier = query_params
                .get("tier")
                .and_then(|v| v.as_str())
                .unwrap_or("standard");
            let industry = query_params
                .get("industry")
                .and_then(|v| v.as_str())
                .unwrap_or("general");
            let voice = query_params
                .get("voice")
                .and_then(|v| v.as_str())
                .unwrap_or("inworld.Mark");
            let test_group = query_params
                .get("test_group")
                .and_then(|v| v.as_str())
                .unwrap_or("A");
            let customer_id = query_params
                .get("customer_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Voice
            agent.add_language("English", "en-US", voice);

            // Tier-based params
            let (timeout, temp) = match tier {
                "premium" => (300, 0.4),
                "enterprise" => (200, 0.3),
                _ => (500, 0.6),
            };
            agent.set_params(json!({
                "end_of_speech_timeout": timeout,
                "attention_timeout": 20000,
            }));
            agent.set_prompt_llm_params(json!({"temperature": temp}));

            // Industry-specific prompt
            let role_desc = match industry {
                "healthcare" => "You are a HIPAA-compliant healthcare assistant. Handle all information with strict confidentiality.",
                "finance" => "You are a financial services assistant. Follow regulatory compliance guidelines strictly.",
                "retail" => "You are an enthusiastic retail customer service specialist focused on customer satisfaction.",
                _ => "You are a helpful general assistant.",
            };
            agent.prompt_add_section("Role", role_desc, vec![]);

            // A/B testing
            let style = match test_group {
                "B" => "Use a more conversational and casual tone.",
                _ => "Use a professional and structured tone.",
            };
            agent.prompt_add_section("Communication Style", style, vec![]);

            // Global data
            agent.set_global_data(json!({
                "tier": tier,
                "industry": industry,
                "customer_id": customer_id,
                "test_group": test_group,
            }));
        },
    ));

    agent.run();
}
