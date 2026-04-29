// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! MCP Agent — both MCP client and server in one agent.
//!
//! - MCP Server: exposes tools at /agent/mcp for external clients
//! - MCP Client: connects to external MCP servers for additional tools/resources

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "mcp-agent".to_string(),
        route: Some("/agent".to_string()),
        ..AgentOptions::new("mcp-agent")
    });

    // -- MCP Gateway skill --
    // Add the mcp_gateway skill to expose external MCP servers as SWAIG tools.
    agent.add_skill("mcp_gateway", json!({
        "gateway_url": "https://mcp.example.com/tools",
        "headers": {"Authorization": "Bearer sk-your-mcp-api-key"},
        "tool_prefix": "mcp_",
    }));

    // -- Agent configuration --
    agent.prompt_add_section("Role", "", vec![]);
    agent.prompt_add_to_section(
        "Role",
        Some(
            "You are a helpful customer support agent. \
             You have access to the customer's profile via global_data. \
             Use the available tools to look up information and assist the caller.",
        ),
        vec![],
    );

    agent.prompt_add_section(
        "Customer Context",
        "Customer name: ${global_data.customer_name}\n\
         Account status: ${global_data.account_status}\n\
         If customer data is not available, ask the caller for their name.",
        vec![],
    );

    agent.set_params(json!({"attention_timeout": 15000}));

    // Local tool (available via both SWAIG and MCP)
    agent.define_tool(
        "lookup_order",
        "Look up an order by ID",
        json!({"order_id": {"type": "string", "description": "Order ID"}}),
        Box::new(|args, _raw| {
            let id = args.get("order_id").and_then(|v| v.as_str()).unwrap_or("?");
            FunctionResult::with_response(&format!(
                "Order {id}: shipped on 2024-01-10, ETA 2024-01-15."
            ))
        }),
        false,
    );

    println!("MCP agent:");
    println!("  SWML:   http://localhost:3000/agent");
    println!("  MCP:    http://localhost:3000/agent/mcp");
    agent.run();
}
