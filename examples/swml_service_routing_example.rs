// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! SWML Service Routing — route calls based on caller ID or time of day.

use signalwire::swml::service::{Service, ServiceOptions};
use serde_json::json;

fn business_hours_document() -> serde_json::Value {
    json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": "say:Thank you for calling during business hours. How can we help you?"}},
                {"ai": {
                    "prompt": {"text": "You are a helpful assistant. Our business hours are 9 AM to 5 PM."},
                    "languages": [{"name": "English", "code": "en-US", "voice": "inworld.Mark"}]
                }}
            ]
        }
    })
}

fn after_hours_document() -> serde_json::Value {
    json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": "say:We are currently closed. Our business hours are 9 AM to 5 PM, Monday through Friday."}},
                {"play": {"url": "say:Please leave a message after the beep and we will return your call on the next business day."}},
                {"record": {"beep": true, "terminators": "#", "max_length": 120}},
                {"hangup": {}}
            ]
        }
    })
}

fn vip_document() -> serde_json::Value {
    json!({
        "version": "1.0.0",
        "sections": {
            "main": [
                {"answer": {}},
                {"play": {"url": "say:Welcome back, VIP customer. Connecting you to your account manager now."}},
                {"connect": {"to": "+15551234567"}},
                {"hangup": {}}
            ]
        }
    })
}

fn main() {
    println!("=== Business Hours SWML ===");
    println!("{}", serde_json::to_string_pretty(&business_hours_document()).unwrap());

    println!("\n=== After Hours SWML ===");
    println!("{}", serde_json::to_string_pretty(&after_hours_document()).unwrap());

    println!("\n=== VIP SWML ===");
    println!("{}", serde_json::to_string_pretty(&vip_document()).unwrap());

    // In production, on_request() would check caller ID and time to choose the right document.
    let service = Service::new(ServiceOptions {
        name: "routing".to_string(),
        route: Some("/routing".to_string()),
        host: Some("0.0.0.0".to_string()),
        port: Some(3000),
        basic_auth_user: None,
        basic_auth_password: None,
    });

    println!("\nRouting service at http://localhost:3000/routing");
    service.run();
}
