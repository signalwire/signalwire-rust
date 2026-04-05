// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! IVR menu with DTMF collection and call transfer.
//!
//! Answers inbound calls, presents a menu, collects a digit,
//! and transfers to the appropriate department.
//!
//! Environment:
//!   SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE
//!   SALES_NUMBER   - sales department number (default: +15551111111)
//!   SUPPORT_NUMBER - support department number (default: +15552222222)

use signalwire::relay::RelayClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RelayClient::builder()
        .project(&env::var("SIGNALWIRE_PROJECT_ID")?)
        .token(&env::var("SIGNALWIRE_API_TOKEN")?)
        .space(&env::var("SIGNALWIRE_SPACE")?)
        .contexts(vec!["default".into()])
        .build()?;

    let sales_number = env::var("SALES_NUMBER").unwrap_or_else(|_| "+15551111111".into());
    let support_number = env::var("SUPPORT_NUMBER").unwrap_or_else(|_| "+15552222222".into());

    client.on_call(move |call| {
        let sales = sales_number.clone();
        let support = support_number.clone();
        async move {
            println!("Incoming call from: {}", call.from());
            call.answer().await?;

            // Play the IVR menu and collect a digit
            let result = call.prompt(
                vec![serde_json::json!({
                    "type": "tts",
                    "params": {
                        "text": "Welcome to ACME Corporation. \
                                 Press 1 for sales. \
                                 Press 2 for support. \
                                 Press 3 to leave a voicemail."
                    }
                })],
                serde_json::json!({
                    "digits": {
                        "max": 1,
                        "digit_timeout": 5.0,
                        "terminators": "#"
                    }
                }),
            ).await?.wait().await?;

            match result.digits.as_str() {
                "1" => {
                    println!("Transferring to sales: {sales}");
                    call.play_tts("Connecting you to our sales team.").await?.wait().await?;
                    let action = call.connect(serde_json::json!({
                        "devices": [[{
                            "type": "phone",
                            "params": {
                                "to_number": sales,
                                "from_number": call.to()
                            }
                        }]]
                    })).await?;
                    action.wait().await?;
                }
                "2" => {
                    println!("Transferring to support: {support}");
                    call.play_tts("Connecting you to technical support.").await?.wait().await?;
                    let action = call.connect(serde_json::json!({
                        "devices": [[{
                            "type": "phone",
                            "params": {
                                "to_number": support,
                                "from_number": call.to()
                            }
                        }]]
                    })).await?;
                    action.wait().await?;
                }
                "3" => {
                    println!("Recording voicemail");
                    call.play_tts(
                        "Please leave a message after the beep. Press pound when finished.",
                    ).await?.wait().await?;
                    let action = call.record(serde_json::json!({
                        "direction": "speak",
                        "format": "wav",
                        "beep": true,
                        "terminators": "#",
                        "end_silence_timeout": 3.0,
                    })).await?;
                    let rec = action.wait().await?;
                    println!("Voicemail recorded: {}", rec.url);
                    call.play_tts("Thank you for your message. Goodbye!").await?.wait().await?;
                }
                _ => {
                    call.play_tts("Invalid selection. Goodbye!").await?.wait().await?;
                }
            }

            call.hangup().await?;
            println!("Call ended: {}", call.call_id);
            Ok(())
        }
    });

    println!("IVR system running. Waiting for inbound calls ...");
    client.run().await?;
    Ok(())
}
