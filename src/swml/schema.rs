use std::sync::LazyLock;

use serde_json::Value;

/// Metadata about a single SWML verb extracted from the schema.
#[derive(Debug, Clone)]
pub struct VerbInfo {
    pub name: String,
    pub schema_name: String,
}

struct SchemaData {
    verbs: Vec<VerbInfo>,
}

static SCHEMA: LazyLock<SchemaData> = LazyLock::new(|| {
    let raw = include_str!("schema.json");
    let data: Value = serde_json::from_str(raw).expect("schema.json must be valid JSON");

    let defs = match data.get("$defs") {
        Some(d) => d,
        None => return SchemaData { verbs: Vec::new() },
    };

    let any_of = match defs.get("SWMLMethod").and_then(|m| m.get("anyOf")) {
        Some(Value::Array(arr)) => arr,
        _ => return SchemaData { verbs: Vec::new() },
    };

    let mut verbs = Vec::new();

    for entry in any_of {
        let ref_str = match entry.get("$ref").and_then(|r| r.as_str()) {
            Some(s) => s,
            None => continue,
        };

        // e.g. "#/$defs/Answer" -> "Answer"
        let def_name = match ref_str.rsplit('/').next() {
            Some(n) => n,
            None => continue,
        };

        let defn = match defs.get(def_name) {
            Some(d) => d,
            None => continue,
        };

        let props = match defn.get("properties").and_then(|p| p.as_object()) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        // The first property key is the actual verb name
        let actual_verb = match props.keys().next() {
            Some(k) => k.clone(),
            None => continue,
        };

        verbs.push(VerbInfo {
            name: actual_verb,
            schema_name: def_name.to_string(),
        });
    }

    SchemaData { verbs }
});

/// Check whether a verb name is valid.
pub fn is_valid_verb(name: &str) -> bool {
    SCHEMA.verbs.iter().any(|v| v.name == name)
}

/// Get sorted list of all verb names.
pub fn get_verb_names() -> Vec<String> {
    let mut names: Vec<String> = SCHEMA.verbs.iter().map(|v| v.name.clone()).collect();
    names.sort();
    names
}

/// Get verb metadata, or `None` if not found.
pub fn get_verb(name: &str) -> Option<&VerbInfo> {
    SCHEMA.verbs.iter().find(|v| v.name == name)
}

/// Number of verbs defined in the schema.
pub fn verb_count() -> usize {
    SCHEMA.verbs.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verb_count_at_least_38() {
        assert!(
            verb_count() >= 38,
            "Expected at least 38 verbs, got {}",
            verb_count()
        );
    }

    #[test]
    fn test_known_verbs_exist() {
        assert!(is_valid_verb("answer"));
        assert!(is_valid_verb("hangup"));
        assert!(is_valid_verb("play"));
        assert!(is_valid_verb("sleep"));
        assert!(is_valid_verb("ai"));
        assert!(is_valid_verb("connect"));
        assert!(is_valid_verb("record"));
        assert!(is_valid_verb("transfer"));
    }

    #[test]
    fn test_unknown_verb_invalid() {
        assert!(!is_valid_verb("nonexistent_verb"));
        assert!(!is_valid_verb(""));
        assert!(!is_valid_verb("ANSWER")); // case-sensitive
    }

    #[test]
    fn test_get_verb_names_sorted() {
        let names = get_verb_names();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn test_get_verb_returns_info() {
        let info = get_verb("answer").expect("answer should exist");
        assert_eq!(info.name, "answer");
        assert!(!info.schema_name.is_empty());
    }

    #[test]
    fn test_get_verb_returns_none_for_unknown() {
        assert!(get_verb("nonexistent").is_none());
    }

    #[test]
    fn test_all_38_verbs_present() {
        let expected = vec![
            "ai",
            "amazon_bedrock",
            "answer",
            "cond",
            "connect",
            "denoise",
            "detect_machine",
            "enter_queue",
            "execute",
            "goto",
            "hangup",
            "join_conference",
            "join_room",
            "label",
            "live_transcribe",
            "live_translate",
            "pay",
            "play",
            "prompt",
            "receive_fax",
            "record",
            "record_call",
            "request",
            "return",
            "send_digits",
            "send_fax",
            "send_sms",
            "set",
            "sip_refer",
            "sleep",
            "stop_denoise",
            "stop_record_call",
            "stop_tap",
            "switch",
            "tap",
            "transfer",
            "unset",
            "user_event",
        ];
        for verb in &expected {
            assert!(
                is_valid_verb(verb),
                "Expected verb '{}' to be valid",
                verb
            );
        }
    }

    #[test]
    fn test_verb_info_has_schema_name() {
        let info = get_verb("sleep").expect("sleep should exist");
        assert_eq!(info.name, "sleep");
        // schema_name should be the $defs key (e.g. "Sleep")
        assert!(!info.schema_name.is_empty());
    }
}
