// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Gather Info Demo — structured data collection using low-level contexts API.
//!
//! Uses set_gather_info() and add_gather_question() on steps to collect
//! patient intake information one question at a time.

use signalwire::agent::{AgentBase, AgentOptions};

fn main() {
    let mut agent = AgentBase::new(AgentOptions {
        name: "Patient Intake Agent".to_string(),
        route: Some("/patient-intake".to_string()),
        ..AgentOptions::new("patient-intake")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Role",
        "You are a friendly medical office intake assistant. \
         Collect patient information accurately and professionally.",
        vec![],
    );

    let ctx_builder = agent.define_contexts();
    let ctx = ctx_builder.add_context("default");

    // Step 1: Demographics
    let step1 = ctx.add_step("demographics");
    step1.set_text("Collect the patient's basic information.");
    step1.set_gather_info(
        Some("patient_demographics"),
        None,
        Some("Please collect the following patient information."),
    );
    step1.add_gather_question("full_name", "What is your full name?", "string", false, None, None);
    step1.add_gather_question("date_of_birth", "What is your date of birth?", "string", false, None, None);
    step1.add_gather_question("phone_number", "What is your phone number?", "string", true, None, None);
    step1.add_gather_question("email", "What is your email address?", "string", false, None, None);
    step1.set_valid_steps(vec!["symptoms"]);

    // Step 2: Symptoms
    let step2 = ctx.add_step("symptoms");
    step2.set_text("Ask about the patient's current symptoms and reason for visit.");
    step2.set_gather_info(
        Some("patient_symptoms"),
        None,
        Some("Now let's talk about why you're visiting today."),
    );
    step2.add_gather_question(
        "reason_for_visit",
        "What is the main reason for your visit today?",
        "string",
        false,
        None,
        None,
    );
    step2.add_gather_question(
        "symptom_duration",
        "How long have you been experiencing these symptoms?",
        "string",
        false,
        None,
        None,
    );
    step2.add_gather_question(
        "pain_level",
        "On a scale of 1 to 10, how would you rate your discomfort?",
        "string",
        false,
        None,
        None,
    );
    step2.set_valid_steps(vec!["confirmation"]);

    // Step 3: Confirmation
    let step3 = ctx.add_step("confirmation");
    step3.set_text(
        "Summarize all the information collected and confirm with the patient \
         that everything is correct. Thank them for their time.",
    );
    step3.set_step_criteria("Patient has confirmed all information is correct.");

    agent.run();
}
