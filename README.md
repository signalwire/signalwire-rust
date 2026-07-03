<!-- Header -->
<div align="center">
    <a href="https://signalwire.com" target="_blank">
        <img src="https://github.com/user-attachments/assets/0c8ed3b9-8c50-4dc6-9cc4-cc6cd137fd50" width="500" />
    </a>

# SignalWire SDK for Rust

_Build AI voice agents, control live calls over WebSocket, and manage every SignalWire resource over REST -- all from one crate._

<p align="center">
  <a href="https://developer.signalwire.com/sdks/agents-sdk" target="_blank">Documentation</a> &middot;
  <a href="https://github.com/signalwire/signalwire-docs/issues/new/choose" target="_blank">Report an Issue</a> &middot;
  <a href="https://crates.io/crates/signalwire-sdk" target="_blank">crates.io</a>
</p>

<a href="https://discord.com/invite/F2WNYTNjuF" target="_blank"><img src="https://img.shields.io/badge/Discord%20Community-5865F2" alt="Discord" /></a>
<a href="LICENSE"><img src="https://img.shields.io/badge/MIT-License-blue" alt="MIT License" /></a>
<a href="https://github.com/signalwire/signalwire-rust" target="_blank"><img src="https://img.shields.io/github/stars/signalwire/signalwire-rust" alt="GitHub Stars" /></a>

<a href="https://codespaces.new/signalwire/signalwire-rust" target="_blank"><img src="https://github.com/codespaces/badge.svg" alt="Open in GitHub Codespaces" /></a>
<a href="https://replit.com/new/github/signalwire/signalwire-rust" target="_blank"><img src="https://replit.com/badge/github/signalwire/signalwire-rust" alt="Run on Replit" /></a>

</div>

---

## What's in this SDK

