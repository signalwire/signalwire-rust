// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `emit_corpus` — the Rust port's EMISSION-DUMP program for the cross-port
//! emission differ (porting-sdk/scripts/diff_port_emission.py).
//!
//! It builds the shared `FunctionResult` corpus
//! (porting-sdk/scripts/emission_corpus.py — the single source of truth) using
//! the Rust SDK's native `swaig::FunctionResult` API, serialises each entry the
//! same way the SDK serialises on the wire (`to_value()`), and prints ONE JSON
//! object mapping
//!
//!     corpus-id -> emission
//!
//! to stdout. The differ runs this program, parses that object, and
//! byte-compares each entry against Python's `to_dict()`. See the "per-port dump
//! contract" in the differ's `--help` and `IDIOM_PASS_JOURNAL.md` §4 Tier-0.
//!
//! CONTRACT (why this file looks the way it does):
//!   - Every corpus id in `emission_corpus.corpus_ids()` MUST appear here exactly
//!     once (the differ rejects an id-set mismatch as a setup error — a skewed
//!     set would mask real diffs). When the shared corpus grows, add the new id
//!     here.
//!   - The argument VALUES are the WIRE values (plain strings/numbers/bools/maps).
//!     Where the Rust API types a closed set (`RecordFormat`, `RecordDirection`,
//!     `TapDirection`, Codec) we pass the typed constant whose string value is the
//!     wire value, proving the typed path emits byte-identically to the string.
//!   - Only stdout carries the JSON object; nothing else is printed there.
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example emit_corpus

// `corpus()` is a flat table of emission entries that must match the Python
// single-source corpus 1:1 (see the contract docstring); it is data, not
// logic, so its length is inherent and splitting it adds no clarity.
#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use signalwire::swaig::{Codec, FunctionResult, RecordDirection, RecordFormat, TapDirection};

/// Build a fresh `FunctionResult` with the given response text (mirrors
/// `FunctionResult(response=...)` on the Python side). An empty string is the
/// default (no response) ctor.
fn fr(response: &str) -> FunctionResult {
    FunctionResult::with_response(response)
}

/// One corpus entry: a stable id paired with a builder that returns the
/// serialised emission (`to_value()`). Building lazily (a closure) keeps each
/// case a single, readable native call — the Rust `&mut self` API needs a local
/// binding, so each closure owns its `FunctionResult`.
struct Entry {
    id: &'static str,
    build: fn() -> Value,
}

/// Helper that fully constructs an entry: takes the response text + a closure
/// that mutates the `FunctionResult`, then serialises.
macro_rules! entry {
    ($id:literal, $resp:expr, |$fr:ident| $body:block) => {
        Entry {
            id: $id,
            build: || {
                // Most entries mutate the result; the two envelope-only cases
                // (empty / response-only) do not — allow either.
                #[allow(unused_mut)]
                let mut $fr = fr($resp);
                $body;
                $fr.to_value()
            },
        }
    };
}

