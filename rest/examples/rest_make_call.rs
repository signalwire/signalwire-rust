// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Initiate an outbound phone call via the REST API.
//!
//! The REST calling namespace is a synchronous command-dispatch surface: `dial`
//! takes a typed `CallingDialRequest` builder (`from` and `to` are required;
//! optional fields via chained methods) and returns
//! `Result<Value, SignalWireRestError>`.
//!
//! Usage:
//!   `FROM_NUMBER=+15559876543 TO_NUMBER=+15551234567`
//!   `CALL_URL=https://example.com/handler cargo run --example rest_make_call`
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use signalwire::rest::RestClient;
use signalwire::rest::namespaces::generated::calling_resources_generated::CallingDialRequest;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    let from = env::var("FROM_NUMBER").unwrap_or_else(|_| "+15559876543".into());
    let to = env::var("TO_NUMBER").unwrap_or_else(|_| "+15551234567".into());
    let url = env::var("CALL_URL").unwrap_or_else(|_| "https://example.com/call-handler".into());

    println!("Dialing {to} from {from} ...");

    let result = client.calling().dial(
        CallingDialRequest::new(from, to)
            .url(&url)
            .status_url(format!("{url}/status")), None
    )?;

    println!("Call SID: {}", result["sid"]);
    println!("Status: {}", result["status"]);

    Ok(())
}
