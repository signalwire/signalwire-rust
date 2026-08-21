//! `hangup.reason` is a CLOSED set of six values, because that is what the
//! engine enforces. The contract is stated once, in C, at
//! `mod_infrastructure/relay_apis.c:1105`:
//!
//! ```text
//! JSON_CHECK_STRING_MATCHES_OPTIONAL(reason, "hangup,cancel,busy,noAnswer,decline,error")
//! ```
//!
//! and a non-match is a hard reject (libks `ks_json_check.h` sets `*error_msg`
//! and returns 0). The SWML layer types the field as a bare string
//! (`swml_schema.c:1571`) and `swml.c` forwards it verbatim into the `end` RPC
//! on the same call, so the contract a document must satisfy is the
//! COMPOSITION of the two layers — exactly these six values.
//!
//! This replaces `schema_widen.rs`, which asserted that `no_answer`,
//! `user_hangup` and `timeout` must be accepted. The engine refuses all three,
//! so those rows pinned a bug: the bundled schema listed only
//! `hangup|busy|decline` and carried `x-sdk-widen`, and the SDK stripped the
//! value set before compiling — which accepted the three engine values the
//! schema omitted, but accepted everything else too.
//!
//! Note the engine spells it camelCase `noAnswer`; `no_answer` is not an
//! engine value in any spelling.

use serde_json::json;
use signalwire::SWMLService;
use signalwire::swml::service::ServiceOptions;
use signalwire::utils::SchemaUtils;

/// The six values from `relay_apis.c:1105`, in source order.
const ENGINE_REASONS: [&str; 6] = ["hangup", "cancel", "busy", "noAnswer", "decline", "error"];

/// Every value the engine accepts must validate. `cancel`, `noAnswer` and
/// `error` were absent from the old three-const union and validated only
/// because the widen transform removed the constraint altogether.
#[test]
fn every_engine_reason_validates() {
    let su = SchemaUtils::new(None, true);
    assert!(
        su.full_validation_available(),
        "this test is vacuous without the full validator"
    );
    for reason in ENGINE_REASONS {
        let (ok, errs) = su.validate_verb("hangup", &json!({"reason": reason}));
        assert!(ok, "reason={reason:?} is engine-valid, got {errs:?}");
    }
}

/// The behaviour change, and it is intended: these previously validated.
/// Rejecting locally is STRICTER and correct — the caller finds out at the
/// call site instead of getting an opaque server-side rejection.
#[test]
fn a_reason_the_engine_refuses_is_rejected() {
    let su = SchemaUtils::new(None, true);
    for reason in ["no_answer", "user_hangup", "timeout", "HANGUP", ""] {
        let (ok, _) = su.validate_verb("hangup", &json!({"reason": reason}));
        assert!(
            !ok,
            "reason={reason:?} is refused by relay_apis.c:1105, so the SDK must \
             reject it rather than emit a document the server will fail"
        );
    }
}

/// A non-string is still rejected, so the enum did not become the only check.
#[test]
fn a_wrong_typed_reason_is_rejected() {
    let su = SchemaUtils::new(None, true);
    for reason in [json!(42), json!(true), json!({}), json!([])] {
        let (ok, _) = su.validate_verb("hangup", &json!({"reason": reason}));
        assert!(!ok, "reason={reason} is not a string and must be rejected");
    }
}

/// The end-to-end row: emitting an engine-valid reason must not panic.
/// `SWMLService::add_verb` panics on a validation failure, so this is the path
/// where an over-strict validator would abort a caller's process — which is
/// exactly why the schema had to gain the three missing values BEFORE the
/// relaxation was removed.
#[test]
fn emitting_every_engine_reason_does_not_panic() {
    for reason in ENGINE_REASONS {
        let mut svc = SWMLService::new(ServiceOptions::new("t").route("/t"));
        svc.add_verb("hangup", json!({ "reason": reason }));
        let doc = svc.get_document();
        let main = doc["sections"]["main"]
            .as_array()
            .expect("main section renders");
        assert_eq!(main[0]["hangup"]["reason"], reason);
    }
}

/// Guard the artifact itself, so a re-vendor that reintroduces the three-value
/// union or the marker is caught here rather than only through behaviour.
#[test]
fn the_bundled_schema_publishes_the_engine_values() {
    let su = SchemaUtils::new(None, true);
    let schema = su.load_schema();
    let reason = &schema["$defs"]["Hangup"]["properties"]["hangup"]["properties"]["reason"];

    assert!(
        reason.get(concat!("x", "-sdk-widen")).is_none(),
        "the widen marker must be gone from hangup.reason"
    );
    let listed: Vec<&str> = reason["enum"]
        .as_array()
        .expect("hangup.reason publishes an enum")
        .iter()
        .map(|v| v.as_str().expect("enum values are strings"))
        .collect();
    assert_eq!(listed, ENGINE_REASONS);
}