fn corpus() -> Vec<Entry> {
    vec![
        // ---- envelope edge cases (to_value() shape) -------------------------
        entry!("envelope.empty", "", |fr| {
            // No method: ctor-only (FunctionResult() with empty response).
            let _ = &fr;
        }),
        entry!("envelope.response_only", "Hello, world!", |fr| {
            let _ = &fr;
        }),
        entry!("envelope.post_process_no_action", "hi", |fr| {
            fr.set_post_process(true);
        }),
        entry!("envelope.action_only", "", |fr| {
            fr.hangup();
        }),
        entry!("envelope.post_process_with_action", "Transferring", |fr| {
            fr.set_post_process(true).hangup();
        }),
        entry!("envelope.response_and_action", "Goodbye", |fr| {
            fr.hangup();
        }),
        // ---- connect (final true/false, from override) ----------------------
        entry!("connect.final_true", "", |fr| {
            fr.connect("+15551234567", None, None);
        }),
        entry!("connect.final_false", "", |fr| {
            fr.connect("+15551234567", Some(false), None);
        }),
        entry!("connect.from_addr", "", |fr| {
            fr.connect("support@example.com", Some(false), Some("+15559876543"));
        }),
        // ---- swml_transfer --------------------------------------------------
        entry!("swml_transfer.default", "", |fr| {
            fr.swml_transfer("https://dest.example.com/swml", "Goodbye!", None);
        }),
        entry!("swml_transfer.final_false", "", |fr| {
            fr.swml_transfer(
                "https://dest.example.com/swml",
                "Welcome back. How else can I help?",
                Some(false),
            );
        }),
        // ---- simple call-control actions ------------------------------------
        entry!("hangup", "", |fr| {
            fr.hangup();
        }),
        entry!("hold.default", "", |fr| {
            // Python hold() defaults timeout=300.
            fr.hold(None);
        }),
        entry!("hold.value", "", |fr| {
            fr.hold(Some(120));
        }),
        entry!("hold.clamp_high", "", |fr| {
            fr.hold(Some(5000));
        }),
        entry!("hold.clamp_low", "", |fr| {
            fr.hold(Some(-5));
        }),
        entry!("stop", "", |fr| {
            fr.stop();
        }),
        entry!("say", "", |fr| {
            fr.say("Please hold while I connect you.");
        }),
        // ---- wait_for_user (each branch) ------------------------------------
        entry!("wait_for_user.default", "", |fr| {
            fr.wait_for_user(None, None, None);
        }),
        entry!("wait_for_user.answer_first", "", |fr| {
            fr.wait_for_user(None, None, Some(true));
        }),
        entry!("wait_for_user.timeout", "", |fr| {
            fr.wait_for_user(None, Some(30), None);
        }),
        entry!("wait_for_user.enabled_true", "", |fr| {
            fr.wait_for_user(Some(true), None, None);
        }),
        entry!("wait_for_user.enabled_false", "", |fr| {
            fr.wait_for_user(Some(false), None, None);
        }),
        // ---- global data / metadata (set/unset, str + list) -----------------
        entry!("set_global_data", "", |fr| {
            fr.update_global_data(json!({"plan": "premium", "chips": 1000}));
        }),
        entry!("unset_global_data.list", "", |fr| {
            fr.remove_global_data(vec!["plan", "chips"]);
        }),
        entry!("unset_global_data.str", "", |fr| {
            // Python's Union[str, List[str]] str arm: a bare key STRING, passed
            // through verbatim -> {"unset_global_data": "plan"} (NOT a list).
            // Rust's remove_global_data accepts `&str` via `impl Into<KeysArg>`.
            fr.remove_global_data("plan");
        }),
        entry!("set_metadata", "", |fr| {
            fr.set_metadata(json!({"token": "abc", "count": 3}));
        }),
        entry!("unset_metadata.list", "", |fr| {
            fr.remove_metadata(vec!["token", "count"]);
        }),
        entry!("unset_metadata.str", "", |fr| {
            // str arm -> bare string {"unset_meta_data": "token"} (not a list).
            fr.remove_metadata("token");
        }),
        // ---- swml_user_event ------------------------------------------------
        entry!("swml_user_event", "", |fr| {
            fr.swml_user_event(json!({
                "type": "cards_dealt",
                "player_hand": ["AS", "KH"],
                "player_score": 21
            }));
        }),
        // ---- step / context changes -----------------------------------------
        entry!("change_step", "", |fr| {
            fr.swml_change_step("collect_payment");
        }),
        entry!("change_context", "", |fr| {
            fr.swml_change_context("billing");
        }),
        // ---- switch_context (simple-string vs object branches) --------------
        // Rust's switch_context has a trailing isolated param (a documented
        // PORT_ADDITION); the corpus exercises the Python-equivalent paths with
        // isolated=false. Python's user_prompt=None maps to "" (omitted).
        entry!("switch_context.simple", "", |fr| {
            fr.switch_context("You are now a billing agent.", "", false, false, false);
        }),
        entry!("switch_context.object", "", |fr| {
            fr.switch_context(
                "New system prompt",
                "User said something",
                true,
                false,
                false,
            );
        }),
        entry!("switch_context.full_reset", "", |fr| {
            fr.switch_context("Reset prompt", "", false, true, false);
        }),
        // ---- background file play/stop --------------------------------------
        entry!("playback_bg.simple", "", |fr| {
            fr.play_background_file("music.mp3", None);
        }),
        entry!("playback_bg.wait", "", |fr| {
            fr.play_background_file("music.mp3", Some(true));
        }),
        entry!("stop_playback_bg", "", |fr| {
            fr.stop_background_file();
        }),
        // ---- join_room / sip_refer ------------------------------------------
        entry!("join_room", "", |fr| {
            fr.join_room("team-standup");
        }),
        entry!("sip_refer", "", |fr| {
            fr.sip_refer("sip:agent@example.com");
        }),
        // ---- send_sms -------------------------------------------------------
        entry!("send_sms.body", "", |fr| {
            fr.send_sms(
                "+15551112222",
                "+15553334444",
                Some("Your appointment is confirmed."),
                None,
                None,
                None,
            )
            .expect("send_sms.body");
        }),
        entry!("send_sms.full", "", |fr| {
            fr.send_sms(
                "+15551112222",
                "+15553334444",
                Some("See attached."),
                Some(vec!["https://ex.com/a.jpg"]),
                Some(vec!["receipt", "vip"]),
                Some("us"),
            )
            .expect("send_sms.full");
        }),
        // ---- pay (full + helper-shaped prompts/parameters) ------------------
        entry!("pay.minimal", "", |fr| {
            // Python pay() with only the connector URL: every other arg is
            // OMITTED (`None`), so this entry proves the port's defaults —
            // input_method="dtmf", payment_method="credit-card", timeout=5,
            // max_attempts=1, security_code=True, postal_code=True → wire
            // "true", min_postal_code_length=0, token_type="reusable",
            // currency="usd", language="en-US", voice="woman",
            // valid_card_types="visa mastercard amex", and the default
            // ai_response — land on the wire without being passed.
            fr.pay(
                "https://pay.example.com/connector",
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }),
        entry!("pay.full", "", |fr| {
            let parameters = json!([{"name": "order_id", "value": "42"}]);
            let prompts = json!([{
                "for": "payment-card-number",
                "actions": [{"type": "Say", "phrase": "Enter your card number"}],
                "card_type": "visa amex"
            }]);
            fr.pay(
                "https://pay.example.com/connector",
                None, // input_method (default "dtmf")
                Some("https://ex.com/status"),
                None,              // payment_method (default "credit-card")
                Some(7),           // timeout
                Some(2),           // max_attempts
                Some(false),       // security_code
                Some("90210"),     // postal_code
                Some(5),           // min_postal_code_length
                Some("one-time"),  // token_type
                Some("9.99"),      // charge_amount
                None,              // currency (default "usd")
                None,              // language (default "en-US")
                None,              // voice (default "woman")
                Some("Order 42"),  // description
                Some("visa amex"), // valid_card_types
                Some(parameters),
                Some(prompts),
                None, // ai_response (default status message)
            );
        }),
        entry!("pay.postal_bool", "", |fr| {
            // postal_code passed EXPLICITLY as the bool-True wire spelling,
            // with every other arg omitted (`None` = Python's default).
            fr.pay(
                "https://pay.example.com/connector",
                None,
                None,
                None,
                None,
                None,
                None,
                Some("true"), // postal_code bool True -> wire "true"
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }),
        // ---- record_call (incl. mp4 + each direction) -----------------------
        entry!("record_call.defaults", "", |fr| {
            // Python defaults: format="wav", direction="both",
            // input_sensitivity=44.0; everything else unset.
            fr.record_call(
                None, None, None, None, None, None, None, None, None, None, None,
            )
            .expect("record_call.defaults");
        }),
        entry!("record_call.wav_speak", "", |fr| {
            fr.record_call(
                None,
                None,
                None,
                Some("speak".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("record_call.wav_speak");
        }),
        entry!("record_call.mp3_listen", "", |fr| {
            fr.record_call(
                None,
                None,
                Some("mp3".into()),
                Some("listen".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("record_call.mp3_listen");
        }),
        entry!("record_call.mp4_both", "", |fr| {
            // Typed path: prove RecordFormat::Mp4 / RecordDirection::Both emit
            // byte-identically to the bare strings.
            fr.record_call(
                None,
                None,
                Some(RecordFormat::Mp4.into()),
                Some(RecordDirection::Both.into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("record_call.mp4_both");
        }),
        entry!("record_call.full", "", |fr| {
            fr.record_call(
                Some("rec1"),
                Some(true),
                Some("mp3".into()),
                Some("both".into()),
                Some("#"),
                Some(true),
                Some(30.0),
                Some(5.0),
                Some(3.0),
                Some(120.0),
                Some("https://ex.com/rec"),
            )
            .expect("record_call.full");
        }),
        entry!("stop_record_call.bare", "", |fr| {
            fr.stop_record_call(None);
        }),
        entry!("stop_record_call.id", "", |fr| {
            fr.stop_record_call(Some("rec1"));
        }),
        // ---- tap (each direction / codec) -----------------------------------
        entry!("tap.defaults", "", |fr| {
            // Every optional arg OMITTED, so this entry proves the port's
            // defaults (direction="both", codec="PCMU", rtp_ptime=20) land on
            // the wire without being passed.
            fr.tap("rtp://10.0.0.1:5004", None, None, None, None, None)
                .expect("tap.defaults");
        }),
        entry!("tap.speak_pcma", "", |fr| {
            fr.tap(
                "ws://ex.com/tap",
                None,
                Some("speak".into()),
                Some("PCMA".into()),
                None,
                None,
            )
            .expect("tap.speak_pcma");
        }),
        entry!("tap.hear_pcmu", "", |fr| {
            // Typed path: TapDirection::Hear / Codec::Pcmu collapse to the same
            // wire strings as "hear"/"PCMU".
            fr.tap(
                "wss://ex.com/tap",
                None,
                Some(TapDirection::Hear.into()),
                Some(Codec::Pcmu.into()),
                None,
                None,
            )
            .expect("tap.hear_pcmu");
        }),
        entry!("tap.both_full", "", |fr| {
            fr.tap(
                "rtp://10.0.0.1:5004",
                Some("tap1"),
                Some("both".into()),
                Some("PCMA".into()),
                Some(40),
                Some("https://ex.com/tapstatus"),
            )
            .expect("tap.both_full");
        }),
        entry!("stop_tap.bare", "", |fr| {
            fr.stop_tap(None);
        }),
        entry!("stop_tap.id", "", |fr| {
            fr.stop_tap(Some("tap1"));
        }),
        // ---- join_conference (simple + full) --------------------------------
        entry!("join_conference.simple", "", |fr| {
            // Every optional arg OMITTED -> the simple bare-string form. This
            // entry proves the port's defaults (muted=false, beep="true",
            // start_on_enter=true, end_on_exit=false, max_participants=250,
            // record="do-not-record", trim="trim-silence", both callback
            // methods "POST", recording_status_callback_event="completed")
            // without any of them being passed.
            fr.join_conference(
                "sales-floor",
                None, // muted
                None, // beep
                None, // start_on_enter
                None, // end_on_exit
                None, // wait_url
                None, // max_participants
                None, // record
                None, // region
                None, // trim
                None, // coach
                None, // status_callback_event
                None, // status_callback
                None, // status_callback_method
                None, // recording_status_callback
                None, // recording_status_callback_method
                None, // recording_status_callback_event
                None, // result
            )
            .expect("join_conference.simple");
        }),
        entry!("join_conference.full", "", |fr| {
            fr.join_conference(
                "sales-floor",
                Some(true),      // muted
                Some("onEnter"), // beep
                Some(false),     // start_on_enter
                Some(true),      // end_on_exit
                Some("https://ex.com/hold"),
                Some(50),                  // max_participants
                Some("record-from-start"), // record
                Some("us-east"),
                Some("do-not-trim"), // trim
                Some("call-123"),    // coach
                Some("start end join leave"),
                Some("https://ex.com/cb"),
                Some("GET"), // status_callback_method
                Some("https://ex.com/rcb"),
                Some("GET"), // recording_status_callback_method
                Some("in-progress completed"),
                None, // result
            )
            .expect("join_conference.full");
        }),
        // ---- execute_rpc + the three rpc helpers ----------------------------
        entry!("execute_rpc.minimal", "", |fr| {
            fr.execute_rpc("ai_unhold", None, None, None);
        }),
        entry!("execute_rpc.full", "", |fr| {
            fr.execute_rpc(
                "ai_message",
                Some(json!({"role": "system", "message_text": "Hello"})),
                Some("call-abc"),
                Some("node-1"),
            );
        }),
        entry!("rpc_dial", "", |fr| {
            // `device_type` OMITTED, proving the port's default "phone".
            fr.rpc_dial(
                "+15551234567",
                "+15559876543",
                "https://ex.com/call-agent",
                None,
            );
        }),
        entry!("rpc_ai_message", "", |fr| {
            // Python rpc_ai_message default role="system".
            fr.rpc_ai_message("call-abc", "Please take a message.", None);
        }),
        entry!("rpc_ai_unhold", "", |fr| {
            fr.rpc_ai_unhold("call-abc");
        }),
        // ---- simulate_user_input --------------------------------------------
        entry!("simulate_user_input", "", |fr| {
            fr.simulate_user_input("I'd like to pay my bill.");
        }),
        // ---- dynamic hints --------------------------------------------------
        entry!("add_dynamic_hints", "", |fr| {
            fr.add_dynamic_hints(vec![
                json!("Cabby"),
                json!({"pattern": "cab bee", "replace": "Cabby", "ignore_case": true}),
            ]);
        }),
        entry!("clear_dynamic_hints", "", |fr| {
            fr.clear_dynamic_hints();
        }),
        // ---- toggle_functions / functions-on-timeout ------------------------
        entry!("toggle_functions", "", |fr| {
            // The corpus passes an ORDERED list of {function, active} dicts.
            // Rust's toggle_functions takes the same shape (Vec<Value>) so the
            // emitted array preserves order + arbitrary keys, byte-identical to
            // Python's `add_action("toggle_functions", function_toggles)`.
            fr.toggle_functions(vec![
                json!({"function": "transfer", "active": false}),
                json!({"function": "lookup", "active": true}),
            ]);
        }),
        entry!("functions_on_speaker_timeout.true", "", |fr| {
            fr.enable_functions_on_timeout(None);
        }),
        entry!("functions_on_speaker_timeout.false", "", |fr| {
            fr.enable_functions_on_timeout(Some(false));
        }),
        // ---- extensive_data -------------------------------------------------
        entry!("extensive_data.true", "", |fr| {
            fr.enable_extensive_data(None);
        }),
        entry!("extensive_data.false", "", |fr| {
            fr.enable_extensive_data(Some(false));
        }),
        // ---- replace_in_history (str + bool branches) -----------------------
        entry!("replace_in_history.bool", "", |fr| {
            // Python replace_in_history() with no arg -> the bool-true branch.
            fr.replace_in_history(None);
        }),
        entry!("replace_in_history.str", "", |fr| {
            fr.replace_in_history(Some("Summarized the order."));
        }),
        // ---- settings -------------------------------------------------------
        entry!("settings", "", |fr| {
            fr.update_settings(json!({"temperature": 0.7, "max-tokens": 256, "top-p": 0.9}));
        }),
        // ---- speech timeouts ------------------------------------------------
        entry!("end_of_speech_timeout", "", |fr| {
            fr.set_end_of_speech_timeout(800);
        }),
        entry!("speech_event_timeout", "", |fr| {
            fr.set_speech_event_timeout(1200);
        }),
        // ---- execute_swml (dict + JSON-string + transfer) -------------------
        entry!("execute_swml.dict", "", |fr| {
            fr.execute_swml(
                json!({"version": "1.0.0", "sections": {"main": [{"answer": {}}]}}),
                None,
            );
        }),
        entry!("execute_swml.dict_transfer", "", |fr| {
            fr.execute_swml(
                json!({"version": "1.0.0", "sections": {"main": [{"answer": {}}]}}),
                Some(true),
            );
        }),
        entry!("execute_swml.json_string", "", |fr| {
            // Python accepts a JSON STRING and parses it; Rust's execute_swml
            // takes a Value, so feed the parsed object (the wire value is the
            // same parsed SWML document either way).
            fr.execute_swml(
                json!({"version": "1.0.0", "sections": {"main": [{"hangup": {}}]}}),
                None,
            );
        }),
    ]
}

fn main() {
    let entries = corpus();

    // BTreeMap keeps stdout deterministic (sorted ids); the differ compares by
    // id so order is irrelevant, but determinism aids debugging.
    let mut out: BTreeMap<&str, Value> = BTreeMap::new();
    let mut seen: Map<String, Value> = Map::new();
    for e in &entries {
        if seen.contains_key(e.id) {
            eprintln!("emit-corpus: duplicate corpus id {:?}", e.id);
            std::process::exit(1);
        }
        seen.insert(e.id.to_string(), Value::Null);
        out.insert(e.id, (e.build)());
    }

    // One JSON object to stdout, nothing else. serde_json does not escape
    // '+'/'&', matching Python's json output.
    match serde_json::to_string(&out) {
        Ok(s) => println!("{s}"),
        Err(err) => {
            eprintln!("emit-corpus: encode failed: {err}");
            std::process::exit(1);
        }
    }
}
