//! RELAY module -- real-time event signalling over WebSocket (JSON-RPC 2.0).
//!
//! Provides constants, event/action primitives, call control, message
//! tracking, and the async-ready `Client` that ties everything together.

/// Upper bound on the in-memory "sent frames" inspection logs
/// (`Client::sent_messages`, `Call::sent_commands`, `Action::sent_commands`).
/// These record what the client *would* transmit for tests / debug
/// introspection; on a long-running session they must not grow without limit,
/// so each push drops the oldest entry once this many are retained (ring
/// buffer, most-recent-N semantics). Internal — not part of the public surface.
pub(crate) const SENT_LOG_CAP: usize = 1024;

pub mod action;
pub mod call;
pub mod client;
pub mod constants;
pub mod device;
pub mod error;
pub mod event;
pub mod message;
// Generated RELAY protocol types — exempt from the missing_docs floor (§6.3
// allow-budget); schema-derived DTOs, annotated at the declaration site so no
// generated file is edited (GEN-FRESH stays clean).
#[allow(missing_docs)]
pub mod protocol_types_generated;
pub mod state_enums;

pub use action::{
    AIAction, Action, CollectAction, DetectAction, FaxAction, PayAction, PlayAction, RecordAction,
    StandaloneCollectAction, StreamAction, TapAction, TranscribeAction,
};
pub use call::Call;
pub use client::Client;
pub use constants::*;
pub use device::Device;
pub use error::RelayError;
pub use event::Event;
pub use message::Message;
pub use state_enums::{CallState, DialState, MessageState};
