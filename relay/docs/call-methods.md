# Call Methods

## Overview

The RELAY client provides 57+ methods for controlling live calls. Every method returns an action object that can be awaited, stopped, paused, or resumed.

## Answering and Hanging Up

```rust
call.answer().await?;
call.hangup().await?;
```

## Playing Audio

### TTS (Text-to-Speech)

```rust
let action = call.play_tts("Hello, world!").await?;
action.wait().await?;
```

### Audio File

```rust
let action = call.play_url("https://example.com/audio.mp3").await?;
action.wait().await?;
```

### Mixed Playlist

```rust
let action = call.play(vec![
    json!({"type": "tts", "params": {"text": "Please hold."}}),
    json!({"type": "audio", "params": {"url": "https://example.com/hold.mp3"}}),
]).await?;
action.wait().await?;
```

### Controlling Playback

```rust
let action = call.play_tts("Long message...").await?;
action.pause().await?;   // pause playback
action.resume().await?;  // resume playback
action.stop().await?;    // stop entirely
```

## Recording

```rust
let action = call.record(json!({
    "direction": "both",
    "format": "wav",
    "stereo": true,
    "terminators": "#",
})).await?;

let result = action.wait().await?;
println!("Recording URL: {}", result.url);
```

## Collecting Input

### DTMF Digits

```rust
let result = call.prompt(vec![
    json!({"type": "tts", "params": {"text": "Press 1 for sales."}})
], json!({
    "digits": {"max": 1, "terminators": "#"},
})).await?.wait().await?;

println!("Pressed: {}", result.digits);
```

### Speech

```rust
let result = call.prompt(vec![
    json!({"type": "tts", "params": {"text": "How can I help you?"}})
], json!({
    "speech": {"end_silence_timeout": 2.0},
})).await?.wait().await?;

println!("Said: {}", result.speech);
```

## Connecting / Transferring

```rust
let action = call.connect(json!({
    "devices": [[
        {"type": "phone", "params": {"to_number": "+15551234567", "from_number": "+15559876543"}}
    ]]
})).await?;

action.wait().await?;
```

## Detecting

### Detect Machine vs Human

```rust
let result = call.detect(json!({
    "type": "machine",
    "params": {"initial_timeout": 5.0},
})).await?.wait().await?;

println!("Detected: {}", result.result_type);
```

### Detect Fax

```rust
let result = call.detect(json!({
    "type": "fax",
})).await?.wait().await?;
```

## Tapping (Media Streaming)

```rust
let action = call.tap(json!({
    "type": "audio",
    "params": {
        "direction": "both",
        "codec": "PCMU",
        "rate": 8000,
    },
    "target": {
        "type": "rtp",
        "params": {"addr": "192.168.1.100", "port": 9000}
    }
})).await?;
```

## Send DTMF

```rust
call.send_digits("1234#").await?;
```

## Conference

```rust
call.join_conference("my-conference", json!({
    "muted": false,
    "deaf": false,
})).await?;
```

## Action Object Methods

Every long-running operation returns an action object:

| Method | Description |
|--------|-------------|
| `wait()` | Block until the operation completes |
| `stop()` | Cancel the operation |
| `pause()` | Pause (play, record) |
| `resume()` | Resume from pause |
| `volume(db)` | Adjust volume (play) |

## Call Properties

| Property | Type | Description |
|----------|------|-------------|
| `call_id` | `String` | Unique call identifier |
| `from()` | `&str` | Caller number |
| `to()` | `&str` | Called number |
| `state()` | `CallState` | Current call state |
| `direction()` | `&str` | `inbound` or `outbound` |
