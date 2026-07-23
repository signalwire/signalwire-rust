// Copyright (c) 2026 SignalWire
//
// This file is part of the SignalWire SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Async client for the SignalWire AI Chat service.
//!
//! The public surface lives in [`client`] and is re-exported here, so callers
//! write `use signalwire::ai_chat::AIChatClient`. The module layout mirrors the
//! python reference `signalwire/ai_chat/client.py`.

pub mod client;

pub use client::{
    AIChatClient, AIChatClientBuilder, AIChatError, AIChatErrorKind, ChatLog, ChatOptions,
    ChatResponse, ConversationInfo, CreateOptions, SummarizeOptions,
};
