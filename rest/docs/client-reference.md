# Client Reference

## RestClient

The main entry point for REST API operations. The client is **synchronous** —
every method makes a blocking HTTP call and returns
`Result<Value, SignalWireRestError>` directly. There is no `async`/`await`.

<!-- snippet-setup -->
```rust
use signalwire::rest::RestClient;
use serde_json::json;
use std::collections::HashMap;

let client = RestClient::new("project-id", "api-token", "example.signalwire.com").unwrap();
```

### Construction

Both constructors return `Result<RestClient, String>`:

```rust
// From environment variables
let from_env = RestClient::from_env();

// Explicit configuration
let explicit = RestClient::new("project-id", "api-token", "example.signalwire.com");
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `SIGNALWIRE_PROJECT_ID` | Project identifier |
| `SIGNALWIRE_API_TOKEN` | API token |
| `SIGNALWIRE_SPACE` | Space hostname |
| `SIGNALWIRE_REST_BASE_URL` | Override the REST base URL (`from_env` only). When set, replaces the `https://{SIGNALWIRE_SPACE}` resolution — point the client at a regional host, a proxy, or a local fixture without a code change; `SIGNALWIRE_SPACE` is then not required |

### Per-call timeout (`RequestOptions`)

Every request accepts an optional [`RequestOptions`] carrying a per-call
`timeout` (seconds). Construct a client with a default `RequestOptions` applied
to every request via `RestClient::with_base_url_and_options`, and override it per
request where the call supports it. A per-request value shallow-overrides the
client default; an unset field falls back to the client default, then the
built-in.

```rust
use signalwire::rest::RequestOptions;

// A client-default 5s timeout on every request through this client:
let opts = RequestOptions::default().timeout(5.0);
let client = RestClient::with_base_url_and_options(
    "project-id", "api-token", "https://example.signalwire.com", Some(opts),
);
```

### Namespace Accessors

The client exposes each REST namespace as a method:

| Method | Description |
|--------|-------------|
| `fabric()` | Fabric AI platform APIs |
| `calling()` | Call command dispatch |
| `phone_numbers()` | Number management |
| `addresses()` | Fabric addresses |
| `video()` | Video rooms |
| `datasphere()` | Document search |
| `queues()` | Call queues |
| `recordings()` | Recording management |
| `number_groups()` | Number groups |
| `verified_callers()` | Verified caller IDs |
| `sip_profile()` | SIP profile |
| `lookup()` | Number lookup |
| `short_codes()` | Short codes |
| `imported_numbers()` | Imported numbers |
| `mfa()` | Multi-factor auth |
| `registry()` | Registry |
| `logs()` | Logs |
| `project()` | Project settings |
| `pubsub()` | Pub/Sub |
| `chat()` | Chat |

### Common CRUD Method Patterns

CRUD resources follow consistent signatures. `list`/`search` take a
`&HashMap<String, String>` of query params; `create`/`update` take a
`&serde_json::Value` body:

```rust
// List resources
let listed = client.fabric().ai_agents().list(&HashMap::new(), None).unwrap();

// Get a single resource
let one = client.fabric().ai_agents().get("resource-id", None).unwrap();

// Create a resource
let created = client.fabric().ai_agents().create(&json!({"name": "Bot"}), None).unwrap();

// Update a resource
let updated = client.fabric().ai_agents().update("resource-id", &json!({"name": "Bot 2"}), None).unwrap();

// Delete a resource
client.fabric().ai_agents().delete("resource-id", None).unwrap();
```

### Return Type

All methods return `Result<Value, SignalWireRestError>` where `Value` is
`serde_json::Value`. There are no wrapper types — you get the raw JSON.

### Authentication

The client uses HTTP Basic auth with `project_id:api_token`, added to every
request automatically.
