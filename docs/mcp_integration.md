# MCP Integration

## Overview

Model Context Protocol (MCP) integration allows SignalWire AI agents to both consume and expose tools via the MCP standard. This creates a bridge between voice AI agents and the broader MCP ecosystem.

## Two Modes

### MCP Client (Consume External Tools)

Your agent connects to external MCP servers and uses their tools during voice calls:

```rust
agent.add_mcp_server(
    "https://mcp.example.com/tools",
    json!({"Authorization": "Bearer sk-key"}),
);
```

### MCP Server (Expose Agent Tools)

Your agent exposes its SWAIG tools as MCP endpoints for external clients:

```rust
agent.enable_mcp_server();
// Tools now available at /agent/mcp
```

## Client Integration

### Basic Tool Discovery

```rust
// Connect to an MCP server
// Tools are auto-discovered at call start and added to the AI's tool list
agent.add_mcp_server(
    "https://mcp.example.com/tools",
    json!({
        "Authorization": "Bearer sk-your-key"
    }),
);
```

### With Resource Fetching

MCP servers can expose resources (read-only data). With resources enabled, data is fetched into `global_data` at session start:

```rust
agent.add_mcp_server_with_resources(
    "https://mcp.example.com/crm",
    json!({"Authorization": "Bearer sk-crm-key"}),
    true,  // fetch resources
    json!({
        "caller_id": "${caller_id_number}",
        "tenant": "acme-corp"
    }),
);

// Reference resource data in prompts
agent.prompt_add_section("Customer Context", "", vec![]);
agent.prompt_add_to_section(
    "Customer Context",
    Some("Customer name: ${global_data.customer_name}\nAccount status: ${global_data.account_status}"),
    vec![],
);
```

### Resource Variables

Resource variables substitute caller information into URI templates:

| Variable | Description |
|----------|-------------|
| `${caller_id_number}` | Caller's phone number |
| `${caller_id_name}` | Caller's name |
| `${call_id}` | Call identifier |
| `${ai_session_id}` | AI session identifier |

## Server Integration

### Exposing Tools

```rust
agent.enable_mcp_server();
```

This adds an `/mcp` endpoint that speaks JSON-RPC 2.0. MCP clients discover tools via `tools/list` and invoke them via `tools/call`.

### Connecting from Claude Desktop

```json
{
    "mcpServers": {
        "my-agent": {
            "url": "http://user:pass@localhost:3000/agent/mcp"
        }
    }
}
```

## Combined Example

An agent that both consumes and exposes MCP tools:

```rust
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions::new("mcp-agent"));

    // Expose our tools via MCP
    agent.enable_mcp_server();

    // Consume tools from an external MCP server
    agent.add_mcp_server(
        "https://mcp.example.com/tools",
        json!({"Authorization": "Bearer sk-key"}),
    );

    // Consume resources from a CRM MCP server
    agent.add_mcp_server_with_resources(
        "https://mcp.example.com/crm",
        json!({"Authorization": "Bearer sk-crm-key"}),
        true,
        json!({"caller_id": "${caller_id_number}"}),
    );

    // Agent configuration
    agent.prompt_add_section("Role", "You are a customer support agent.", vec![]);
    agent.prompt_add_section("Customer Context",
        "Customer: ${global_data.customer_name}", vec![]);

    // Define a tool (available via both SWAIG and MCP)
    agent.define_tool(
        "lookup_order",
        "Look up an order by ID",
        json!({"order_id": {"type": "string"}}),
        Box::new(|args, _raw| {
            let id = args.get("order_id").and_then(|v| v.as_str()).unwrap_or("?");
            FunctionResult::with_response(&format!("Order {id}: shipped"))
        }),
        false,
    );

    agent.run();
}
```

## Via MCP Gateway Skill

For simpler integration without direct MCP server management:

```rust
agent.add_skill("mcp_gateway", Some(json!({
    "gateway_url": "http://localhost:8080",
    "auth_user": "admin",
    "auth_password": "changeme",
    "services": [{"name": "todo"}, {"name": "calendar"}]
})));
```

See [mcp_gateway_reference.md](mcp_gateway_reference.md) for gateway setup details.
