# Cloud Functions Guide

## Overview

SignalWire AI agents can be deployed to serverless platforms. The agent generates SWML and handles SWAIG callbacks in the same way -- the only difference is how HTTP requests reach the agent.

## The Serverless Adapter

The SDK ships a synchronous serverless adapter in `signalwire::serverless`. There is no
async runtime or `get_app()`: an agent satisfies the `RequestHandler` trait via its
`handle_request(method, path, headers, body) -> (u16, HashMap<String, String>, String)`
method, and `Adapter` translates platform event JSON to and from that call.

`Adapter::detect()` returns a `RuntimeEnvironment` (`Lambda`, `Gcf`, `Azure`, `Cgi`, or
`Server`) based on environment variables, so a single binary can branch at startup.

### Building the Agent

<!-- snippet: no-compile helper-function definition (item-only `fn create_agent`); the snippet checker compiles each fragment as a binary, so a fragment with no `fn main` cannot stand alone. Referenced by the adapter fragments below. -->
```rust
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn create_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions::new("lambda-agent").route("/"));

    agent.add_language("English", "en-US", "inworld.Mark");
    agent.prompt_add_section(
        "Role",
        "You are a helpful AI assistant running in a serverless function.",
        vec![],
    );

    agent.define_tool(
        "greet_user",
        "Greet a user by name",
        json!({"name": {"type": "string", "description": "User's name"}}),
        Box::new(|args, _raw| {
            let name = args.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("friend");
            FunctionResult::with_response(&format!("Hello, {name}! Welcome."))
        }),
        false,
    );

    agent
}
```

## AWS Lambda

`Adapter::handle_lambda(&agent, &event)` parses an API Gateway event and returns an API
Gateway response object:

<!-- snippet: no-compile item-only `fn lambda_entry` (no `fn main`) and calls the `create_agent` helper defined in the item-only block above; both are cross-fragment/item-level, which the per-fragment binary compile cannot stand up alone -->
```rust
use signalwire::serverless::Adapter;
use serde_json::Value;

fn lambda_entry(event: Value) -> Value {
    let agent = create_agent();
    Adapter::handle_lambda(&agent, &event)
}
```

### Environment Variables

Set these in your Lambda configuration:

```
SWML_BASIC_AUTH_USER=myuser
SWML_BASIC_AUTH_PASSWORD=mypassword
```

## Azure Functions

`Adapter::handle_azure(&agent, &request)` does the same for the Azure request shape:

<!-- snippet: no-compile item-only `fn azure_entry` (no `fn main`) and calls the cross-fragment `create_agent` helper; not a standalone compilation unit under the per-fragment binary compile -->
```rust
use signalwire::serverless::Adapter;
use serde_json::Value;

fn azure_entry(request: Value) -> Value {
    let agent = create_agent();
    Adapter::handle_azure(&agent, &request)
}
```

## Google Cloud Functions / CGI

For environments without a dedicated adapter method, call `handle_request` directly with
the method, path, headers, and body extracted from the incoming request, then build the
platform response from the returned `(status, headers, body)` tuple:

<!-- snippet-setup -->
```rust
use signalwire::agent::{AgentBase, AgentOptions};
use std::collections::HashMap;

let agent = AgentBase::new(AgentOptions::new("gcf-agent"));
let request_headers: HashMap<String, String> = HashMap::new();
let request_body = String::new();
```

```rust
let (status, headers, body) =
    agent.handle_request("POST", "/", &request_headers, Some(&request_body));
```

## Deployment Considerations

### Cold Starts

Agent construction is fast (no network calls). The first request after a cold start adds only the agent initialisation time.

### Statelessness

Each invocation creates a fresh agent. Session state is maintained by the SignalWire platform, not the agent. Use `global_data` and `on_summary` for persistent data.

### Authentication

Set `SWML_BASIC_AUTH_USER` and `SWML_BASIC_AUTH_PASSWORD` as environment variables in your serverless configuration. Do not use auto-generated credentials (they change on every cold start).

### URL Configuration

Set `SWML_PROXY_URL_BASE` to the function's public URL so SWML webhook URLs are correct:

```
SWML_PROXY_URL_BASE=https://abc123.execute-api.us-east-1.amazonaws.com/prod
```

### Timeouts

Ensure your serverless platform timeout exceeds the expected SWAIG function execution time. SWML generation is near-instant. SWAIG functions may take longer if they call external APIs.

### Package Size

The Rust SDK compiles to a single binary. Lambda deployment packages are typically under 10 MB.
