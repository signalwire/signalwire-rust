// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Multi-Agent Server — multiple agents on one server.
//!
//! Agents:
//!   /healthcare — HIPAA-compliant healthcare agent
//!   /finance    — regulatory-compliant finance agent
//!   /retail     — customer service retail agent

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::server::AgentServer;
use signalwire::swaig::FunctionResult;
use serde_json::json;
use std::sync::Arc;

fn healthcare_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "healthcare-agent".to_string(),
        route: Some("/healthcare".to_string()),
        ..AgentOptions::new("healthcare")
    });
    agent.add_language("English", "en-US", "inworld.Sarah");
    agent.prompt_add_section(
        "Role",
        "You are a HIPAA-compliant healthcare assistant. \
         Handle all patient information with strict confidentiality.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Never share patient information without verification",
        "Always confirm identity before discussing records",
        "Remind callers about privacy protections",
    ]);
    agent.set_params(json!({"end_of_speech_timeout": 400}));
    agent
}

fn finance_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "finance-agent".to_string(),
        route: Some("/finance".to_string()),
        ..AgentOptions::new("finance")
    });
    agent.add_language("English", "en-US", "inworld.Mark");
    agent.prompt_add_section(
        "Role",
        "You are a financial services assistant. \
         Follow all regulatory compliance guidelines.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Never give specific investment advice",
        "Always include appropriate disclaimers",
        "Verify account ownership before sharing details",
    ]);
    agent.define_tool(
        "check_balance",
        "Check an account balance (simulated)",
        json!({"account_id": {"type": "string"}}),
        Box::new(|args, _raw| {
            let id = args.get("account_id").and_then(|v| v.as_str()).unwrap_or("?");
            FunctionResult::with_response(&format!("Account {id}: balance $12,345.67."))
        }),
        true, // secure
    );
    agent
}

fn retail_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "retail-agent".to_string(),
        route: Some("/retail".to_string()),
        ..AgentOptions::new("retail")
    });
    agent.add_language("English", "en-US", "inworld.Sarah");
    agent.prompt_add_section(
        "Role",
        "You are an enthusiastic retail customer service specialist.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Focus on customer satisfaction",
        "Proactively offer related products",
        "Handle returns and exchanges gracefully",
    ]);
    agent.set_dynamic_config_callback(Arc::new(Box::new(
        |query_params, _body, _headers, agent| {
            let department = query_params
                .get("department")
                .and_then(|v| v.as_str())
                .unwrap_or("general");
            agent.prompt_add_section(
                "Department",
                &format!("You are in the {department} department."),
                vec![],
            );
        },
    )));
    agent
}

fn main() {
    let mut server = AgentServer::new("0.0.0.0", 3000);
    server.add_agent(healthcare_agent());
    server.add_agent(finance_agent());
    server.add_agent(retail_agent());

    println!("Multi-agent server:");
    println!("  Healthcare: http://localhost:3000/healthcare");
    println!("  Finance:    http://localhost:3000/finance");
    println!("  Retail:     http://localhost:3000/retail");
    server.run();
}
