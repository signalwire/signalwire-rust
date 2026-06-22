// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Shared REST helpers used across every namespace.
//!
//! These were previously copy-pasted into ~9 namespace modules; factored here
//! so there is one definition of each.

use std::collections::HashMap;

use serde_json::Value;

/// Join path segments with `/`. (Thin wrapper kept for call-site readability.)
pub(crate) fn join(parts: &[&str]) -> String {
    parts.join("/")
}

/// Flatten a JSON object into a `{string: string}` query-parameter map.
///
/// String values pass through; `null` values are dropped; any other value is
/// rendered with its JSON `to_string()`. A non-object `params` yields an empty
/// map. Mirrors the Python reference's `params or None` query handling.
pub(crate) fn params_to_string_map(params: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(obj) = params.as_object() {
        for (k, v) in obj {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Null => continue,
                other => other.to_string(),
            };
            out.insert(k.clone(), s);
        }
    }
    out
}
