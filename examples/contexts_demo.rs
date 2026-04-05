// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Advanced Contexts Demo — multi-persona workflow with context switching.
//!
//! Three personas:
//! - Franklin (greeter) — welcomes the caller
//! - Rachael (sales) — handles product inquiries
//! - Dwight (support) — handles technical issues

use signalwire::agent::{AgentBase, AgentOptions};

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "Advanced Computer Sales Agent".to_string(),
        route: Some("/advanced-contexts-demo".to_string()),
        ..AgentOptions::new("contexts")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    // Top-level prompt (available in non-isolated contexts)
    agent.prompt_add_section(
        "Company",
        "You work for TechCo, a premium computer retailer.",
        vec![],
    );

    // -- Define contexts --
    let ctx_builder = agent.define_contexts();

    // Franklin — greeter
    {
        let greeter = ctx_builder.add_context("greeter");
        greeter.set_isolated(true);
        greeter.add_section(
            "Persona",
            "You are Franklin, a friendly greeter. Welcome callers and determine their needs.",
            vec![],
        );
        greeter.add_enter_filler("Hey there! Welcome to TechCo. I'm Franklin!");

        let step = greeter.add_step("welcome");
        step.set_text("Welcome the caller. Ask if they need sales or support.");
        step.set_valid_contexts(vec!["sales", "support"]);
    }

    // Rachael — sales
    {
        let sales = ctx_builder.add_context("sales");
        sales.set_isolated(true);
        sales.add_section(
            "Persona",
            "You are Rachael, a knowledgeable sales specialist.",
            vec![],
        );
        sales.add_enter_filler("Hi! I'm Rachael from sales. Let me help you find the perfect system.");

        let step1 = sales.add_step("needs");
        step1.set_text("Ask about the customer's computing needs: use case, budget, preferences.");
        step1.set_valid_steps(vec!["recommendation"]);

        let step2 = sales.add_step("recommendation");
        step2.set_text(
            "Based on their needs, recommend a system configuration and price. \
             Offer to transfer to support if they have technical questions.",
        );
        step2.set_step_criteria("Customer has described their needs and budget.");
        step2.set_valid_contexts(vec!["support"]);
    }

    // Dwight — support
    {
        let support = ctx_builder.add_context("support");
        support.set_isolated(true);
        support.add_section(
            "Persona",
            "You are Dwight, a no-nonsense technical support specialist.",
            vec![],
        );
        support.add_enter_filler("Dwight here. Let's get your issue sorted out.");

        let step1 = support.add_step("diagnose");
        step1.set_text("Ask about the technical issue. Gather symptoms and error messages.");
        step1.set_valid_steps(vec!["resolution"]);

        let step2 = support.add_step("resolution");
        step2.set_text("Provide a solution or escalation path. Offer to transfer back to sales.");
        step2.set_step_criteria("Issue has been described with enough detail.");
        step2.set_valid_contexts(vec!["sales"]);
    }

    println!("Contexts demo running at http://localhost:3000/advanced-contexts-demo");
    agent.run();
}
