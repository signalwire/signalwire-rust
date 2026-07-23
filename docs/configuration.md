# Configuration

## Environment Variables

The SDK reads configuration from environment variables at startup.

### Agent Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SWML_BASIC_AUTH_USER` | auto-generated | Basic auth username |
| `SWML_BASIC_AUTH_PASSWORD` | auto-generated | Basic auth password |
| `SWML_PROXY_URL_BASE` | none | Public base URL when behind a reverse proxy |
| `SWML_SSL_ENABLED` | `false` | Enable HTTPS (`true`, `1`, `yes`) |
| `SWML_SSL_CERT_PATH` | none | Path to SSL certificate PEM file |
| `SWML_SSL_KEY_PATH` | none | Path to SSL private key PEM file |

### Security Environment Variables

Read by `SecurityConfig` (`src/core/security_config.rs`); all are secure-by-default.

| Variable | Default | Description |
|----------|---------|-------------|
| `SWML_DOMAIN` | none | Public domain used when composing HTTPS URLs |
| `SWML_SSL_VERIFY_MODE` | `CERT_REQUIRED` | TLS peer-certificate verification mode |
| `SWML_ALLOWED_HOSTS` | `*` | Comma-separated host allowlist for inbound requests |
| `SWML_CORS_ORIGINS` | `*` | Comma-separated allowed CORS origins |
| `SWML_MAX_REQUEST_SIZE` | `10485760` | Maximum inbound request body size, in bytes |
| `SWML_RATE_LIMIT` | `60` | Requests-per-minute rate limit |
| `SWML_REQUEST_TIMEOUT` | `30` | Request timeout, in seconds |
| `SWML_USE_HSTS` | `true` | Emit the `Strict-Transport-Security` header |
| `SWML_HSTS_MAX_AGE` | `31536000` | `max-age` (seconds) for the HSTS header |
| `SWML_ALLOW_PRIVATE_URLS` | `false` | Permit fetches to private/loopback URLs (SSRF guard escape hatch) |
| `SWML_SKIP_SCHEMA_VALIDATION` | `false` | Skip SWML schema validation (unsafe; testing only) |

### RELAY and REST Environment Variables

| Variable | Description |
|----------|-------------|
| `SIGNALWIRE_PROJECT_ID` | Project identifier |
| `SIGNALWIRE_API_TOKEN` | API token |
| `SIGNALWIRE_SPACE` | Space hostname (e.g. `example.signalwire.com`) |
| `SIGNALWIRE_REST_BASE_URL` | Override the REST base URL (used by `RestClient::from_env`). When set, it replaces the `https://{SIGNALWIRE_SPACE}` resolution — point the client at a regional host, a proxy, or a local fixture without a code change; `SIGNALWIRE_SPACE` is then not required |
| `SIGNALWIRE_RELAY_HOST` | Override the RELAY WebSocket host (advanced/testing) |
| `SIGNALWIRE_RELAY_SCHEME` | Override the RELAY WebSocket scheme (`ws`/`wss`; default `wss`) |
| `SIGNALWIRE_RELAY_CA_FILE` | Path to a PEM CA bundle for the RELAY WebSocket TLS trust store |
| `SIGNALWIRE_REST_CA_FILE` | Path to a PEM CA bundle for the REST HTTP client TLS trust store |
| `SIGNALWIRE_SIGNING_KEY` | Webhook-signature signing key for the agent |

### Logging Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SIGNALWIRE_LOG_LEVEL` | `info` | Log level: `debug`, `info`, `warn`, `error` |
| `SIGNALWIRE_LOG_MODE` | normal | Set to `off` to suppress all logging |

## Programmatic Configuration

### AgentOptions

```rust
use signalwire::agent::AgentOptions;

let mut opts = AgentOptions::new("my-agent");
opts.route = Some("/my-route".to_string());
opts.host = Some("0.0.0.0".to_string());
opts.port = Some(8080);
opts.basic_auth_user = Some("user".to_string());
opts.basic_auth_password = Some("pass".to_string());
opts.auto_answer = true;
opts.record_call = false;
opts.use_pom = true;
```

### AI Parameters

<!-- snippet-setup -->
```rust
use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;

let mut agent = AgentBase::new(AgentOptions::new("config-guide"));
```

```rust
agent.set_params(json!({
    "ai_model": "gpt-4.1-nano",
    "end_of_speech_timeout": 500,
    "attention_timeout": 15000,
    "background_file_volume": -20,
    "barge_match_string": "stop",
}));
```

### Recording Configuration

Recording is enabled through `AgentOptions` (the `record_call` flag) or by setting AI
params. Use the fluent option builder, or assign the field directly:

```rust
let agent = AgentBase::new(AgentOptions::new("recorder").record_call(true));
```

### Proxy Configuration

```rust
// Via environment variable (preferred)
// export SWML_PROXY_URL_BASE=https://agents.example.com

// Or programmatically
agent.manual_set_proxy_url("https://agents.example.com");
```

### SSL Configuration

```bash
export SWML_SSL_ENABLED=true
export SWML_SSL_CERT_PATH=/etc/ssl/certs/agent.pem
export SWML_SSL_KEY_PATH=/etc/ssl/private/agent-key.pem
```

## Kubernetes Configuration

For Kubernetes deployments, set the port via environment:

```rust
let port = std::env::var("PORT")
    .ok()
    .and_then(|p| p.parse::<u16>().ok())
    .unwrap_or(8080);

let mut opts = AgentOptions::new("k8s-agent");
opts.host = Some("0.0.0.0".to_string());
opts.port = Some(port);
```

Health and readiness endpoints are automatically available at `/health` and `/ready`.

## Multi-Agent Configuration

When hosting multiple agents, use `AgentServer`:

```rust
use signalwire::AgentServer;

let agent_a = AgentBase::new(AgentOptions::new("agent-a"));
let agent_b = AgentBase::new(AgentOptions::new("agent-b"));

let mut server = AgentServer::new(Some("0.0.0.0"), Some(3000));
server.register(agent_a, Some("/agent-a")).unwrap();
server.register(agent_b, Some("/agent-b")).unwrap();
server.run(None, None);
```

Each agent retains its own route, auth, and configuration.
