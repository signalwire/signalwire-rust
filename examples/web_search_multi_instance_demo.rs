// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Web Search Multi-Instance — multiple search agents with different configurations.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::server::AgentServer;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn news_search_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "news-search".to_string(),
        route: Some("/news-search".to_string()),
        ..AgentOptions::new("news-search")
    });
    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a news search assistant focused on current events.",
        vec![],
    );
    agent.define_tool(
        "search_news",
        "Search for current news articles",
        json!({"topic": {"type": "string", "description": "News topic"}}),
        Box::new(|args, _raw| {
            let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("general");
            FunctionResult::with_response(&format!("Latest news on {topic}: [simulated results]"))
        }),
        false,
    );
    agent
}

fn tech_search_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "tech-search".to_string(),
        route: Some("/tech-search".to_string()),
        ..AgentOptions::new("tech-search")
    });
    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a technical documentation search assistant.",
        vec![],
    );
    agent.define_tool(
        "search_docs",
        "Search technical documentation",
        json!({"query": {"type": "string", "description": "Technical query"}}),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            FunctionResult::with_response(&format!("Documentation results for '{query}': [simulated]"))
        }),
        false,
    );
    agent
}

fn main() {
    let mut server = AgentServer::new(Some("0.0.0.0"), Some(3000));
    server.register(news_search_agent(), None).unwrap();
    server.register(tech_search_agent(), None).unwrap();

    println!("Web search multi-instance:");
    println!("  News: http://localhost:3000/news-search");
    println!("  Tech: http://localhost:3000/tech-search");
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
