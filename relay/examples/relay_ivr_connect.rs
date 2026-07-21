// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! IVR menu with DTMF collection and call transfer.
//!
//! Answers inbound calls, presents a menu, collects a digit, and connects to
//! the appropriate department. The RELAY client is synchronous — no
//! `async`/`await`. Prompt/collect verbs return an `Arc<Action>`; block on
//! `wait()` for the collected result (a `serde_json::Value`).
//!
//! Environment:
//!   `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`
//!   `SALES_NUMBER`   - sales department number (default: +15551111111)
//!   `SUPPORT_NUMBER` - support department number (default: +15552222222)

use serde_json::{Value, json};
use signalwire::relay::Client;
use std::env;
use std::sync::Arc;
use std::time::Duration;

/// Pull the collected DTMF digits out of a prompt/collect result `Value`.
/// The server nests them under `result.params.digits`; fall back to a
/// top-level `digits` if present. Returns an empty string when none.
fn collected_digits(result: &Value) -> String {
    result
        .get("result")
        .and_then(|r| r.get("params"))
        .and_then(|p| p.get("digits"))
        .or_else(|| result.get("digits"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(Client::from_env()?);

    let sales_number = env::var("SALES_NUMBER").unwrap_or_else(|_| "+15551111111".into());
    let support_number = env::var("SUPPORT_NUMBER").unwrap_or_else(|_| "+15552222222".into());

    client.on_call(move |call, _event| {
        println!("Incoming call: {}", call.repr());
        let _ = call.answer();

        // Play the IVR menu and collect a single digit.
        let prompt = call
            .prompt_tts(
                "Welcome to ACME Corporation. Press 1 for sales. \
             Press 2 for support. Press 3 to leave a voicemail.",
                json!({
                    "digits": {
                        "max": 1,
                        "digit_timeout": 5.0,
                        "terminators": "#"
                    }
                }),
                json!({}),
            )
            .expect("relay verb must start against the server");

        let digit = prompt
            .wait(Some(Duration::from_secs(30)))
            .as_ref()
            .map(collected_digits)
            .unwrap_or_default();

        match digit.as_str() {
            "1" => {
                println!("Transferring to sales: {sales_number}");
                let action = call
                    .play_tts("Connecting you to our sales team.", json!({}))
                    .expect("relay verb must start against the server");
                let _ = action.wait(Some(Duration::from_secs(15)));
                // `connect` is a simple control verb: returns the transmitted params.
                let _ = call.connect(json!({
                    "devices": [[{
                        "type": "phone",
                        "params": {"to_number": sales_number, "from_number": "+15550000000"}
                    }]]
                }));
            }
            "2" => {
                println!("Transferring to support: {support_number}");
                let action = call
                    .play_tts("Connecting you to technical support.", json!({}))
                    .expect("relay verb must start against the server");
                let _ = action.wait(Some(Duration::from_secs(15)));
                let _ = call.connect(json!({
                    "devices": [[{
                        "type": "phone",
                        "params": {"to_number": support_number, "from_number": "+15550000000"}
                    }]]
                }));
            }
            "3" => {
                println!("Recording voicemail");
                let action = call
                    .play_tts(
                        "Please leave a message after the beep. Press pound when finished.",
                        json!({}),
                    )
                    .expect("relay verb must start against the server");
                let _ = action.wait(Some(Duration::from_secs(15)));

                let rec = call
                    .record(json!({
                        "direction": "speak",
                        "format": "wav",
                        "beep": true,
                        "terminators": "#",
                        "end_silence_timeout": 3.0
                    }))
                    .expect("relay verb must start against the server");
                if let Some(result) = rec.wait(Some(Duration::from_secs(120))) {
                    println!("Voicemail recorded: {result}");
                }
                let action = call
                    .play_tts("Thank you for your message. Goodbye!", json!({}))
                    .expect("relay verb must start against the server");
                let _ = action.wait(Some(Duration::from_secs(15)));
            }
            _ => {
                let action = call
                    .play_tts("Invalid selection. Goodbye!", json!({}))
                    .expect("relay verb must start against the server");
                let _ = action.wait(Some(Duration::from_secs(15)));
            }
        }

        let _ = call.hangup();
        println!("Call ended: {}", call.repr());
    });

    println!("IVR system running. Waiting for inbound calls ...");
    client.connect()?;
    client.receive(&["default".to_string()]);

    client.run();
    Ok(())
}
