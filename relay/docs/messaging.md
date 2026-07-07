# Messaging

## Overview

The RELAY client sends and receives SMS/MMS via synchronous methods directly on
`Client` — there is no `messaging()` sub-namespace and no `async`/`await`.

<!-- snippet-setup -->
```rust
use signalwire::relay::{Client, Event, Message};
use signalwire::relay::state_enums::MessageState;
use serde_json::{json, Value};
use std::sync::Arc;

let client: Arc<Client> = Arc::new(Client::new("p", "t", "example.signalwire.com"));
```

## Sending SMS

`send_message(to_number, from_number, body, media, tags, context)` — `body`,
`media`, `tags`, and `context` are all `Option`. It blocks on the
`messaging.send` RPC and returns `Arc<Message>`:

```rust
let message = client.send_message(
    "+15551234567",  // to
    "+15559876543",  // from
    Some("Hello from SignalWire!"),
    None,            // media
    None,            // tags
    None,            // context (defaults)
).unwrap();

println!("Message ID: {:?}", message.message_id());
println!("State: {:?}", message.state());
```

## Sending MMS

Pass media URLs via the `media` argument (`Option<&[String]>`); `body` may be
`None` for a media-only message:

```rust
let media = vec!["https://example.com/photo.jpg".to_string()];
let message = client.send_message(
    "+15551234567",
    "+15559876543",
    Some("Check out this image!"),
    Some(&media),
    None,
    None,
).unwrap();
```

## Receiving Messages

`on_message` registers a synchronous handler receiving `(&Event, &Value)` — the
raw event and its params object:

```rust
client.on_message(|event, params| {
    println!("inbound message event {}", event.event_type());
    if let Some(body) = params.get("body").and_then(Value::as_str) {
        println!("Body: {body}");
    }
    if let Some(from) = params.get("from_number").and_then(Value::as_str) {
        println!("From: {from}");
    }
});
```

## Delivery State

A tracked `Arc<Message>` (returned by `send_message`, or looked up via
`client.get_message(id)`) exposes its current delivery state. `MessageState`
carries an `Other(String)` catch-all, so a `match` needs a wildcard arm:

```rust
if let Some(message) = client.get_message("msg-id") {
    match message.message_state() {
        Some(MessageState::Queued) => println!("queued"),
        Some(MessageState::Sent) => println!("sent to carrier"),
        Some(MessageState::Delivered) => println!("delivered"),
        Some(MessageState::Undelivered) => println!("delivery failed"),
        Some(MessageState::Failed) => println!("permanent failure"),
        _ => {}
    }
}
```

## Message Properties

`Message` accessors (all methods, not fields):

| Method | Returns | Description |
|--------|---------|-------------|
| `message_id()` | `Option<&str>` | Message identifier |
| `from_number()` | `Option<&str>` | Sender number |
| `to_number()` | `Option<&str>` | Recipient number |
| `body()` | `Option<String>` | Message text |
| `media()` | `Vec<String>` | MMS media URLs |
| `tags()` | `Vec<String>` | Message tags |
| `direction()` | `Option<&str>` | `inbound` or `outbound` |
| `state()` | `Option<String>` | Current delivery state (string) |
| `message_state()` | `Option<MessageState>` | Current delivery state (typed) |

## Contexts

Messages are received on subscribed contexts, just like calls:

```rust
client.receive(&["messaging".to_string()]);
```

## SMS During a Call

From within a SWAIG tool handler, use `FunctionResult::send_sms()`. This is an
agent-side action, not a RELAY operation:

```rust
use signalwire::swaig::FunctionResult;

let mut result = FunctionResult::with_response("Confirmation sent.");
let _ = result.send_sms(
    "+15559876543",
    "+15551234567",
    "Your appointment is confirmed.",
    vec![],  // media
    vec![],  // tags
    "",      // region
);
```
