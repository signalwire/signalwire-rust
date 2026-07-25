// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Auto-Vivified SWML Service — verb methods called directly on the service.

use serde_json::json;
use signalwire::swml::service::{Service, ServiceOptions};

fn main() {
    let mut service = Service::new(
        ServiceOptions::new("voicemail")
            .route("/voicemail")
            .host("0.0.0.0")
            .port(3000),
    );

    // Build the SWML document using verb helper methods
    service.document_mut().reset();
    service.add_verb_to_section("main", "answer", json!({}));

    // Play greeting
    service.add_verb_to_section(
        "main",
        "play",
        json!({
            "url": "say:Hello! You've reached our voicemail. Please leave a message after the beep."
        }),
    );

    // Pause
    service.sleep(1000, "main");

    // Beep
    service.add_verb_to_section(
        "main",
        "play",
        json!({
            "url": "https://example.com/beep.wav"
        }),
    );

    // Record
    service.add_verb_to_section(
        "main",
        "record",
        json!({
            "stereo": true,
            "format": "wav",
            "direction": "speak",
            "terminators": "#",
            "beep": false,
            "max_length": 120,
            "end_silence_timeout": 3.0
        }),
    );

    // Thank and hang up
    service.add_verb_to_section(
        "main",
        "play",
        json!({
            "url": "say:Thank you. Goodbye!"
        }),
    );
    service.add_verb_to_section("main", "hangup", json!({}));

    // Dump the document
    let doc = service.render_pretty();
    println!("Generated SWML:");
    println!("{doc}");

    // Serve
    println!("\nStarting voicemail service at http://localhost:3000/voicemail");
    service.run();
}
