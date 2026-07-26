//! Construction-contract tests for `AgentOptions` / `ServiceOptions`.
//!
//! Ported from the Python reference's `tests/unit/core/test_agent_base.py`
//! and `test_swml_service.py`. The reference's `AgentBase.__init__` FORWARDS
//! several params to collaborators rather than storing them on `self`:
//!
//! - `schema_path` / `config_file` / `schema_validation` → `SWMLService.__init__`
//!   (`agent_base.py:205-207`), which in turn hands `schema_path` +
//!   `schema_validation` to `SchemaUtils` and `config_file` to `SecurityConfig`;
//! - `token_expiry_secs` → `SessionManager(...)` (`agent_base.py:247`).
//!
//! These tests exercise the REAL behavior each param buys (validation actually
//! off, config file actually read, token actually expiring), not the presence
//! of a field.

use std::collections::HashMap;

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha1::Sha1;

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::swaig::FunctionResult;
use signalwire::swml::service::{Service, ServiceOptions};

type HmacSha1 = Hmac<Sha1>;

/// Write a scratch file under the repo-local `.sw-tmp/` (never `/tmp`), keyed
/// by a caller-supplied unique name so parallel tests never collide.
fn write_scratch(unique: &str, contents: &str) -> String {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push(".sw-tmp");
    dir.push("construction-contract");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join(unique);
    std::fs::write(&path, contents).expect("write scratch file");
    path.to_string_lossy().into_owned()
}

fn no_headers() -> HashMap<String, String> {
    HashMap::new()
}

fn echo_tool(agent: &mut AgentBase) {
    agent.define_tool(
        "t",
        "t",
        json!({}),
        Box::new(|_, _| FunctionResult::with_response("ok")),
        false,
    );
}

// ── schema_validation ────────────────────────────────────────────────────

/// Reference: `AgentBase(..., schema_validation=False)` is used throughout
/// `test_agent_base.py` precisely so a verb config the schema would reject
/// still renders. With validation ON, `add_verb` fails loudly.
#[test]
fn schema_validation_false_accepts_a_config_validation_would_reject() {
    let mut agent = AgentBase::new(AgentOptions::new("novalidate").schema_validation(false));
    // `answer` has no `zzz_unknown_key` in the schema; with validation on this
    // is a hard failure (the schema's verb objects are closed).
    let ok = agent.add_verb("answer", json!({"zzz_unknown_key": 1}));
    assert!(
        ok,
        "schema_validation=false must let an otherwise-invalid verb through"
    );
}

/// The default is validation ON — the same config must be rejected.
#[test]
#[should_panic(expected = "Schema validation failed")]
fn schema_validation_defaults_on_and_rejects() {
    let mut agent = AgentBase::new(AgentOptions::new("validate"));
    agent.add_verb("answer", json!({"zzz_unknown_key": 1}));
}

/// Same contract one layer down, on `Service` — the reference forwards
/// `schema_validation` from `AgentBase.__init__` into `SWMLService.__init__`,
/// so a bare `Service` must honor it too.
#[test]
fn service_schema_validation_false_accepts_invalid_verb() {
    let mut svc = Service::new(ServiceOptions::new("svc-novalidate").schema_validation(false));
    assert!(svc.add_verb("answer", json!({"zzz_unknown_key": 1})));
}

/// `schema_validation` reaches `SchemaUtils`, whose `full_validation_available`
/// is false when validation is off (reference `SchemaUtils.__init__`).
#[test]
fn schema_validation_reaches_schema_utils() {
    let on = AgentBase::new(AgentOptions::new("su-on"));
    assert!(on.schema_utils().full_validation_available());

    let off = AgentBase::new(AgentOptions::new("su-off").schema_validation(false));
    assert!(!off.schema_utils().full_validation_available());
}

// ── schema_path ──────────────────────────────────────────────────────────

