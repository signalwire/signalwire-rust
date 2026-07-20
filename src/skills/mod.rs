//! Pluggable agent capabilities.
//!
//! Skills implement the [`skill_base::SkillBase`] trait and are loaded/managed by
//! the skill manager; [`builtin`] ships 17 ready-to-use skills (datetime, math,
//! `web_search`, `weather_api`, …). Add one to an agent via `agent.add_skill(...)`.

pub mod builtin;
pub mod skill_base;
pub mod skill_manager;
pub mod skill_name;
pub mod skill_registry;

pub use skill_base::SkillBase;
pub use skill_manager::SkillManager;
pub use skill_name::{ParseSkillNameError, SkillName};
pub use skill_registry::SkillRegistry;
