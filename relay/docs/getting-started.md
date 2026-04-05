# Getting Started with RELAY

## Prerequisites

- A SignalWire account with a project
- A phone number configured to receive calls
- Rust 1.85+ with tokio runtime

## Environment Setup

```bash
export SIGNALWIRE_PROJECT_ID="your-project-id"
export SIGNALWIRE_API_TOKEN="your-api-token"
export SIGNALWIRE_SPACE="example.signalwire.com"
```

## First Application

Create a simple call handler that answers and plays a greeting:

```rust
use signalwire::relay::RelayClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RelayClient::builder()
        .project(&env::var("SIGNALWIRE_PROJECT_ID")?)
        .token(&env::var("SIGNALWIRE_API_TOKEN")?)
        .space(&env::var("SIGNALWIRE_SPACE")?)
        .contexts(vec!["default".into()])
        .build()?;

    client.on_call(|call| async move {
        println!("Incoming call from: {}", call.from());
        call.answer().await?;
        call.play_tts("Hello! This is my first SignalWire application.").await?.wait().await?;
        call.hangup().await?;
        Ok(())
    });

    println!("Listening for calls on context 'default' ...");
    client.run().await?;
    Ok(())
}
```

## How It Works

1. `RelayClient::builder()` creates a WebSocket connection to SignalWire
2. `.contexts()` subscribes to one or more inbound call contexts
3. `.on_call()` registers a handler for incoming calls
4. `.run()` blocks and processes events until the program exits

## Contexts

Contexts route inbound calls to your application. Configure your phone number's context in the SignalWire dashboard.

```rust
// Listen on multiple contexts
.contexts(vec!["sales".into(), "support".into()])
```

## Making Outbound Calls

```rust
let call = client.calling()
    .dial("+15551234567", "+15559876543")  // (to, from)
    .await?;

call.play_tts("This is an automated message.").await?.wait().await?;
call.hangup().await?;
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
