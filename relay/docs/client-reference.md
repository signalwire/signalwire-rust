# Client Reference

## RelayClient

The main entry point for RELAY real-time communication.

### Builder

```rust
let client = RelayClient::builder()
    .project("project-id")
    .token("api-token")
    .space("example.signalwire.com")
    .contexts(vec!["default".into()])
    .build()?;
```

### Builder Methods

| Method | Type | Description |
|--------|------|-------------|
| `project` | `&str` | SignalWire project ID |
| `token` | `&str` | API token |
| `space` | `&str` | Space hostname |
| `contexts` | `Vec<String>` | Inbound call/message contexts |

### Environment Variables

If not provided explicitly, the builder reads:

- `SIGNALWIRE_PROJECT_ID`
- `SIGNALWIRE_API_TOKEN`
- `SIGNALWIRE_SPACE`

### Event Registration

| Method | Signature | Description |
|--------|-----------|-------------|
| `on_call` | `(Fn(Call) -> Future)` | Inbound call handler |
| `on_message` | `(Fn(Message) -> Future)` | Inbound message handler |
| `on_connect` | `(Fn() -> Future)` | Connection established |
| `on_disconnect` | `(Fn() -> Future)` | Connection lost |
| `on_reconnect` | `(Fn(u32) -> Future)` | Reconnection attempt |

### Execution

| Method | Signature | Description |
|--------|-----------|-------------|
| `run` | `async (&self) -> Result<()>` | Block and process events |
| `disconnect` | `async (&self) -> Result<()>` | Gracefully disconnect |

### Subsystems

| Method | Returns | Description |
|--------|---------|-------------|
| `calling()` | `CallingClient` | Call control methods |
| `messaging()` | `MessagingClient` | Messaging methods |

---

## CallingClient

### Outbound Calls

```rust
let call = client.calling().dial(to, from).await?;
```

### Call Methods

See [call-methods.md](call-methods.md) for the complete list of 57+ methods.

---

## MessagingClient

### Send SMS

```rust
client.messaging().send(from, to, body).await?;
```

### Send MMS

```rust
client.messaging().send_mms(from, to, body, media_urls).await?;
```

---

## Call

Represents a live phone call.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `call_id` | `String` | Unique identifier |
| `from()` | `&str` | Caller number |
| `to()` | `&str` | Called number |
| `state()` | `CallState` | Current state |
| `direction()` | `&str` | `inbound` or `outbound` |

### Key Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `answer()` | `Result<()>` | Answer the call |
| `hangup()` | `Result<()>` | End the call |
| `play(items)` | `Result<PlayAction>` | Play audio/TTS |
| `play_tts(text)` | `Result<PlayAction>` | Play TTS |
| `play_url(url)` | `Result<PlayAction>` | Play audio URL |
| `record(params)` | `Result<RecordAction>` | Record audio |
| `prompt(play, collect)` | `Result<PromptAction>` | Collect input |
| `connect(params)` | `Result<ConnectAction>` | Connect/transfer |
| `detect(params)` | `Result<DetectAction>` | Detect machine/fax |
| `tap(params)` | `Result<TapAction>` | Media streaming |
| `send_digits(digits)` | `Result<()>` | Send DTMF |

---

## Action Objects

Long-running operations return typed action objects.

### Common Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `wait()` | `Result<ActionResult>` | Wait for completion |
| `stop()` | `Result<()>` | Cancel the operation |

### PlayAction Additional Methods

| Method | Description |
|--------|-------------|
| `pause()` | Pause playback |
| `resume()` | Resume playback |
| `volume(db)` | Adjust volume |

---

## Connection Behaviour

- Auto-reconnect with exponential backoff (1s, 2s, 4s, 8s, ... up to 60s)
- Subscriptions are restored automatically on reconnect
- In-progress calls are not affected by brief disconnections
