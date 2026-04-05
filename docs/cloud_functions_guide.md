# Cloud Functions Guide

## Overview

SignalWire AI agents can be deployed to serverless platforms. The agent generates SWML and handles SWAIG callbacks in the same way -- the only difference is how HTTP requests reach the agent.

## AWS Lambda

### Setup

Use a Lambda-compatible HTTP adapter. The agent's `get_app()` method returns a framework-agnostic app that can be wrapped.

```rust
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn create_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "lambda-agent".to_string(),
        route: Some("/".to_string()),
        ..AgentOptions::new("lambda-agent")
    });

    agent.add_language("English", "en-US", "inworld.Mark");
    agent.prompt_add_section(
        "Role",
        "You are a helpful AI assistant running in AWS Lambda.",
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

### Lambda Handler

```rust
use lambda_http::{run, service_fn, Body, Error, Request, Response};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let agent = create_agent();
    let app = agent.get_app();
    run(service_fn(|req: Request| async {
        // Route to the agent app
        app.handle(req).await
    })).await
}
```

### Environment Variables

Set these in your Lambda configuration:

```
SWML_BASIC_AUTH_USER=myuser
SWML_BASIC_AUTH_PASSWORD=mypassword
```

## Google Cloud Functions

```rust
use signalwire::agent::{AgentBase, AgentOptions};

fn create_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions::new("gcf-agent"));
    agent.prompt_add_section("Role", "You are a helpful assistant.", vec![]);
    agent
}

// Export the handler for Cloud Functions
pub fn handler(req: HttpRequest) -> HttpResponse {
    let agent = create_agent();
    agent.handle_request(req)
}
```

## Azure Functions

```rust
use signalwire::agent::{AgentBase, AgentOptions};

fn create_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions::new("azure-agent"));
    agent.prompt_add_section("Role", "You are a helpful assistant.", vec![]);
    agent
}

pub async fn handler(req: HttpRequest) -> HttpResponse {
    let agent = create_agent();
    agent.handle_request(req)
}
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
