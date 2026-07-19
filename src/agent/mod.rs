//! Agent construction and serving.
//!
//! [`AgentBase`] is the central builder: compose prompts, SWAIG tools, skills,
//! and AI config with chained `&mut self` methods, then serve the 5-phase SWML
//! pipeline over HTTP. Construct one from [`AgentOptions`].

pub mod agent_base;
pub mod type_inference;

pub use agent_base::{AgentBase, AgentOptions, FunctionHandler};
