# Fabric

## Overview

Fabric is SignalWire's AI and communication platform. The REST client provides
access to the Fabric resource types. The REST client is **synchronous** — every
method returns `Result<Value, SignalWireRestError>` directly; there is no
`.await`. `create`/`update` take a `&serde_json::Value` body; `list`/`search`
take a `&HashMap<String, String>` of query params.

<!-- snippet-setup -->
```rust
use signalwire::rest::RestClient;
use serde_json::json;
use std::collections::HashMap;

let client = RestClient::new("project-id", "api-token", "example.signalwire.com").unwrap();
```

## AI Agents

### Create an Agent

```rust
let agent = client.fabric().ai_agents().create(&json!({
    "name": "Support Bot",
    "prompt": {
        "text": "You are a helpful support agent."
    },
    "languages": [{
        "name": "English",
        "code": "en-US",
        "voice": "inworld.Mark"
    }]
}), None).unwrap();

println!("Agent ID: {}", agent["id"]);
```

### List Agents

```rust
let agents = client.fabric().ai_agents().list(&HashMap::new(), None).unwrap();
for a in agents.as_array().unwrap_or(&vec![]) {
    println!("{}: {}", a["id"], a["name"]);
}
```

### Update an Agent

```rust
client.fabric().ai_agents().update("agent-id", &json!({
    "prompt": {"text": "Updated prompt."}
}), None).unwrap();
```

### Delete an Agent

```rust
client.fabric().ai_agents().delete("agent-id", None).unwrap();
```

## Addresses

Top-level fabric addresses are read-only (`list` / `get`):

```rust
let addrs = client.fabric().addresses().list(&HashMap::new(), None).unwrap();
for a in addrs.as_array().unwrap_or(&vec![]) {
    println!("{}", a["id"]);
}
```

## Subscribers

```rust
// Create a subscriber
let sub = client.fabric().subscribers().create(&json!({
    "email": "user@example.com",
    "first_name": "Alice",
    "last_name": "Smith"
}), None).unwrap();

// List subscribers
let subs = client.fabric().subscribers().list(&HashMap::new(), None).unwrap();
```

## SIP Endpoints

```rust
let endpoint = client.fabric().sip_endpoints().create(&json!({
    "username": "alice",
    "password": "secure-password",
    "caller_id": "+15551234567"
}), None).unwrap();
```

## Tokens

Fabric tokens use dedicated request-builder methods (there is no generic
`create`). Generate a subscriber token from the subscriber's reference:

```rust
use signalwire::rest::namespaces::generated::fabric_resources_generated::FabricTokensCreateSubscriberTokenRequest;

let token = client.fabric().tokens().create_subscriber_token(
    FabricTokensCreateSubscriberTokenRequest::new("subscriber-reference").expire_at(3600),
    None,
).unwrap();

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
