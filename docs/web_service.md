# Web Service

## Overview

Every agent is an HTTP server. The SDK handles routing, authentication, and request dispatching automatically.

## Default Endpoints

When an agent is created with route `/agent`, these endpoints are available:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/agent` | POST | SWML document generation |
| `/agent/swaig` | POST | SWAIG function dispatch |
| `/agent/debug` | GET | Debug info (SWML dump, tool list) |
| `/health` | GET | Health check (returns 200) |
| `/ready` | GET | Readiness check (returns 200) |

## Request Flow

### SWML Request (POST /agent)

1. Platform sends POST with call metadata (caller ID, call ID, etc.)
2. SDK validates basic auth credentials
3. If `dynamic_config_callback` is set, it is called with query params, body, headers
4. Agent renders SWML document and returns it as JSON

### SWAIG Request (POST /agent/swaig)

1. Platform sends POST with function name and arguments
2. SDK validates basic auth (and HMAC token if function is secure)
3. SDK dispatches to the registered handler
4. Handler returns `FunctionResult`
5. SDK serialises and returns the response

## Server Configuration

### Host and Port

```rust
let mut opts = AgentOptions::new("my-agent");
opts.host = Some("0.0.0.0".to_string());
opts.port = Some(8080);
```

### Starting the Server

```rust
// Blocking run
agent.run();

// Or get the app for custom hosting (e.g. behind actix-web or axum)
let app = agent.get_app();
```

## Multi-Agent Server

`AgentServer` mounts multiple agents on a single HTTP server:

```rust
use signalwire::server::AgentServer;

let mut server = AgentServer::new("0.0.0.0", 3000);
server.add_agent(sales_agent);    // /sales
server.add_agent(support_agent);  // /support
server.run();
```

Each agent keeps its own route prefix, authentication, and SWAIG endpoints.

## Custom Endpoints

Add custom routes alongside the agent:

```rust
// Custom health endpoint with application-specific checks
// Custom API endpoint for non-voice interactions
// Custom static file serving

// Override _register_routes to add custom endpoints alongside agent routes
```

## Authentication

All SWML and SWAIG endpoints require basic authentication. The health and readiness endpoints (`/health`, `/ready`) are unauthenticated.

### Headers

```
Authorization: Basic base64(username:password)
```

### Retrieving Credentials

```rust
let (user, pass) = agent.get_basic_auth_credentials();
println!("Configure your phone number with: http://{user}:{pass}@host:port/agent");
```

## Proxy Support

When behind a reverse proxy, set the base URL so SWML webhook URLs are correct:

```bash
export SWML_PROXY_URL_BASE=https://agents.example.com
```

Without this, the SDK generates `http://localhost:3000/agent/swaig` as the webhook URL, which the platform cannot reach.

## CORS

The SDK does not add CORS headers by default. If you need CORS for browser-based testing, configure it at the reverse proxy level or add middleware to the app.

## Request/Response Format

### SWML Response (application/json)

```json
{
  "version": "1.0.0",
  "sections": {
    "main": [
      {"answer": {}},
      {"ai": {
        "prompt": {"text": "..."},
        "SWAIG": {"functions": [...]},
        "languages": [...],
        "params": {...}
      }}
    ]
  }
}
```

### SWAIG Response (application/json)

```json
{
  "response": "The order has shipped.",
  "action": [
    {"update_global_data": {"status": "shipped"}}
  ]
}
```
