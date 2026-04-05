// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Record Call Example — record_call and stop_record_call virtual helpers.

use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    // Basic recording
    println!("=== Basic Recording ===");
    let result = FunctionResult::with_response("Starting basic call recording.");
    let mut result = result;
    result.add_action(json!({"record_call": {}}));
    println!("{}", result.to_json());

    // Advanced recording with options
    println!("\n=== Advanced Recording ===");
    let mut result = FunctionResult::with_response("Starting advanced call recording.");
    result.add_action(json!({
        "record_call": {
            "control_id": "support_call_2024_001",
            "stereo": true,
            "format": "mp3",
            "direction": "both",
            "terminators": "*#",
            "beep": true,
            "max_length": 600,
            "status_url": "https://api.company.com/recording-webhook"
        }
    }));
    println!("{}", result.to_json());

    // Voicemail recording
    println!("\n=== Voicemail Recording ===");
    let mut result = FunctionResult::with_response("Please leave your message after the beep.");
    result.add_action(json!({
        "record_call": {
            "control_id": "voicemail_123456",
            "format": "wav",
            "direction": "speak",
            "terminators": "#",
            "beep": true,
            "initial_timeout": 5.0,
            "end_silence_timeout": 3.0,
            "max_length": 120
        }
    }));
    println!("{}", result.to_json());

    // Stop recording
    println!("\n=== Stop Recording ===");
    let mut result = FunctionResult::with_response("Stopping the recording.");
    result.add_action(json!({"stop_record_call": {"control_id": "support_call_2024_001"}}));
    println!("{}", result.to_json());

    // Chain: start recording, notify, update global data
    println!("\n=== Chained Recording Actions ===");
    let mut result = FunctionResult::with_response("Recording and notifying.");
    result.add_action(json!({"record_call": {"control_id": "chain_demo", "stereo": true}}));
    result.add_action(json!({
        "send_sms": {"to": "+15551234567", "from": "+15559876543", "body": "Call recording started."}
    }));
    result.add_action(json!({"update_global_data": {"recording_active": true}}));
    println!("{}", result.to_json());
}
