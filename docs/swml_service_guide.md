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

### Answer and Play

```rust
service.reset_document();
service.add_answer_verb();
service.add_verb("play", json!({
    "url": "say:Hello, you have reached our voicemail. Please leave a message."
}));
service.add_verb("sleep", json!(1000));
service.add_verb("record", json!({
    "stereo": true,
    "format": "wav",
    "direction": "both",
    "terminators": "#"
}));
service.add_hangup_verb();
```

### IVR Menu

```rust
service.reset_document();
service.add_answer_verb();
service.add_verb("prompt", json!({
    "play": "say:Press 1 for sales, 2 for support.",
    "max_digits": 1,
    "terminators": "#"
}));
```

### Call Transfer

```rust
service.add_verb("connect", json!({
    "to": "+15551234567",
    "from": "+15559876543"
}));
```

## Dynamic SWML

Override `on_request` to generate different SWML per request:

```rust
fn on_request(&mut self, request_data: &Map<String, Value>) -> Value {
    let caller = request_data.get("caller_id_number")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    self.reset_document();
    self.add_answer_verb();
    self.add_verb("play", json!({
        "url": format!("say:Welcome, caller {caller}.")
    }));
    self.render()
}
```

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
