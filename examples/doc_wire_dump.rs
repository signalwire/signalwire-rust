// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! DOC-WIRE fixture runner for signalwire-rust.
//!
//! The DOC-WIRE gate (porting-sdk `scripts/doc_wire.py`) spawns
//! `mock_signalwire` in flag mode, exports `MOCK_SIGNALWIRE_PORT`, then runs
//! THIS program; it reads the mock's `wire_violations` journal and fails on any
//! violation. This program's only job is to DRIVE the documented REST calls
//! against the mock so the mock journals what the documented fixtures actually
//! put on the wire — a doc lie like `area_code=` (spec `areacode`) would show up
//! as a journaled violation and fail the gate.
//!
//! It replays the wire-bearing REST fixtures shown in `README.md`,
//! `rest/docs/*`, and `rest/examples/*` — the exact argument shapes the docs
//! teach. The blocking agent/relay quickstarts are covered by EXAMPLES-RUN, not
//! here.
//!
//! Run via the DOC-WIRE gate, or directly:
//!
//! ```bash
//! MOCK_SIGNALWIRE_PORT=8080 cargo run --example doc_wire_dump
//! ```

use std::collections::HashMap;

use serde_json::json;
use signalwire::rest::RestClient;
use signalwire::rest::namespaces::generated::calling_resources_generated::{
    CallingDialRequest, CallingPlayRequest,
};
use signalwire::rest::namespaces::generated::datasphere_resources_generated::DatasphereDocumentsSearchRequest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("MOCK_SIGNALWIRE_PORT")
        .map_err(|_| "DOC-WIRE: MOCK_SIGNALWIRE_PORT not set")?;
    let base_url = format!("http://127.0.0.1:{port}");

    let client = RestClient::with_base_url("test_proj", "test_tok", &base_url)?;

    let call_id = "call-doc-wire";

    // --- README (region: rest quickstart) ------------------------------------
    // fabric().ai_agents().create({name, prompt:{text}})
    client.fabric().ai_agents().create(&json!({
        "name": "Support Bot",
        "prompt": {"text": "You are helpful."}
    }))?;

    // calling().dial(from, to).url(...)
    client.calling().dial(
        CallingDialRequest::new("+15559876543", "+15551234567")
            .url("https://example.com/call-handler"),
    )?;

    // phone_numbers().search({areacode})  — spec key is `areacode`, NOT area_code
    let query = HashMap::from([("areacode".to_string(), "512".to_string())]);
    client.phone_numbers().search(&query)?;

    // --- rest/docs/getting-started.md ----------------------------------------
    let mut params = HashMap::new();
    params.insert("areacode".to_string(), "512".to_string());
    client.phone_numbers().search(&params)?;
    client.fabric().ai_agents().list(&HashMap::new())?;

    // --- rest/docs/calling.md play (nested params:{text}) --------------------
    client.calling().play(
        call_id,
        CallingPlayRequest::new(json!([
            {"type": "tts", "params": {"text": "Please hold."}}
        ])),
    )?;
    // With an optional volume set (rest/docs/calling.md variant).
    client.calling().play(
        call_id,
        CallingPlayRequest::new(json!([
            {"type": "tts", "params": {"text": "Hello!"}}
        ]))
        .volume(5.0),
    )?;

    // --- rest/docs/datasphere search -----------------------------------------
    client
        .datasphere()
        .documents()
        .search(DatasphereDocumentsSearchRequest::new("billing policy"))?;

    println!("doc_wire_dump: replayed documented REST fixtures against the mock");
    Ok(())
}
