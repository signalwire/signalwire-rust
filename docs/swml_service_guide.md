# SWML Service Guide

## Overview

SWML (SignalWire Markup Language) is a JSON document that tells the SignalWire platform what to do with a call. The `Service` struct provides low-level SWML document construction.

Most users will use `AgentBase`, which builds SWML internally. Use `Service` directly only when you need non-AI call flows (IVR, voicemail, call routing).

## Service Construction

```rust
use signalwire::swml::service::{Service, ServiceOptions};

let service = Service::new(
    ServiceOptions::new("voicemail")
        .route("/voicemail")
        .host("0.0.0.0")
        .port(3000),
);
```

## Building SWML Documents

Add a verb to the primary flow with `service.add_verb(verb, config)`, or target a
named section with `service.add_verb_to_section(section, verb, config)`. Verbs with
no config take `json!({})`:

<!-- snippet-setup -->
```rust
use serde_json::json;

let mut service = signalwire::swml::service::Service::new(
    signalwire::swml::service::ServiceOptions::new("guide-service"),
);
let agent = signalwire::agent::AgentBase::new(
    signalwire::agent::AgentOptions::new("guide-agent"),
);
```

### Answer and Play

```rust
service.add_verb_to_section("main", "answer", json!({}));
service.add_verb_to_section("main", "play", json!({
    "url": "say:Hello, you have reached our voicemail. Please leave a message."
}));
service.add_verb_to_section("main", "sleep", json!(1000));
service.add_verb_to_section("main", "record", json!({
    "stereo": true,
    "format": "wav",
    "direction": "speak",
    "terminators": "#"
}));
service.add_verb_to_section("main", "hangup", json!({}));
```

### IVR Menu

```rust
service.add_verb_to_section("main", "answer", json!({}));
service.add_verb_to_section("main", "prompt", json!({
    "play": "say:Press 1 for sales, 2 for support.",
    "max_digits": 1,
    "terminators": "#"
}));
```

### Call Transfer

```rust
service.add_verb_to_section("main", "connect", json!({
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
