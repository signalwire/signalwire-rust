// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! SWML Service Example — full SWML service patterns.

use signalwire::swml::service::{Service, ServiceOptions};
use serde_json::json;

fn main() {
    let mut service = Service::new(ServiceOptions {
        name: "swml-demo".to_string(),
        route: Some("/swml-demo".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(3000),
        basic_auth_user: None,
        basic_auth_password: None,
    });

    // Build a complex SWML document
    service.document_mut().reset();
    service.add_verb("answer", "main", json!({}));

    // Welcome message
    service.add_verb("play", "main", json!({
        "url": "say:Welcome to the SWML service demo."
    }));

    // Collect DTMF input
    service.add_verb("prompt", "main", json!({
        "play": "say:Press 1 to hear music. Press 2 to record a message. Press 3 to be transferred.",
        "max_digits": 1,
        "terminators": "#",
        "digit_timeout": 5.0
    }));

    // Set a variable
    service.add_verb("set", "main", json!({
        "call_status": "active",
        "menu_selection": "pending"
    }));

    // Conditional routing would happen on the platform side via switch verb
    service.add_verb("play", "main", json!({
        "url": "say:Thank you for using the SWML service demo."
    }));

    service.add_verb("hangup", "main", json!({}));

    // Render and display
    let doc = service.render_pretty();
    println!("SWML document:");
    println!("{}", doc);

    println!("\nServing at http://localhost:3000/swml-demo");
    service.run();
}
