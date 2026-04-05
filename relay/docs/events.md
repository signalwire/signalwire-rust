# Events

## Overview

The RELAY client emits events for call state changes, message delivery, and connection lifecycle. Register handlers with closures to react to events asynchronously.

## Call Events

### on_call

Triggered when an inbound call arrives:

```rust
client.on_call(|call| async move {
    println!("Incoming: {} -> {}", call.from(), call.to());
    call.answer().await?;
    // handle the call ...
    Ok(())
});
```

### Call State Changes

```rust
call.on_state_change(|state| async move {
    match state {
        CallState::Ringing => println!("Ringing"),
        CallState::Answered => println!("Answered"),
        CallState::Ending => println!("Ending"),
        CallState::Ended => println!("Ended"),
        _ => {}
    }
    Ok(())
});
```

### Call States

| State | Description |
|-------|-------------|
| `Created` | Call object created, not yet signalled |
| `Ringing` | Call is ringing |
| `Answered` | Call has been answered |
| `Ending` | Hangup initiated, not yet complete |
| `Ended` | Call has ended |

## Messaging Events

### on_message

Triggered when an inbound SMS/MMS arrives:

```rust
client.on_message(|message| async move {
    println!("From: {}, Body: {}", message.from(), message.body());
    Ok(())
});
```

### Message States

| State | Description |
|-------|-------------|
| `Queued` | Message accepted, pending delivery |
| `Initiated` | Delivery in progress |
| `Sent` | Delivered to carrier |
| `Delivered` | Confirmed delivered |
| `Undelivered` | Delivery failed |
| `Failed` | Permanent failure |

## Connection Events

### on_connect

```rust
client.on_connect(|| async {
    println!("Connected to SignalWire");
    Ok(())
});
```

### on_disconnect

```rust
client.on_disconnect(|| async {
    println!("Disconnected from SignalWire");
    Ok(())
});
```

### on_reconnect

```rust
client.on_reconnect(|attempt| async move {
    println!("Reconnecting (attempt {attempt})");
    Ok(())
});
```

## Event Flow

```
Inbound call
    │
    ├─→ on_call handler fires
    │     │
    │     ├─→ call.answer()
    │     │     └─→ state: Answered
    │     │
    │     ├─→ call.play_tts(...)
    │     │     └─→ action.wait()
    │     │
    │     └─→ call.hangup()
    │           └─→ state: Ending → Ended
    │
    └─→ Ready for next call
```

## Error Handling

Event handlers return `Result<(), Box<dyn std::error::Error>>`. Errors are logged but do not crash the client:

```rust
client.on_call(|call| async move {
    call.answer().await?;  // ? propagates errors
    // If this fails, error is logged and client continues
    call.play_tts("Hello").await?.wait().await?;
    call.hangup().await?;
    Ok(())
});
```

## Multiple Handlers

You can register multiple handlers for the same event. They fire in registration order:

```rust
client.on_call(|call| async move {
    println!("Handler 1: logging call");
    Ok(())
});

client.on_call(|call| async move {
    call.answer().await?;
    call.play_tts("Hello").await?.wait().await?;
    call.hangup().await?;
    Ok(())
});
```