| Capability | What it does | Quick link |
|-----------|-------------|------------|
| **AI Agents** | Build voice agents that handle calls autonomously -- the platform runs the AI pipeline, your code defines the persona, tools, and call flow | [Agent Guide](#ai-agents) |
| **RELAY Client** | Control live calls and SMS/MMS in real time over WebSocket -- answer, play, record, collect DTMF, conference, transfer, and more | [RELAY docs](relay/README.md) |
| **REST Client** | Manage SignalWire resources over HTTP -- phone numbers, SIP endpoints, Fabric AI agents, video rooms, messaging, and 21 API namespaces | [REST docs](rest/README.md) |

```bash
cargo add signalwire-sdk
```

> Published as `signalwire-sdk` on crates.io. The library import path is still `signalwire`, so `use signalwire::...` works unchanged.

---

## AI Agents

Each agent is a self-contained microservice that generates [SWML](docs/swml_service_guide.md) (SignalWire Markup Language) and handles [SWAIG](docs/swaig_reference.md) (SignalWire AI Gateway) tool calls. The SignalWire platform runs the entire AI pipeline (STT, LLM, TTS) -- your agent just defines the behavior.

```rust
use serde_json::json;
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;

fn main() {
    let mut agent = AgentBase::new(AgentOptions::new("my-agent"));

    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section("Role", "You are a helpful assistant.", vec![]);

    agent.define_tool(
        "get_time",
        "Get the current time",
        json!({}),
        Box::new(|_args, _raw| {
            let now = chrono::Local::now().format("%H:%M:%S");
            FunctionResult::with_response(&format!("The time is {now}"))
        }),
        false,
    );

    agent.run();
}
```

Test locally without binding a port -- introspect an example by name, or
point at a running SWAIG endpoint:

```bash
# Introspect a SWMLService example in-process (by example name)
cargo run --bin swaig-test -- --example swmlservice_swaig_standalone --list-tools
cargo run --bin swaig-test -- --example swmlservice_swaig_standalone --dump-swml

# Exercise a live SWAIG endpoint over HTTP
cargo run --bin swaig-test -- --url http://user:pass@localhost:3000/ --list-tools
cargo run --bin swaig-test -- --url http://user:pass@localhost:3000/ --exec get_time
```

### Agent Features

- **Prompt Object Model (POM)** -- structured prompt composition via `prompt_add_section()`
- **SWAIG tools** -- define functions with `define_tool()` that the AI calls mid-conversation, with native access to the call's media stack
- **Skills system** -- add capabilities with one-liners: `agent.add_skill("datetime", json!({}))`
- **Contexts and steps** -- structured multi-step workflows with navigation control
- **DataMap tools** -- tools that execute on SignalWire's servers, calling REST APIs without your own webhook
- **Dynamic configuration** -- per-request agent customization for multi-tenant deployments
- **Call flow control** -- pre-answer, post-answer, and post-AI verb insertion
- **Prefab agents** -- ready-to-use archetypes (InfoGatherer, Survey, FAQ, Receptionist, Concierge)
- **Multi-agent hosting** -- serve multiple agents on a single server with `AgentServer`
- **SIP routing** -- route SIP calls to agents based on usernames
- **Session state** -- persistent conversation state with global data and post-prompt summaries
- **Security** -- auto-generated basic auth, function-specific HMAC tokens, SSL support
- **Serverless** -- deploy to Lambda, Cloud Functions, Azure Functions

### Agent Examples

The [`examples/`](examples/) directory contains working examples:

| Example | What it demonstrates |
|---------|---------------------|
| [simple_agent.rs](examples/simple_agent.rs) | POM prompts, SWAIG tools, multilingual support, LLM tuning |
| [contexts_demo.rs](examples/contexts_demo.rs) | Multi-persona workflow with context switching and step navigation |
| [data_map_demo.rs](examples/data_map_demo.rs) | Server-side API tools without webhooks |
| [skills_demo.rs](examples/skills_demo.rs) | Loading built-in skills (datetime, math) |
| [call_flow_and_actions_demo.rs](examples/call_flow_and_actions_demo.rs) | Call flow verbs, debug events, FunctionResult actions |
| [session_and_state_demo.rs](examples/session_and_state_demo.rs) | OnSummary, global data, post-prompt summaries |
| [multi_agent_server.rs](examples/multi_agent_server.rs) | Multiple agents on one server |
| [lambda_agent.rs](examples/lambda_agent.rs) | AWS Lambda deployment |
| [comprehensive_dynamic_agent.rs](examples/comprehensive_dynamic_agent.rs) | Per-request dynamic configuration, multi-tenant routing |

See [examples/README.md](examples/README.md) for the full list organized by category.

---

## RELAY Client

Real-time call control and messaging over WebSocket. The RELAY client connects to SignalWire via the Blade protocol and gives you imperative control over live phone calls and SMS/MMS. The client runs its event loop on a background thread, so the handler closures are synchronous.

```rust
use signalwire::relay::Client;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
    let client = Arc::new(Client::from_env()?);

    client.on_call(|call, _event| {
        let _ = call.answer();
        let action = call.play(serde_json::json!({
            "play": [{
                "type": "tts",
                "params": {"text": "Welcome to SignalWire!"}
            }]
        }));
        let _ = action.is_done();
        let _ = call.hangup();
    });

    println!("Waiting for inbound calls ...");
    client.connect()?;
    client.receive(&["default".to_string()]);

    // Block while the relay loop runs in the background.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
```

- 50+ calling methods (play, record, collect, detect, tap, stream, AI, conferencing, and more)
- SMS/MMS messaging with delivery tracking
- `Action` objects with `is_done()`, `state()`, `result()`, `on_completed()`, `stop()`
- Auto-reconnect with exponential backoff

See the **[RELAY documentation](relay/README.md)** for the full guide, API reference, and examples.

---

## REST Client

Blocking (synchronous) REST client for managing SignalWire resources and controlling calls over HTTP. No WebSocket and no async runtime required.

```rust
use serde_json::json;
use signalwire::rest::RestClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
    let client = RestClient::from_env().expect("missing SIGNALWIRE_* env vars");

    client.fabric().ai_agents().create(&json!({
        "name": "Support Bot",
        "prompt": {"text": "You are helpful."}
    }))?;

    client.calling().dial(json!({
        "from": "+15559876543",
        "to": "+15551234567",
        "url": "https://example.com/call-handler"
    }))?;

    let results = client.phone_numbers().search(&json!({ "area_code": "512" }))?;
    println!("{results:#?}");

    Ok(())
}
```

- Namespaced API surfaces: Fabric, Calling, Video, Datasphere, Phone Numbers, SIP, Queues, Recordings, and more
- Backed by `ureq` (blocking HTTP) with a reusable `ureq::Agent` for connection pooling
- `serde_json::Value` returns -- raw JSON, no wrapper objects; errors surface as `SignalWireRestError`

See the **[REST documentation](rest/README.md)** for the full guide, API reference, and examples.

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
signalwire = { package = "signalwire-sdk", version = "1" }
```

The crate is published as `signalwire-sdk` (the bare `signalwire` name is held by an unofficial wrapper); the `package =` rename keeps the import path as `use signalwire::...`.

Or with `cargo`:

```bash
cargo add signalwire-sdk
```

Requires Rust edition 2024 with a minimum supported `rustc` of 1.88 (let-chains, enforced via `rust-version` in `Cargo.toml`).

## Documentation

Full reference documentation is available at **[developer.signalwire.com/sdks/agents-sdk](https://developer.signalwire.com/sdks/agents-sdk)**.

Guides are also available in the [`docs/`](docs/) directory:

### Getting Started

- [Agent Guide](docs/agent_guide.md) -- creating agents, prompt configuration, dynamic setup
- [Architecture](docs/architecture.md) -- SDK architecture and core concepts
- [SDK Features](docs/sdk_features.md) -- feature overview, SDK vs raw SWML comparison

### Core Features

- [SWAIG Reference](docs/swaig_reference.md) -- function results, actions, post_data lifecycle
- [Contexts and Steps](docs/contexts_guide.md) -- structured workflows, navigation, gather mode
- [DataMap Guide](docs/datamap_guide.md) -- serverless API tools without webhooks
- [LLM Parameters](docs/llm_parameters.md) -- temperature, top_p, barge confidence tuning
- [SWML Service Guide](docs/swml_service_guide.md) -- low-level construction of SWML documents

### Skills and Extensions

- [Skills System](docs/skills_system.md) -- built-in skills and the modular framework
- [Third-Party Skills](docs/third_party_skills.md) -- creating and publishing custom skills
- [MCP Gateway](docs/mcp_gateway_reference.md) -- Model Context Protocol integration

### Deployment

- [CLI Guide](docs/cli_guide.md) -- `swaig-test` command reference
- [Cloud Functions](docs/cloud_functions_guide.md) -- Lambda, Cloud Functions, Azure deployment
- [Configuration](docs/configuration.md) -- environment variables, SSL, proxy setup
- [Security](docs/security.md) -- authentication and security model

### Reference

- [API Reference](docs/api_reference.md) -- complete type and method reference
- [Web Service](docs/web_service.md) -- HTTP server and endpoint details
- [Skills Parameter Schema](docs/skills_parameter_schema.md) -- skill parameter definitions

## Environment Variables

| Variable | Used by | Description |
|----------|---------|-------------|
| `SIGNALWIRE_PROJECT_ID` | RELAY, REST | Project identifier |
| `SIGNALWIRE_API_TOKEN` | RELAY, REST | API token |
| `SIGNALWIRE_SPACE` | RELAY, REST | Space hostname (e.g. `example.signalwire.com`) |
| `SWML_BASIC_AUTH_USER` | Agents | Basic auth username (default: auto-generated) |
| `SWML_BASIC_AUTH_PASSWORD` | Agents | Basic auth password (default: auto-generated) |
| `SWML_PROXY_URL_BASE` | Agents | Base URL when behind a reverse proxy |
| `SWML_SSL_ENABLED` | Agents | Enable HTTPS (`true`, `1`, `yes`) |
| `SWML_SSL_CERT_PATH` | Agents | Path to SSL certificate |
| `SWML_SSL_KEY_PATH` | Agents | Path to SSL private key |
| `SIGNALWIRE_LOG_LEVEL` | All | Logging level (`debug`, `info`, `warn`, `error`) |
| `SIGNALWIRE_LOG_MODE` | All | Set to `off` to suppress all logging |

## Testing, Linting, Formatting

Format, lint, and test go through three canonical scripts under `scripts/`. They
self-bootstrap the Rust toolchain (adding `rustfmt`/`clippy` if missing) and
resolve the repo from their own path, so they run identically from **any** working
directory. `scripts/run-ci.sh`'s FMT/LINT/TEST gates call these same scripts — no
drift between a local run and CI.

```bash
# Run the test suite (optional filter passes through to cargo)
bash scripts/run-tests.sh
bash scripts/run-tests.sh test_connect_returns_protocol_string

# Lint (cargo clippy --all-targets, -D warnings); --fix applies clippy autofixes
bash scripts/run-lint.sh

# Format in place; --check for verify-only (the CI FMT gate)
bash scripts/run-format.sh
bash scripts/run-format.sh --check
```

Direct cargo invocations still work when you want them:

```bash
# Run with verbose output
cargo test -- --nocapture

# Coverage (requires cargo-tarpaulin)
cargo tarpaulin --out html
```

## License

MIT -- see [LICENSE](LICENSE) for details.
