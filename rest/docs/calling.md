# Calling

## Overview

The calling namespace provides HTTP-based call management. Use it to initiate outbound calls, update active calls, and query call history.

## Initiating a Call

```rust
let response = client.calling().dial(json!({
    "from": "+15559876543",
    "to": "+15551234567",
    "url": "https://example.com/call-handler",
    "status_callback": "https://example.com/call-status",
})).await?;

println!("Call SID: {}", response["sid"]);
```

## Updating an Active Call

```rust
// Redirect to a different URL
client.calling().update("call-sid", json!({
    "url": "https://example.com/new-handler",
})).await?;

// Hang up the call
client.calling().update("call-sid", json!({
    "status": "completed",
})).await?;
```

## Listing Calls

```rust
let calls = client.calling().list(&[
    ("status", "in-progress"),
    ("limit", "10"),
]).await?;

for call in calls.as_array().unwrap_or(&vec![]) {
    println!("{}: {} -> {} ({})",
        call["sid"], call["from"], call["to"], call["status"]);
}
```

## Getting Call Details

```rust
let call = client.calling().get("call-sid").await?;
println!("Duration: {}s", call["duration"]);
println!("Status: {}", call["status"]);
```

## Call Recordings

```rust
let recordings = client.calling().recordings("call-sid").await?;
for rec in recordings.as_array().unwrap_or(&vec![]) {
    println!("Recording: {} ({}s)", rec["sid"], rec["duration"]);
}
```

## Call Parameters

### Outbound Call Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `from` | `string` | Caller ID (your SignalWire number) |
| `to` | `string` | Destination number |
| `url` | `string` | SWML/TwiML handler URL |
| `status_callback` | `string` | Status webhook URL |
| `timeout` | `integer` | Ring timeout in seconds |
| `record` | `boolean` | Record the call |
| `machine_detection` | `string` | AMD mode |

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
