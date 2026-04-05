# Fabric

## Overview

Fabric is SignalWire's AI and communication platform. The REST client provides access to 13 Fabric resource types.

## AI Agents

### Create an Agent

```rust
let agent = client.fabric().ai_agents().create(json!({
    "name": "Support Bot",
    "prompt": {
        "text": "You are a helpful support agent."
    },
    "languages": [{
        "name": "English",
        "code": "en-US",
        "voice": "inworld.Mark"
    }]
})).await?;

println!("Agent ID: {}", agent["id"]);
```

### List Agents

```rust
let agents = client.fabric().ai_agents().list(&[]).await?;
for a in agents.as_array().unwrap_or(&vec![]) {
    println!("{}: {}", a["id"], a["name"]);
}
```

### Update an Agent

```rust
client.fabric().ai_agents().update("agent-id", json!({
    "prompt": {"text": "Updated prompt."}
})).await?;
```

### Delete an Agent

```rust
client.fabric().ai_agents().delete("agent-id").await?;
```

## Addresses

Addresses map phone numbers, SIP URIs, and agent endpoints to resources.

```rust
// Create an address
let addr = client.fabric().addresses().create(json!({
    "name": "Support Line",
    "type": "phone",
    "phone_number": "+15551234567",
    "resource_id": "agent-id"
})).await?;

// List addresses
let addrs = client.fabric().addresses().list(&[]).await?;
```

## Subscribers

```rust
// Create a subscriber
let sub = client.fabric().subscribers().create(json!({
    "email": "user@example.com",
    "first_name": "Alice",
    "last_name": "Smith"
})).await?;

// List subscribers
let subs = client.fabric().subscribers().list(&[]).await?;
```

## SIP Endpoints

```rust
let endpoint = client.fabric().sip_endpoints().create(json!({
    "username": "alice",
    "password": "secure-password",
    "caller_id": "+15551234567"
})).await?;
```

## Conversations

```rust
// List conversations
let convos = client.fabric().conversations().list(&[]).await?;

// Send a message in a conversation
client.fabric().conversations().send_message("convo-id", json!({
    "body": "Hello from the REST API!"
})).await?;
```

## Tokens

Generate authentication tokens for client-side applications:

```rust
let token = client.fabric().tokens().create(json!({
    "subscriber_id": "sub-id",
    "expires_in": 3600,
})).await?;

println!("Token: {}", token["token"]);
```

## Full Resource List

| Resource | Methods |
|----------|---------|
| `ai_agents()` | create, list, get, update, delete |
| `addresses()` | create, list, get, update, delete |
| `subscribers()` | create, list, get, update, delete |
| `sip_endpoints()` | create, list, get, update, delete |
| `phone_numbers()` | list, assign, unassign |
| `conversations()` | list, get, send_message |
| `devices()` | list, get |
| `tokens()` | create |
| `policies()` | create, list, get, update, delete |
| `calls()` | list, get |
| `logs()` | list |
| `features()` | list, get, update |
| `webhooks()` | create, list, get, update, delete |
