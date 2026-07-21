# Calling

## Overview

The REST calling namespace is an HTTP **command-dispatch** surface: each
operation is a method on `client.calling()` that takes a typed request builder
and returns `Result<Value, SignalWireRestError>`. The client is **synchronous** —
there is no `.await`. In-call commands take the `call_id` as their first
argument.

<!-- snippet-setup -->
```rust
use signalwire::rest::RestClient;
use signalwire::rest::namespaces::generated::calling_resources_generated::CallingDialRequest;
use signalwire::rest::namespaces::generated::calling_resources_generated::CallingEndRequest;
use signalwire::rest::namespaces::generated::calling_resources_generated::CallingPlayRequest;
use serde_json::json;

let client = RestClient::new("project-id", "api-token", "example.signalwire.com").unwrap();
```

## Initiating a Call

`dial` takes a `CallingDialRequest`. `from` and `to` are required; optional
fields are set via chained builder methods:

```rust
let response = client.calling().dial(
    CallingDialRequest::new("+15559876543", "+15551234567")
        .url("https://example.com/call-handler")
        .status_url("https://example.com/call-status"),
    None,
).unwrap();

println!("Response: {}", response["sid"]);
```

## Ending a Call

In-call commands take the `call_id` first, then a request builder:

```rust
let _ = client.calling().end("call-id", CallingEndRequest::new(), None).unwrap();
```

## Playing Media

`play` takes a `CallingPlayRequest::new(play)` where `play` is the media array
value:

```rust
let _ = client.calling().play(
    "call-id",
    CallingPlayRequest::new(json!([
        {"type": "tts", "params": {"text": "Please hold."}}
    ])),
    None,
).unwrap();
```

## Outbound Dial Parameters

`CallingDialRequest` fields:

| Field / builder | Type | Description |
|-----------------|------|-------------|
| `new(from, to)` | `String, String` | Caller ID and destination (required) |
| `caller_id(..)` | `String` | Override caller ID |
| `url(..)` | `String` | SWML/cXML handler URL |
| `url_method(..)` | `String` | HTTP method for the handler URL |
| `status_url(..)` | `String` | Status webhook URL |
| `status_events(..)` | `Value` | Which status events to POST |
| `fallback_url(..)` | `String` | Fallback handler URL |
| `codecs(..)` | `Value` | Codec preferences |
| `swml(..)` | `Value` | Inline SWML document |

### Call Status Values

| Status | Description |
|--------|-------------|
| `queued` | Call is queued |
| `ringing` | Call is ringing |
| `in-progress` | Call is active |
| `completed` | Call ended normally |
| `busy` | Destination busy |
| `failed` | Call failed |
| `no-answer` | No answer within timeout |
| `canceled` | Call was canceled |
