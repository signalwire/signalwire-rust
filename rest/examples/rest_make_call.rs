// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Initiate an outbound phone call via the REST API.
//!
//! Usage:
//!   FROM_NUMBER=+15559876543 TO_NUMBER=+15551234567 \
//!     CALL_URL=https://example.com/handler \
//!     cargo run --example rest_make_call
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    let from = env::var("FROM_NUMBER").unwrap_or_else(|_| "+15559876543".into());
    let to = env::var("TO_NUMBER").unwrap_or_else(|_| "+15551234567".into());
    let url = env::var("CALL_URL")
        .unwrap_or_else(|_| "https://example.com/call-handler".into());

    println!("Dialing {to} from {from} ...");

    let result = client.calling().dial(serde_json::json!({
        "from": from,
        "to": to,
        "url": url,
        "status_callback": format!("{url}/status"),
    })).await?;

    println!("Call SID: {}", result["sid"]);
    println!("Status: {}", result["status"]);

    Ok(())
}
