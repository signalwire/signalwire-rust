//! Reading what a client says it can do.
//!
//! A browser client — the SignalWire address widget, or anything speaking the
//! same convention — declares its rendering capabilities in the user variables
//! it sends at dial time:
//!
//! ```json
//! {"vars": {"userVariables": {
//!     "capabilities": {"display_content": true, "transcript": true, "chat_handoff": false},
//!     "metadata": {"page": {}, "client": {}, "widget": {}}
//! }}}
//! ```
//!
//! **These are declarations of what the client can RENDER, not grants of
//! authority.** Treat them as hints for deciding what to offer — never as
//! permission to do anything privileged; a caller controls its own user
//! variables.
//!
//! **Absence means no.** Every function here resolves errors and missing data to
//! "not declared": offering a caller something they cannot reach (a PSTN caller
//! has no browser) is worse than never mentioning it.
//!
//! Deliberately not provided: an enum of known capability names (a client must
//! be able to declare something this SDK has never heard of), or any wiring of
//! capabilities to tools (that is application policy).

use std::collections::BTreeSet;

use serde_json::{Map, Value};

/// Return the user variables from a SWML request body (`vars.userVariables`),
/// or an empty object when any level is missing or not an object.
#[must_use]
pub fn user_variables(body_params: &Value) -> Map<String, Value> {
    body_params
        .get("vars")
        .and_then(|v| v.get("userVariables"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// Python truthiness of a declared capability value.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// The capability names the client declared as truthy.
///
/// Accepts either a full SWML request body or an already-extracted user
/// variables object, so it works from a dynamic-config callback and from a
/// SWAIG handler alike. Empty when nothing was declared, the payload was
/// malformed, or the client is not a browser at all.
#[must_use]
pub fn declared_capabilities(body_params: &Value) -> BTreeSet<String> {
    let mut variables = user_variables(body_params);
    if variables.is_empty()
        && let Some(direct) = body_params.as_object()
    {
        // Already-extracted user variables were passed directly.
        variables.clone_from(direct);
    }
    match variables.get("capabilities") {
        Some(Value::Object(caps)) => caps
            .iter()
            .filter(|(_, v)| truthy(v))
            .map(|(k, _)| k.clone())
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// Whether the client declared `name` — true only when explicitly declared
/// truthy.
#[must_use]
pub fn has_capability(body_params: &Value, name: &str) -> bool {
    declared_capabilities(body_params).contains(name)
}

#[cfg(test)]
mod tests {
    //! Ported from signalwire-python `tests/unit/core/test_capabilities.py`.
    use super::*;
    use serde_json::json;

    fn body() -> Value {
        json!({"vars": {"userVariables": {
            "capabilities": {"display_content": true, "transcript": true, "chat_handoff": false},
            "metadata": {"widget": {"opened_at": "2026-01-01T00:00:00Z"}},
        }}})
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn extracts_from_the_nested_shape() {
        assert!(user_variables(&body()).contains_key("capabilities"));
    }

    #[test]
    fn missing_levels_yield_an_empty_object() {
        for junk in [
            Value::Null,
            json!({}),
            json!("nonsense"),
            json!(42),
            json!({"vars": null}),
            json!({"vars": {}}),
            json!({"vars": {"userVariables": null}}),
            json!({"vars": {"userVariables": "not a dict"}}),
        ] {
            assert!(user_variables(&junk).is_empty(), "{junk}");
        }
    }

    #[test]
    fn only_truthy_names_are_returned() {
        assert_eq!(
            declared_capabilities(&body()),
            set(&["display_content", "transcript"])
        );
        assert!(!declared_capabilities(&body()).contains("chat_handoff"));
    }

    #[test]
    fn accepts_already_extracted_user_variables() {
        assert_eq!(
            declared_capabilities(&json!({"capabilities": {"a": true}})),
            set(&["a"])
        );
    }

    #[test]
    fn unknown_names_pass_through() {
        assert!(has_capability(
            &json!({"capabilities": {"future_thing": true}}),
            "future_thing"
        ));
    }

    #[test]
    fn absence_and_malformation_both_mean_no() {
        for junk in [
            Value::Null,
            json!({}),
            json!("nonsense"),
            json!(42),
            json!({"vars": {"userVariables": {"capabilities": "not a dict"}}}),
            json!({"vars": {"userVariables": {"capabilities": null}}}),
            json!({"vars": {"userVariables": {}}}),
        ] {
            assert!(declared_capabilities(&junk).is_empty(), "{junk}");
            assert!(!has_capability(&junk, "display_content"));
        }
    }

    #[test]
    fn has_capability_cases() {
        assert!(has_capability(&body(), "display_content"));
        assert!(!has_capability(&body(), "chat_handoff"));
        assert!(!has_capability(&body(), "telepathy"));
    }
}
