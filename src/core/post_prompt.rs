//! Post-prompt normalization.
//!
//! One conversation can run over voice and over text chat, and both ends
//! produce "the post-prompt" — but not in the same shape, and the differences
//! are not documented anywhere a caller would find them. This module absorbs
//! that divergence so an application sees one artifact regardless of which
//! engine finished the conversation.
//!
//! | field              | voice                      | chat                          |
//! |--------------------|----------------------------|-------------------------------|
//! | `app_name`         | `"swml app"`               | `"ai_chat"`                   |
//! | `conversation_id`  | absent                     | present at top level          |
//! | full log           | `raw_call_log`             | `raw_messages`                |
//! | summary arrives as | `summarize_conversation` tool call | a bare `role: assistant` turn inside `call_log` |
//! | `post_prompt_data` | parsed object              | `{"raw": "```json ...```"}`   |
//!
//! `conversation_type` is a reliable top-level discriminator on both.
//!
//! A *third* `post_prompt_data` shape comes from the voice engine:
//! `{"parsed": [ {...} ], "raw": "..."}` — the object wrapped in a list under
//! `parsed`. It survives structurally and misses every field lookup, so a
//! caller that does not handle it silently gets nothing while appearing to work.
//!
//! What this module does NOT do is decide what a summary should contain: the
//! schema is the application's (whatever its post-prompt asked the model to
//! produce), so parsing is schema-agnostic and returns the object as found.
//!
//! Nothing here returns an error: the conversation that produced the input is
//! already over and there is nobody to show an error to, so malformed input
//! degrades to empty fields.

use serde_json::{Map, Value};

/// Roles that are actual dialogue. Everything else in a call log is machinery:
/// `system` is the prompt, `system-log` is lifecycle and step tracing, `tool` is
/// function output, and `assistant-manual` is filler speech ("let me look that
/// up") that was spoken but carries nothing worth replaying.
pub const DIALOGUE_ROLES: &[&str] = &["user", "assistant"];

/// One finished conversation leg, in a shape that does not vary by engine.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NormalizedPostPrompt {
    /// `conversation_type` as reported, e.g. `"voice"` or `"chat"`; empty when
    /// the engine did not say.
    pub medium: String,
    /// Present on chat, absent on voice. Callers that need a stable key should
    /// fall back to their own (`global_data`, `call_id`) rather than treating
    /// this as authoritative.
    pub conversation_id: Option<String>,
    /// The parsed `post_prompt_data` — whatever keys the application's
    /// post-prompt asked for. Empty when there was none or it could not be
    /// parsed. A model that answered in prose instead of JSON yields
    /// `{"summary": "<the prose>"}`.
    pub summary: Map<String, Value>,
    /// `user` / `assistant` turns only (`{"role", "content"}` objects), with
    /// tool calls and the chat engine's summary echo removed.
    pub dialogue: Vec<Map<String, Value>>,
    /// The platform call id, when present.
    pub call_id: Option<String>,
    /// The complete request body, untouched.
    pub raw: Map<String, Value>,
}

