# Compat (Twilio-Compatible API)

## Overview

SignalWire provides a Twilio-compatible REST API for easy migration. The compat namespace mirrors the Twilio API structure.

## Calls

### Create a Call

```rust
let call = client.compat().calls().create(json!({
    "From": "+15559876543",
    "To": "+15551234567",
    "Url": "https://example.com/twiml-handler",
})).await?;

println!("Call SID: {}", call["sid"]);
```

### List Calls

```rust
let calls = client.compat().calls().list(&[
    ("Status", "completed"),
    ("PageSize", "20"),
]).await?;
```

### Update a Call

```rust
client.compat().calls().update("CA-sid", json!({
    "Status": "completed",  // hang up
})).await?;
```

## Messages

### Send SMS

```rust
let msg = client.compat().messages().create(json!({
    "From": "+15559876543",
    "To": "+15551234567",
    "Body": "Hello from SignalWire!",
})).await?;

println!("Message SID: {}", msg["sid"]);
```

### Send MMS

```rust
let msg = client.compat().messages().create(json!({
    "From": "+15559876543",
    "To": "+15551234567",
    "Body": "Check this out!",
    "MediaUrl": "https://example.com/image.jpg",
})).await?;
```

### List Messages

```rust
let messages = client.compat().messages().list(&[
    ("DateSent>", "2024-01-01"),
    ("PageSize", "10"),
]).await?;
```

## Phone Numbers

### List Incoming Numbers

```rust
let numbers = client.compat().incoming_phone_numbers().list(&[]).await?;
for n in numbers.as_array().unwrap_or(&vec![]) {
    println!("{}: {}", n["sid"], n["phone_number"]);
}
```

### Search Available Numbers

```rust
let available = client.compat().available_phone_numbers()
    .local("US", &[("AreaCode", "512")])
    .await?;
```

## Recordings

```rust
let recordings = client.compat().recordings().list(&[]).await?;
for r in recordings.as_array().unwrap_or(&vec![]) {
    println!("{}: {}s", r["sid"], r["duration"]);
}
```

## Migration from Twilio

The compat API uses the same parameter names (PascalCase) as the Twilio API. Replace:

| Twilio | SignalWire |
|--------|-----------|
| `api.twilio.com` | `{space}.signalwire.com` |
| Account SID | `SIGNALWIRE_PROJECT_ID` |
| Auth Token | `SIGNALWIRE_API_TOKEN` |

The request/response format is the same. Most Twilio code can be migrated by changing the client configuration.
