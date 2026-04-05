// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Search for available phone numbers by area code.
//!
//! Usage: AREA_CODE=512 cargo run --example rest_search_phone_numbers
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;
    let area_code = env::var("AREA_CODE").unwrap_or_else(|_| "512".into());

    println!("Searching for numbers with area code {area_code} ...");

    let results = client.phone_numbers().search(&[
        ("area_code", &area_code),
        ("limit", "10"),
    ]).await?;

    if let Some(arr) = results.as_array() {
        println!("Found {} available numbers:", arr.len());
        for n in arr {
            println!(
                "  {} - ${}/mo",
                n["phone_number"],
                n["monthly_cost"]
            );
        }
    } else {
        println!("No numbers available for area code {area_code}.");
    }

    Ok(())
}