/// `schema_path` is forwarded to `SchemaUtils(schema_path=…)`, which loads the
/// schema from that path instead of the embedded default. Point it at a
/// minimal schema and prove the loaded verb set comes from the FILE.
#[test]
fn schema_path_loads_the_named_schema_file() {
    // Same shape as the real `schema.json`: `$defs.SWMLMethod.anyOf` is a list
    // of `$ref`s into `$defs`, and each referenced def's first `properties`
    // key is the verb name.
    let schema = json!({
        "$defs": {
            "SWMLMethod": {
                "anyOf": [{"$ref": "#/$defs/OnlyVerbInThisFile"}]
            },
            "OnlyVerbInThisFile": {
                "type": "object",
                "properties": {"only_verb_in_this_file": {"type": "object"}}
            }
        }
    });
    let path = write_scratch("mini_schema.json", &schema.to_string());

    let agent = AgentBase::new(AgentOptions::new("schemapath").schema_path(&path));
    let verbs = agent.schema_utils().get_all_verb_names();
    assert!(
        verbs.contains(&"only_verb_in_this_file".to_string()),
        "schema_path must load the named file; got verbs {verbs:?}"
    );
    assert!(
        !verbs.contains(&"answer".to_string()),
        "the embedded schema must NOT be used when schema_path is set"
    );
}

// ── config_file ──────────────────────────────────────────────────────────

/// Reference `AgentBase._load_service_config(config_file, name)` reads the
/// file's `service` section and applies `name`/`route`/`host`/`port` with the
/// CONSTRUCTOR params taking precedence (`agent_base.py:191-196`).
#[test]
fn config_file_supplies_route_host_port_and_name() {
    let cfg = json!({
        "service": {
            "name": "from-config",
            "route": "/from-config",
            "host": "127.0.0.1",
            "port": 4321
        }
    });
    let path = write_scratch("service_config.json", &cfg.to_string());

    let agent = AgentBase::new(AgentOptions::new("ctor-name").config_file(&path));
    assert_eq!(agent.name(), "from-config");
    assert_eq!(agent.route(), "/from-config");
    assert_eq!(agent.host(), "127.0.0.1");
    assert_eq!(agent.port(), 4321);
}

/// Constructor params WIN over the config file — the reference only consults
/// the config when the param is still at its default (`route == "/"`,
/// `host == "0.0.0.0"`, `port is None`).
#[test]
fn explicit_ctor_params_beat_the_config_file() {
    let cfg = json!({
        "service": {"route": "/from-config", "host": "127.0.0.1", "port": 4321}
    });
    let path = write_scratch("service_config_precedence.json", &cfg.to_string());

    let agent = AgentBase::new(
        AgentOptions::new("prec")
            .config_file(&path)
            .route("/explicit")
            .host("10.0.0.1")
            .port(9999),
    );
    assert_eq!(agent.route(), "/explicit");
    assert_eq!(agent.host(), "10.0.0.1");
    assert_eq!(agent.port(), 9999);
}

/// `config_file` also reaches `SecurityConfig(config_file=…)` in the reference
/// (`swml_service.py:139`) — a `security.basic_auth` section supplies the
/// credentials when none were passed explicitly.
#[test]
fn config_file_supplies_basic_auth_credentials() {
    let cfg = json!({
        "service": {"name": "cfgauth"},
        "security": {
            "auth": {"basic": {"user": "cfg_user", "password": "cfg_password"}}
        }
    });
    let path = write_scratch("security_config.json", &cfg.to_string());

    let agent = AgentBase::new(AgentOptions::new("cfgauth").config_file(&path));
    let (user, password) = agent.basic_auth_credentials();
    assert_eq!(user, "cfg_user");
    assert_eq!(password, "cfg_password");
}

// ── token_expiry_secs ────────────────────────────────────────────────────

