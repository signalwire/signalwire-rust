//! # SignalWire AI Agents SDK
//!
//! Build, serve, and drive AI voice/messaging agents on the
//! [SignalWire](https://signalwire.com) platform. This crate is the Rust port of
//! the SignalWire AI Agents framework — it retains 100% of the reference
//! functionality, expressed in Rust idioms (builders, traits, `Result`).
//!
//! The library is published as [`signalwire-sdk`](https://crates.io/crates/signalwire-sdk);
//! the import path is `signalwire`.
//!
//! ## What you can build
//!
//! - **Agents** ([`agent`]) — an [`agent::AgentBase`] composes prompts, SWAIG
//!   tools, skills, and AI config, then serves the 5-phase SWML pipeline over
//!   HTTP.
//! - **SWML documents** ([`swml`]) — construct SignalWire Markup Language call
//!   flows programmatically; 38 verb builders generated from the schema.
//! - **SWAIG tools** ([`swaig`]) — define server-callable tool functions whose
//!   results are built with the fluent [`swaig::SwaigFunctionResult`].
//! - **Real-time call control** ([`relay`]) — a synchronous RELAY WebSocket
//!   client (Blade / JSON-RPC 2.0) for dialing, messaging, and event handling.
//! - **REST APIs** ([`rest`]) — a synchronous namespaced REST client
//!   ([`rest::RestClient`]) over Fabric, Calling, Video, Messaging, and more,
//!   with `links.next` cursor pagination.
//! - **Skills & prefabs** ([`skills`], [`prefabs`]) — pluggable capabilities and
//!   ready-made agent archetypes.
//!
//! ## Quick start
//!
//! ```no_run
//! use signalwire::agent::{AgentBase, AgentOptions};
//! use serde_json::json;
//!
//! let mut agent = AgentBase::new(AgentOptions::new("my-agent"));
//! agent
//!     .add_language("English", "en-US", "rime.spore")
//!     .prompt_add_section("Role", "You are a helpful assistant.", vec![])
//!     .set_prompt_llm_params(json!({ "temperature": 0.7 }));
//! // agent.run(); // serves the SWML/SWAIG endpoints (blocks)
//! ```
//!
//! ## Error handling
//!
//! Fallible operations return [`Result`]; each subsystem defines a typed error
//! (e.g. [`rest::SignalWireRestError`], the RELAY and skill error enums). No
//! panics on the happy path — construction, parsing, and I/O surface failures as
//! `Err`.
//!
//! ## Design & idioms
//!
//! Class inheritance in the reference maps to Rust **traits**; constructor-with-
//! subclassing maps to **builders** (`Options` + `new` + chained `&mut self`).
//! Shared mutable state is `Arc`-wrapped; every JSON-crossing type derives serde.
//! See `PORT_PHILOSOPHY_RUST.md` in the repository for the full rationale.

