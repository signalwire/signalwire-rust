// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Call Flow and Actions Demo — call-flow verbs, debug events, FunctionResult actions.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use serde_json::json;
use std::sync::Arc;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "call-flow-demo".to_string(),
        route: Some("/call-flow-demo".to_string()),
        ..AgentOptions::new("call-flow-demo")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Role",
        "You are a call center demo agent that showcases call-flow features.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Use transfer_to_support when the caller asks to speak to a person",
        "Use send_confirmation to send the caller an SMS",
        "Use start_recording when the caller agrees to be recorded",
        "Use play_hold_music to play background music",
        "Use adjust_speech when the caller mentions unusual names or terms",
    ]);

    // -- Call flow verbs --
    agent.add_pre_answer_verb("play", json!({"url": "say:Please hold while we connect you."}));
    agent.add_post_answer_verb("record", json!({"stereo": true}));
    agent.add_post_ai_verb("hangup", json!({}));

    // -- Debug events --
    agent.enable_debug_events("all");
    agent.on_debug_event(Box::new(|event, _headers| {
        println!("Debug event: {}", event);
    }));

    // -- Tools demonstrating FunctionResult actions --
    agent.define_tool(
        "transfer_to_support",
        "Transfer the call to a support agent",
        json!({}),
        Box::new(|_args, _raw| {
            let mut result = FunctionResult::with_response(
                "I'll transfer you to our support team now."
            );
            result.set_post_process(true);
            result.add_action(json!({
                "SWML": {
                    "sections": {"main": [{"connect": {"to": "+15551234567"}}]},
                    "version": "1.0.0"
                }
            }));
            result
        }),
        false,
    );

    agent.define_tool(
        "send_confirmation",
        "Send an SMS confirmation to the caller",
        json!({
            "phone": {"type": "string", "description": "Caller's phone number"},
            "details": {"type": "string", "description": "Confirmation details"}
        }),
        Box::new(|args, _raw| {
            let phone = args.get("phone").and_then(|v| v.as_str()).unwrap_or("");
            let details = args.get("details").and_then(|v| v.as_str()).unwrap_or("");
            let mut result = FunctionResult::with_response("Confirmation SMS sent.");
            result.add_action(json!({
                "send_sms": {
                    "to": phone,
                    "from": "+15559876543",
                    "body": format!("Confirmation: {details}")
                }
            }));
            result
        }),
        false,
    );

    agent.define_tool(
        "start_recording",
        "Start recording the call",
        json!({}),
        Box::new(|_args, _raw| {
            let mut result = FunctionResult::with_response("Recording has started.");
            result.add_action(json!({"record_call": {"stereo": true, "format": "wav"}}));
            result
        }),
        false,
    );

    agent.define_tool(
        "play_hold_music",
        "Play background hold music",
        json!({}),
        Box::new(|_args, _raw| {
            let mut result = FunctionResult::with_response("Playing hold music.");
            result.add_action(json!({
                "play_background": {"url": "https://cdn.signalwire.com/default-music/hold.mp3"}
            }));
            result
        }),
        false,
    );

    agent.define_tool(
        "adjust_speech",
        "Add speech hints for unusual terms",
        json!({
            "terms": {"type": "string", "description": "Comma-separated terms to add as hints"}
        }),
        Box::new(|args, _raw| {
            let terms = args.get("terms").and_then(|v| v.as_str()).unwrap_or("");
            let hints: Vec<&str> = terms.split(',').map(|s| s.trim()).collect();
            let mut result = FunctionResult::with_response("Speech hints updated.");
            result.add_action(json!({"add_dynamic_hints": hints}));
            result
        }),
        false,
    );

    agent.run();
}
