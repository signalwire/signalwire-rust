// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! AWS Lambda Agent — serverless deployment example.
//!
//! Deploy to Lambda with API Gateway. Set environment variables:
//!   SWML_BASIC_AUTH_USER, SWML_BASIC_AUTH_PASSWORD

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn create_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "lambda-agent".to_string(),
        route: Some("/".to_string()),
        ..AgentOptions::new("lambda-agent")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Role",
        "You are a helpful AI assistant running in AWS Lambda.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Greet users warmly and offer help",
        "Use the greet_user function when asked to greet someone",
        "Be concise — you're running serverless!",
    ]);

    agent.define_tool(
        "greet_user",
        "Greet a user by name",
        json!({"name": {"type": "string", "description": "User's name"}}),
        Box::new(|args, _raw| {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("friend");
            FunctionResult::with_response(&format!("Hello, {name}! Welcome to our service."))
        }),
        false,
    );

    agent.define_tool(
        "get_status",
        "Get the system status",
        json!({}),
        Box::new(|_args, _raw| {
            FunctionResult::with_response(
                "All systems operational. Running in AWS Lambda."
            )
        }),
        false,
    );

    agent
}

fn main() {
    let agent = create_agent();

    // In production Lambda, you would wrap with lambda_http:
    // let handler = agent.get_app();
    // lambda_http::run(handler).await

    // For local testing:
    println!("Lambda agent (local testing mode)");
    let (user, pass) = agent.get_basic_auth_credentials();
    println!("  Auth: {user}:{pass}");
    agent.run();
}
