// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Concierge Agent — virtual concierge for venues.

use signalwire::prefabs::ConciergeAgent;
use serde_json::json;

fn main() {
    let venue_name = "Oceanview Resort";

    let services = vec![
        "room service",
        "spa bookings",
        "restaurant reservations",
        "activity bookings",
        "airport shuttle",
        "valet parking",
        "concierge assistance",
    ];

    let amenities = vec![
        json!({"name": "Infinity Pool", "hours": "6 AM - 10 PM", "location": "Level 3"}),
        json!({"name": "Spa & Wellness", "hours": "8 AM - 8 PM", "location": "Level 2"}),
        json!({"name": "Fitness Center", "hours": "24 hours", "location": "Level 1"}),
        json!({"name": "Beach Bar", "hours": "11 AM - Midnight", "location": "Beachfront"}),
        json!({"name": "Fine Dining", "hours": "6 PM - 10 PM", "location": "Level 5, reservations required"}),
    ];

    let mut agent = ConciergeAgent::new(
        venue_name,
        "/concierge",
        services,
        amenities,
    );

    agent.add_language("English", "en-US", "inworld.Sarah");

    agent.prompt_add_section("Greeting", "", vec![
        &format!("Welcome guests to {venue_name} with warmth"),
        "Offer to help with any of the available services",
        "Provide detailed information about amenities when asked",
    ]);

    println!("Concierge agent for {venue_name}");
    println!("  URL: http://localhost:3000/concierge");
    agent.run();
}