// `needless_pass_by_value` is allowed crate-wide as a deliberate parity choice.
// This port's public constructors and builders take owned `Value`, `Vec<_>`,
// `HashMap<_>`, and `String` params because they mirror Python's by-value
// `**kwargs` / positional-list / keyword arguments — the shape the
// cross-language signature audit maps (var_keyword / positional). Converting
// these to `&T` to satisfy the lint would distort the parity surface the audit
// checks, so we keep the owned signatures (the parity meta-rule: a pedantic
// lint that fights parity is allowed, not obeyed). The few internal/test sites
// the lint also flags (e.g. functions that genuinely consume the value) are
// consuming-by-design, so a blanket allow loses nothing real.
#![allow(clippy::needless_pass_by_value)]
// `too_many_lines` is allowed crate-wide. The functions it flags are all
// configuration/registration builders whose length is inherent: prefab
// constructors (ConciergeAgent::new etc.) and skill `register_tools` mirror
// their Python `__init__` / `register_tools` counterparts, which parse a
// config map and register many tools inline in one place; the compat verb
// builders (e.g. join_conference) carry the full cXML attribute set plus a
// validation block that must emit the reference's exact ValueError messages
// verbatim. Splitting these to satisfy a 100-line heuristic would fragment a
// parity-locked 1:1 mapping for no functional gain (the lint is
// surface-invisible — it changes no signature or emission). The line count is
// a readability proxy that doesn't fit builder/registration code; keeping the
// reference's shape wins (the parity meta-rule).
#![allow(clippy::too_many_lines)]
// `must_use_candidate` is allowed crate-wide; `#[must_use]` is added by hand
// instead. The lint's own docs say "Not bad at all, this lint just shows
// places where you could add the attribute" and "Expect many false positives"
// — it's allow-by-default in `pedantic` for exactly that reason, because it
// can't tell a function called for its return value from one called for a side
// effect. We follow the std-dev-guide test ("add #[must_use] when failing to
// consider the output is almost certainly a bug") and the practice of every
// comparable public SDK (reqwest / octocrab / aws-sdk / clap / uuid / chrono
// all decline this lint): `#[must_use]` lives on the value producers where
// dropping the result is meaningless — `render*` / `to_value` / `to_json` /
// `render_swml` / `build_ai_verb` / `to_swaig_function` / the enum `as_str`
// conversions / the schema code-gen — not on field getters or sub-namespace
// accessors, where discarding the result can be legitimate (the noise std and
// aws-sdk deliberately avoid). Per RULES.md this is a parity-neutral idiom
// choice governed by PORT_PHILOSOPHY_RUST.md.
#![allow(clippy::must_use_candidate)]
// DOC-SURFACE floor (plan §6.3): warn on any undocumented public item so the
// docs.rs reference renders complete. `warn` (not `deny`) is the shrinking
// allow-budget: newly-added public surface is nudged to carry a doc comment
// without a hard build break while the last undocumented modules (the generated
// RELAY Simple-RPC method block) are still being papered. As those land their
// docs this ratchets toward `deny`; it must never regress upward.
#![warn(missing_docs)]

// DOC-SURFACE allow-budget (§6.3). Each `#[allow(missing_docs)]` below exempts a
// module whose item-level public surface (methods/fields/fns) is not yet fully
// doc-commented. This is the SHRINKING budget: the crate `//!` landing page and
// every module's own `//!` header ARE documented (so docs.rs renders), and as a
// module's items get their doc comments its allow is removed here — the budget
// only shrinks, never grows. The ledger + counts live in DOC_SURFACE_ALLOW.md.
// Un-annotated modules (pom, datamap) already meet the floor; adding an
// undocumented public item to one reds LINT (-D warnings), which is the point.
#[allow(missing_docs)]
pub mod core;
#[allow(missing_docs)]
pub mod datamap;
#[allow(missing_docs)]
pub mod logging;
pub mod pom;
#[allow(missing_docs)]
pub mod swml;
#[allow(missing_docs)]
pub mod utils;

#[allow(missing_docs)]
pub mod agent;
#[allow(missing_docs)]
pub mod contexts;
#[allow(missing_docs)]
pub mod security;
#[allow(missing_docs)]
pub mod swaig;

#[allow(missing_docs)]
pub mod prefabs;
#[allow(missing_docs)]
pub mod server;
#[allow(missing_docs)]
pub mod skills;

// The RELAY "Simple-RPC" block (action/call/client — the 57+ calling verbs) is
// the largest undocumented cluster; it carries the bulk of the budget.
#[allow(missing_docs)]
pub mod relay;
#[allow(missing_docs)]
pub mod rest;
#[allow(missing_docs)]
pub mod serverless;
#[allow(missing_docs)]
pub mod web;

// ─── Top-level re-exports for parity with Python's `signalwire` package
//
// Python's `signalwire/__init__.py` re-exports a fixed set of names so
// users can write `from signalwire import AgentBase, RestClient,
// BedrockAgent, …`. Rust's idiom is `use signalwire::agent::AgentBase`,
// but for surface parity (and so a one-line `use signalwire::*` brings
// these in at the crate root) we re-export the same set here.

pub use agent::{AgentBase, AgentOptions};
// The typed error returned by `"level".parse::<logging::Level>()` — re-exported
// at the crate root so callers can name it (`signalwire::ParseLevelError`)
// without reaching into the `logging` module.
pub use logging::ParseLevelError;
pub use prefabs::{
    BedrockAgent, BedrockOptions, ConciergeAgent, FAQBotAgent, InfoGathererAgent,
    ReceptionistAgent, SurveyAgent,
};
pub use rest::RestClient;
pub use server::AgentServer;
pub use swml::service::Service as SWMLService;

// ─── Top-level helpers (mirror Python's `signalwire/__init__.py`) ──────

