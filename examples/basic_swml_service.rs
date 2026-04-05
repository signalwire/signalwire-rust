// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Basic SWML Service — non-AI SWML flows (voicemail, IVR, call transfer).

use signalwire::swml::service::{Service, ServiceOptions};
use serde_json::json;

fn voicemail_document() -> serde_json::Value {
    json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": "say:Hello, you've reached the voicemail service. Please leave a message after the beep."}},
                {"sleep": 1000},
                {"play": {"url": "https://example.com/beep.wav"}},
                {"record": {
                    "stereo": true,
                    "format": "wav",
                    "direction": "speak",
                    "terminators": "#",
                    "beep": true,
                    "max_length": 120,
                    "end_silence_timeout": 3.0
                }},
                {"play": {"url": "say:Thank you for your message. Goodbye!"}},
                {"hangup": {}}
            ]
        }
    })
}

fn ivr_document() -> serde_json::Value {
    json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"prompt": {
                    "play": "say:Welcome! Press 1 for sales, 2 for support, or 3 to leave a message.",
                    "max_digits": 1,
                    "terminators": "#"
                }},
                {"hangup": {}}
            ]
        }
    })
}

fn transfer_document() -> serde_json::Value {
    json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": "say:Connecting you now. Please hold."}},
                {"connect": {
                    "to": "+15551234567",
                    "from": "+15559876543"
                }},
                {"hangup": {}}
            ]
        }
    })
}

fn main() {
    println!("=== Voicemail SWML ===");
    println!("{}", serde_json::to_string_pretty(&voicemail_document()).unwrap());

    println!("\n=== IVR SWML ===");
    println!("{}", serde_json::to_string_pretty(&ivr_document()).unwrap());

    println!("\n=== Transfer SWML ===");
    println!("{}", serde_json::to_string_pretty(&transfer_document()).unwrap());

    // Serve the voicemail document
    let service = Service::new(ServiceOptions {
        name: "voicemail".to_string(),
        route: Some("/voicemail".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(3000),
        basic_auth_user: None,
        basic_auth_password: None,
    });

    println!("\nStarting voicemail service at http://localhost:3000/voicemail");
    service.run();
}
