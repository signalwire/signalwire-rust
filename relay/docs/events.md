# Events

## Overview

The RELAY client dispatches events for inbound calls, inbound messages, and
per-call state changes. Handlers are **synchronous** closures that run on the
relay reader thread — there is no `async`/`await`.

<!-- snippet-setup -->
```rust
use signalwire::relay::{Call, Client, Event};
use signalwire::relay::state_enums::CallState;
use serde_json::{json, Value};
use std::sync::Arc;

let client: Arc<Client> = Arc::new(Client::new("p", "t", "example.signalwire.com"));
let call: Arc<Call> = Arc::new(Call::new(&json!({"call_id": "c-1", "context": "default"})));
```

## Call Events

### on_call

Registered on the client; fires when an inbound call arrives. The closure
receives `(Arc<Call>, &Event)`:

```rust
client.on_call(|call, event| {
    println!("Incoming call {} (event {})", call.repr(), event.event_type());
    let _ = call.answer();
    // handle the call ...
    let _ = call.hangup();
});
```

### Per-Call Event Listeners

`Call::on` registers a listener for every event dispatched to that call. The
closure receives `(&Event, &Call)`:

```rust
call.on(|event, call| {
    match call.call_state() {
        CallState::Ringing => println!("Ringing"),
        CallState::Answered => println!("Answered"),
        CallState::Ending => println!("Ending"),
        CallState::Ended => println!("Ended"),
        _ => println!("event: {}", event.event_type()),
    }
});
```

### Blocking State Waits

The synchronous `Call` offers blocking waiters instead of async state
callbacks. Each returns `Option<Event>` (`None` on timeout):

```rust
use std::time::Duration;

let _ = call.wait_for_answered(Some(Duration::from_secs(30)));
let _ = call.wait_for_ended(Some(Duration::from_secs(60)));
```

### Call States

`CallState` (in `signalwire::relay::state_enums`) has an `Other(String)`
catch-all, so a `match` needs a wildcard arm.

| State | Description |
|-------|-------------|
| `Created` | Call object created, not yet signalled |
| `Ringing` | Call is ringing |
| `Answered` | Call has been answered |
| `Ending` | Hangup initiated, not yet complete |
| `Ended` | Call has ended |
| `Other(String)` | Any unrecognised server value |

## Messaging Events

### on_message

Registered on the client; fires when an inbound SMS/MMS arrives. The closure
receives `(&Event, &Value)` — the raw event and its params:

```rust
client.on_message(|event, params| {
    println!("message event {}: {}", event.event_type(), params);
});
```

### Message States

`MessageState` (in `signalwire::relay::state_enums`) also carries an
`Other(String)` catch-all.

| State | Description |
|-------|-------------|
| `Queued` | Message accepted, pending delivery |
| `Initiated` | Delivery in progress |
| `Sent` | Delivered to carrier |
| `Delivered` | Confirmed delivered |
| `Undelivered` | Delivery failed |
| `Failed` | Permanent failure |
| `Other(String)` | Any unrecognised server value |

## Generic Events

`Client::on_event` registers a catch-all handler for every dispatched event.
The closure receives `(&Event, &Value)`:

```rust
client.on_event(|event, params| {
    println!("event {}: {}", event.event_type(), params);
});
```

## Event Flow

```
Inbound call
    │
    ├─→ on_call handler fires
    │     │
    │     ├─→ call.answer()      → state: Answered
    │     ├─→ call.play_tts(...) → action.wait(...)
    │     └─→ call.hangup()      → state: Ending → Ended
    │
    └─→ Ready for next call
```

## Error Handling

Handlers are plain closures with no return value. Fallible SDK calls inside a
handler return `Value` (for control verbs) or `Result` / `Option` (for
`Client`-level operations) — handle them inline; a handler cannot propagate an
error to the client with `?`:

```rust
client.on_call(|call, _event| {
    let _ = call.answer();
    let action = call.play_tts("Hello", serde_json::json!({})).unwrap();
    let _ = action.is_done();
    let _ = call.hangup();
});
```

## Handler Registration

`on_call`, `on_message`, and `on_event` each store a single handler — a later
registration replaces the previous one. Compose multiple concerns inside one
closure rather than registering twice:

```rust
client.on_call(|call, _event| {
    println!("logging call {}", call.repr());
    let _ = call.answer();
    let _ = call.hangup();
});
```
