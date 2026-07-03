//! RELAY module -- real-time event signalling over WebSocket (JSON-RPC 2.0).
//!
//! Provides constants, event/action primitives, call control, message
//! tracking, and the async-ready `Client` that ties everything together.

pub mod action;
pub mod call;
pub mod client;
pub mod constants;
pub mod device;
pub mod error;
pub mod event;
pub mod message;
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
