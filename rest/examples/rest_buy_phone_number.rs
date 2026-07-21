// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Purchase a phone number.
//!
//! The REST client is synchronous. There is no dedicated `buy()` method:
//! purchasing a number is a `create` on the phone-numbers resource, which
//! takes a `&serde_json::Value` body and returns
//! `Result<Value, SignalWireRestError>`.
//!
//! Usage: `PHONE_NUMBER=+15125551234 cargo run --example rest_buy_phone_number`
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use serde_json::json;
use signalwire::rest::RestClient;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;
    let number = env::var("PHONE_NUMBER").expect("Set PHONE_NUMBER env var (e.g. +15125551234)");

    println!("Purchasing {number} ...");

    let result = client
        .phone_numbers()
        .create(&json!({ "number": number }), None)?;

    println!("Purchased: {}", result["phone_number"]);
    println!("ID: {}", result["id"]);
    println!("Capabilities: {}", result["capabilities"]);

    Ok(())
}
