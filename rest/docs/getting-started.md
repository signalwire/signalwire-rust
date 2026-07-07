# Getting Started with REST

## Prerequisites

- A SignalWire account with a project
- Rust 1.85+ (edition 2024)

## Environment Setup

```bash
export SIGNALWIRE_PROJECT_ID="your-project-id"
export SIGNALWIRE_API_TOKEN="your-api-token"
export SIGNALWIRE_SPACE="example.signalwire.com"
```

## Creating a Client

The REST client is **synchronous** — its methods make blocking HTTP calls and
return `Result<Value, SignalWireRestError>` directly. There is no `async`/`await`
and no runtime to set up.

### From Environment Variables

`RestClient::from_env()` returns `Result<RestClient, String>`:

```rust
use signalwire::rest::RestClient;

let client = RestClient::from_env().unwrap();
```

### Explicit Configuration

`RestClient::new(project_id, token, space)` also returns `Result<RestClient, String>`:

```rust
use signalwire::rest::RestClient;

let client = RestClient::new(
    "your-project-id",
    "your-api-token",
    "example.signalwire.com",
).unwrap();
```

## First API Call

```rust
use signalwire::rest::RestClient;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // List your phone numbers
    let numbers = client.phone_numbers().list(&HashMap::new())?;
    for number in numbers.as_array().unwrap_or(&vec![]) {
        println!("{}", number["phone_number"]);
    }

    Ok(())
}
```

## Response Format

All methods return `serde_json::Value`. There are no wrapper types -- you get the
raw JSON from the API. `search` takes a `&HashMap<String, String>` of query
parameters:

<!-- snippet-setup -->
```rust
use signalwire::rest::RestClient;
use std::collections::HashMap;

let client = RestClient::new("p", "t", "example.signalwire.com").unwrap();
```

```rust
let mut params = HashMap::new();
params.insert("area_code".to_string(), "512".to_string());
params.insert("limit".to_string(), "5".to_string());

let result = client.phone_numbers().search(&params).unwrap();

// result is a serde_json::Value
if let Some(numbers) = result.as_array() {
    for n in numbers {
        println!("{}: {}", n["phone_number"], n["friendly_name"]);
    }
}
```

## Error Handling

API errors are returned as `Result` errors carrying a `SignalWireRestError`:

```rust
match client.fabric().ai_agents().list(&HashMap::new()) {
    Ok(agents) => println!("count: {}", agents.as_array().map_or(0, Vec::len)),
    Err(e) => eprintln!("API error: {e}"),
}
```

## Next Steps

- [Calling](calling.md) -- voice call management
- [Fabric](fabric.md) -- AI agents and addresses
