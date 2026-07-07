# Getting Started with RELAY

## Prerequisites

- A SignalWire account with a project
- A phone number configured to receive calls
- Rust 1.88+ (edition 2024)

## Environment Setup

```bash
export SIGNALWIRE_PROJECT_ID="your-project-id"
export SIGNALWIRE_API_TOKEN="your-api-token"
export SIGNALWIRE_SPACE="example.signalwire.com"
```

## First Application

The RELAY client is synchronous: it runs its WebSocket/Blade event loop on a
background thread and invokes your handler closures on that thread. There is no
`async`/`await` and no `tokio` runtime to set up. Create a simple call handler
that answers and plays a greeting:

```rust
use signalwire::relay::Client;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
    let client = Arc::new(Client::from_env()?);

    client.on_call(|call, _event| {
        println!("Incoming call: {}", call.repr());
        let _ = call.answer();

        let action = call.play(serde_json::json!({
            "play": [{
                "type": "tts",
                "params": {"text": "Hello! This is my first SignalWire application."}
            }]
        }));
        let _ = action.is_done();

        let _ = call.hangup();
    });

    println!("Listening for calls on context 'default' ...");
    client.connect()?;
    client.receive(&["default".to_string()]);

    // Block while the relay loop runs in the background.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
```

## How It Works

1. `Client::from_env()` builds a client and reads credentials from the
   environment (or use `Client::new(project, token, space)`).
2. `on_call()` registers a handler closure for incoming calls. The closure
   receives `(Arc<Call>, &Event)` and runs on the relay background thread.
3. `connect()` opens the WebSocket connection and authenticates.
4. `receive(&[..])` subscribes to one or more inbound call/message contexts.

## Contexts

Contexts route inbound calls to your application. Configure your phone number's
context in the SignalWire dashboard.

<!-- snippet-setup -->
```rust
use signalwire::relay::Client;
use std::sync::Arc;

let client = Arc::new(Client::new("p", "t", "example.signalwire.com"));
```

```rust
// Listen on multiple contexts
client.receive(&["sales".to_string(), "support".to_string()]);
```

## Making Outbound Calls

`dial_blocking` initiates an outbound call and blocks until the call is answered
(or the dial times out), returning the resolved `Call`. `devices` is the
standard serial/parallel device matrix.

```rust
use std::time::Duration;

let call = client.dial_blocking(
    serde_json::json!([[
        {"type": "phone", "params": {"to_number": "+15551234567", "from_number": "+15559876543"}}
    ]]),
    None,                       // tag (auto-generated when None)
    None,                       // max_duration (seconds)
    Duration::from_secs(30),    // dial timeout
).unwrap();

let action = call.play(serde_json::json!({
    "play": [{"type": "tts", "params": {"text": "This is an automated message."}}]
}));
let _ = action.is_done();
let _ = call.hangup();
```

## Debug Logging

For full WebSocket and JSON-RPC output:

```bash
SIGNALWIRE_LOG_LEVEL=debug cargo run --example relay_answer_and_welcome
```

## Next Steps

- [Call Methods](call-methods.md) -- complete reference for call control
- [Events](events.md) -- handling call state changes
- [Messaging](messaging.md) -- SMS/MMS support
