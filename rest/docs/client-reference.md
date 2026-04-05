# Client Reference

## RestClient

The main entry point for REST API operations.

### Construction

```rust
// From environment variables
let client = RestClient::from_env()?;

// Explicit configuration
let client = RestClient::new(project_id, api_token, space)?;
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `SIGNALWIRE_PROJECT_ID` | Project identifier |
| `SIGNALWIRE_API_TOKEN` | API token |
| `SIGNALWIRE_SPACE` | Space hostname |

### Namespace Accessors

| Method | Returns | Description |
|--------|---------|-------------|
| `fabric()` | `FabricClient` | Fabric AI platform APIs |
| `calling()` | `CallingClient` | Call management |
| `messaging()` | `MessagingClient` | SMS/MMS |
| `phone_numbers()` | `PhoneNumbersClient` | Number management |
| `sip()` | `SipClient` | SIP operations |
| `video()` | `VideoClient` | Video rooms |
| `datasphere()` | `DatasphereClient` | Document search |
| `queues()` | `QueuesClient` | Call queues |
| `recordings()` | `RecordingsClient` | Recording management |
| `compat()` | `CompatClient` | Twilio-compatible API |
| `fax()` | `FaxClient` | Fax operations |
| `conferences()` | `ConferencesClient` | Conferences |
| `transcriptions()` | `TranscriptionsClient` | Transcription operations |
| `applications()` | `ApplicationsClient` | Application management |
| `usage()` | `UsageClient` | Usage data |

### Common Method Patterns

All namespaces follow consistent patterns:

```rust
// List resources (query parameters as key-value tuples)
client.namespace().list(&[("key", "value")]).await?;

// Get a single resource
client.namespace().get("resource-id").await?;

// Create a resource (JSON body)
client.namespace().create(json!({...})).await?;

// Update a resource
client.namespace().update("resource-id", json!({...})).await?;

// Delete a resource
client.namespace().delete("resource-id").await?;

// Search (query parameters)
client.namespace().search(&[("key", "value")]).await?;
```

### Return Type

All methods return `Result<Value, Box<dyn std::error::Error>>` where `Value` is `serde_json::Value`.

### Connection Pooling

The client uses `reqwest::Client` internally with connection pooling. Creating multiple `RestClient` instances is safe but unnecessary -- a single client can handle concurrent requests.

### Timeouts

Default request timeout is 30 seconds. Configure via the builder:

```rust
let client = RestClient::builder()
    .project(project_id)
    .token(api_token)
    .space(space)
    .timeout(std::time::Duration::from_secs(60))
    .build()?;
```

### Authentication

The client uses HTTP Basic auth with `project_id:api_token`. Authentication is added to every request automatically.
