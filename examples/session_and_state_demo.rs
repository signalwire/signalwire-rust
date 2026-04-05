// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Session and State Demo — on_summary, global data, post-prompt features.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;
use std::sync::Arc;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "session-state-demo".to_string(),
        route: Some("/session-state-demo".to_string()),
        ..AgentOptions::new("session-state-demo")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Role",
        "You are a customer service agent that tracks session state.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Greet the caller and ask how you can help",
        "Use update_customer_info to record information the caller provides",
        "Use get_session_info to check what information has been collected",
        "Use end_session when the caller is done",
    ]);

    // Seed session with default global data
    agent.set_global_data(json!({
        "status": "active",
        "customer_name": "",
        "issue_type": "",
    }));

    // Post-prompt for summary generation
    agent.set_post_prompt(
        "Summarize: customer name, issue type, resolution status, and any follow-up needed.",
    );

    // Summary callback
    agent.set_summary_callback(Arc::new(Box::new(|summary, raw_data, _headers| {
        println!("=== Call Summary ===");
        println!("{summary}");
        println!("Raw data: {raw_data}");
        println!("====================");
    })));

    // Tool: update customer info
    agent.define_tool(
        "update_customer_info",
        "Save customer information to the session",
        json!({
            "field": {"type": "string", "description": "Field name (customer_name, issue_type, etc.)"},
            "value": {"type": "string", "description": "Field value"}
        }),
        Box::new(|args, _raw| {
            let field = args.get("field").and_then(|v| v.as_str()).unwrap_or("");
            let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let mut result = FunctionResult::with_response(
                &format!("Saved {field} = {value}.")
            );
            result.add_action(json!({"update_global_data": {field: value}}));
            result
        }),
        false,
    );

    // Tool: get session info
    agent.define_tool(
        "get_session_info",
        "Get all information collected in this session",
        json!({}),
        Box::new(|_args, raw_data| {
            let global_data = raw_data.get("global_data")
                .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                .unwrap_or_else(|| "{}".to_string());
            FunctionResult::with_response(&format!("Session data: {global_data}"))
        }),
        false,
    );

    // Tool: end session
    agent.define_tool(
        "end_session",
        "End the current session and say goodbye",
        json!({}),
        Box::new(|_args, _raw| {
            let mut result = FunctionResult::with_response(
                "Thank you for calling. Goodbye!"
            );
            result.set_post_process(true);
            result.add_action(json!({"update_global_data": {"status": "closed"}}));
            result.add_action(json!({"hangup": {}}));
            result
        }),
        false,
    );

    agent.run();
}
