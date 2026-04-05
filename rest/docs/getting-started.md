# Getting Started with REST

## Prerequisites

- A SignalWire account with a project
- Rust 1.85+ with tokio runtime

## Environment Setup

```bash
export SIGNALWIRE_PROJECT_ID="your-project-id"
export SIGNALWIRE_API_TOKEN="your-api-token"
export SIGNALWIRE_SPACE="example.signalwire.com"
```

## Creating a Client

### From Environment Variables

```rust
use signalwire::rest::RestClient;

let client = RestClient::from_env()?;
```

### Explicit Configuration

```rust
let client = RestClient::new(
    "your-project-id",
    "your-api-token",
    "example.signalwire.com",
)?;
```

## First API Call

```rust
use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // List your phone numbers
    let numbers = client.phone_numbers().list(&[]).await?;
    for number in numbers.as_array().unwrap_or(&vec![]) {
        println!("{}", number["phone_number"]);
    }

    Ok(())
}
```

## Response Format

All methods return `serde_json::Value`. There are no wrapper types -- you get the raw JSON from the API:

```rust
let result = client.phone_numbers().search(
    &[("area_code", "512"), ("limit", "5")]
).await?;

// result is a serde_json::Value
if let Some(numbers) = result.as_array() {
    for n in numbers {
        println!("{}: {}", n["phone_number"], n["friendly_name"]);
    }
}
```

## Error Handling

API errors are returned as `Result` errors:

```rust
match client.calling().dial(params).await {
    Ok(response) => println!("Call SID: {}", response["sid"]),
    Err(e) => eprintln!("API error: {e}"),
}
```

## Next Steps

- [Namespaces](namespaces.md) -- explore all 21 API surfaces
- [Calling](calling.md) -- voice call management
- [Fabric](fabric.md) -- AI agents and addresses
