// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Dynamic SWML Service — generates different SWML based on POST data.

use signalwire::swml::service::{Service, ServiceOptions};
use serde_json::json;

fn main() {
    let service = Service::new(ServiceOptions {
        name: "dynamic-greeting".to_string(),
        route: Some("/greeting".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(3000),
        basic_auth_user: None,
        basic_auth_password: None,
    });

    // Default SWML
    let default_doc = json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": "say:Hello, thank you for calling our service."}},
                {"prompt": {
                    "play": "say:Please press 1 for sales, 2 for support, or 3 to leave a message.",
                    "max_digits": 1,
                    "terminators": "#"
                }},
                {"hangup": {}}
            ]
        }
    });

    println!("Default SWML document:");
    println!("{}", serde_json::to_string_pretty(&default_doc).unwrap());

    // VIP SWML (would be returned when request includes vip=true)
    let vip_doc = json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": "say:Welcome back, valued customer! Let me connect you to your dedicated account manager."}},
                {"connect": {"to": "+15551234567"}},
                {"hangup": {}}
            ]
        }
    });

    println!("\nVIP SWML document:");
    println!("{}", serde_json::to_string_pretty(&vip_doc).unwrap());

    // In a real implementation, on_request() would choose between these
    // based on the POST body or query parameters.
    println!("\nStarting dynamic greeting service at http://localhost:3000/greeting");
    service.run();
}
