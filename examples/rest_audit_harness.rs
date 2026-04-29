// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `rest_audit_harness` — runtime probe for the REST transport.
//!
//! Driven by porting-sdk's `audit_rest_transport.py`. Reads:
//!   - REST_OPERATION       dotted name (e.g. `calling.list_calls`)
//!   - REST_FIXTURE_URL     `http://127.0.0.1:NNNN`
//!   - REST_OPERATION_ARGS  JSON dict of args for the operation
//!   - SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN
//!
//! Constructs a `RestClient` pointed at REST_FIXTURE_URL (NOT through
//! the usual `https://{space}` resolution — the audit needs to inject
//! its loopback fixture URL), invokes the named operation, and prints
//! the parsed return value as JSON to stdout. Exits non-zero on any
//! error.
//!
//! Operations supported by this harness:
//!   - `calling.list_calls`           GET  /api/laml/2010-04-01/Accounts/{proj}/Calls.json
//!   - `messaging.send`               POST /api/laml/2010-04-01/Accounts/{proj}/Messages.json
//!   - `phone_numbers.list`           GET  /api/relay/rest/phone_numbers
//!   - `fabric.subscribers.list`      GET  /api/fabric/resources/subscribers
//!   - `compatibility.calls.list`     GET  /api/laml/2010-04-01/Accounts/{proj}/Calls.json

use serde_json::Value;
use signalwire::rest::RestClient;
use std::collections::HashMap;
use std::env;
use std::process;

fn main() {
    if env::var("SIGNALWIRE_LOG_MODE").is_err() {
        unsafe {
            env::set_var("SIGNALWIRE_LOG_MODE", "off");
        }
    }

    let operation = env::var("REST_OPERATION")
        .unwrap_or_else(|_| die("REST_OPERATION env var required"));
    let fixture_url = env::var("REST_FIXTURE_URL")
        .unwrap_or_else(|_| die("REST_FIXTURE_URL env var required"));
    let args_raw = env::var("REST_OPERATION_ARGS").unwrap_or_else(|_| "{}".to_string());
    let args: Value = serde_json::from_str(&args_raw)
        .unwrap_or_else(|e| die(&format!("REST_OPERATION_ARGS not JSON: {}", e)));
    let project = env::var("SIGNALWIRE_PROJECT_ID")
        .unwrap_or_else(|_| die("SIGNALWIRE_PROJECT_ID env var required"));
    let token = env::var("SIGNALWIRE_API_TOKEN")
        .unwrap_or_else(|_| die("SIGNALWIRE_API_TOKEN env var required"));

    let client = RestClient::with_base_url(&project, &token, &fixture_url)
        .unwrap_or_else(|e| die(&format!("RestClient init: {}", e)));

    let result = dispatch(&client, &operation, &args).unwrap_or_else(|e| die(&e));

    println!("{}", serde_json::to_string(&result).unwrap_or_default());
    process::exit(0);
}

fn dispatch(client: &RestClient, op: &str, args: &Value) -> Result<Value, String> {
    match op {
        "calling.list_calls" => {
            // The compat namespace handles Twilio-style LAML /Accounts/{proj}/Calls.
            // The audit's expected_path_substring is `/api/laml/2010-04-01/Accounts`.
            let path = format!(
                "/api/laml/2010-04-01/Accounts/{}/Calls.json",
                client.project_id()
            );
            let params = args_to_string_map(args);
            client
                .http()
                .get(&path, &params)
                .map_err(|e| format!("{}: {}", op, e.message()))
        }
        "compatibility.calls.list" => {
            let path = format!(
                "/api/laml/2010-04-01/Accounts/{}/Calls.json",
                client.project_id()
            );
            let params = args_to_string_map(args);
            client
                .http()
                .get(&path, &params)
                .map_err(|e| format!("{}: {}", op, e.message()))
        }
        "messaging.send" => {
            // POST /Accounts/{proj}/Messages.json — audit expects path
            // substring `Messages`.
            let path = format!(
                "/api/laml/2010-04-01/Accounts/{}/Messages.json",
                client.project_id()
            );
            client
                .http()
                .post(&path, args)
                .map_err(|e| format!("{}: {}", op, e.message()))
        }
        "phone_numbers.list" => {
            let params = args_to_string_map(args);
            client
                .phone_numbers()
                .list(&params)
                .map_err(|e| format!("{}: {}", op, e.message()))
        }
        "fabric.subscribers.list" => {
            let params = args_to_string_map(args);
            client
                .fabric()
                .subscribers()
                .list(&params)
                .map_err(|e| format!("{}: {}", op, e.message()))
        }
        other => Err(format!("rest_audit_harness: unsupported operation '{}'", other)),
    }
}

/// Args dict → string map for query-string params.
fn args_to_string_map(args: &Value) -> HashMap<String, String> {
    let mut m = HashMap::new();
    if let Some(obj) = args.as_object() {
        for (k, v) in obj {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => v.to_string(),
            };
            m.insert(k.clone(), s);
        }
    }
    m
}

fn die(msg: &str) -> ! {
    eprintln!("rest_audit_harness: {}", msg);
    process::exit(1);
}
