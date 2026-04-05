# Messaging

## Overview

The RELAY client supports sending and receiving SMS/MMS messages with delivery tracking.

## Sending SMS

```rust
let message = client.messaging()
    .send("+15559876543", "+15551234567", "Hello from SignalWire!")
    .await?;

println!("Message ID: {}", message.id);
println!("Status: {:?}", message.state);
```

## Sending MMS

```rust
let message = client.messaging()
    .send_mms(
        "+15559876543",
        "+15551234567",
        "Check out this image!",
        vec!["https://example.com/photo.jpg"],
    )
    .await?;
```

## Receiving Messages

```rust
client.on_message(|message| async move {
    println!("From: {}", message.from());
    println!("To: {}", message.to());
    println!("Body: {}", message.body());

    if !message.media().is_empty() {
        println!("Media URLs:");
        for url in message.media() {
            println!("  {}", url);
        }
    }

    Ok(())
});
```

## Delivery Tracking

Track message delivery status changes:

```rust
client.on_message_state(|id, state| async move {
    match state {
        MessageState::Queued => println!("{id}: queued"),
        MessageState::Sent => println!("{id}: sent to carrier"),
        MessageState::Delivered => println!("{id}: delivered"),
        MessageState::Undelivered => println!("{id}: delivery failed"),
        MessageState::Failed => println!("{id}: permanent failure"),
        _ => {}
    }
    Ok(())
});
```

## Message Properties

| Property | Type | Description |
|----------|------|-------------|
| `id` | `String` | Message identifier |
| `from()` | `&str` | Sender number |
| `to()` | `&str` | Recipient number |
| `body()` | `&str` | Message text |
| `media()` | `Vec<String>` | MMS media URLs |
| `direction()` | `&str` | `inbound` or `outbound` |
| `state` | `MessageState` | Current delivery state |
| `segments` | `u32` | Number of SMS segments |

## Contexts

Messages are received on subscribed contexts, just like calls:

```rust
let client = RelayClient::builder()
    .project(&env::var("SIGNALWIRE_PROJECT_ID")?)
    .token(&env::var("SIGNALWIRE_API_TOKEN")?)
    .space(&env::var("SIGNALWIRE_SPACE")?)
    .contexts(vec!["messaging".into()])
    .build()?;
```

## SMS During a Call

From within a SWAIG tool handler, use `FunctionResult::send_sms()`:

```rust
FunctionResult::with_response("Confirmation sent.")
    .send_sms("+15559876543", "+15551234567", "Your appointment is confirmed.")
```

This is an agent-side action, not a RELAY operation.
