// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Room and SIP Example — join_room and sip_refer virtual helpers.

use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    // Basic room join
    println!("=== Basic Room Join ===");
    let mut result = FunctionResult::with_response("Joining the support team room.");
    result.add_action(json!({"join_room": {"name": "support_team_room"}}));
    println!("{}", result.to_json());

    // Conference room with metadata
    println!("\n=== Conference Room ===");
    let mut result = FunctionResult::with_response("Setting up daily standup meeting.");
    result.add_action(json!({"join_room": {"name": "daily_standup_room"}}));
    result.add_action(json!({
        "set_metadata": {
            "meeting_type": "daily_standup",
            "participant_id": "user_123",
            "role": "scrum_master"
        }
    }));
    result.add_action(json!({
        "update_global_data": {"meeting_active": true, "room_name": "daily_standup_room"}
    }));
    println!("{}", result.to_json());

    // Basic SIP REFER
    println!("\n=== Basic SIP REFER ===");
    let mut result = FunctionResult::with_response("Transferring your call to support.");
    result.add_action(json!({"sip_refer": {"to": "sip:support@company.com"}}));
    println!("{}", result.to_json());

    // Advanced SIP REFER with metadata
    println!("\n=== Advanced SIP REFER ===");
    let mut result = FunctionResult::with_response("Transferring to technical support.");
    result.add_action(json!({
        "set_metadata": {
            "transfer_type": "technical_support",
            "priority": "high",
            "original_caller": "+15551234567"
        }
    }));
    result.add_action(json!({
        "sip_refer": {"to": "sip:senior-tech@company.com;transport=tls"}
    }));
    println!("{}", result.to_json());

    // Escalation chain: try room, then SIP
    println!("\n=== Escalation Chain ===");
    let mut result = FunctionResult::with_response("Escalating through available channels.");
    result.add_action(json!({"join_room": {"name": "escalation_room"}}));
    result.add_action(json!({
        "update_global_data": {"escalation_level": "tier2", "escalated_at": "2024-01-15T10:30:00Z"}
    }));
    println!("{}", result.to_json());
}
