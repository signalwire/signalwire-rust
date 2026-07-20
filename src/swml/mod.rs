//! SWML (SignalWire Markup Language) document model, builder, and renderer.
//!
//! Construct call-flow documents programmatically with [`SwmlBuilder`], validate
//! them against the embedded schema, and render to JSON. [`service::Service`]
//! (aliased [`crate::SWMLService`]) serves a rendered document over HTTP.

pub mod builder;
pub mod document;
pub mod handler;
pub mod renderer;
#[cfg(feature = "tower-middleware")]
pub mod router;
pub mod schema;
pub mod service;
// Generated SWML-verb config tree — exempt from the missing_docs floor (§6.3
// allow-budget); schema-derived, doc'd at the declaration site so no generated
// file is edited (GEN-FRESH stays clean).
#[allow(missing_docs)]
pub mod swml_verbs_generated;

pub use builder::SwmlBuilder;
pub use handler::{AiVerbHandler, SwmlVerbHandler, VerbHandlerRegistry};
pub use renderer::{RenderSwmlOptions, SwmlRenderer};
