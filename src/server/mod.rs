//! Multi-agent hosting.
//!
//! [`agent_server::AgentServer`] (aliased [`crate::AgentServer`]) hosts multiple
//! agents on one process: register/unregister by route, SIP routing, static-file
//! serving with path-traversal protection, and health/ready endpoints.

pub mod agent_server;
pub mod error;
pub(crate) mod tls;

pub use agent_server::AgentServer;
pub use error::ServerError;
