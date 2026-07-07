# Call Methods

## Overview

`signalwire::relay::Call` exposes the full `calling.*` RPC surface as
first-class Rust methods. The client is **synchronous**: call methods return
either the transmitted JSON-RPC params (`serde_json::Value`) for simple verbs,
or an `Arc<Action>` for long-running media operations that you `wait()` on.
There is no `async`/`await`.

<!-- snippet-setup -->
```rust
use signalwire::relay::Call;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

let call: Arc<Call> = Arc::new(Call::new(&json!({"call_id": "c-1", "context": "default"})));
```

## Answering and Hanging Up

Simple control verbs return the transmitted params synchronously:

```rust
let _ = call.answer();
let _ = call.hangup();
```

## Playing Audio

Media verbs return an `Arc<Action>`. Poll it with `is_done()` or block on
`wait(timeout)`.

### TTS (Text-to-Speech)

`play_tts(text, opts)` — `opts` is a `Value` carrying optional `voice`,
`language`, `gender`, `volume`.

```rust
let action = call.play_tts("Hello, world!", json!({}));
let _ = action.wait(Some(Duration::from_secs(30)));
```

### Audio File

`play_audio(url, opts)`:

```rust
let action = call.play_audio("https://example.com/audio.mp3", json!({}));
let _ = action.wait(Some(Duration::from_secs(30)));
```

### Mixed Playlist

`play(params)` takes the full `play` array as a `Value`:

```rust
let action = call.play(json!({
    "play": [
        {"type": "tts", "params": {"text": "Please hold."}},
        {"type": "audio", "params": {"url": "https://example.com/hold.mp3"}}
    ]
}));
let _ = action.wait(Some(Duration::from_secs(30)));
```

### Controlling Playback

The base `Action` supports `stop()`; `wait()` blocks for completion:

```rust
let action = call.play_tts("Long message...", json!({}));
if !action.is_done() {
    action.stop(); // stop playback
}
```

## Recording

`record(params)` returns an `Arc<Action>`; `wait()` yields the result `Value`:

```rust
let action = call.record(json!({
    "direction": "both",
    "format": "wav",
    "stereo": true,
    "terminators": "#"
}));

if let Some(result) = action.wait(Some(Duration::from_secs(60))) {
    println!("Recording result: {result}");
}
```

## Collecting Input

`prompt_tts(text, collect, opts)` plays a TTS prompt and collects DTMF/speech.
It returns an `Arc<Action>` whose result carries the collected value.

### DTMF Digits

```rust
let action = call.prompt_tts(
    "Press 1 for sales.",
    json!({"digits": {"max": 1, "terminators": "#"}}),
    json!({}),
);
if let Some(result) = action.wait(Some(Duration::from_secs(30))) {
    println!("Collected: {result}");
}
```

### Speech

```rust
let action = call.prompt_tts(
    "How can I help you?",
    json!({"speech": {"end_silence_timeout": 2.0}}),
    json!({}),
);
let _ = action.wait(Some(Duration::from_secs(30)));
```

## Connecting / Transferring

`connect(params)` returns the transmitted `Value` synchronously:

```rust
let _ = call.connect(json!({
    "devices": [[
        {"type": "phone", "params": {"to_number": "+15551234567", "from_number": "+15559876543"}}
    ]]
}));
```

## Detecting

`detect(params)` returns an `Arc<Action>`.

### Detect Machine vs Human

```rust
let action = call.detect(json!({
    "type": "machine",
    "params": {"initial_timeout": 5.0}
}));
if let Some(result) = action.wait(Some(Duration::from_secs(15))) {
    println!("Detected: {result}");
}
```

### Detect Fax

```rust
let action = call.detect(json!({"type": "fax"}));
let _ = action.wait(Some(Duration::from_secs(15)));
```

## Tapping (Media Streaming)

```rust
let action = call.tap(json!({
    "type": "audio",
    "params": {"direction": "both", "codec": "PCMU", "rate": 8000},
    "target": {"type": "rtp", "params": {"addr": "192.168.1.100", "port": 9000}}
}));
let _ = action.is_done();
```

## Send DTMF

`send_digits(params)` returns the transmitted `Value`:

```rust
let _ = call.send_digits(json!({"digits": "1234#"}));
```

## Conference

`join_conference(params)`:

```rust
let _ = call.join_conference(json!({
    "name": "my-conference",
    "muted": false,
    "deaf": false
}));
```

## Action Object Methods

Every long-running operation returns an `Arc<Action>`:

| Method | Returns | Description |
|--------|---------|-------------|
| `wait(timeout)` | `Option<Value>` | Block until completion (or timeout) |
| `is_done()` | `bool` | Whether it has finished |
| `state()` | `Option<String>` | Current action state |
| `result()` | `Option<Value>` | Final result, if any |
| `stop()` | `()` | Cancel the operation |

## Call State

| Item | Type | Description |
|------|------|-------------|
| `call_id` | `Option<String>` | Unique call identifier (public field) |
| `context` | `Option<String>` | Context the call arrived on (public field) |
| `tag` | `Option<String>` | Correlation tag (public field) |
| `current_state()` | `String` | Current state, as a string |
| `call_state()` | `CallState` | Current state, as a typed enum |
