// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Auto-Vivified SWML Service — verb methods called directly on the service.

use signalwire::swml::service::{Service, ServiceOptions};
use serde_json::json;

fn main() {
    let mut service = Service::new(ServiceOptions {
        name: "voicemail".to_string(),
        route: Some("/voicemail".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(3000),
        basic_auth_user: None,
        basic_auth_password: None,
    });

    // Build the SWML document using verb helper methods
    service.reset_document();
    service.add_answer_verb();

    // Play greeting
    service.add_verb("play", json!({
        "url": "say:Hello! You've reached our voicemail. Please leave a message after the beep."
    }));

    // Pause
    service.add_verb("sleep", json!(1000));

    // Beep
    service.add_verb("play", json!({
        "url": "https://example.com/beep.wav"
    }));

    // Record
    service.add_verb("record", json!({
        "stereo": true,
        "format": "wav",
        "direction": "speak",
        "terminators": "#",
        "beep": false,
        "max_length": 120,
        "end_silence_timeout": 3.0
    }));

    // Thank and hang up
    service.add_verb("play", json!({
        "url": "say:Thank you. Goodbye!"
    }));
    service.add_hangup_verb();

    // Dump the document
    let doc = service.render();
    println!("Generated SWML:");
    println!("{}", serde_json::to_string_pretty(&doc).unwrap());

    // Serve
    println!("\nStarting voicemail service at http://localhost:3000/voicemail");
    service.run();
}
