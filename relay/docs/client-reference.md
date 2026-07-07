# Client Reference

<!-- snippet-setup -->
```rust
use signalwire::relay::{Call, Client, Event, Message};
use signalwire::relay::state_enums::{CallState, MessageState};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

let client: Arc<Client> = Arc::new(Client::new("project-id", "api-token", "example.signalwire.com"));
let call: Arc<Call> = Arc::new(Call::new(&json!({"call_id": "c-1", "context": "default"})));
```

## Client

`signalwire::relay::Client` is the entry point for RELAY real-time
communication. It is a **synchronous** client: it runs its WebSocket/Blade
(JSON-RPC 2.0) event loop on a background reader thread and invokes your handler
closures on that thread. There is no `async`/`await` and no external runtime to
stand up.

### Construction

```rust
// Explicit credentials.
let client = Arc::new(Client::new(
    "project-id",
    "api-token",
    "example.signalwire.com",
));
```

`Client::from_env()` reads the three credentials from the environment and
returns `Result<Client, RelayError>`:

```rust
// Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
let client = Arc::new(Client::from_env().unwrap());
```

Wrap the client in `Arc` — the connection lifecycle methods
(`connect`/`reconnect`) take `self: &Arc<Self>` so the reader thread can hold a
strong reference.

### Environment Variables

`Client::from_env()` reads:

- `SIGNALWIRE_PROJECT_ID`
- `SIGNALWIRE_API_TOKEN`
- `SIGNALWIRE_SPACE`

### Connection Lifecycle

| Method | Signature | Description |
|--------|-----------|-------------|
| `connect` | `(self: &Arc<Self>) -> Result<(), RelayError>` | Open the WebSocket and run the `signalwire.connect` handshake |
| `disconnect` | `(&self)` | Close the connection and stop the reader thread |
| `reconnect` | `(self: &Arc<Self>) -> Result<(), RelayError>` | Reconnect with the current backoff delay |
| `run` | `(&self)` | Block the calling thread until the reader thread stops |
| `is_connected` | `(&self) -> bool` | Whether the socket is up |
| `is_running` | `(&self) -> bool` | Whether the reader loop is active |

```rust
client.connect().unwrap();
client.receive(&["default".to_string()]);
client.run(); // blocks until disconnect / connection loss
```

### Contexts

| Method | Signature | Description |
|--------|-----------|-------------|
| `receive` | `(&self, contexts: &[String])` | Subscribe to inbound call/message contexts |
| `unreceive` | `(&self, contexts: &[String])` | Unsubscribe from contexts |

```rust
client.receive(&["sales".to_string(), "support".to_string()]);
```

### Event Registration

Handlers are **synchronous** closures. They run on the relay reader thread; do
not `.await` inside them (there is no async context).

| Method | Closure signature | Description |
|--------|-------------------|-------------|
| `on_call` | `Fn(Arc<Call>, &Event) + Send + Sync + 'static` | Inbound call handler |
| `on_message` | `Fn(&Event, &Value) + Send + Sync + 'static` | Inbound message handler |
| `on_event` | `Fn(&Event, &Value) + Send + Sync + 'static` | Generic catch-all event handler |

```rust
client.on_call(|call, _event| {
    let _ = call.answer();
    let _ = call.hangup();
});
```

### Outbound Operations

The client exposes outbound calling and messaging directly (there are no
`calling()` / `messaging()` sub-namespaces — the operations are methods on
`Client`).

| Method | Signature | Description |
|--------|-----------|-------------|
| `dial` | `(self: &Arc<Self>, devices: Value, tag: Option<&str>, max_duration: Option<u32>, dial_timeout: Duration) -> Result<Arc<Call>, RelayError>` | Place an outbound call, block until answered |
| `send_message` | `(&self, to_number: &str, from_number: &str, body: Option<&str>, media: Option<&[String]>, tags: Option<&[String]>, context: Option<&str>) -> Result<Arc<Message>, RelayError>` | Send an SMS/MMS |
| `execute` | `(&self, method: &str, params: Value) -> Result<Value, RelayError>` | Send an arbitrary JSON-RPC request and block for its result |

```rust
let call = client.dial(
    json!([[{"type": "phone", "params": {"to_number": "+15551234567", "from_number": "+15559876543"}}]]),
    None,
    None,
    Duration::from_secs(30),
).unwrap();
```

---

## Call

`signalwire::relay::Call` represents a live phone call. It is handed to your
`on_call` handler as `Arc<Call>` and returned by `Client::dial`. Every calling
method is synchronous.

### Fields and State

| Item | Type | Description |
|------|------|-------------|
| `call_id` | `Option<String>` | Unique identifier (public field) |
| `context` | `Option<String>` | Context the call arrived on (public field) |
| `tag` | `Option<String>` | Correlation tag (public field) |
| `current_state()` | `-> String` | Current state as a string |
| `call_state()` | `-> CallState` | Current state as a typed enum |
| `repr()` | `-> String` | `Call(call_id=..., state=...)` debug string |

```rust
println!("{}", call.repr());
if call.call_state() == CallState::Answered {
    // ...
}
```

### Key Methods

Simple control verbs return the raw JSON-RPC params (`serde_json::Value`) that
were transmitted:

| Method | Returns | Description |
|--------|---------|-------------|
| `answer()` | `Value` | Answer the call |
| `hangup()` | `Value` | End the call |
| `connect(params)` | `Value` | Connect / transfer |
| `send_digits(params)` | `Value` | Send DTMF |

Long-running media verbs return an `Arc<Action>` you poll or wait on:

| Method | Returns | Description |
|--------|---------|-------------|
| `play(params)` | `Arc<Action>` | Play a mixed audio/TTS playlist |
| `play_tts(text, opts)` | `Arc<Action>` | Play text-to-speech |
| `play_audio(url, opts)` | `Arc<Action>` | Play an audio URL |
| `record(params)` | `Arc<Action>` | Record audio |
| `prompt_tts(text, collect, opts)` | `Arc<Action>` | Play a prompt and collect input |
| `detect(params)` | `Arc<Action>` | Detect machine / fax |
| `tap(params)` | `Arc<Action>` | Media streaming |

```rust
let action = call.play_tts("Hello, world!", json!({}));
let _ = action.wait(Some(Duration::from_secs(30)));
```

---

## Action Objects

Long-running operations return `Arc<Action>`.

### Common Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `wait(timeout)` | `Option<Value>` | Block until completion (or timeout) and return the result |
| `is_done()` | `bool` | Whether the operation has finished |
| `state()` | `Option<String>` | Current action state |
| `result()` | `Option<Value>` | The final result, if any |
| `stop()` | `()` | Cancel the operation |

```rust
let action = call.play_tts("Long message...", json!({}));
if !action.is_done() {
    let _ = action.wait(Some(Duration::from_secs(10)));
}
```

---

## Connection Behaviour

- Auto-reconnect with exponential backoff (starts at 1s, doubles per attempt).
- Contexts subscribed via `receive` are re-sent on the reconnect handshake.
- The reader thread owns all socket I/O; `run()` simply blocks until it stops.
