//! Typed-error + builder integration tests for the `AgentServer` surface.
//!
//! Exercises the public `AgentServer` API and asserts that configuration
//! failures surface as the right [`ServerError`] *variant* — the server-side
//! analogue of the relay typed-error proof. These operations do real work
//! (route-table mutation, real `fs::canonicalize` of a path), so there is
//! nothing to mock: the failures are genuine.
//!
//! Also exercises the `AgentOptions` fluent **builder** (idiom-pass item 4) by
//! constructing the agents through the chained `with_*` methods.

use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::server::{AgentServer, ServerError};

/// Build an agent via the fluent `AgentOptions` builder (the construction path
/// under test for the builder item).
fn agent(name: &str, route: &str) -> AgentBase {
    AgentBase::new(
        AgentOptions::new(name)
            .route(route)
            .basic_auth("user", "pass")
            .auto_answer(true),
    )
}

// ---------------------------------------------------------------------------
// ServerError::RouteAlreadyRegistered — registering two agents on one route.
// ---------------------------------------------------------------------------

#[test]
fn test_duplicate_route_yields_route_already_registered() {
    let mut server = AgentServer::new(None, Some(3000));
    server
        .register(agent("first", "/bot"), None)
        .expect("first registration succeeds");

    let err = server
        .register(agent("second", "/bot"), None)
        .expect_err("second registration on same route must fail");

    match &err {
        ServerError::RouteAlreadyRegistered { route } => {
            assert_eq!(
                route, "/bot",
                "the conflicting route is carried in the variant"
            );
        }
        other => panic!("expected RouteAlreadyRegistered, got {other:?}"),
    }
    assert!(err.to_string().contains("/bot"));
    assert!(err.to_string().contains("already registered"));

    // The first agent is still the only one registered — the failed register
    // did not mutate the table.
    assert_eq!(server.get_agents(), vec!["/bot"]);
}

// ---------------------------------------------------------------------------
// ServerError::StaticDir — serving a non-existent directory.
// ---------------------------------------------------------------------------

#[test]
fn test_serve_static_missing_dir_yields_static_dir_variant() {
    let mut server = AgentServer::new(None, Some(3000));
    let err = server
        .serve_static("/no/such/dir/zzz-typed-err", "/assets")
        .expect_err("serving a missing dir must fail");

    match &err {
        ServerError::StaticDir { path, reason } => {
            assert_eq!(path, "/no/such/dir/zzz-typed-err");
            assert!(!reason.is_empty(), "a human reason is carried");
        }
        other => panic!("expected StaticDir, got {other:?}"),
    }
    let _: &dyn std::error::Error = &err;
    assert!(err.to_string().contains("zzz-typed-err"));
}

#[test]
fn test_serve_static_on_a_file_is_not_a_directory() {
    // Point at a real *file* (this test source) — exists but is not a dir, so
    // the variant's `reason` distinguishes it from the missing-dir case.
    let this_file = file!();
    let mut server = AgentServer::new(None, Some(3000));
    let result = server.serve_static(this_file, "/assets");
    let err = result.expect_err("serving a file path must fail");
    match &err {
        ServerError::StaticDir { reason, .. } => {
            assert!(
                reason.contains("not a directory"),
                "reason should explain it is not a directory, got: {reason:?}"
            );
        }
        other => panic!("expected StaticDir, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Positive control — a valid registration through the builder returns Ok.
// ---------------------------------------------------------------------------

#[test]
fn test_builder_constructed_agent_registers_ok() {
    let mut server = AgentServer::new(None, Some(8080));
    // The fluent builder produces a fully-configured agent; registration of a
    // fresh route succeeds.
    server
        .register(agent("ok", "/ok"), None)
        .expect("a fresh route registers cleanly");
    assert_eq!(server.get_agents(), vec!["/ok"]);
    // And an explicit override route also works.
    server
        .register(agent("ov", "/ignored"), Some("/override"))
        .expect("override route registers cleanly");
    let mut routes = server.get_agents();
    routes.sort();
    assert_eq!(routes, vec!["/ok", "/override"]);
}
