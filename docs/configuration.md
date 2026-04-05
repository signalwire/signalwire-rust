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

### RELAY and REST Environment Variables

| Variable | Description |
|----------|-------------|
| `SIGNALWIRE_PROJECT_ID` | Project identifier |
| `SIGNALWIRE_API_TOKEN` | API token |
| `SIGNALWIRE_SPACE` | Space hostname (e.g. `example.signalwire.com`) |

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

```rust
agent.set_record_call(true);
agent.set_record_format("wav");
agent.set_record_stereo(true);
```

### Proxy Configuration

```rust
// Via environment variable (preferred)
// export SWML_PROXY_URL_BASE=https://agents.example.com

// Or programmatically
agent.set_proxy_url("https://agents.example.com");
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
use signalwire::server::AgentServer;

let mut server = AgentServer::new("0.0.0.0", 3000);
server.add_agent(agent_a);  // route: /agent-a
server.add_agent(agent_b);  // route: /agent-b
server.run();
```

Each agent retains its own route, auth, and configuration.
