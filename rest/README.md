# REST Client

Synchronous REST client for managing SignalWire resources over HTTP. No WebSocket required.

## Quick Start

The client is **synchronous** — every method makes a blocking HTTP call and
returns `Result<Value, SignalWireRestError>` directly. `create`/`update` take a
`&serde_json::Value` body; `list`/`search` take a `&HashMap<String, String>` of
query params. In-call `calling()` commands take a typed request builder.

```rust
use signalwire::rest::RestClient;
use signalwire::rest::namespaces::generated::calling_resources_generated::CallingDialRequest;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a Fabric AI agent
    client.fabric().ai_agents().create(&serde_json::json!({
        "name": "Support Bot",
        "prompt": {"text": "You are helpful."}
    }))?;

    // Make a phone call
    client.calling().dial(
        CallingDialRequest::new("+15559876543", "+15551234567")
            .url("https://example.com/call-handler"),
    )?;

    // Search for phone numbers
    let mut params = HashMap::new();
    params.insert("areacode".to_string(), "512".to_string());
    let results = client.phone_numbers().search(&params)?;
    println!("{results:#?}");

    Ok(())
}
```

## Features

- **Namespaced API surfaces** -- coverage of SignalWire HTTP APIs (Fabric, calling, phone numbers, video, datasphere, and more)
- **Connection pooling** -- via `reqwest::Client`
- **Raw JSON returns** -- `serde_json::Value` with no wrapper objects
- **Synchronous** -- blocking calls, no runtime to set up

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SIGNALWIRE_PROJECT_ID` | Your project ID |
| `SIGNALWIRE_API_TOKEN` | Your API token |
| `SIGNALWIRE_SPACE` | Your space hostname |

## Documentation

- [Getting Started](docs/getting-started.md) -- setup and first API call
- [Namespaces](docs/namespaces.md) -- all API namespaces
- [Calling](docs/calling.md) -- voice call management
- [Fabric](docs/fabric.md) -- AI agents, addresses, subscribers
- [Client Reference](docs/client-reference.md) -- RestClient API (synchronous)

## Examples

| Example | Description |
|---------|-------------|
| [rest_list_phone_numbers.rs](examples/rest_list_phone_numbers.rs) | List phone numbers |
| [rest_search_phone_numbers.rs](examples/rest_search_phone_numbers.rs) | Search available numbers |
| [rest_buy_phone_number.rs](examples/rest_buy_phone_number.rs) | Purchase a number |
| [rest_send_sms.rs](examples/rest_send_sms.rs) | Send an SMS message |
| [rest_make_call.rs](examples/rest_make_call.rs) | Initiate an outbound call |
| [rest_create_sip_endpoint.rs](examples/rest_create_sip_endpoint.rs) | Create a SIP endpoint |
| [rest_manage_queues.rs](examples/rest_manage_queues.rs) | Queue management |
| [rest_list_recordings.rs](examples/rest_list_recordings.rs) | List call recordings |
| [rest_fabric_agent.rs](examples/rest_fabric_agent.rs) | Manage Fabric AI agents |
| [rest_fabric_subscribers.rs](examples/rest_fabric_subscribers.rs) | Manage subscribers |
| [rest_datasphere.rs](examples/rest_datasphere.rs) | Datasphere document search |
| [rest_video_rooms.rs](examples/rest_video_rooms.rs) | Video room management |
