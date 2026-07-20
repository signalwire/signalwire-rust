//! Ready-made agent archetypes.
//!
//! Configurable prebuilt agents — [`InfoGathererAgent`], [`SurveyAgent`],
//! [`ReceptionistAgent`], [`FAQBotAgent`], [`ConciergeAgent`], and the
//! [`BedrockAgent`] — each built through its own builder methods over
//! [`crate::agent::AgentBase`].

use std::collections::HashMap;

use serde_json::Value;

/// Callback type for a prefab's post-prompt summary handler.
///
/// Mirrors `AgentBase`'s summary-callback shape: receives the summary text, the
/// full summary `Value`, and the request headers. Registered via each prefab's
/// `on_summary` method, which delegates to `AgentBase::on_summary`.
pub type PrefabSummaryCallback = Box<dyn Fn(&str, &Value, &HashMap<String, String>) + Send + Sync>;

pub mod bedrock;
pub mod concierge;
pub mod faq_bot;
pub mod info_gatherer;
pub mod receptionist;
pub mod survey;

pub use bedrock::{BedrockAgent, BedrockOptions};
pub use concierge::ConciergeAgent;
pub use faq_bot::FAQBotAgent;
pub use info_gatherer::InfoGathererAgent;
pub use receptionist::ReceptionistAgent;
pub use survey::SurveyAgent;
