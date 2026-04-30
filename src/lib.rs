pub mod core;
pub mod logging;
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

/// Register a custom skill by name + factory.
///
/// Mirrors Python's `signalwire.register_skill(skill_class)` — the
/// Rust signature differs because Rust uses a typed factory rather
/// than reflection-driven class registration.
pub fn register_skill(name: &str, factory: skills::skill_registry::SkillFactory) {
    skills::SkillRegistry::register_skill(name, factory)
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
