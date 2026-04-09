// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Step Function Inheritance Demo
//!
//! This example exists to teach one specific gotcha: the per-step
//! `functions` whitelist INHERITS from the previous step when omitted.
//!
//! # Why this matters
//!
//! A common mistake when building multi-step agents is to assume each
//! step starts with a fresh tool set. It does not. The runtime only
//! resets the active set when a step explicitly declares its `functions`
//! field. If you forget `set_functions()` on a later step, the previous
//! step's tools quietly remain available.
//!
//! This file shows four step-shaped patterns side by side:
//!
//!   1. `step_lookup`   — explicitly whitelists `lookup_account`
//!   2. `step_inherit`  — has NO `set_functions()` call. Inherits
//!                        step_lookup's whitelist, so `lookup_account`
//!                        is still callable here. This is rarely what
//!                        you want.
//!   3. `step_explicit` — explicitly whitelists `process_payment`. The
//!                        previously inherited `lookup_account` is now
//!                        disabled, and only `process_payment` is
//!                        active.
//!   4. `step_disabled` — explicitly disables ALL user functions with
//!                        an empty array (or `"none"`). Internal tools
//!                        like next_step still work.
//!
//! # Best practice
//!
//! Call `set_functions()` on EVERY step that should differ from the
//! previous one. Treat omission as an explicit decision to inherit, not
//! a default.
//!
//! Run this file to see the rendered SWML — there are no real webhook
//! endpoints behind the tools, this is purely a documentation example.

use std::collections::HashMap;

use serde_json::json;
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "step_function_inheritance_demo".to_string(),
        route: Some("/".to_string()),
        ..AgentOptions::new("step_function_inheritance_demo")
    });

    // Register three SWAIG tools so we have something to whitelist.
    // In a real agent these would call out to webhooks; here they're
    // stubs.
    agent.define_tool(
        "lookup_account",
        "Look up customer account details by account number",
        json!({"account_number": {"type": "string"}}),
        Box::new(|_args, _raw| FunctionResult::with_response("looked up")),
        false,
    );
    agent.define_tool(
        "process_payment",
        "Process a payment for the current customer",
        json!({"amount": {"type": "number"}}),
        Box::new(|_args, _raw| FunctionResult::with_response("payment processed")),
        false,
    );
    agent.define_tool(
        "send_receipt",
        "Email a receipt to the customer",
        json!({"email": {"type": "string"}}),
        Box::new(|_args, _raw| FunctionResult::with_response("sent")),
        false,
    );

    // Build the contexts.
    let ctx_builder = agent.define_contexts();
    let ctx = ctx_builder.add_context("default");

    // -- Step 1: explicit whitelist --
    // `lookup_account` is the only tool active in this step.
    let step_lookup = ctx.add_step("step_lookup");
    step_lookup.set_text(
        "Greet the customer and ask for their account number. \
         Use lookup_account to fetch their details.",
    );
    step_lookup.set_functions(json!(["lookup_account"]));
    step_lookup.set_valid_steps(vec!["step_inherit"]);

    // -- Step 2: NO set_functions() call → inheritance --
    // Because we didn't call set_functions(), this step inherits the
    // active set from step_lookup. `lookup_account` is STILL callable
    // here, even though we never asked for it. Most of the time this
    // is a bug. To break the inheritance, call set_functions() with an
    // explicit list (even if it's empty).
    let step_inherit = ctx.add_step("step_inherit");
    step_inherit.set_text(
        "Confirm the customer's identity. (No set_functions() here, \
         so lookup_account is still active — this is the inheritance \
         trap.)",
    );
    step_inherit.set_valid_steps(vec!["step_explicit"]);

    // -- Step 3: explicit replacement --
    // Whitelist replaces the inherited set. lookup_account is now
    // inactive; only process_payment is active.
    let step_explicit = ctx.add_step("step_explicit");
    step_explicit.set_text(
        "Take the customer's payment. Use process_payment. \
         lookup_account is no longer available.",
    );
    step_explicit.set_functions(json!(["process_payment"]));
    step_explicit.set_valid_steps(vec!["step_disabled"]);

    // -- Step 4: explicit disable-all --
    // Pass an empty array (or "none") to lock out every user-defined
    // tool. Internal navigation tools (next_step) are unaffected.
    let step_disabled = ctx.add_step("step_disabled");
    step_disabled.set_text(
        "Thank the customer and wrap up. No tools are needed here, so \
         we lock everything down with set_functions(json!([])).",
    );
    step_disabled.set_functions(json!([]));
    step_disabled.set_end(true);

    // Render the SWML document so you can see exactly which steps have
    // a `functions` key in the output and which don't.
    let swml = agent.render_swml(&HashMap::new());
    println!("{}", serde_json::to_string_pretty(&swml).unwrap());
}