impl NormalizedPostPrompt {
    /// An empty leg (every field at its default) — what an unreadable body
    /// normalizes to.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Python truthiness of a JSON value (`None`/`false`/`0`/`""`/`[]`/`{}` are falsy).
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// Unwrap ```` ```json ... ``` ```` fencing.
///
/// The chat engine hands the model's answer back verbatim, fence and all,
/// where the voice engine parses it first.
#[must_use]
pub fn strip_json_fence(text: &str) -> String {
    let mut stripped = text.trim();
    if stripped.starts_with("```") {
        // Opening fence: ``` plus an optional language tag, then whitespace.
        let after = &stripped[3..];
        let tag_len = after
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(after.len());
        stripped = after[tag_len..].trim_start();
        // Closing fence: optional whitespace then ``` at the very end.
        if let Some(body) = stripped.strip_suffix("```") {
            stripped = body.trim_end();
        }
    }
    stripped.trim().to_string()
}

/// Pull the object out of a `{"parsed": [...]}` wrapper, if present.
fn unwrap_parsed(data: &Map<String, Value>) -> Option<Map<String, Value>> {
    match data.get("parsed") {
        Some(Value::Object(o)) => Some(o.clone()),
        Some(Value::Array(items)) => items.iter().find_map(|item| match item {
            Value::Object(o) if !o.is_empty() => Some(o.clone()),
            _ => None,
        }),
        _ => None,
    }
}

/// Return `post_prompt_data` as a plain object, whichever shape it arrived in.
///
/// Flat keys are returned as found; a `{"parsed": [...]}` / `{"parsed": {...}}`
/// wrapper is unwrapped; a `{"raw": "..."}` string is unfenced and parsed, and
/// prose (or JSON that is not an object) becomes `{"summary": "<text>"}`.
/// Anything unusable yields an empty object — never an error.
#[must_use]
pub fn parse_post_prompt_data(data: &Value) -> Map<String, Value> {
    let Value::Object(data) = data else {
        return Map::new();
    };
    if let Some(unwrapped) = unwrap_parsed(data).filter(|m| !m.is_empty()) {
        return unwrapped;
    }
    // Flat shape: real keys already present (anything but raw/parsed).
    let flat: Map<String, Value> = data
        .iter()
        .filter(|(k, _)| k.as_str() != "raw" && k.as_str() != "parsed")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if !flat.is_empty() {
        return flat;
    }
    let Some(raw) = data.get("raw").and_then(Value::as_str) else {
        return Map::new();
    };
    if raw.trim().is_empty() {
        return Map::new();
    }
    let unfenced = strip_json_fence(raw);
    let mut prose = Map::new();
    match serde_json::from_str::<Value>(&unfenced) {
        Ok(Value::Object(o)) => o,
        Ok(Value::String(s)) => {
            prose.insert("summary".to_string(), Value::String(s));
            prose
        }
        Ok(other) => {
            prose.insert("summary".to_string(), Value::String(other.to_string()));
            prose
        }
        // Prose instead of JSON. Still a summary.
        Err(_) => {
            prose.insert("summary".to_string(), Value::String(unfenced));
            prose
        }
    }
}

/// Extract the real dialogue from a call log.
///
/// Drops everything that is machinery rather than speech: roles outside `roles`
/// (default [`DIALOGUE_ROLES`]), entries carrying `tool_calls`, and empty content.
///
/// `drop_echo` exists for one engine behaviour: the chat engine appends its own
/// post-prompt output to `call_log` as a bare `role: assistant` entry, by role
/// indistinguishable from real speech; replayed into another medium the agent
/// narrates a summary of itself. It is identifiable only by content (it is
/// byte-identical to `post_prompt_data.raw`), which is what this compares
/// against.
///
/// Returns `{"role", "content"}` objects in order; a non-array log yields none.
#[must_use]
pub fn dialogue_turns(
    call_log: &Value,
    roles: Option<&[&str]>,
    drop_echo: Option<&str>,
) -> Vec<Map<String, Value>> {
    let roles = roles.unwrap_or(DIALOGUE_ROLES);
    let Value::Array(entries) = call_log else {
        return Vec::new();
    };
    let echo = drop_echo.unwrap_or_default().trim();
    let mut out = Vec::new();
    for entry in entries {
        let Value::Object(entry) = entry else {
            continue;
        };
        let Some(role) = entry.get("role").and_then(Value::as_str) else {
            continue;
        };
        if !roles.contains(&role) {
            continue;
        }
        if truthy(entry.get("tool_calls")) {
            continue;
        }
        let Some(content) = entry.get("content").and_then(Value::as_str) else {
            continue;
        };
        if content.trim().is_empty() {
            continue;
        }
        if !echo.is_empty() && content.trim() == echo {
            continue;
        }
        let mut turn = Map::new();
        turn.insert("role".to_string(), Value::String(role.to_string()));
        turn.insert("content".to_string(), Value::String(content.to_string()));
        out.push(turn);
    }
    out
}

/// Normalize a post-prompt body from either engine.
///
/// A body this cannot make sense of (not an object) yields a
/// [`NormalizedPostPrompt`] with empty fields — never an error.
#[must_use]
pub fn normalize_post_prompt(body: &Value) -> NormalizedPostPrompt {
    let Value::Object(map) = body else {
        return NormalizedPostPrompt::default();
    };
    let ppd = map.get("post_prompt_data").unwrap_or(&Value::Null);
    let summary = parse_post_prompt_data(ppd);
    // The echo is compared against the RAW string the engine returned, not the
    // parsed summary — the assistant turn carries the fence too.
    let raw_summary = ppd.get("raw").and_then(Value::as_str).unwrap_or("");
    let log = ["call_log", "raw_call_log", "raw_messages"]
        .iter()
        .map(|k| map.get(*k))
        .find(|v| truthy(*v))
        .flatten()
        .unwrap_or(&Value::Null);
    let opt_str = |key: &str| -> Option<String> {
        let v = map.get(key);
        if !truthy(v) {
            return None;
        }
        v.map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
    };
    NormalizedPostPrompt {
        medium: opt_str("conversation_type").unwrap_or_default(),
        conversation_id: opt_str("conversation_id"),
        summary,
        dialogue: dialogue_turns(log, None, (!raw_summary.is_empty()).then_some(raw_summary)),
        call_id: opt_str("call_id"),
        raw: map.clone(),
    }
}

#[cfg(test)]
mod tests {
    //! Ported from signalwire-python
    //! `tests/unit/core/test_post_prompt_normalize.py`.
    use super::*;
    use serde_json::json;