/// Add a directory to the global skill search path.
///
/// Mirrors `signalwire.add_skill_directory(path)`. In Rust the
/// directory contents cannot be loaded at runtime — third-party
/// skills must call [`skills::SkillRegistry::register_skill`] at
/// startup — but the registered path is recorded for introspection.
///
/// # Errors
/// Returns an error string if the directory does not exist or is not
/// a directory.
pub fn add_skill_directory(path: &str) -> Result<(), String> {
    skills::SkillRegistry::add_skill_directory(path)
}

/// Sorted list of every registered skill name.
///
/// Mirrors Python's `signalwire.list_skills()`.
pub fn list_skills() -> Vec<String> {
    skills::SkillRegistry::list_skills()
}

/// Per-skill schema map (parameter metadata) for every registered
/// skill. Currently returns only the skill name as the key with an
/// empty parameter map — Rust skills don't carry rich Python-style
/// parameter introspection. The shape matches Python's contract so
/// downstream tooling can iterate.
///
/// Mirrors Python's `signalwire.list_skills_with_params()`.
pub fn list_skills_with_params() -> std::collections::HashMap<String, serde_json::Value> {
    let mut out = std::collections::HashMap::new();
    for name in skills::SkillRegistry::list_skills() {
        out.insert(name, serde_json::json!({"parameters": {}}));
    }
    out
}

/// Register a custom skill class.
///
/// Mirrors Python's `signalwire.register_skill(skill_class)`. Python's
/// `skill_class` carries both the name (via `SKILL_NAME` attribute) and
/// the factory (via the class itself); Rust packages the same pair as
/// a [`SkillSpec`] tuple — the canonical "skill class" descriptor in
/// Rust.
///
/// # Arguments
/// - `skill_class`: A [`SkillSpec`] describing the skill's name and
///   factory function.
///
/// To preserve the older two-argument call style, [`SkillRegistry::register_skill`]
/// remains available as `signalwire::skills::SkillRegistry::register_skill(name, factory)`.
pub fn register_skill(skill_class: SkillSpec) {
    skills::SkillRegistry::register_skill(&skill_class.name, skill_class.factory);
}

/// Skill registration descriptor — Rust's analogue of a Python skill
/// class. Bundles a skill's registration name with its factory closure
/// into a single value so [`register_skill`] can mirror Python's
/// one-argument signature.
#[must_use]
pub struct SkillSpec {
    /// Snake-case skill name used as the registry key.
    pub name: String,
    /// Factory closure that constructs a [`skills::SkillBase`] from a
    /// JSON parameters map.
    pub factory: skills::skill_registry::SkillFactory,
}

impl SkillSpec {
    /// Convenience constructor.
    pub fn new(name: impl Into<String>, factory: skills::skill_registry::SkillFactory) -> Self {
        SkillSpec {
            name: name.into(),
            factory,
        }
    }
}

