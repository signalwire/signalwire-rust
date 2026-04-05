# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

This is the SignalWire AI Agents Rust SDK -- a Rust port of the Python SignalWire AI Agents framework. It provides tools for building, deploying, and managing AI agents as microservices that expose HTTP endpoints to interact with the SignalWire platform.

**Package:** `signalwire` on [crates.io](https://crates.io/crates/signalwire)

## Development Commands

### Building

```bash
cargo build
```

### Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run tests for a specific module
cargo test logging
cargo test swml
cargo test agent
cargo test swaig
cargo test relay
cargo test rest

# Run a single test by name
cargo test test_logger_creation

# Coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

### Linting and Formatting

```bash
# Lint with Clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting without applying
cargo fmt -- --check
```

### Running Examples

```bash
cargo run --example simple_agent
cargo run --example contexts_demo
```

## Toolchain

- **Rust edition:** 2024 (rustc 1.85+)
- **Serialization:** `serde` / `serde_json` for all JSON handling
- **Crypto:** `hmac` + `sha2` for HMAC-SHA256 token signing, `base64` for encoding
- **Randomness:** `rand` for secure secret generation (basic auth passwords, session secrets)
- **Time:** `chrono` for timestamps and formatting
- **Regex:** `regex` for SIP username extraction and input validation
- **Logging:** `log` facade + `env_logger` backend, custom `Logger` struct for SDK convention
- **Async runtime:** `tokio` (dev-dependency, used by RELAY client and REST client)

## Architecture Overview

### Module Layout

```
signalwire-rust/
├── src/
│   ├── lib.rs              # Crate root, re-exports all public modules
│   ├── logging.rs          # Logger with level filtering and env var config
│   ├── swml/               # SWML document model, builder, schema validation
│   ├── agent/              # AgentBase builder, AI config, prompts, dynamic config
│   ├── swaig/              # FunctionResult, tool registry, SWAIG dispatch
│   ├── datamap/            # DataMap builder for server-side tools
│   ├── contexts/           # ContextBuilder, Context, Step workflows
│   ├── skills/             # SkillBase trait, SkillManager, 18 built-in skills
│   ├── prefabs/            # Pre-built agents (InfoGatherer, Survey, FAQ, etc.)
│   ├── server/             # AgentServer for multi-agent hosting
│   ├── relay/              # RELAY WebSocket client (Blade/JSON-RPC 2.0)
│   ├── rest/               # REST HTTP client with namespaced resources
│   ├── security/           # SessionManager, HMAC tokens, auth middleware
│   └── cli/                # swaig-test binary entry point
├── examples/               # Example agents (one .rs file per example)
├── docs/                   # Markdown documentation
├── relay/                  # RELAY-specific docs and examples
│   ├── docs/
│   └── examples/
├── rest/                   # REST-specific docs and examples
│   ├── docs/
│   └── examples/
├── tests/                  # Integration tests
├── bin/                    # Additional binaries
├── Cargo.toml              # Package manifest and dependencies
└── Cargo.lock              # Locked dependency versions
```

### Core Components (12)

1. **Logging** (`src/logging.rs`) -- Level-filtered logger reading `SIGNALWIRE_LOG_LEVEL` and `SIGNALWIRE_LOG_MODE` from the environment. Custom `Logger` struct wraps the `log` facade.

2. **SWML** (`src/swml/`) -- Document model for SignalWire Markup Language. Builds JSON documents with sections and verbs. Schema loaded from embedded `schema.json`, 38 verb methods auto-generated from schema definitions.

3. **AgentBase** (`src/agent/`) -- Central agent struct built via the builder pattern. Composes prompt management, tool registry, skill manager, AI config, and HTTP serving. Renders the 5-phase SWML pipeline (pre-answer, answer, post-answer, AI, post-AI).

4. **SwaigFunctionResult** (`src/swaig/`) -- Response builder for tool calls with 40+ action methods (connect, hangup, say, send_sms, update_global_data, toggle_functions, execute_rpc, payment helpers). All methods return `&mut Self` for chaining.

5. **DataMap** (`src/datamap/`) -- Fluent builder for server-side API tools that execute on SignalWire's servers without webhook infrastructure. Supports webhook config, expressions, variable expansion.

6. **Contexts & Steps** (`src/contexts/`) -- ContextBuilder, Context, and Step structs for structured multi-step conversation workflows. Validation ensures single contexts are named "default".

7. **Skills** (`src/skills/`) -- `SkillBase` trait, `SkillManager`, `SkillRegistry`. 18 built-in skills (datetime, math, joke, weather_api, web_search, wikipedia_search, google_maps, spider, datasphere, datasphere_serverless, swml_transfer, play_background_file, api_ninjas_trivia, native_vector_search, info_gatherer, claude_skills, mcp_gateway, custom_skills).

8. **Prefabs** (`src/prefabs/`) -- Ready-to-use agent archetypes: InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent. Each configurable through builder methods.

9. **AgentServer** (`src/server/`) -- Multi-agent hosting. Register/unregister agents by route, SIP routing, static file serving with path traversal protection, health/ready endpoints, root index page.

10. **RELAY** (`src/relay/`) -- WebSocket client for real-time call control. Blade/JSON-RPC 2.0 protocol, 4 correlation mechanisms (JSON-RPC id, call_id, control_id, tag), auto-reconnect with exponential backoff, 57+ calling methods, SMS/MMS messaging.

11. **REST** (`src/rest/`) -- Async HTTP client with Basic Auth. CrudResource trait for List/Create/Get/Update/Delete. 21 namespaced API surfaces (Fabric, Calling, Video, Datasphere, Compat, PhoneNumbers, SIP, Queues, Recordings, and more). Pagination support.

12. **Security** (`src/security/`) -- SessionManager with HMAC-SHA256 token creation and timing-safe validation. Random 32-byte secrets. Auth middleware for basic auth with timing-safe comparison.

### HTTP Endpoints (served by AgentBase)

| Path | Method | Auth | Purpose |
|------|--------|------|---------|
| `/` | POST | Basic | Returns rendered SWML document |
| `/swaig` | POST | Basic | SWAIG tool dispatch |
| `/post_prompt` | POST | Basic | Post-prompt summary callback |
| `/health` | GET | None | Health check |
| `/ready` | GET | None | Readiness check |

## Rust-Specific Patterns

### Traits Instead of Inheritance

The Python SDK uses class inheritance (AgentBase extends SWMLService, skills extend SkillBase). In Rust, use traits:

```rust
pub trait SkillBase: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn setup(&mut self, config: &HashMap<String, serde_json::Value>) -> Result<(), SkillError>;
    fn register_tools(&self, agent: &mut AgentBase);
}
```

### Builder Pattern for AgentBase

Replace Python's constructor-with-subclassing pattern with a typed builder:

```rust
let agent = AgentBase::builder("my-agent", "/agent")
    .add_language("English", "en-US", "rime.spore")
    .prompt_add_section("Role", "You are a helpful assistant.")
    .define_tool("get_time", "Get the current time", handler_fn)
    .set_prompt_llm_params(LlmParams { temperature: Some(0.7), ..Default::default() })
    .build();
```

### Arc<dyn Fn> for Tool Handlers

Tool handlers must be thread-safe closures stored behind `Arc`:

```rust
type ToolHandler = Arc<dyn Fn(HashMap<String, Value>, HashMap<String, Value>) -> FunctionResult + Send + Sync>;
```

This allows handlers to be cloned for dynamic config (clone agent, apply callback, render from clone).

### derive(Clone) for Dynamic Config

Dynamic configuration clones the entire agent, applies a callback, renders SWML from the clone, then discards it. All agent structs must `#[derive(Clone)]` to support this. The original agent is never mutated by a request.

### Result Types for Fallible Operations

Use `Result<T, E>` instead of exceptions. Define SDK-specific error enums:

```rust
#[derive(Debug, thiserror::Error)]
pub enum SignalWireError {
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("SWML rendering error: {0}")]
    Swml(String),
    #[error("REST API error: {status} {body}")]
    Rest { status: u16, body: String },
    #[error("RELAY connection error: {0}")]
    Relay(String),
}
```

### Thread Safety for Shared State

All shared mutable state (global data, tool registry, RELAY correlation maps) must be protected:

- `Arc<RwLock<HashMap<...>>>` for data read often, written rarely (tool registry, global data)
- `Arc<Mutex<HashMap<...>>>` for RELAY correlation maps with frequent writes
- `tokio::sync::RwLock` when held across `.await` points

### Serde for All Serialization

Every struct that crosses a JSON boundary derives `Serialize` and `Deserialize`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwmlDocument {
    pub version: String,
    pub sections: HashMap<String, Vec<Value>>,
}
```

Use `#[serde(skip_serializing_if = "Option::is_none")]` to match the Python SDK's behavior of omitting empty/None fields.

### HMAC Token Security

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;  // timing-safe comparison

type HmacSha256 = Hmac<Sha256>;
```

Tokens encode `function_name:call_id:expiry`, are HMAC-signed, then base64-encoded. Validation uses constant-time comparison.

### Async for RELAY and REST

RELAY and REST clients use `async`/`await` with `tokio`:

```rust
impl RelayClient {
    pub async fn run(&self) -> Result<(), SignalWireError> { ... }
}

impl RestClient {
    pub async fn get(&self, path: &str) -> Result<Value, SignalWireError> { ... }
}
```

AgentBase itself is synchronous (serves HTTP via a sync framework or spawns a runtime internally).

## File Locations

| What | Where |
|------|-------|
| Crate root | `src/lib.rs` |
| Logger | `src/logging.rs` |
| CLI binary | `src/cli/main.rs` |
| Package manifest | `Cargo.toml` |
| Locked dependencies | `Cargo.lock` |
| Integration tests | `tests/` |
| Examples | `examples/` |
| Top-level docs | `docs/` |
| RELAY docs & examples | `relay/docs/`, `relay/examples/` |
| REST docs & examples | `rest/docs/`, `rest/examples/` |
| SWML schema | `src/swml/schema.json` (embedded) |

## Environment Variables

| Variable | Used by | Description |
|----------|---------|-------------|
| `SIGNALWIRE_PROJECT_ID` | RELAY, REST | Project identifier |
| `SIGNALWIRE_API_TOKEN` | RELAY, REST | API token |
| `SIGNALWIRE_SPACE` | RELAY, REST | Space hostname (e.g. `example.signalwire.com`) |
| `SWML_BASIC_AUTH_USER` | Agents | Basic auth username (auto-generated if unset) |
| `SWML_BASIC_AUTH_PASSWORD` | Agents | Basic auth password (auto-generated if unset) |
| `SWML_PROXY_URL_BASE` | Agents | Base URL when behind a reverse proxy |
| `SWML_SSL_ENABLED` | Agents | Enable HTTPS (`true`, `1`, `yes`) |
| `SWML_SSL_CERT_PATH` | Agents | Path to SSL certificate |
| `SWML_SSL_KEY_PATH` | Agents | Path to SSL private key |
| `SIGNALWIRE_LOG_LEVEL` | All | Logging level (`debug`, `info`, `warn`, `error`) |
| `SIGNALWIRE_LOG_MODE` | All | Set to `off` to suppress all logging |
