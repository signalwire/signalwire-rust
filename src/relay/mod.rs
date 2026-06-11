//! RELAY module -- real-time event signalling over WebSocket (JSON-RPC 2.0).
//!
//! Provides constants, event/action primitives, call control, message
//! tracking, and the async-ready `Client` that ties everything together.

pub mod constants;
pub mod state_enums;
pub mod device;
pub mod error;
pub mod event;
pub mod action;
pub mod message;
pub mod call;
pub mod client;

pub use constants::*;
pub use state_enums::{CallState, DialState, MessageState};
pub use device::Device;
pub use error::RelayError;
pub use event::Event;
pub use action::{
    Action, PlayAction, RecordAction, CollectAction, DetectAction,
    FaxAction, TapAction, StreamAction, PayAction, TranscribeAction, AIAction,
};
pub use message::Message;
pub use call::Call;
pub use client::Client;
