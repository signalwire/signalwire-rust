// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Kubernetes-Ready Agent — health checks, structured logging, env-based config.
//!
//! Usage:
//!   PORT=8080 cargo run --example kubernetes_ready_agent

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;
use std::env;

fn main() {
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    let mut agent = AgentBase::new(AgentOptions {
        name: "k8s-agent".to_string(),
        route: Some("/".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(port),
        ..AgentOptions::new("k8s-agent")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Role",
        "You are a production-ready AI agent running in Kubernetes. \
         Help users with general questions and demonstrate cloud-native deployment.",
        vec![],
    );

    agent.define_tool(
        "health_status",
        "Get the health status of this agent",
        json!({}),
        Box::new(move |_args, _raw| {
            FunctionResult::with_response(&format!(
                "Agent is healthy, running on port {port} in Kubernetes."
            ))
        }),
        false,
    );

    agent.define_tool(
        "get_environment",
        "Get deployment environment info",
        json!({}),
        Box::new(|_args, _raw| {
            let pod = env::var("HOSTNAME").unwrap_or_else(|_| "local".into());
            let namespace = env::var("NAMESPACE").unwrap_or_else(|_| "default".into());
            FunctionResult::with_response(&format!(
                "Pod: {pod}, Namespace: {namespace}, Status: running"
            ))
        }),
        false,
    );

    // Health endpoints are automatically at /health and /ready
    println!("Kubernetes-ready agent:");
    println!("  Agent:  http://0.0.0.0:{port}/");
    println!("  Health: http://0.0.0.0:{port}/health");
    println!("  Ready:  http://0.0.0.0:{port}/ready");
    agent.run();
}
