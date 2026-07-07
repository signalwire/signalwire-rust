// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! README quickstart — AI Agents.
//!
//! The `agent` region below is the exact code shown in the "AI Agents" section
//! of the crate README. It is included there via a `<!-- include: -->` marker
//! and asserted byte-identical at gate time, so the README code can never rot.

// region: agent
use serde_json::json;
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;

fn main() {
    let mut agent = AgentBase::new(AgentOptions::new("my-agent"));

    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section("Role", "You are a helpful assistant.", vec![]);

    agent.define_tool(
        "get_time",
        "Get the current time",
        json!({}),
        Box::new(|_args, _raw| {
            let now = chrono::Local::now().format("%H:%M:%S");
            FunctionResult::with_response(&format!("The time is {now}"))
        }),
        false,
    );

    agent.run();
}
// endregion: agent