    const FENCED: &str = "```json\n{\"summary\": \"s\", \"already_answered\": [\"pricing\"]}\n```";

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().cloned().expect("object")
    }

    #[test]
    fn flat_keys_from_the_voice_engine() {
        assert_eq!(
            parse_post_prompt_data(&json!({"summary": "s", "user_goal": "g"})),
            obj(json!({"summary": "s", "user_goal": "g"}))
        );
    }

    #[test]
    fn fenced_raw_from_the_chat_engine() {
        assert_eq!(
            parse_post_prompt_data(&json!({"raw": FENCED})),
            obj(json!({"summary": "s", "already_answered": ["pricing"]}))
        );
    }

    #[test]
    fn object_wrapped_in_a_list_under_parsed() {
        assert_eq!(
            parse_post_prompt_data(&json!({"parsed": [{"summary": "s3"}], "raw": "..."})),
            obj(json!({"summary": "s3"}))
        );
    }

    #[test]
    fn parsed_wrapper_wins_over_the_generic_sweep() {
        let result = parse_post_prompt_data(&json!({"parsed": [{"summary": "s"}]}));
        assert!(!result.contains_key("parsed"));
    }

    #[test]
    fn parsed_as_a_bare_dict() {
        assert_eq!(
            parse_post_prompt_data(&json!({"parsed": {"summary": "s"}})),
            obj(json!({"summary": "s"}))
        );
    }

    #[test]
    fn prose_instead_of_json_is_kept() {
        assert_eq!(
            parse_post_prompt_data(&json!({"raw": "They asked about pricing."})),
            obj(json!({"summary": "They asked about pricing."}))
        );
    }

    #[test]
    fn json_that_is_not_an_object() {
        assert_eq!(
            parse_post_prompt_data(&json!({"raw": "\"just a string\""})),
            obj(json!({"summary": "just a string"}))
        );
    }

    #[test]
    fn junk_degrades_rather_than_raising() {
        for junk in [
            Value::Null,
            json!({}),
            json!("text"),
            json!(42),
            json!([]),
            json!({"raw": ""}),
            json!({"raw": "   "}),
            json!({"raw": null}),
        ] {
            assert!(parse_post_prompt_data(&junk).is_empty(), "{junk}");
        }
    }

