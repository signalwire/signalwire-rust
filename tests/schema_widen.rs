//! A schema field marked as widenable enumerates the values the platform
//! DOCUMENTS, not the values it ACCEPTS — the platform takes any value of the
//! same base type. Validating the union as if it were closed rejects documents
//! the platform executes happily, and in this crate that is not a soft reject:
//! every SWML emission routes through `SWMLService::add_verb`, which PANICS on
//! a validation failure, so a legitimate-but-unlisted value would abort the
//! process.
//!
//! `hangup.reason` is the one marked field today. `no_answer` and
//! `user_hangup` are real platform hangup reasons that its documented union
//! does not list.

use serde_json::json;
use signalwire::SWMLService;
use signalwire::swml::service::ServiceOptions;
use signalwire::utils::SchemaUtils;

/// Values outside the documented union but of the right base type: ACCEPT.
#[test]
fn widened_field_accepts_an_unlisted_value_of_the_base_type() {
    let su = SchemaUtils::new(None, true);
    assert!(
        su.full_validation_available(),
        "this test is vacuous without the full validator"
    );
    for reason in ["no_answer", "user_hangup", "cancel", "timeout"] {
        let (ok, errs) = su.validate_verb("hangup", &json!({"reason": reason}));
        assert!(ok, "reason={reason:?} must be accepted, got {errs:?}");
    }
}

/// The documented values keep working — widening relaxes, it does not replace.
#[test]
fn widened_field_still_accepts_the_documented_values() {
    let su = SchemaUtils::new(None, true);
    for reason in ["hangup", "busy", "decline"] {
        let (ok, errs) = su.validate_verb("hangup", &json!({"reason": reason}));
        assert!(ok, "reason={reason:?} must stay accepted, got {errs:?}");
    }
}

/// Widening drops the VALUE constraint, not the TYPE constraint. Recovering
/// the base type is load-bearing: the field declares no `type` of its own,
/// carrying it only inside the union branches, so dropping the union without
/// setting the type would leave it accepting anything at all.
#[test]
fn widened_field_still_rejects_a_wrong_typed_value() {
    let su = SchemaUtils::new(None, true);
    for reason in [json!(42), json!(true), json!({}), json!([])] {
        let (ok, _) = su.validate_verb("hangup", &json!({"reason": reason}));
        assert!(!ok, "reason={reason} is not a string and must be rejected");
    }
}

/// The end-to-end consequence: emitting the verb must not panic.
#[test]
fn emitting_an_unlisted_hangup_reason_does_not_panic() {
    let mut svc = SWMLService::new(ServiceOptions::new("t").route("/t"));
    svc.add_verb("hangup", json!({"reason": "no_answer"}));
    let doc = svc.get_document();
    let main = doc["sections"]["main"]
        .as_array()
        .expect("main section renders");
    assert_eq!(main[0]["hangup"]["reason"], "no_answer");
}
