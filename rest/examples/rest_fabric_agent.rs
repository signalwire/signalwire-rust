// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Create and manage Fabric AI agents via the REST API.
//!
//! The REST client is synchronous. `create`/`update` take a
//! `&serde_json::Value` body; `list` takes a `&HashMap<String, String>`.
//! Every method returns `Result<Value, SignalWireRestError>`.
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use serde_json::json;
use signalwire::rest::RestClient;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create an AI agent.
    println!("Creating AI agent ...");
    let agent = client.fabric().ai_agents().create(&json!({
        "name": "Demo Support Bot",
        "prompt": {
            "text": "You are a helpful support agent for ACME Corporation."
        },
        "languages": [{
            "name": "English",
            "code": "en-US",
            "voice": "inworld.Mark"
        }],
        "params": {
            "end_of_speech_timeout": 500,
            "attention_timeout": 15000
        }
    }), None)?;

    let agent_id = agent["id"].as_str().unwrap_or("unknown").to_string();
    println!("Agent created: {agent_id}");
    println!("  Name: {}", agent["name"]);

    // List all agents.
    let agents = client.fabric().ai_agents().list(&HashMap::new(), None)?;
    if let Some(arr) = agents.as_array() {
        println!("\nAll AI agents ({}):", arr.len());
        for a in arr {
            println!("  {} - {}", a["id"], a["name"]);
        }
    }

    // Update the agent.
    println!("\nUpdating agent prompt ...");
    client.fabric().ai_agents().update(
        &agent_id,
        &json!({
            "prompt": {
                "text": "You are a senior support agent. Be thorough and precise."
            }
        }), None
    )?;
    println!("Agent updated.");

    // Get the updated agent.
    let updated = client.fabric().ai_agents().get(&agent_id, None)?;
    println!("Updated prompt: {}", updated["prompt"]["text"]);

    Ok(())
}
