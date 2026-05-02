pub mod core;
pub mod logging;
pub mod pom;
pub mod swml;
pub mod utils;

pub mod agent;
pub mod contexts;
pub mod datamap;
pub mod security;
pub mod swaig;

pub mod skills;
pub mod prefabs;
pub mod server;

pub mod relay;
pub mod rest;
pub mod serverless;

// ─── Top-level re-exports for parity with Python's `signalwire` package
//
// Python's `signalwire/__init__.py` re-exports a fixed set of names so
// users can write `from signalwire import AgentBase, RestClient,
// BedrockAgent, …`. Rust's idiom is `use signalwire::agent::AgentBase`,
// but for surface parity (and so a one-line `use signalwire::*` brings
// these in at the crate root) we re-export the same set here.

pub use agent::{AgentBase, AgentOptions};
pub use prefabs::{
    BedrockAgent, BedrockOptions, ConciergeAgent, FAQBotAgent, InfoGathererAgent,
    ReceptionistAgent, SurveyAgent,
};
pub use rest::RestClient;
pub use server::AgentServer;
pub use swml::service::Service as SWMLService;

// ─── Top-level helpers (mirror Python's `signalwire/__init__.py`) ──────

use std::collections::HashMap;

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

/// Construct an [`AgentServer`] containing a single [`AgentBase`] and
/// run it on the configured `host:port`. Blocking call.
///
/// Mirrors Python's `signalwire.start_agent(agent, host=..., port=...)`
/// — the Python helper instantiates an AgentServer behind the scenes,
/// registers the agent, and runs it. The Rust signature accepts an
/// owned `AgentBase` (Rust's ownership model makes "move into the
/// server, then run" the natural shape).
pub fn start_agent(agent: AgentBase, host: Option<&str>, port: Option<u16>) {
    let mut server = AgentServer::new(host, port);
    if let Err(e) = server.register(agent, None) {
        panic!("start_agent: failed to register: {}", e);
    }
    server.run(None, None);
}

/// Run the supplied [`AgentBase`] directly (without an
/// [`AgentServer`]) on its configured host/port. Blocking call.
///
/// Mirrors Python's `signalwire.run_agent(agent, host=..., port=...)`.
/// Useful when a caller wants the agent's own routes (`/`, `/swaig`,
/// `/post_prompt`, `/health`) without the multi-agent wrapper.
pub fn run_agent(agent: &AgentBase, _host: Option<&str>, _port: Option<u16>) {
    // AgentBase delegates to its embedded Service for the HTTP loop.
    // Service::run respects the configured host:port; we accept the
    // host/port arguments for signature parity but currently honor the
    // values supplied at construction.
    let _ = HashMap::<String, String>::new(); // suppress unused-import lint when no helpers are used
    agent.run();
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
    skills::SkillRegistry::register_skill(&skill_class.name, skill_class.factory)
}

/// Skill registration descriptor — Rust's analogue of a Python skill
/// class. Bundles a skill's registration name with its factory closure
/// into a single value so [`register_skill`] can mirror Python's
/// one-argument signature.
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
        SkillSpec { name: name.into(), factory }
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
pub fn RestClient(
    args: Vec<String>,
    kwargs: std::collections::HashMap<String, String>,
) -> Result<rest::RestClient, String> {
    // Resolve credentials in this order:
    //   1. positional args[0..3] = (project, token, space)
    //   2. kwargs["project"|"project_id"], kwargs["token"], kwargs["space"|"host"]
    //   3. environment variables (via from_env)
    let project = args.first().cloned()
        .or_else(|| kwargs.get("project").cloned())
        .or_else(|| kwargs.get("project_id").cloned())
        .or_else(|| std::env::var("SIGNALWIRE_PROJECT_ID").ok())
        .unwrap_or_default();
    let token = args.get(1).cloned()
        .or_else(|| kwargs.get("token").cloned())
        .or_else(|| std::env::var("SIGNALWIRE_API_TOKEN").ok())
        .unwrap_or_default();
    let space = args.get(2).cloned()
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
        assert!(result.is_ok(), "top-level add_skill_directory failed: {:?}", result);
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
            vec!["proj".to_string(), "tok".to_string(), "test.signalwire.com".to_string()],
            std::collections::HashMap::new(),
        ).expect("factory should succeed with positional args");
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
        let client = RestClient(vec![], kw)
            .expect("factory should succeed with kwargs");
        assert_eq!(client.project_id(), "kproj");
        assert_eq!(client.token(), "ktok");
        assert_eq!(client.space(), "kw.signalwire.com");
    }

    #[test]
    fn test_top_level_rest_client_factory_rejects_empty() {
        // Validation matches RestClient::new — empty credentials are
        // rejected with a descriptive error.
        let r = RestClient(vec!["".to_string(), "tok".to_string(), "space".to_string()],
                           std::collections::HashMap::new());
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
