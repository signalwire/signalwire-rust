// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! LLM Parameters Demo — different agent personalities via LLM tuning.
//!
//! Hosts two agents:
//! - /precise — low temperature, consistent, technical
//! - /creative — high temperature, varied, imaginative

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::server::AgentServer;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn precise_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "precise-assistant".to_string(),
        route: Some("/precise".to_string()),
        ..AgentOptions::new("precise")
    });

    agent.add_language("English", "en-US", "inworld.Mark");

    agent.prompt_add_section("Role", "You are a precise technical assistant.", vec![]);
    agent.prompt_add_section("Instructions", "", vec![
        "Provide accurate, factual information",
        "Be concise and direct",
        "Avoid speculation or guessing",
    ]);

    agent.set_prompt_llm_params(json!({
        "temperature": 0.2,
        "top_p": 0.85,
        "barge_confidence": 0.8,
        "presence_penalty": 0.0,
        "frequency_penalty": 0.1,
    }));

    agent.set_post_prompt("Provide a brief technical summary of the key points discussed.");
    agent.set_post_prompt_llm_params(json!({"temperature": 0.1}));

    agent.define_tool(
        "get_system_info",
        "Get technical system information",
        json!({}),
        Box::new(|_args, _raw| {
            FunctionResult::with_response(
                "System Status: CPU 45%, Memory 8GB free, Disk 250GB free, Uptime 14 days."
            )
        }),
        false,
    );

    agent
}

fn creative_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "creative-assistant".to_string(),
        route: Some("/creative".to_string()),
        ..AgentOptions::new("creative")
    });

    agent.add_language("English", "en-US", "inworld.Sarah");

    agent.prompt_add_section("Role", "You are a creative writing assistant.", vec![]);
    agent.prompt_add_section("Instructions", "", vec![
        "Be imaginative and creative",
        "Use varied vocabulary and expressions",
        "Encourage creative thinking",
    ]);

    agent.set_prompt_llm_params(json!({
        "temperature": 0.8,
        "top_p": 0.95,
        "barge_confidence": 0.3,
        "presence_penalty": 0.4,
        "frequency_penalty": 0.3,
    }));

    agent.define_tool(
        "generate_story_idea",
        "Generate a creative story premise",
        json!({"genre": {"type": "string", "description": "Story genre"}}),
        Box::new(|args, _raw| {
            let genre = args.get("genre").and_then(|v| v.as_str()).unwrap_or("fantasy");
            FunctionResult::with_response(&format!(
                "Here's a {genre} premise: A lighthouse keeper discovers their light \
                 doesn't guide ships — it guides something else entirely."
            ))
        }),
        false,
    );

    agent
}

fn main() {
    let mut server = AgentServer::new(Some("0.0.0.0"), Some(3000));
    server.register(precise_agent(), None).unwrap();
    server.register(creative_agent(), None).unwrap();

    println!("LLM Params Demo:");
    println!("  Precise: http://localhost:3000/precise");
    println!("  Creative: http://localhost:3000/creative");
    run_server(server);
}

fn run_server(server: AgentServer) {
    use std::collections::HashMap;
    use std::io::Read as _;

    let addr = format!("{}:{}", server.host(), server.port());
    let http = tiny_http::Server::http(&addr)
        .unwrap_or_else(|e| panic!("Failed to bind {}: {}", addr, e));

    for mut request in http.incoming_requests() {
        let method = request.method().as_str().to_string();
        let path = request.url().to_string();

        let mut req_headers = HashMap::new();
        for h in request.headers() {
            req_headers.insert(
                h.field.as_str().as_str().to_string(),
                h.value.as_str().to_string(),
            );
        }

        let mut body_buf = String::new();
        let _ = request.as_reader().read_to_string(&mut body_buf);

        let (status, resp_headers, resp_body) =
            server.handle_request(&method, &path, &req_headers, &body_buf);

        let mut response =
            tiny_http::Response::from_string(&resp_body).with_status_code(status);
        for (k, v) in &resp_headers {
            if let Ok(header) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                response = response.with_header(header);
            }
        }
        let _ = request.respond(response);
    }
}
