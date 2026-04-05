# Agent Guide

## Creating an Agent

Every AI agent starts with `AgentBase`. The agent produces SWML for the SignalWire platform and handles SWAIG tool callbacks.

```rust
use signalwire::agent::{AgentBase, AgentOptions};

fn main() {
    let mut agent = AgentBase::new(AgentOptions::new("my-agent"));

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a helpful customer service agent.",
        vec![],
    );

    agent.prompt_add_section("Instructions", "", vec![
        "Greet the caller warmly",
        "Answer questions about our products",
        "Transfer to a human if you cannot help",
    ]);

    agent.run();
}
```

## Prompt Configuration (POM)

The Prompt Object Model structures prompts as titled sections:

```rust
// Top-level section with body text
agent.prompt_add_section("Role", "You are a sales assistant.", vec![]);

// Section with bullet points
agent.prompt_add_section("Rules", "", vec![
    "Never discuss competitor products",
    "Always confirm the order before processing",
]);

// Subsection
agent.prompt_add_subsection("Role", "Tone", "Be friendly and professional.");

// Append to an existing section
agent.prompt_add_to_section("Rules", None, vec![
    "Offer a discount if the caller hesitates",
]);
```

## Defining Tools

Tools are SWAIG functions the AI can call mid-conversation:

```rust
use signalwire::swaig::FunctionResult;
use serde_json::json;

agent.define_tool(
    "check_order",
    "Look up an order by ID",
    json!({
        "order_id": {"type": "string", "description": "The order ID to look up"}
    }),
    Box::new(|args, _raw| {
        let order_id = args.get("order_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        FunctionResult::with_response(&format!("Order {order_id} is shipped."))
    }),
    false, // not secure (no HMAC)
);
```

## Dynamic Configuration

For multi-tenant or per-request customisation, register a dynamic config callback:

```rust
use std::sync::Arc;

agent.set_dynamic_config_callback(Arc::new(Box::new(
    |query_params, body_params, headers, agent| {
        let tier = query_params.get("tier").map(|s| s.as_str()).unwrap_or("standard");

        if tier == "premium" {
            agent.add_language("English", "en-US", "inworld.Sarah");
            agent.set_params_value("end_of_speech_timeout", json!(300));
        } else {
            agent.add_language("English", "en-US", "inworld.Mark");
        }

        agent.prompt_add_section("Role", "You are a helpful assistant.", vec![]);
    },
)));
```

## Languages and Voices

```rust
agent.add_language("English", "en-US", "rime.spore");
agent.add_language("Spanish", "es-ES", "inworld.Sarah");
```

## LLM Parameters

```rust
agent.set_prompt_llm_params(json!({
    "temperature": 0.3,
    "top_p": 0.9,
    "barge_confidence": 0.6,
}));

agent.set_post_prompt_llm_params(json!({
    "temperature": 0.1,
}));
```

## Hints

Speech recognition hints improve accuracy for domain-specific terms:

```rust
agent.add_hints(vec!["SignalWire", "SWML", "SWAIG"]);
```

## Global Data

Session-wide key/value pairs accessible in prompts via `${global_data.key}`:

```rust
agent.set_global_data(json!({
    "status": "active",
    "customer_tier": "premium",
}));
```

## Post-Prompt and Summaries

```rust
agent.set_post_prompt("Summarise the call: customer name, issue, resolution.");

agent.set_summary_callback(Arc::new(Box::new(|summary, raw_data, headers| {
    println!("Call summary: {summary}");
})));
```

## Call Flow Verbs

Insert SWML verbs at specific points in the call lifecycle:

```rust
agent.add_pre_answer_verb("play", json!({"url": "say:Please hold..."}));
agent.add_post_answer_verb("record", json!({"stereo": true}));
agent.add_post_ai_verb("hangup", json!({}));
```

## Running the Agent

```rust
// Single agent on port 3000
agent.run();

// Or get the underlying app for custom hosting
let app = agent.get_app();
```

## Multi-Agent Server

```rust
use signalwire::server::AgentServer;

let mut server = AgentServer::new("0.0.0.0", 3000);
server.add_agent(sales_agent);
server.add_agent(support_agent);
server.run();
```

## CLI Testing

Test locally without a running server:

```bash
cargo run --bin swaig-test -- --list-tools examples/simple_agent.rs
cargo run --bin swaig-test -- --dump-swml examples/simple_agent.rs
cargo run --bin swaig-test -- --exec get_time examples/simple_agent.rs
```
