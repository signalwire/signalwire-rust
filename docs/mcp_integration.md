# MCP Integration

## Overview

Model Context Protocol (MCP) integration lets a SignalWire AI agent consume tools that
live behind an MCP gateway. The integration is provided entirely by the built-in
`mcp_gateway` skill — there is no separate agent-level MCP API. The gateway bridges MCP
servers to SWAIG so their tools become callable during a voice conversation.

## Using the MCP Gateway Skill

Add the `mcp_gateway` skill, pointing it at a running gateway:

```rust
use serde_json::json;

agent.add_skill("mcp_gateway", json!({
    "gateway_url": "http://localhost:8080",
    "auth_user": "admin",
    "auth_password": "changeme",
    "services": [{"name": "todo"}, {"name": "calendar"}]
}));
```

If `services` is omitted (or empty), the skill registers a single generic gateway tool
(`<tool_prefix>call`, default prefix `mcp_`) that takes a `service`, a `tool`, and an
`arguments` object. When `services` are listed, the skill registers one tool per
service/tool pair.

### Configuration Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `gateway_url` | `string` | yes | URL of the MCP gateway server |
| `auth_user` | `string` | no | Basic auth username for the gateway |
| `auth_password` | `string` | no | Basic auth password for the gateway |
| `services` | `array` | no | MCP services to expose; one tool per service/tool pair |
| `tool_prefix` | `string` | no | Prefix for generated tool names (default `mcp_`) |

## How Tool Invocation Works

1. SignalWire POSTs to the agent's `/swaig` endpoint when the AI calls an MCP-backed tool
2. The skill's handler forwards the request to the MCP gateway
3. The gateway invokes the tool on the appropriate MCP server via `tools/call`
4. The result is returned to the AI as a `FunctionResult`

## Combined Example

```rust
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions::new("mcp-agent"));

    // Bridge external MCP services through a gateway
    agent.add_skill("mcp_gateway", json!({
        "gateway_url": "http://localhost:8080",
        "services": [{"name": "todo"}, {"name": "calendar"}]
    }));

    agent.prompt_add_section("Role", "You are a customer support agent.", vec![]);

    // A native SWAIG tool can coexist with the MCP-backed tools
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

See [mcp_gateway_reference.md](mcp_gateway_reference.md) for gateway setup details.
