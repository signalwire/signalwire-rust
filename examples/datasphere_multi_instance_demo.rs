// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Datasphere Multi-Instance — multiple Datasphere document collections.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::server::AgentServer;
use signalwire::swaig::FunctionResult;
use serde_json::json;

fn product_kb_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "product-kb".to_string(),
        route: Some("/product-kb".to_string()),
        ..AgentOptions::new("product-kb")
    });

    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a product knowledge base assistant.",
        vec![],
    );

    agent.define_tool(
        "search_products",
        "Search the product knowledge base",
        json!({"query": {"type": "string", "description": "Product query"}}),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            FunctionResult::with_response(&format!(
                "Product KB results for '{query}': [simulated Datasphere results]"
            ))
        }),
        false,
    );

    agent
}

fn support_kb_agent() -> AgentBase {
    let mut agent = AgentBase::new(AgentOptions {
        name: "support-kb".to_string(),
        route: Some("/support-kb".to_string()),
        ..AgentOptions::new("support-kb")
    });

    agent.add_language("English", "en-US", "rime.spore");
    agent.prompt_add_section(
        "Role",
        "You are a support knowledge base assistant for troubleshooting.",
        vec![],
    );

    agent.define_tool(
        "search_support",
        "Search the support knowledge base",
        json!({"query": {"type": "string", "description": "Support query"}}),
        Box::new(|args, _raw| {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            FunctionResult::with_response(&format!(
                "Support KB results for '{query}': [simulated Datasphere results]"
            ))
        }),
        false,
    );

    agent
}

fn main() {
    let mut server = AgentServer::new(Some("0.0.0.0"), Some(3000));
    server.register(product_kb_agent(), None).unwrap();
    server.register(support_kb_agent(), None).unwrap();

    println!("Datasphere multi-instance:");
    println!("  Products: http://localhost:3000/product-kb");
    println!("  Support:  http://localhost:3000/support-kb");
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
