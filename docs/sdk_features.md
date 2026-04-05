# SDK Features

## Feature Overview

The SignalWire Rust SDK provides three major capabilities in a single crate:

| Feature | Description |
|---------|-------------|
| AI Agents | Build voice agents that handle calls autonomously |
| RELAY Client | Real-time call/message control over WebSocket |
| REST Client | Manage SignalWire resources over HTTP |

## AI Agent Features

### Prompt Object Model (POM)

Structured prompt composition using titled sections, bullets, and subsections instead of raw text strings.

```rust
agent.prompt_add_section("Role", "You are a sales agent.", vec![]);
agent.prompt_add_section("Rules", "", vec!["Be helpful", "Be concise"]);
agent.prompt_add_subsection("Role", "Tone", "Professional but friendly.");
```

### SWAIG Tools

Define functions the AI can call mid-conversation. Handlers receive arguments and return `FunctionResult` with optional actions.

### Skills System

One-liner integration of reusable capabilities: `agent.add_skill("datetime", None)`. Built-in skills include datetime, math, joke, and mcp_gateway.

### Contexts and Steps

Structured multi-step workflows with navigation control. Each context can have its own persona, prompts, and tools.

### DataMap Tools

Server-side API tools that execute on SignalWire's servers without requiring your own webhook infrastructure.

### Dynamic Configuration

Per-request agent customisation via callbacks. Enables multi-tenant deployments where each caller gets a tailored experience.

### Call Flow Control

Insert SWML verbs at pre-answer, post-answer, and post-AI points in the call lifecycle.

### Prefab Agents

Ready-to-use agent archetypes:

| Prefab | Purpose |
|--------|---------|
| `InfoGathererAgent` | Collect structured data from callers |
| `SurveyAgent` | Conduct surveys with configurable questions |
| `FAQBotAgent` | Answer FAQs from a knowledge base |
| `ReceptionistAgent` | Route calls to departments |
| `ConciergeAgent` | Virtual concierge with amenity info |

### Multi-Agent Hosting

`AgentServer` hosts multiple agents on a single server, each at its own route.

### Session State

Persistent conversation state with `global_data` and post-prompt summaries.

### Security

Auto-generated basic auth, HMAC token signing for secure functions, and SSL support.

### Serverless Deployment

Deploy to AWS Lambda, Google Cloud Functions, or Azure Functions.

### CLI Testing

Test agents locally with `swaig-test`: list tools, dump SWML, execute functions.

## SDK vs Raw SWML

| Task | Raw SWML | SDK |
|------|----------|-----|
| Define a prompt | Write JSON by hand | `prompt_add_section()` |
| Add a tool | JSON function definition + webhook server | `define_tool()` with closure |
| Multi-step flow | Nested JSON contexts | `define_contexts()` builder |
| DataMap tool | Raw JSON data_map | `DataMap::new().webhook().output()` |
| Multi-agent | Manual routing | `AgentServer::add_agent()` |
| Auth | Manual header checks | Automatic basic auth |
| Testing | curl + manual JSON | `swaig-test` CLI |

## RELAY Features

- 57+ calling methods (play, record, collect, detect, tap, stream, conference, ...)
- SMS/MMS messaging with delivery tracking
- Action objects with `wait()`, `stop()`, `pause()`, `resume()`
- Auto-reconnect with exponential backoff
- Async/await with tokio

## REST Features

- 21 namespaced API surfaces
- Fabric (13 resource types), Calling (37 commands), Video, Datasphere
- Compat (Twilio-compatible), Phone Numbers, SIP, Queues, Recordings
- Connection pooling via reqwest
- Raw `serde_json::Value` returns
