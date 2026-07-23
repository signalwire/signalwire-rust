// Copyright (c) 2026 SignalWire
//
// This file is part of the SignalWire SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

// ai-chat-dump — the Rust port's AI-CHAT dump program for the cross-port
// wire-behavioral gate (porting-sdk/scripts/diff_port_ai_chat.py, on the
// `ai-chat-client` branch — a COORDINATED pass).
//
// The gate boots the in-process mock_ai_chat server, exports MOCK_AI_CHAT_URL +
// SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN into this program's env, runs it,
// and asserts the JSON it prints (+ the wire requests the mock recorded) speak
// the AI Chat protocol per the vendored spec (ai-chat-specs/ai-chat.yaml).
//
// This mirrors porting-sdk/scripts/ai_chat_dump_reference.py EXACTLY: it drives
// the Rust AIChatClient through the shared ai_chat_corpus and emits ONE JSON
// object to stdout (nothing else), keyed by corpus step:
//
//   success steps (create/chat/end/delete/log/summarize):
//       { wire_method, decoded: { <spec result fields> } }
//   summarize_failed (the summarize {error} one_of branch — must SURFACE, not swallow):
//       { wire_method:"summarize", raised:true, error_type, message }
//   error steps (err_notfound/err_ratelimit/err_inprogress/err_auth/err_unmapped):
//       { raised:true, error_code, error_type }
//
// The corpus (steps + SUMMARIZE_ERROR_ID + ERROR_STEPS + force_error_id) is data,
// identical for every language; it is mirrored inline here from ai_chat_corpus.py.
//
// Run from the signalwire-rust repo root against a running mock:
//
//     MOCK_AI_CHAT_URL=http://127.0.0.1:PORT/api/ai/chat cargo run --quiet --bin ai-chat-dump
//
// Nothing but the JSON object is written to stdout on success.

use std::process::ExitCode;

use serde_json::{Map, Value, json};

use signalwire::ai_chat::{
    AIChatClient, AIChatError, AIChatErrorKind, ChatOptions, CreateOptions, SummarizeOptions,
};

// ── the shared corpus (mirror of porting-sdk/scripts/ai_chat_corpus.py) ──────

/// The sentinel conversation id that makes summarize return its {error} branch.
const SUMMARIZE_ERROR_ID: &str = "__summarize_error";

/// error step id -> the JSON-RPC code the port's raised error MUST carry.
const ERROR_STEPS: &[(&str, i64)] = &[
    ("err_notfound", -32001),   // ConversationNotFound
    ("err_ratelimit", -32005),  // RateLimit
    ("err_inprogress", -32007), // ChatInProgress
    ("err_auth", -32009),       // Authentication
    ("err_unmapped", -32602),   // base AIChatError (unmapped code)
];

/// The sentinel conversation id that makes the mock return `code`.
fn force_error_id(code: i64) -> String {
    format!("__err_{code}")
}

async fn run(url: &str) -> Result<Map<String, Value>, AIChatError> {
    let mut out = Map::new();
    let client = AIChatClient::builder().url(url).build()?;

    // ── success steps ──────────────────────────────────────────────────
    let info = client
        .create_conversation(
            "conv-1",
            CreateOptions::new("http://cfg").timeout(30).reinit(true),
        )
        .await?;
    out.insert(
        "create".to_string(),
        json!({
            "wire_method": "create_conversation",
            "decoded": {
                "status": info.status,
                "id": info.id,
                "initial_message": info.initial_message,
            }
        }),
    );

    let reply = client
        .chat(
            "conv-1",
            "hello",
            ChatOptions::default().timeout(30).reinit(true),
        )
        .await?;
    out.insert(
        "chat".to_string(),
        json!({
            "wire_method": "chat",
            "decoded": { "response": reply.text, "user_event": reply.user_event }
        }),
    );

    // end/delete return bool idiomatically; the wire result also carries the
    // conversation id (the caller's own input, echoed). Report both the derived
    // status and the id operated on — mirroring the reference dump.
    let ended = client.end("conv-1").await?;
    out.insert(
        "end".to_string(),
        json!({
            "wire_method": "end_conversation",
            "decoded": { "status": if ended { "ended" } else { "?" }, "id": "conv-1" }
        }),
    );

    let deleted = client.delete("conv-1").await?;
    out.insert(
        "delete".to_string(),
        json!({
            "wire_method": "delete",
            "decoded": { "status": if deleted { "deleted" } else { "?" }, "id": "conv-1" }
        }),
    );

    let log = client.log("conv-1").await?;
    out.insert(
        "log".to_string(),
        json!({
            "wire_method": "chat_log",
            "decoded": { "chat_log": log.messages, "call_timeline": log.call_timeline }
        }),
    );

    let summary = client
        .summarize("conv-1", SummarizeOptions::default())
        .await?;
    out.insert(
        "summarize".to_string(),
        json!({ "wire_method": "summarize", "decoded": { "summary": summary } }),
    );

    // ── summarize one_of {error} branch: must SURFACE, not swallow ───────
    match client
        .summarize(SUMMARIZE_ERROR_ID, SummarizeOptions::default())
        .await
    {
        Ok(swallowed) => {
            out.insert(
                "summarize_failed".to_string(),
                json!({
                    "wire_method": "summarize",
                    "raised": false,
                    "decoded": { "summary": swallowed }
                }),
            );
        }
        Err(e) if e.kind == AIChatErrorKind::Summary => {
            out.insert(
                "summarize_failed".to_string(),
                json!({
                    "wire_method": "summarize",
                    "raised": true,
                    "error_type": e.kind.name(),
                    "message": e.message,
                }),
            );
        }
        Err(e) => return Err(e),
    }

    // ── error-code steps (JSON-RPC error object) ─────────────────────────
    for (step, code) in ERROR_STEPS {
        let id = force_error_id(*code);
        match client.chat(&id, "x", ChatOptions::default()).await {
            Ok(_) => {
                out.insert((*step).to_string(), json!({ "raised": false }));
            }
            Err(e) => {
                out.insert(
                    (*step).to_string(),
                    json!({ "raised": true, "error_code": e.code, "error_type": e.kind.name() }),
                );
            }
        }
    }

    Ok(out)
}

fn main() -> ExitCode {
    let Ok(url) = std::env::var("MOCK_AI_CHAT_URL") else {
        eprintln!("MOCK_AI_CHAT_URL not set");
        return ExitCode::from(2);
    };
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("ai-chat-dump: tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    match rt.block_on(run(&url)) {
        Ok(out) => {
            println!("{}", Value::Object(out));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ai-chat-dump: {e}");
            ExitCode::FAILURE
        }
    }
}
