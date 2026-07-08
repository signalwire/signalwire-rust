# Web Service

## Overview

Every agent is an HTTP server. The SDK handles routing, authentication, and request dispatching automatically.

## Default Endpoints

Endpoints are served relative to the agent's route. With the default route `/`, these
are available (an agent mounted at `/sales` would serve `/sales`, `/sales/swaig`, etc.):

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/` (the route) | POST | Basic | SWML document generation |
| `/swaig` | POST | Basic | SWAIG function dispatch |
| `/post_prompt` | POST | Basic | Post-prompt summary callback |
| `/health` | GET | None | Health check (returns 200) |
| `/ready` | GET | None | Readiness check (returns 200) |

## Request Flow

### SWML Request (POST to the route)

1. Platform sends POST with call metadata (caller ID, call ID, etc.)
2. SDK validates basic auth credentials (and the `X-SignalWire-Signature` header when a signing key is set)
3. If `dynamic_config_callback` is set, it is called with query params, body, headers
4. Agent renders SWML document and returns it as JSON

### SWAIG Request (POST /swaig)

1. Platform sends POST with function name and arguments
2. SDK validates basic auth (and HMAC token if the function is secure)
3. SDK dispatches to the registered handler
4. Handler returns `FunctionResult`
5. SDK serialises and returns the response

## Server Configuration

<!-- snippet-setup -->
```rust
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::AgentServer;

let mut agent = AgentBase::new(AgentOptions::new("web-agent"));
let mut server = AgentServer::new(Some("0.0.0.0"), Some(3000));
```

### Host and Port

```rust
let mut opts = AgentOptions::new("my-agent");
opts.host = Some("0.0.0.0".to_string());
opts.port = Some(8080);
```

### Starting the Server

```rust
// Blocking run — serves on the host/port from AgentOptions
agent.run();
```

## Multi-Agent Server

`AgentServer` mounts multiple agents on a single HTTP server. `new` takes optional
host/port, and `register(agent, route)` returns a `Result`:

```rust
let sales_agent = AgentBase::new(AgentOptions::new("sales"));
let support_agent = AgentBase::new(AgentOptions::new("support"));

let mut multi = AgentServer::new(Some("0.0.0.0"), Some(3000));
multi.register(sales_agent, Some("/sales")).unwrap();
multi.register(support_agent, Some("/support")).unwrap();
multi.run(None, None);
```

Each agent keeps its own route prefix, authentication, and SWAIG endpoints.

## Static File Serving

`AgentServer` can serve static files alongside agents, with path-traversal protection:

```rust
server.serve_static("./public", "/static").unwrap();
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
println!("Configure your phone number with: http://{user}:{pass}@host:port/");
```

## Proxy Support

When behind a reverse proxy, set the base URL so SWML webhook URLs are correct:

```bash
export SWML_PROXY_URL_BASE=https://agents.example.com
```

Without this, the SDK generates `http://localhost:3000/swaig` as the webhook URL, which the platform cannot reach.

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
