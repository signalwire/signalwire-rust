# MCP Gateway Reference

## Overview

The MCP (Model Context Protocol) gateway bridges external MCP servers into the SignalWire AI agent ecosystem. Tools exposed by MCP servers become SWAIG functions the AI can call during voice conversations.

## Architecture

```
MCP Server (external)
    ↕ JSON-RPC 2.0
MCP Gateway (signalwire)
    ↕ SWAIG
SignalWire AI Agent
    ↕ Voice
Caller
```

## Using the MCP Gateway Skill

The simplest way to connect to MCP servers:

```rust
agent.add_skill("mcp_gateway", Some(json!({
    "gateway_url": "http://localhost:8080",
    "auth_user": "admin",
    "auth_password": "changeme",
    "services": [
        {"name": "todo"},
        {"name": "calendar"}
    ]
})));
```

### Configuration Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `gateway_url` | `string` | yes | URL of the MCP gateway server |
| `auth_user` | `string` | no | Basic auth username |
| `auth_password` | `string` | no | Basic auth password |
| `services` | `array` | yes | List of MCP services to expose |

### Service Configuration

Each service entry specifies which MCP server to bridge:

```json
{
    "name": "todo",
    "description": "Task management tools",
    "tool_filter": ["create_task", "list_tasks"]
}
```

## Direct MCP Integration

For more control, use the MCP client API directly:

```rust
// Add an MCP server as a tool source
agent.add_mcp_server(
    "https://mcp.example.com/tools",
    json!({"Authorization": "Bearer sk-key"}),
);

// Add an MCP server with resource fetching
agent.add_mcp_server_with_resources(
    "https://mcp.example.com/crm",
    json!({"Authorization": "Bearer sk-key"}),
    true,  // fetch resources into global_data
    json!({
        "caller_id": "${caller_id_number}",
        "tenant": "acme-corp"
    }),
);
```

## MCP Server Mode

Expose agent tools as MCP endpoints for external clients:

```rust
agent.enable_mcp_server();
```

This adds an `/mcp` endpoint that speaks JSON-RPC 2.0. External MCP clients (Claude Desktop, other agents) can connect and discover tools.

## Running the Gateway

### Prerequisites

```bash
cargo install signalwire-mcp-gateway
```

### Configuration File

```json
{
    "servers": {
        "todo": {
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-todo"]
        },
        "filesystem": {
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
        }
    },
    "port": 8080,
    "auth": {
        "username": "admin",
        "password": "changeme"
    }
}
```

### Start the Gateway

```bash
mcp-gateway -c config.json
```

## Protocol Details

### Tool Discovery

The gateway queries each MCP server for available tools using `tools/list`. Discovered tools are mapped to SWAIG function definitions.

### Tool Invocation

When the AI calls a tool:

1. SignalWire POSTs to the agent's SWAIG endpoint
2. The agent forwards the call to the MCP gateway
3. The gateway invokes the tool on the appropriate MCP server via `tools/call`
4. The result is returned as a `FunctionResult`

### Resource Fetching

With `resources=True`, the gateway fetches resource data at session start:

1. Query the MCP server for `resources/list`
2. Substitute `resource_vars` into URI templates
3. Fetch each resource via `resources/read`
4. Store results in `global_data`
