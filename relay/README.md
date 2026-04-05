# RELAY Client

Real-time call control and messaging over WebSocket. The RELAY client connects to SignalWire via the Blade protocol and gives you async, imperative control over live phone calls and SMS/MMS.

## Quick Start

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
        call.answer().await?;
        call.play_tts("Welcome to SignalWire!").await?.wait().await?;
        call.hangup().await?;
        Ok(())
    });

    println!("Waiting for inbound calls ...");
    client.run().await?;
    Ok(())
}
```

## Features

- **57+ calling methods** -- play, record, collect, detect, tap, stream, conference, AI, and more
- **SMS/MMS messaging** -- send and receive with delivery tracking
- **Action objects** -- `wait()`, `stop()`, `pause()`, `resume()` on long-running operations
- **Auto-reconnect** -- exponential backoff with configurable retries
- **Async/await** -- built on tokio for efficient concurrent handling

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SIGNALWIRE_PROJECT_ID` | Your SignalWire project ID |
| `SIGNALWIRE_API_TOKEN` | Your SignalWire API token |
| `SIGNALWIRE_SPACE` | Your space hostname (e.g. `example.signalwire.com`) |
| `SIGNALWIRE_LOG_LEVEL` | Log level (`debug` for WebSocket/JSON-RPC output) |

## Documentation

- [Getting Started](docs/getting-started.md) -- setup, first call, environment
- [Call Methods](docs/call-methods.md) -- complete call control reference
- [Events](docs/events.md) -- event handling and callbacks
- [Messaging](docs/messaging.md) -- SMS/MMS send and receive
- [Client Reference](docs/client-reference.md) -- RelayClient API

## Examples

| Example | Description |
|---------|-------------|
| [relay_answer_and_welcome.rs](examples/relay_answer_and_welcome.rs) | Answer and play TTS |
| [relay_dial_and_play.rs](examples/relay_dial_and_play.rs) | Outbound call with audio |
| [relay_ivr_connect.rs](examples/relay_ivr_connect.rs) | IVR menu with DTMF and transfer |
