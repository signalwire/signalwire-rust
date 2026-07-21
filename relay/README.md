# RELAY Client

Real-time call control and messaging over WebSocket. The RELAY client connects to SignalWire via the Blade protocol and gives you imperative control over live phone calls and SMS/MMS. The client is **synchronous**: it runs its event loop on a background reader thread and invokes your handler closures on that thread — there is no `async`/`await`.

## Quick Start

```rust
use signalwire::relay::Client;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
    let client = Arc::new(Client::from_env()?);

    client.on_call(|call, _event| {
        let _ = call.answer();
        let action = call.play_tts("Welcome to SignalWire!", serde_json::json!({})).unwrap();
        let _ = action.is_done();
        let _ = call.hangup();
    });

    println!("Waiting for inbound calls ...");
    client.connect()?;
    client.receive(&["default".to_string()]);
    client.run(); // blocks until the connection is torn down
    Ok(())
}
```

## Features

- **57+ calling methods** -- play, record, collect, detect, tap, stream, conference, AI, and more
- **SMS/MMS messaging** -- send and receive with delivery tracking
- **Action objects** -- `wait()`, `is_done()`, `stop()` on long-running operations
- **Caller-driven reconnect** -- `reconnect()` applies exponential backoff and re-sends the subscribed contexts on the new handshake. The client does NOT auto-reconnect: on connection loss it stops (`is_running()` returns `false`, pending requests fault), so a caller can detect the drop and decide whether to `reconnect()`.
- **Synchronous API** -- no async runtime to set up; an `on_call` handler runs on its own dispatcher thread, so it can send verbs and `Action::wait()` for their completion without blocking the client's reader/writer

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
- [Client Reference](docs/client-reference.md) -- Client API

## Examples

| Example | Description |
|---------|-------------|
| [relay_answer_and_welcome.rs](examples/relay_answer_and_welcome.rs) | Answer and play TTS |
| [relay_dial_and_play.rs](examples/relay_dial_and_play.rs) | Outbound call with audio |
| [relay_ivr_connect.rs](examples/relay_ivr_connect.rs) | IVR menu with DTMF and transfer |
