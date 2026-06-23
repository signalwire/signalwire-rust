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
agent.add_skill("mcp_gateway", json!({
    "gateway_url": "http://localhost:8080",
    "auth_user": "admin",
    "auth_password": "changeme",
    "services": [
        {"name": "todo"},
        {"name": "calendar"}
    ]
}));
```

### Configuration Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `gateway_url` | `string` | yes | URL of the MCP gateway server |
| `auth_user` | `string` | no | Basic auth username |
| `auth_password` | `string` | no | Basic auth password |
| `services` | `array` | yes | List of MCP services to expose |

### Service Configuration

Each service entry names an MCP service to bridge; when listed, the skill registers one
tool per service/tool pair. A service entry can carry a `tools` list:

```json
{
    "name": "todo",
    "tools": ["create_task", "list_tasks"]
}
```

When `services` is empty or omitted, the skill instead registers a single generic
`mcp_call` tool that takes `service`, `tool`, and `arguments`.

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
