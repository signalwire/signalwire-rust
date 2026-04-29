// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Concierge Agent — virtual concierge for venues.

use signalwire::prefabs::ConciergeAgent;
use serde_json::{json, Map};

fn main() {
    let venue_name = "Oceanview Resort";

    let mut venue_info: Map<String, serde_json::Value> = Map::new();
    venue_info.insert("venue_name".to_string(), json!(venue_name));
    venue_info.insert("services".to_string(), json!([
        "room service",
        "spa bookings",
        "restaurant reservations",
        "activity bookings",
        "airport shuttle",
        "valet parking",
        "concierge assistance",
    ]));
    venue_info.insert("amenities".to_string(), json!({
        "Infinity Pool": {"hours": "6 AM - 10 PM", "location": "Level 3"},
        "Spa & Wellness": {"hours": "8 AM - 8 PM", "location": "Level 2"},
        "Fitness Center": {"hours": "24 hours", "location": "Level 1"},
        "Beach Bar": {"hours": "11 AM - Midnight", "location": "Beachfront"},
        "Fine Dining": {"hours": "6 PM - 10 PM", "location": "Level 5, reservations required"},
    }));

    let mut agent = ConciergeAgent::new(
        "concierge",
        &venue_info,
        Some("/concierge"),
    );

    agent.agent_mut().add_language("English", "en-US", "inworld.Sarah");

    let greeting = format!("Welcome guests to {venue_name} with warmth");
    agent.agent_mut().prompt_add_section("Greeting", "", vec![
        &greeting,
        "Offer to help with any of the available services",
        "Provide detailed information about amenities when asked",
    ]);

    println!("Concierge agent for {venue_name}");
    println!("  URL: http://localhost:3000/concierge");
    agent.agent().run();
}
