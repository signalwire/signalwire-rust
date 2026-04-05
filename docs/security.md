# Security

## Overview

The SDK provides three layers of security for protecting agent endpoints:

1. **Basic Authentication** -- username/password for all SWML and SWAIG endpoints
2. **HMAC Token Signing** -- per-function tokens that prevent unauthorized tool invocations
3. **SSL/TLS** -- encrypted transport with certificate configuration

## Basic Authentication

By default, the SDK auto-generates a random username and password at startup. Every request to the agent's SWML and SWAIG endpoints must include valid basic auth credentials.

### Auto-Generated Credentials

```rust
let agent = AgentBase::new(AgentOptions::new("my-agent"));
let (user, pass) = agent.get_basic_auth_credentials();
println!("Auth: {user}:{pass}");
```

### Custom Credentials

```rust
let mut opts = AgentOptions::new("my-agent");
opts.basic_auth_user = Some("myuser".to_string());
opts.basic_auth_password = Some("mypassword".to_string());
let agent = AgentBase::new(opts);
```

### Environment Variables

```bash
export SWML_BASIC_AUTH_USER=myuser
export SWML_BASIC_AUTH_PASSWORD=mypassword
```

Environment variables override programmatic values.

## HMAC Token Signing

When a tool is defined with `secure: true`, the SDK generates an HMAC-SHA256 token for that function's URL. The token is included in the SWML document. When the platform calls the function, it sends the token in the request. The SDK verifies the token before dispatching to the handler.

```rust
agent.define_tool(
    "transfer_funds",
    "Transfer money",
    json!({"amount": {"type": "number"}}),
    Box::new(|args, _raw| FunctionResult::with_response("Done.")),
    true,  // <-- secure: generates HMAC token
);
```

### How It Works

1. At SWML render time, the SDK generates `token = HMAC-SHA256(secret, function_url)`
2. The token is embedded in the SWAIG function definition
3. When the platform POSTs to the function, it includes the token
4. The SDK verifies `HMAC-SHA256(secret, url) == received_token`
5. If verification fails, the request is rejected with 403

## SSL/TLS Configuration

### Environment Variables

```bash
export SWML_SSL_ENABLED=true
export SWML_SSL_CERT_PATH=/path/to/cert.pem
export SWML_SSL_KEY_PATH=/path/to/key.pem
```

### Programmatic Configuration

SSL is configured via the underlying service options. The agent reads the environment variables automatically.

## Proxy URL

When running behind a reverse proxy (e.g. nginx, AWS ALB), the SDK needs to know the public URL to generate correct webhook URLs in SWML:

```bash
export SWML_PROXY_URL_BASE=https://agents.example.com
```

```rust
agent.set_proxy_url("https://agents.example.com");
```

## Session Management

The `SessionManager` handles:

- Session ID tracking across requests
- Credential rotation support
- Token validation for secure functions

```rust
let session_manager = SessionManager::with_defaults();
```

## Security Best Practices

1. **Always use basic auth in production** -- even if your agent is behind a firewall
2. **Use secure functions for sensitive tools** -- transfers, payments, data access
3. **Enable SSL** -- especially when the agent is publicly accessible
4. **Set SWML_PROXY_URL_BASE** -- when behind a reverse proxy, to prevent URL mismatches
5. **Rotate credentials** -- change auth credentials periodically
6. **Limit tool access** -- use `set_functions()` on steps to restrict which tools are available at each stage