    #[test]
    fn strip_fence_unwraps() {
        for (raw, expected) in [
            ("```json\n{\"a\":1}\n```", "{\"a\":1}"),
            ("```\nplain\n```", "plain"),
            ("no fence at all", "no fence at all"),
            ("", ""),
        ] {
            assert_eq!(strip_json_fence(raw), expected);
        }
    }

    fn log() -> Vec<Value> {
        vec![
            json!({"role": "user", "content": "hi"}),
            json!({"role": "assistant", "content": "hello"}),
            json!({"role": "system", "content": "the prompt"}),
            json!({"role": "system-log", "content": "step trace"}),
            json!({"role": "tool", "content": "tool output"}),
            json!({"role": "assistant", "content": "", "tool_calls": [{"id": 1}]}),
            json!({"role": "assistant-manual", "content": "let me look that up"}),
            json!({"role": "assistant", "content": "   "}),
            json!("not even a dict"),
        ]
    }

    #[test]
    fn keeps_only_real_dialogue() {
        assert_eq!(
            dialogue_turns(&Value::Array(log()), None, None),
            vec![
                obj(json!({"role": "user", "content": "hi"})),
                obj(json!({"role": "assistant", "content": "hello"})),
            ]
        );
    }

    #[test]
    fn drops_the_chat_summary_echo() {
        let mut l = log();
        l.push(json!({"role": "assistant", "content": FENCED}));
        let turns = dialogue_turns(&Value::Array(l), None, Some(FENCED));
        assert!(!turns.contains(&obj(json!({"role": "assistant", "content": FENCED}))));
    }

    #[test]
    fn keeps_the_echo_when_not_asked_to_drop_it() {
        let mut l = log();
        l.push(json!({"role": "assistant", "content": FENCED}));
        assert_eq!(dialogue_turns(&Value::Array(l), None, None).len(), 3);
    }

    #[test]
    fn junk_logs_yield_nothing() {
        for junk in [Value::Null, json!([]), json!("nonsense"), json!(42)] {
            assert!(dialogue_turns(&junk, None, None).is_empty());
        }
    }

    #[test]
    fn voice_body() {
        let r = normalize_post_prompt(&json!({
            "conversation_type": "voice",
            "call_id": "c-1",
            "post_prompt_data": {"parsed": [{"summary": "v"}]},
            "raw_call_log": [{"role": "user", "content": "hi"}],
        }));
        assert_eq!(r.medium, "voice");
        assert_eq!(r.conversation_id, None);
        assert_eq!(r.summary, obj(json!({"summary": "v"})));
        assert_eq!(r.call_id.as_deref(), Some("c-1"));
        assert_eq!(r.dialogue.len(), 1);
    }

    #[test]
    fn chat_body() {
        let r = normalize_post_prompt(&json!({
            "conversation_type": "chat",
            "conversation_id": "conv-9",
            "post_prompt_data": {"raw": FENCED},
            "raw_messages": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": FENCED},
            ],
        }));
        assert_eq!(r.medium, "chat");
        assert_eq!(r.conversation_id.as_deref(), Some("conv-9"));
        assert_eq!(r.summary["already_answered"], json!(["pricing"]));
        assert_eq!(
            r.dialogue,
            vec![obj(json!({"role": "user", "content": "hi"}))]
        );
    }

    #[test]
    fn call_log_key_is_also_accepted() {
        let r = normalize_post_prompt(&json!({"call_log": [{"role": "user", "content": "hi"}]}));
        assert_eq!(r.dialogue.len(), 1);
    }

    #[test]
    fn junk_body_yields_empty_fields() {
        for junk in [Value::Null, json!("text"), json!(42), json!([])] {
            let r = normalize_post_prompt(&junk);
            assert_eq!(r.medium, "");
            assert!(r.summary.is_empty());
            assert!(r.dialogue.is_empty());
        }
    }

    #[test]
    fn raw_is_preserved() {
        let body = json!({"conversation_type": "voice", "extra": "kept"});
        assert_eq!(Value::Object(normalize_post_prompt(&body).raw), body);
    }
}
