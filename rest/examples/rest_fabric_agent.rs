// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Create and manage Fabric AI agents via the REST API.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create an AI agent
    println!("Creating AI agent ...");
    let agent = client.fabric().ai_agents().create(serde_json::json!({
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
    })).await?;

    let agent_id = agent["id"].as_str().unwrap_or("unknown");
    println!("Agent created: {agent_id}");
    println!("  Name: {}", agent["name"]);

    // List all agents
    let agents = client.fabric().ai_agents().list(&[]).await?;
    if let Some(arr) = agents.as_array() {
        println!("\nAll AI agents ({}):", arr.len());
        for a in arr {
            println!("  {} - {}", a["id"], a["name"]);
        }
    }

    // Update the agent
    println!("\nUpdating agent prompt ...");
    client.fabric().ai_agents().update(agent_id, serde_json::json!({
        "prompt": {
            "text": "You are a senior support agent. Be thorough and precise."
        }
    })).await?;
    println!("Agent updated.");

    // Get the updated agent
    let updated = client.fabric().ai_agents().get(agent_id).await?;
    println!("Updated prompt: {}", updated["prompt"]["text"]);

    Ok(())
}
