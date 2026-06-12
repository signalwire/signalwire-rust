//! Integration tests: `AgentBase` auto-mounts webhook signature
//! validation when a Signing Key is configured.
//!
//! Drives the agent's `handle_request` directly with crafted POSTs
//! to `/`, `/swaig`, and `/post_prompt`, asserting:
//!
//! - signed-and-valid POSTs are accepted (status != 403)
//! - signed-but-tampered POSTs return 403 with no handler side-effect
//! - missing header on POST returns 403
//! - GETs are not signature-checked (would break `/health`, `/ready`)
//! - `signing_key` resolves from explicit option THEN env var
//! - `signing_key=None` bypasses validation entirely

use std::collections::HashMap;

use signalwire::agent::AgentBase;
use signalwire::AgentOptions;

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

const KEY: &str = "PSKtest1234567890abcdef";

// Build a minimal AgentBase with the given signing key.
fn make_agent(signing_key: Option<&str>) -> AgentBase {
    let mut opts = AgentOptions::new("test-agent");
    opts.basic_auth_user = Some("user".into());
    opts.basic_auth_password = Some("pass".into());
    // Pin the proxy base via SWML_PROXY_URL_BASE so URL reconstruction
    // is deterministic in this test (otherwise it would fall back to
    // `http://0.0.0.0:3000`, which makes signing values cumbersome).
    if signing_key.is_some() {
        // Set per-test below so we can sweep.
    }
    opts.signing_key = signing_key.map(std::string::ToString::to_string);
    AgentBase::new(opts)
}

fn auth_headers() -> HashMap<String, String> {
    let mut h = HashMap::new();
    let token = base64::engine::general_purpose::STANDARD.encode("user:pass");
    h.insert("Authorization".into(), format!("Basic {token}"));
    h
}

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

