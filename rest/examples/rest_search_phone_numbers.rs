// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Search for available phone numbers by area code.
//!
//! The REST client is synchronous. `search` takes a `&HashMap<String, String>`
//! of query params and returns `Result<Value, SignalWireRestError>`.
//!
//! Usage: `AREA_CODE=512 cargo run --example rest_search_phone_numbers`
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use signalwire::rest::RestClient;
use std::collections::HashMap;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;
    let area_code = env::var("AREA_CODE").unwrap_or_else(|_| "512".into());

    println!("Searching for numbers with area code {area_code} ...");

    let params = HashMap::from([
        ("areacode".to_string(), area_code.clone()),
        ("limit".to_string(), "10".to_string()),
    ]);
    let results = client.phone_numbers().search(&params)?;

    if let Some(arr) = results.as_array() {
        println!("Found {} available numbers:", arr.len());
        for n in arr {
            println!("  {} - ${}/mo", n["phone_number"], n["monthly_cost"]);
        }
    } else {
        println!("No numbers available for area code {area_code}.");
    }

    Ok(())
}
