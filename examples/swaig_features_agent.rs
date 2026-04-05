// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! SWAIG Features Agent — showcases advanced SWAIG tool patterns.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "swaig-features".to_string(),
        route: Some("/swaig-features".to_string()),
        ..AgentOptions::new("swaig-features")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section("Role", "You are a demo agent showcasing SWAIG features.", vec![]);
    agent.prompt_add_section("Instructions", "", vec![
        "Use the available tools to demonstrate SWAIG capabilities",
        "Explain what each tool does before using it",
    ]);

    // Tool with actions
    agent.define_tool(
        "send_notification",
        "Send an SMS notification to a phone number",
        json!({
            "phone": {"type": "string", "description": "Phone number"},
            "message": {"type": "string", "description": "Message text"}
        }),
        Box::new(|args, _raw| {
            let phone = args.get("phone").and_then(|v| v.as_str()).unwrap_or("");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
            let mut result = FunctionResult::with_response(
                &format!("Notification sent to {phone}.")
            );
            result.add_action(json!({
                "send_sms": {
                    "to": phone,
                    "from": "+15559876543",
                    "body": message
                }
            }));
            result
        }),
        false,
    );

    // Tool with post_process
    agent.define_tool(
        "schedule_callback",
        "Schedule a callback for the caller",
        json!({
            "time": {"type": "string", "description": "Preferred callback time"}
        }),
        Box::new(|args, _raw| {
            let time = args.get("time").and_then(|v| v.as_str()).unwrap_or("later");
            let mut result = FunctionResult::with_response(
                &format!("Your callback has been scheduled for {time}. Is there anything else?")
            );
            result.set_post_process(true);
            result
        }),
        false,
    );

    // Secure tool
    agent.define_tool(
        "verify_identity",
        "Verify the caller's identity with an account number",
        json!({
            "account_number": {"type": "string", "description": "Account number"}
        }),
        Box::new(|args, _raw| {
            let acct = args.get("account_number").and_then(|v| v.as_str()).unwrap_or("");
            if acct.len() >= 6 {
                FunctionResult::with_response("Identity verified successfully.")
            } else {
                FunctionResult::with_response("Invalid account number. Please try again.")
            }
        }),
        true, // secure — HMAC signed
    );

    // Tool that updates global data
    agent.define_tool(
        "save_preference",
        "Save a caller preference",
        json!({
            "key": {"type": "string", "description": "Preference name"},
            "value": {"type": "string", "description": "Preference value"}
        }),
        Box::new(|args, _raw| {
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("pref");
            let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let mut result = FunctionResult::with_response(
                &format!("Preference '{key}' saved as '{value}'.")
            );
            result.add_action(json!({
                "update_global_data": {key: value}
            }));
            result
        }),
        false,
    );

    agent.run();
}
