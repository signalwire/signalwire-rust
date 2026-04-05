# Architecture

## Overview

The SignalWire Rust SDK is organised into three major subsystems that share a common foundation of types, logging, and configuration.

```
signalwire (crate root)
 ├── agent        – AI agent framework (AgentBase, AgentServer)
 │    ├── swml/   – SWML document builder & service
 │    ├── swaig/  – FunctionResult, tool definitions
 │    ├── contexts/ – Multi-step conversation flows
 │    ├── datamap/  – Server-side API tools (no webhook)
 │    ├── skills/   – Modular skill system (built-in + third-party)
 │    ├── prefabs/  – Ready-made agent archetypes
 │    ├── security/ – Auth, HMAC tokens, SSL
 │    └── server/   – AgentServer for multi-agent hosting
 ├── relay        – Real-time call/message control over WebSocket
 └── rest         – Async REST client for all SignalWire HTTP APIs
```

## Core Concepts

### SWML (SignalWire Markup Language)

SWML is a JSON document that tells the SignalWire platform what to do with a call. An agent's job is to produce SWML. The SDK builds SWML internally; you never need to write raw JSON.

### SWAIG (SignalWire AI Gateway)

SWAIG functions are tools the AI model can invoke mid-conversation. When the AI decides to call a tool, the platform POSTs to your agent's `/swaig` endpoint. The SDK routes the request to the correct handler and returns a `FunctionResult`.

### Prompt Object Model (POM)

POM is a structured prompt representation. Instead of a single string, prompts are arrays of titled sections with optional bullets and subsections. POM prompts are easier to compose programmatically and produce more consistent AI behaviour.

## Request Lifecycle

```
Inbound call
    │
    ▼
Platform requests SWML (POST /agent)
    │
    ▼
AgentBase builds SWML document
  ├─ dynamic_config_callback (if set)
  ├─ POM prompt assembly
  ├─ Tool definitions (native + DataMap + skills)
  ├─ Contexts / steps
  └─ Call-flow verbs (pre-answer, post-answer, post-AI)
    │
    ▼
Platform runs AI pipeline (STT → LLM → TTS)
    │
    ▼
AI invokes tool → POST /agent/swaig
    │
    ▼
AgentBase dispatches to handler → FunctionResult
    │
    ▼
Platform processes actions, continues conversation
    │
    ▼
Call ends → on_summary callback (if post-prompt configured)
```

## Composition Over Inheritance

`AgentBase` composes a `Service` (HTTP server), a `SessionManager`, and an optional `ContextBuilder`. In Rust, this is expressed as struct fields rather than class inheritance:

```rust
pub struct AgentBase {
    service: Service,
    session_manager: SessionManager,
    context_builder: Option<ContextBuilder>,
    // ...
}
```

Builder methods return `&mut Self` for chaining:

```rust
agent
    .prompt_add_section("Role", "You are helpful.", vec![])
    .define_tool("get_time", "Get current time", json!({}), handler, false)
    .add_language("English", "en-US", "rime.spore");
```

## Module Boundaries

| Module | Responsibility |
|--------|---------------|
| `agent` | `AgentBase`, `AgentOptions`, builder pattern, SWML rendering |
| `swml` | `Service`, `ServiceOptions`, low-level SWML document construction |
| `swaig` | `FunctionResult` with action helpers (connect, send_sms, hangup, etc.) |
| `contexts` | `ContextBuilder`, `Context`, `Step`, `GatherInfo`, `GatherQuestion` |
| `datamap` | `DataMap` builder for server-side API tools |
| `skills` | `SkillBase`, `SkillManager`, `SkillRegistry`, built-in skills |
| `prefabs` | `InfoGathererAgent`, `SurveyAgent`, `FAQBotAgent`, `ReceptionistAgent`, `ConciergeAgent` |
| `server` | `AgentServer` for hosting multiple agents |
| `security` | `SessionManager`, basic auth, HMAC token generation |
| `logging` | Structured logging with `SIGNALWIRE_LOG_LEVEL` |
| `relay` | `RelayClient`, call/message control, action objects |
| `rest` | `RestClient`, 21 namespaced API surfaces |

## Thread Safety

All public types are `Send + Sync`. Handlers are stored as `Arc<Box<dyn Fn(...) + Send + Sync>>`. The agent itself is typically cloned per-request when used with `AgentServer`.

## Error Handling

The SDK uses `Result<T, Box<dyn std::error::Error>>` for fallible operations. `FunctionResult` is infallible by design -- tool handlers always return a response, even if it describes an error condition.