/// `token_expiry_secs` is forwarded to `SessionManager(token_expiry_secs=…)`
/// (`agent_base.py:247`). The minted token carries `now + token_expiry_secs`
/// as its expiry field, so the constructor param is observable on the token
/// itself — no sleep needed to prove the forward happened.
#[test]
fn token_expiry_secs_reaches_the_session_manager() {
    /// Decode the base64url token and read its `{call_id}.{fn}.{expiry}.…` field.
    ///
    /// PADDED `URL_SAFE`, matching the reference: it mints with
    /// `base64.urlsafe_b64encode` (which pads) and validates with
    /// `base64.urlsafe_b64decode` (which REQUIRES padding). This test previously used
    /// `URL_SAFE_NO_PAD` — encoding rust's own wrong convention, so it passed while the
    /// tokens were unusable to every other implementation. A test that decodes with the
    /// same non-standard engine the mint used can never catch an encoding divergence.
    fn token_expiry(token: &str) -> u64 {
        let raw = base64::engine::general_purpose::URL_SAFE
            .decode(token)
            .expect("token is padded base64url, as the reference mints it");
        let decoded = String::from_utf8(raw).expect("token is utf-8");
        decoded
            .split('.')
            .nth(2)
            .expect("expiry field")
            .parse()
            .expect("expiry is numeric")
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut short = AgentBase::new(AgentOptions::new("tok").token_expiry_secs(60));
    echo_tool(&mut short);
    let expiry = token_expiry(&short.create_tool_token("t", "call-1"));
    assert!(
        (now + 55..=now + 65).contains(&expiry),
        "token_expiry_secs=60 must mint a token expiring ~60s out; got {expiry} vs now {now}"
    );

    // Default: the reference's `token_expiry_secs=3600`.
    let mut default = AgentBase::new(AgentOptions::new("tok-default"));
    echo_tool(&mut default);
    let expiry = token_expiry(&default.create_tool_token("t", "call-1"));
    assert!(
        (now + 3595..=now + 3605).contains(&expiry),
        "the default token_expiry_secs must be the reference's 3600; got {expiry} vs now {now}"
    );

    // And the token still validates on the long path.
    assert!(default.validate_tool_token("t", &default.create_tool_token("t", "call-1"), "call-1"));
}

// ── agent_id ─────────────────────────────────────────────────────────────

/// Reference `test_auto_generated_agent_id` / `test_custom_agent_id`.
#[test]
fn agent_id_defaults_to_a_uuid_and_honors_an_override() {
    let auto = AgentBase::new(AgentOptions::new("id_test"));
    let generated = auto.agent_id().to_string();
    assert_eq!(
        generated.len(),
        36,
        "auto agent_id must be a UUID, got {generated:?}"
    );
    assert_eq!(generated.matches('-').count(), 4);
    // Two agents must not share an id.
    let other = AgentBase::new(AgentOptions::new("id_test"));
    assert_ne!(generated, other.agent_id());

    let custom = AgentBase::new(AgentOptions::new("id_test").agent_id("custom-123"));
    assert_eq!(custom.agent_id(), "custom-123");
}

// ── record_format / record_stereo ────────────────────────────────────────

fn find_verb(doc: &Value, verb: &str) -> Value {
    doc["sections"]["main"]
        .as_array()
        .expect("main section")
        .iter()
        .find(|v| v.get(verb).is_some())
        .unwrap_or_else(|| panic!("{verb} verb present"))
        .clone()
}

/// Reference `test_render_swml_record_call_format`: `record_format` /
/// `record_stereo` land on the rendered `record_call` verb.
#[test]
fn record_format_and_stereo_render_onto_the_record_call_verb() {
    let agent = AgentBase::new(
        AgentOptions::new("rec")
            .record_call(true)
            .record_format("wav")
            .record_stereo(false),
    );
    let doc = agent.render_swml(&no_headers());
    let rec = find_verb(&doc, "record_call");
    assert_eq!(rec["record_call"]["format"], json!("wav"));
    assert_eq!(rec["record_call"]["stereo"], json!(false));
}

/// The reference DEFAULTS are `record_format="mp4"`, `record_stereo=True`
/// (`agent_base.py:131-132`) — a port that defaults to `wav`/`false` emits a
/// different recording on the wire for the identical reference program.
#[test]
fn record_format_and_stereo_defaults_match_the_reference() {
    let agent = AgentBase::new(AgentOptions::new("recdef").record_call(true));
    let doc = agent.render_swml(&no_headers());
    let rec = find_verb(&doc, "record_call");
    assert_eq!(rec["record_call"]["format"], json!("mp4"));
    assert_eq!(rec["record_call"]["stereo"], json!(true));
}

// ── native_functions ─────────────────────────────────────────────────────

/// Reference `test_render_swml_with_native_functions`.
#[test]
fn native_functions_render_into_swaig() {
    let agent = AgentBase::new(
        AgentOptions::new("nf").native_functions(vec!["transfer".into(), "check_time".into()]),
    );
    let doc = agent.render_swml(&no_headers());
    let ai = find_verb(&doc, "ai");
    let native = ai["ai"]["SWAIG"]["native_functions"]
        .as_array()
        .expect("native_functions array")
        .clone();
    assert!(native.contains(&json!("transfer")));
    assert!(native.contains(&json!("check_time")));
}

/// Reference `test_initialization_with_default_params`: `native_functions`
/// defaults to the empty list.
#[test]
fn native_functions_defaults_to_empty() {
    let agent = AgentBase::new(AgentOptions::new("nf-default"));
    assert!(agent.native_functions().is_empty());
}

// ── default_webhook_url ──────────────────────────────────────────────────

/// `default_webhook_url` is stored on the agent as `_default_webhook_url`
/// (`agent_base.py:225`). Verified against the reference source: that attribute
/// is written by `__init__` and read NOWHERE — the SWAIG `defaults.web_hook_url`
/// the renderer emits is computed from `_build_webhook_url("swaig", …)` plus
/// `_web_hook_url_override` (`agent_base.py:972-979`), not from this param. So
/// the contract is store-and-expose; asserting a render effect here would
/// invent behavior the reference does not have.
#[test]
fn default_webhook_url_is_stored_and_readable() {
    assert_eq!(
        AgentBase::new(AgentOptions::new("dwu").default_webhook_url("https://example.com/swaig"))
            .default_webhook_url(),
        Some("https://example.com/swaig")
    );
    assert_eq!(
        AgentBase::new(AgentOptions::new("dwu")).default_webhook_url(),
        None
    );
}

// ── suppress_logs ────────────────────────────────────────────────────────

#[test]
fn suppress_logs_is_readable_and_defaults_off() {
    assert!(!AgentBase::new(AgentOptions::new("sl")).suppress_logs());
    assert!(AgentBase::new(AgentOptions::new("sl").suppress_logs(true)).suppress_logs());
}

// ── enable_post_prompt_override / check_for_input_override ───────────────

#[test]
fn post_prompt_and_check_for_input_overrides_round_trip() {
    let d = AgentBase::new(AgentOptions::new("ov"));
    assert!(!d.enable_post_prompt_override());
    assert!(!d.check_for_input_override());

    let on = AgentBase::new(
        AgentOptions::new("ov")
            .enable_post_prompt_override(true)
            .check_for_input_override(true),
    );
    assert!(on.enable_post_prompt_override());
    assert!(on.check_for_input_override());
}

// ── trust_proxy_for_signature ────────────────────────────────────────────

fn hex_sig(key: &str, url: &str, body: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(key.as_bytes()).unwrap();
    mac.update(format!("{url}{body}").as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Reference: `trust_proxy_for_signature` defaults FALSE — "proxy headers are
/// spoofable, so opt in only when you control the proxy chain"
/// (`agent_base.py` docstring). With it off, a spoofed `X-Forwarded-Host` must
/// NOT change the URL the signature is checked against, so a signature minted
/// for the spoofed host is REJECTED. With it on, the same request passes.
#[test]
fn trust_proxy_for_signature_gates_forwarded_headers() {
    const KEY: &str = "PSKtest1234567890abcdef";
    let body = json!({"call_id": "abc"}).to_string();

    // A signature the attacker computed for the host they injected.
    let spoofed_sig = hex_sig(KEY, "https://attacker.example/swaig", &body);

    let mut headers = HashMap::new();
    headers.insert("X-Forwarded-Proto".to_string(), "https".to_string());
    headers.insert(
        "X-Forwarded-Host".to_string(),
        "attacker.example".to_string(),
    );
    headers.insert("X-SignalWire-Signature".to_string(), spoofed_sig);
    headers.insert(
        "Authorization".to_string(),
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("user:pass")
        ),
    );

    let opts = |trust: bool| {
        AgentOptions::new("np")
            .basic_auth("user", "pass")
            .signing_key(KEY)
            .trust_proxy_for_signature(trust)
    };

    // Default (trust_proxy_for_signature = false): forwarded headers ignored,
    // so the reconstructed URL is the agent's own host → signature mismatch.
    let untrusting = AgentBase::new(opts(false));
    let (status, _, _) = untrusting.handle_request("POST", "/swaig", &headers, &body);
    assert_eq!(
        status, 403,
        "spoofable X-Forwarded-* must NOT be honored by default"
    );

    // Opt in: the forwarded pair IS honored, so the same signature validates.
    let trusting = AgentBase::new(opts(true));
    let (status, _, _) = trusting.handle_request("POST", "/swaig", &headers, &body);
    assert_ne!(
        status, 403,
        "trust_proxy_for_signature=true must honor X-Forwarded-*"
    );
}
