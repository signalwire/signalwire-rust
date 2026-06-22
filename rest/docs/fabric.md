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
| `ai_agents()` | list, create, get, update, delete, list_addresses |
| `sip_gateways()` | list, create, get, update, delete, list_addresses |
| `cxml_webhooks()` | list, create, get, update, delete, list_addresses |
| `swml_webhooks()` | list, create, get, update, delete, list_addresses |
| `sip_endpoints()` | list, create, get, update, delete, list_addresses |
| `swml_scripts()` | list, create, get, update, delete, list_addresses |
| `cxml_scripts()` | list, create, get, update, delete, list_addresses |
| `relay_applications()` | list, create, get, update, delete, list_addresses |
| `freeswitch_connectors()` | list, create, get, update, delete, list_addresses |
| `conference_rooms()` | list, create, get, update, delete, list_addresses |
| `cxml_applications()` | list, get, update, delete, list_addresses (create returns an error by design) |
| `call_flows()` | list, create, get, update, delete, list_addresses, list_versions, deploy_version |
| `subscribers()` | list, create, get, update, delete, list_addresses, list_sip_endpoints, create_sip_endpoint, get_sip_endpoint, update_sip_endpoint, delete_sip_endpoint |
| `addresses()` | list, get (read-only top-level fabric addresses) |
| `resources()` | list, get, delete, list_addresses, assign_domain_application, assign_phone_route |
| `tokens()` | create_subscriber_token, refresh_subscriber_token, create_invite_token, create_guest_token, create_embed_token |