/// Construct a [`RestClient`] from positional or keyword credentials.
///
/// Mirrors Python's top-level `signalwire.RestClient(*args, **kwargs)`
/// factory — in Python that's a thin wrapper that lazy-imports
/// `signalwire.rest.RestClient` and instantiates it. The Rust struct
/// is exposed at `signalwire::rest::RestClient`; this free function
/// provides the same one-line entry point under `signalwire::`.
///
/// The struct re-export at `signalwire::RestClient` (a type) and this
/// function at `signalwire::RestClient` (a value) coexist because
/// they live in distinct namespaces — types and values, respectively.
///
/// The signature mirrors Python's `(*args, **kwargs)` shape so the
/// cross-language signature audit recognises them as compatible. In
/// practice callers pass either:
///   * `args = ["proj", "tok", "space"]` (three positional strings), or
///   * `args = []` and `kwargs = {"project": ..., "token": ..., "host": ...}`
///
/// Either form maps onto [`rest::RestClient::new`].
///
/// # Errors
/// Returns an error string if credentials cannot be derived from either
/// `args` or `kwargs` (or fall back to the standard environment
/// variables `SIGNALWIRE_PROJECT_ID` / `SIGNALWIRE_API_TOKEN` /
/// `SIGNALWIRE_SPACE`).
#[allow(non_snake_case)]
// kwargs mirrors Python's **kwargs (the audit maps this to var_keyword); a
// concrete HashMap<String, String> keeps that surface 1:1. clippy::implicit_hasher
// would push a `S: BuildHasher` generic onto this public factory, uglifying the
// Python-parity signature for a caller-hasher flexibility no consumer needs.
#[allow(clippy::implicit_hasher)]
pub fn RestClient(
    args: Vec<String>,
    kwargs: std::collections::HashMap<String, String>,
) -> Result<rest::RestClient, rest::RestClientBuilderError> {
    // Resolve credentials in this order:
    //   1. positional args[0..3] = (project, token, space)
    //   2. kwargs["project"|"project_id"], kwargs["token"], kwargs["space"|"host"]
    //   3. environment variables (via from_env)
    let project = args
        .first()
        .cloned()
        .or_else(|| kwargs.get("project").cloned())
        .or_else(|| kwargs.get("project_id").cloned())
        .or_else(|| std::env::var("SIGNALWIRE_PROJECT_ID").ok())
        .unwrap_or_default();
    let token = args
        .get(1)
        .cloned()
        .or_else(|| kwargs.get("token").cloned())
        .or_else(|| std::env::var("SIGNALWIRE_API_TOKEN").ok())
        .unwrap_or_default();
    let space = args
        .get(2)
        .cloned()
        .or_else(|| kwargs.get("space").cloned())
        .or_else(|| kwargs.get("host").cloned())
        .or_else(|| std::env::var("SIGNALWIRE_SPACE").ok())
        .unwrap_or_default();
    rest::RestClient::new(&project, &token, &space)
}

#[cfg(test)]
mod top_level_tests {
    use super::*;

    #[test]
    fn test_add_skill_directory_top_level_helper() {
        // Smoke test: the top-level helper forwards to SkillRegistry.
        let dir = std::env::current_dir().unwrap().join("src");
        let result = add_skill_directory(dir.to_str().unwrap());
        assert!(
            result.is_ok(),
            "top-level add_skill_directory failed: {result:?}"
        );
    }

    #[test]
    fn test_add_skill_directory_top_level_helper_rejects_missing() {
        let result = add_skill_directory("/no-such-dir-zzz-789");
        assert!(result.is_err());
    }

    #[test]
    fn test_top_level_rest_client_constructible() {
        // RestClient::new is happy with any non-empty creds; with_base_url is
        // the real path used for tests. Just exercise the re-export.
        let client = RestClient::with_base_url("p", "t", "http://127.0.0.1:1");
        assert!(client.is_ok());
    }

    #[test]
    fn test_top_level_rest_client_factory_positional() {
        // Positional form: args=[project, token, space].
        let client = RestClient(
            vec![
                "proj".to_string(),
                "tok".to_string(),
                "test.signalwire.com".to_string(),
            ],
            std::collections::HashMap::new(),
        )
        .expect("factory should succeed with positional args");
        assert_eq!(client.project_id(), "proj");
        assert_eq!(client.token(), "tok");
        assert_eq!(client.space(), "test.signalwire.com");
    }

    #[test]
    fn test_top_level_rest_client_factory_kwargs() {
        // Keyword form: kwargs={"project":..., "token":..., "host":...}.
        let mut kw = std::collections::HashMap::new();
        kw.insert("project".to_string(), "kproj".to_string());
        kw.insert("token".to_string(), "ktok".to_string());
        kw.insert("host".to_string(), "kw.signalwire.com".to_string());
        let client = RestClient(vec![], kw).expect("factory should succeed with kwargs");
        assert_eq!(client.project_id(), "kproj");
        assert_eq!(client.token(), "ktok");
        assert_eq!(client.space(), "kw.signalwire.com");
    }

    #[test]
    fn test_top_level_rest_client_factory_rejects_empty() {
        // Validation matches RestClient::new — empty credentials are
        // rejected with a descriptive error.
        let r = RestClient(
            vec![String::new(), "tok".to_string(), "space".to_string()],
            std::collections::HashMap::new(),
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_top_level_agentbase_constructible() {
        // Exercise the AgentBase re-export at the crate root.
        let opts = AgentOptions::new("smoke");
        let _agent = AgentBase::new(opts);
    }

    #[test]
    fn test_top_level_bedrock_agent_constructible() {
        // Exercise the BedrockAgent re-export at the crate root.
        let _agent = BedrockAgent::new(BedrockOptions::default());
    }
}
