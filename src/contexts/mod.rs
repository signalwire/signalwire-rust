//! Structured multi-step conversation workflows.
//!
//! A [`context_builder::ContextBuilder`] defines named [`context_builder::Context`]s,
//! each a sequence of [`context_builder::Step`]s with their own prompt, tools, and
//! completion criteria. Agents switch contexts at runtime to drive guided flows.

pub mod context_builder;

pub use context_builder::{
    Context, ContextBuilder, GatherInfo, GatherQuestion, Step, create_simple_context,
};