// Pin the proxy base used by Service::get_proxy_url_base via env.
// Tests that share this fixture must run serially because they
// mutate process-global state (which run-ci.sh does via
// `--test-threads=1`).
fn with_proxy_base<F: FnOnce()>(base: &str, f: F) {
    // SAFETY: tests are single-threaded under run-ci.sh; we just set
    // the env var, run the test, and restore.
    let prev = std::env::var("SWML_PROXY_URL_BASE").ok();
    unsafe {
        std::env::set_var("SWML_PROXY_URL_BASE", base);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    match prev {
        Some(v) => unsafe { std::env::set_var("SWML_PROXY_URL_BASE", v) },
        None => unsafe { std::env::remove_var("SWML_PROXY_URL_BASE") },
    }
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

// ---------------------------------------------------------------------------
//  Tests
// ---------------------------------------------------------------------------

#[test]
fn signed_post_to_root_is_accepted() {
    with_proxy_base("https://agent.example.com", || {
        let agent = make_agent(Some(KEY));
        let body = ""; // GET-style SWML render with empty body
        let url = "https://agent.example.com/";
        let sig = hex_sig(KEY, url, body);

        let mut headers = auth_headers();
        headers.insert("X-SignalWire-Signature".into(), sig);

        let (status, _, _) = agent.handle_request("POST", "/", &headers, body);
        // Should NOT be 403 — should pass through to handle_swml_request.
        assert_ne!(status, 403, "valid signature must not 403");
        // SWML render returns 200.
        assert_eq!(status, 200);
    });
}

#[test]
fn unsigned_post_to_root_is_rejected_403() {
    with_proxy_base("https://agent.example.com", || {
        let agent = make_agent(Some(KEY));
        let headers = auth_headers();
        let (status, _, _) = agent.handle_request("POST", "/", &headers, "");
        assert_eq!(status, 403);
    });
}

#[test]
fn tampered_post_body_is_rejected_403() {
    with_proxy_base("https://agent.example.com", || {
        let agent = make_agent(Some(KEY));
        let body = r#"{"call_id":"abc-123"}"#;
        let url = "https://agent.example.com/swaig";
        let sig = hex_sig(KEY, url, body);

        let mut headers = auth_headers();
        headers.insert("X-SignalWire-Signature".into(), sig);

        // Tamper the body before sending — signature no longer matches.
        let tampered = r#"{"call_id":"OTHER"}"#;
        let (status, _, _) = agent.handle_request("POST", "/swaig", &headers, tampered);
        assert_eq!(status, 403);
    });
}

#[test]
fn signed_post_to_swaig_dispatches_to_handler() {
    with_proxy_base("https://agent.example.com", || {
        let agent = make_agent(Some(KEY));
        // Empty function body → /swaig returns 400 ("missing function").
        // What we care about here is that the signature check passed.
        let body = "{}";
        let url = "https://agent.example.com/swaig";
        let sig = hex_sig(KEY, url, body);

        let mut headers = auth_headers();
        headers.insert("X-SignalWire-Signature".into(), sig);

        let (status, _, _) = agent.handle_request("POST", "/swaig", &headers, body);
        assert_ne!(status, 403, "valid signature must not 403");
        // 400 because "{}" has no function name; that's fine — it
        // reached the handler, which is the assertion we care about.
        assert!(
            status == 200 || status == 400,
            "expected 200 or 400, got {status}"
        );
    });
}

#[test]
fn signed_post_to_post_prompt_is_accepted() {
    with_proxy_base("https://agent.example.com", || {
        let agent = make_agent(Some(KEY));
        let body = r#"{"summary":"call done"}"#;
        let url = "https://agent.example.com/post_prompt";
        let sig = hex_sig(KEY, url, body);

        let mut headers = auth_headers();
        headers.insert("X-SignalWire-Signature".into(), sig);

        let (status, _, _) = agent.handle_request("POST", "/post_prompt", &headers, body);
        assert_eq!(status, 200);
    });
}

#[test]
fn get_requests_are_not_signature_checked() {
    with_proxy_base("https://agent.example.com", || {
        // Even with a signing key configured, GET / (the SWML render
        // endpoint) must not require a signature. Same for /health.
        let agent = make_agent(Some(KEY));
        let headers = auth_headers();

        let (status, _, _) = agent.handle_request("GET", "/", &headers, "");
        assert_eq!(status, 200);

        // /health doesn't even hit auth; works without anything.
        let empty_headers = HashMap::new();
        let (status, _, _) = agent.handle_request("GET", "/health", &empty_headers, "");
        assert_eq!(status, 200);
    });
}

#[test]
fn no_signing_key_means_no_validation() {
    with_proxy_base("https://agent.example.com", || {
        // Without a signing key, unsigned POSTs are accepted (legacy
        // behaviour with a startup warning logged at construction).
        let agent = make_agent(None);
        let headers = auth_headers();
        let (status, _, _) = agent.handle_request("POST", "/", &headers, "");
        assert_ne!(status, 403, "no-key mode must not 403");
        assert_eq!(status, 200);
    });
}

#[test]
fn signing_key_falls_back_to_env_var() {
    // Leak isolation: this test runs serially via --test-threads=1.
    let prev_key = std::env::var("SIGNALWIRE_SIGNING_KEY").ok();
    let prev_base = std::env::var("SWML_PROXY_URL_BASE").ok();
    unsafe {
        std::env::set_var("SIGNALWIRE_SIGNING_KEY", "env-key-fallback");
        std::env::set_var("SWML_PROXY_URL_BASE", "https://agent.example.com");
    }

    let result = std::panic::catch_unwind(|| {
        // No explicit signing_key on the options — must pick up env.
        let mut opts = AgentOptions::new("env-test");
        opts.basic_auth_user = Some("user".into());
        opts.basic_auth_password = Some("pass".into());
        let agent = AgentBase::new(opts);

        assert_eq!(agent.signing_key(), Some("env-key-fallback"));

        let body = "";
        let url = "https://agent.example.com/";
        let sig = hex_sig("env-key-fallback", url, body);

        let mut headers = auth_headers();
        headers.insert("X-SignalWire-Signature".into(), sig);
        let (status, _, _) = agent.handle_request("POST", "/", &headers, body);
        assert_eq!(status, 200, "env-key signature must validate");
    });

    // Restore env regardless of result.
    unsafe {
        match prev_key {
            Some(v) => std::env::set_var("SIGNALWIRE_SIGNING_KEY", v),
            None => std::env::remove_var("SIGNALWIRE_SIGNING_KEY"),
        }
        match prev_base {
            Some(v) => std::env::set_var("SWML_PROXY_URL_BASE", v),
            None => std::env::remove_var("SWML_PROXY_URL_BASE"),
        }
    }
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

#[test]
fn explicit_option_overrides_env_var() {
    let prev = std::env::var("SIGNALWIRE_SIGNING_KEY").ok();
    unsafe {
        std::env::set_var("SIGNALWIRE_SIGNING_KEY", "from-env");
    }

    let result = std::panic::catch_unwind(|| {
        let mut opts = AgentOptions::new("override-test");
        opts.signing_key = Some("from-options".into());
        let agent = AgentBase::new(opts);
        assert_eq!(agent.signing_key(), Some("from-options"));
    });

    unsafe {
        match prev {
            Some(v) => std::env::set_var("SIGNALWIRE_SIGNING_KEY", v),
            None => std::env::remove_var("SIGNALWIRE_SIGNING_KEY"),
        }
    }
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

#[test]
fn empty_signing_key_is_ignored() {
    // Empty string in either source must be treated as "no key".
    let prev = std::env::var("SIGNALWIRE_SIGNING_KEY").ok();
    unsafe {
        std::env::remove_var("SIGNALWIRE_SIGNING_KEY");
    }

    let result = std::panic::catch_unwind(|| {
        let mut opts = AgentOptions::new("empty-key-test");
        opts.signing_key = Some(String::new());
        let agent = AgentBase::new(opts);
        assert_eq!(agent.signing_key(), None);
    });

    unsafe {
        if let Some(v) = prev {
            std::env::set_var("SIGNALWIRE_SIGNING_KEY", v);
        }
    }
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

#[test]
fn x_twilio_signature_header_is_also_accepted_on_agent() {
    // Cxml/legacy path: agents may also receive X-Twilio-Signature.
    with_proxy_base("https://agent.example.com", || {
        let agent = make_agent(Some(KEY));
        let body = "";
        let url = "https://agent.example.com/";
        let sig = hex_sig(KEY, url, body);

        let mut headers = auth_headers();
        headers.insert("X-Twilio-Signature".into(), sig);
        let (status, _, _) = agent.handle_request("POST", "/", &headers, body);
        assert_eq!(status, 200);
    });
}

#[test]
fn set_signing_key_after_construction_works() {
    with_proxy_base("https://agent.example.com", || {
        let mut agent = make_agent(None);
        agent.set_signing_key(Some("late-binding"));
        assert_eq!(agent.signing_key(), Some("late-binding"));

        // Now an unsigned POST should be rejected.
        let headers = auth_headers();
        let (status, _, _) = agent.handle_request("POST", "/", &headers, "");
        assert_eq!(status, 403);
    });
}
