# SWML Service Guide

## Overview

SWML (SignalWire Markup Language) is a JSON document that tells the SignalWire platform what to do with a call. The `Service` struct provides low-level SWML document construction.

Most users will use `AgentBase`, which builds SWML internally. Use `Service` directly only when you need non-AI call flows (IVR, voicemail, call routing).

## Service Construction

```rust
use signalwire::swml::service::{Service, ServiceOptions};

let service = Service::new(ServiceOptions {
    name: "voicemail".to_string(),
    route: Some("/voicemail".to_string()),
    host: Some("0.0.0.0".to_string()),
    port: Some(3000),
    basic_auth_user: None,
    basic_auth_password: None,
});
```

## Building SWML Documents

Add verbs with `service.add_verb(verb, section, config)`. The second argument is the
section name (use `"main"` for the primary flow). Verbs with no config take `json!({})`:

### Answer and Play

```rust
service.add_verb("answer", "main", json!({}));
service.add_verb("play", "main", json!({
    "url": "say:Hello, you have reached our voicemail. Please leave a message."
}));
service.add_verb("sleep", "main", json!(1000));
service.add_verb("record", "main", json!({
    "stereo": true,
    "format": "wav",
    "direction": "speak",
    "terminators": "#"
}));
service.add_verb("hangup", "main", json!({}));
```

### IVR Menu

```rust
service.add_verb("answer", "main", json!({}));
service.add_verb("prompt", "main", json!({
    "play": "say:Press 1 for sales, 2 for support.",
    "max_digits": 1,
    "terminators": "#"
}));
```

### Call Transfer

```rust
service.add_verb("connect", "main", json!({
    "to": "+15551234567",
    "from": "+15559876543"
}));
```

You can also build a document literal and serve it directly — the SWML JSON is just a
`version` plus a `sections` map of verb objects:

```rust
let doc = json!({
    "version": "1.0.0",
    "sections": {
        "main": [
            {"answer": {}},
            {"play": {"url": "say:Connecting you now."}},
            {"connect": {"to": "+15551234567", "from": "+15559876543"}},
            {"hangup": {}}
        ]
    }
});
```

## Dynamic SWML

Register a hook with `set_on_swml_request_hook` to customise SWML per request. The hook
receives the parsed request body and an optional callback path, and returns
`Option<Value>` modifications to merge (or `None` for default rendering):

```rust
service.set_on_swml_request_hook(|request_data, _callback_path| {
    let caller = request_data
        .and_then(|d| d.get("caller_id_number"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Some(json!({
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": format!("say:Welcome, caller {caller}.")}}
            ]
        }
    }))
});
```

To inspect the rendered document, `service.render()` returns the SWML JSON as a `String`
(`render_pretty()` for indented output).

## SWML Document Structure

A minimal SWML document:

```json
{
  "version": "1.0.0",
  "sections": {
    "main": [
      {"answer": {}},
      {"play": {"url": "say:Hello"}},
      {"hangup": {}}
    ]
  }
}
```

## Common Verbs

| Verb | Description |
|------|-------------|
| `answer` | Answer the inbound call |
| `hangup` | End the call |
| `play` | Play audio or TTS |
| `record` | Record audio |
| `prompt` | Play audio and collect DTMF |
| `connect` | Connect/transfer the call |
| `sleep` | Pause execution (milliseconds) |
| `ai` | Start the AI pipeline |
| `set` | Set variables |
| `switch` | Conditional branching |

## Integration with AgentBase

`AgentBase` uses `Service` internally. You rarely need to access it directly:

```rust
// Access the underlying service
let service = agent.service();
let route = service.route();
let port = service.port();
let (user, pass) = service.basic_auth_credentials();
```
