// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! List all phone numbers in your SignalWire project.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    let numbers = client.phone_numbers().list(&[]).await?;

    if let Some(arr) = numbers.as_array() {
        println!("Phone numbers ({}):", arr.len());
        for n in arr {
            println!(
                "  {} - {} ({})",
                n["phone_number"],
                n["friendly_name"],
                n["capabilities"]
            );
        }
    } else {
        println!("No phone numbers found.");
    }

    Ok(())
}
