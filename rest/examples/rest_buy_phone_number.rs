// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Purchase a phone number.
//!
//! Usage: PHONE_NUMBER=+15125551234 cargo run --example rest_buy_phone_number
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;
    let number = env::var("PHONE_NUMBER")
        .expect("Set PHONE_NUMBER env var (e.g. +15125551234)");

    println!("Purchasing {number} ...");

    let result = client.phone_numbers().buy(serde_json::json!({
        "phone_number": number,
    })).await?;

    println!("Purchased: {}", result["phone_number"]);
    println!("SID: {}", result["sid"]);
    println!("Capabilities: {}", result["capabilities"]);

    Ok(())
}
