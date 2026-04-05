// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! MCP Gateway Demo — connect to MCP servers via the mcp_gateway skill.
//!
//! Prerequisites:
//!   cargo install signalwire-mcp-gateway
//!   mcp-gateway -c config.json
//!
//! Environment:
//!   MCP_GATEWAY_URL           (default: http://localhost:8080)
//!   MCP_GATEWAY_AUTH_USER     (default: admin)
//!   MCP_GATEWAY_AUTH_PASSWORD (default: changeme)

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;
use std::env;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "MCP Gateway Agent".to_string(),
        route: Some("/mcp-gateway".to_string()),
        ..AgentOptions::new("mcp-gateway")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Role",
        "You are a helpful assistant with access to external tools provided \
         through MCP servers. Use the available tools to help users accomplish their tasks.",
        vec![],
    );

    // Connect to MCP gateway — tools are discovered automatically
    agent.add_skill("mcp_gateway", Some(json!({
        "gateway_url": env::var("MCP_GATEWAY_URL")
            .unwrap_or_else(|_| "http://localhost:8080".into()),
        "auth_user": env::var("MCP_GATEWAY_AUTH_USER")
            .unwrap_or_else(|_| "admin".into()),
        "auth_password": env::var("MCP_GATEWAY_AUTH_PASSWORD")
            .unwrap_or_else(|_| "changeme".into()),
        "services": [{"name": "todo"}]
    })));

    println!("MCP Gateway agent at http://localhost:3000/mcp-gateway");
    agent.run();
}
