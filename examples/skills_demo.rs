// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Skills Demo — loading built-in skills (datetime, math).

use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "skills-demo".to_string(),
        route: Some("/skills-demo".to_string()),
        ..AgentOptions::new("skills-demo")
    });

    agent.add_language("English", "en-US", "rime.spore");

    agent.prompt_add_section(
        "Role",
        "You are a helpful assistant with date/time and math skills.",
        vec![],
    );
    agent.prompt_add_section("Instructions", "", vec![
        "Use get_current_time when asked about the time",
        "Use get_current_date when asked about today's date",
        "Use calculate when asked to do math",
    ]);

    // Add built-in skills
    agent.add_skill("datetime", json!({"timezone": "America/Chicago"}));
    agent.add_skill("math", json!({}));

    let (user, pass) = agent.get_basic_auth_credentials();
    println!("Skills demo running");
    println!("  URL: http://localhost:3000/skills-demo");
    println!("  Auth: {user}:{pass}");
    agent.run();
}
