// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Receptionist Agent — route calls to departments.

use signalwire::prefabs::ReceptionistAgent;
use serde_json::json;

fn main() {
    let departments = vec![
        json!({
            "name": "sales",
            "description": "For product inquiries, pricing, and purchasing",
            "number": "+15551235555"
        }),
        json!({
            "name": "support",
            "description": "For technical assistance, troubleshooting, and bug reports",
            "number": "+15551236666"
        }),
        json!({
            "name": "billing",
            "description": "For payment questions, invoices, and subscription changes",
            "number": "+15551237777"
        }),
        json!({
            "name": "general",
            "description": "For all other inquiries",
            "number": "+15551238888"
        }),
    ];

    let greeting =
        "Hello, thank you for calling ACME Corporation. How may I direct your call today?";

    let mut agent = ReceptionistAgent::new(
        "acme-receptionist",
        departments,
        Some(greeting),
        Some("/reception"),
    );

    agent.agent_mut().add_language("English", "en-US", "inworld.Mark");

    agent.agent_mut().prompt_add_section(
        "Company Information",
        "ACME Corporation is a leading provider of innovative solutions. \
         Our business hours are Monday through Friday, 9 AM to 5 PM Eastern Time.",
        vec![],
    );

    // Summary callback
    agent.agent_mut().on_summary(Box::new(|summary, _raw, _headers| {
        println!("Call summary: {summary}");
    }));

    let (user, pass) = agent.agent().get_basic_auth_credentials();
    println!("Receptionist agent");
    println!("  URL: http://localhost:3000/reception");
    println!("  Auth: {user}:{pass}");
    agent.agent().run();
}
