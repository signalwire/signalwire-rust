# REST Client

Async REST client for managing SignalWire resources over HTTP. No WebSocket required.

## Quick Start

```rust
use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a Fabric AI agent
    client.fabric().ai_agents().create(serde_json::json!({
        "name": "Support Bot",
        "prompt": {"text": "You are helpful."}
    })).await?;

    // Make a phone call
    client.calling().dial(serde_json::json!({
        "from": "+15559876543",
        "to": "+15551234567",
        "url": "https://example.com/call-handler"
    })).await?;

    // Search for phone numbers
    let results = client.phone_numbers().search(
        &[("area_code", "512")]
    ).await?;
    println!("{results:#?}");

    Ok(())
}
```

## Features

- **21 namespaced API surfaces** -- complete coverage of SignalWire HTTP APIs
- **Connection pooling** -- via `reqwest::Client`
- **Raw JSON returns** -- `serde_json::Value` with no wrapper objects
- **Async/await** -- built on tokio

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SIGNALWIRE_PROJECT_ID` | Your project ID |
| `SIGNALWIRE_API_TOKEN` | Your API token |
| `SIGNALWIRE_SPACE` | Your space hostname |

## Documentation

- [Getting Started](docs/getting-started.md) -- setup and first API call
- [Namespaces](docs/namespaces.md) -- all 21 API namespaces
- [Calling](docs/calling.md) -- voice call management
- [Fabric](docs/fabric.md) -- AI agents, addresses, subscribers
- [Compat](docs/compat.md) -- Twilio-compatible APIs
- [Client Reference](docs/client-reference.md) -- RestClient API

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
