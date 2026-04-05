// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Survey Agent — conduct a survey using the SurveyAgent prefab.

use signalwire::prefabs::SurveyAgent;
use serde_json::json;

fn main() {
    let mut agent = SurveyAgent::new(
        "customer-survey",
        "/survey",
        vec![
            json!({
                "key_name": "satisfaction",
                "question_text": "On a scale of 1 to 10, how satisfied are you with our service?",
                "confirm": true
            }),
            json!({
                "key_name": "recommendation",
                "question_text": "How likely are you to recommend us to a friend? 1 to 10.",
                "confirm": true
            }),
            json!({
                "key_name": "improvement",
                "question_text": "What is one thing we could improve?"
            }),
            json!({
                "key_name": "additional_comments",
                "question_text": "Do you have any other comments or feedback?"
            }),
        ],
    );

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section(
        "Introduction",
        "Thank you for taking the time to complete our customer satisfaction survey.",
        vec![],
    );

    agent.set_post_prompt(
        "Summarize survey responses: satisfaction score, NPS score, \
         improvement suggestion, and additional comments.",
    );

    println!("Survey agent at http://localhost:3000/survey");
    agent.run();
}
