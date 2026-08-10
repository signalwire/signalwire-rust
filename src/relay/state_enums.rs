//! Typed RELAY lifecycle-state enums.
//!
//! The RELAY protocol carries three *distinct* server-emitted state
//! vocabularies — call lifecycle, dial outcome, and message delivery — as
//! bare strings on the wire. [`constants`](super::constants) keeps the raw
//! string consts and the `is_*_terminal(&str)` predicates (the byte-level
//! contract); this module layers a typed, idiomatic view on top for the
//! highest-traffic of those vocabularies.
//!
//! Three deliberately separate types — [`CallState`], [`DialState`],
//! [`MessageState`] — because the three vocabularies are *not*
//! interchangeable even where strings coincide (`"answered"` is both a
//! [`CallState`] and a [`DialState`], `"failed"` is both a [`DialState`]
//! and a [`MessageState`], but conflating them would let a message state
//! leak into a call predicate). The compiler now keeps them apart.
//!
//! ## Server-growth semantics (why `#[non_exhaustive]` + `Other`)
//!
//! Unlike the client-validated closed sets in
//! [`media_enums`](crate::swaig::media_enums) (which mirror Python
//! `ValueError` checks and reject unknown input), these mirror values the
//! *server* emits and may add to over time. So each enum:
//!
//! - is `#[non_exhaustive]` — downstream `match` must carry a wildcard arm,
//!   so a future server value can't break a consumer at compile time;
//! - carries an [`Other(String)`](CallState::Other) catch-all so parsing a
//!   never-before-seen state **never panics or loses data** — it round-trips
//!   verbatim through [`as_str`](CallState::as_str);
//! - parses infallibly via [`from_str`](CallState::from_str) /
//!   `FromStr` (an unknown string becomes `Other`, not an error).
//!
//! Grounded in the wire contract `relay/constants.py`
//! (`CALL_STATE_*` / `MESSAGE_STATE_*` / `MESSAGE_TERMINAL_STATES`) and the
//! port's own [`constants`](super::constants) (`DIAL_STATE_*`). The typed
//! accessors (`Call::call_state` / `Message::message_state`) are exposed
//! *alongside* the existing string accessors (`Call::current_state` /
//! `Message::state`).

use std::fmt;
use std::str::FromStr;

use super::constants;

/// Call lifecycle state, as carried by `calling.call.state` events.
///
/// Matches `relay/constants.py` `CALL_STATE_*`
/// (`created` → `ringing` → `answered` → `ending` → `ended`). The terminal
/// state is `ended` (see [`is_terminal`](CallState::is_terminal)), matching
/// [`constants::is_call_terminal`].
///
/// `#[non_exhaustive]` — server-emitted; an unrecognised value parses to
/// [`Other`](CallState::Other) rather than panicking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[must_use]
pub enum CallState {
    /// `created` — the call object exists but has not started ringing.
    Created,
    /// `ringing` — the call is ringing the destination.
    Ringing,
    /// `answered` — the call has been answered and is in progress.
    Answered,
    /// `ending` — the call is in the process of tearing down.
    Ending,
    /// `ended` — the call has fully ended (terminal).
    Ended,
    /// Any state the server emits that this enum does not (yet) model.
    /// Carries the raw wire string so it round-trips losslessly.
    Other(String),
}

impl CallState {
    /// The canonical wire string for this state (e.g. `"ringing"`).
    ///
    /// For [`Other`](CallState::Other) this is the captured raw string, so
    /// `CallState::from_str(s).as_str() == s` for every `s`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            CallState::Created => constants::CALL_STATE_CREATED,
            CallState::Ringing => constants::CALL_STATE_RINGING,
            CallState::Answered => constants::CALL_STATE_ANSWERED,
            CallState::Ending => constants::CALL_STATE_ENDING,
            CallState::Ended => constants::CALL_STATE_ENDED,
            CallState::Other(s) => s.as_str(),
        }
    }

    /// Parse a wire string into a [`CallState`].
    ///
    /// Infallible: an unrecognised value becomes [`Other`](CallState::Other)
    /// (server states can grow). Provided as an inherent method for
    /// ergonomics alongside the [`FromStr`] impl.
    ///
    /// `clippy::should_implement_trait` is suppressed deliberately: the
    /// [`FromStr`] impl exists below and delegates here, and this inherent
    /// method is the infallible companion that returns a [`CallState`]
    /// directly rather than a `Result`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> CallState {
        match s {
            constants::CALL_STATE_CREATED => CallState::Created,
            constants::CALL_STATE_RINGING => CallState::Ringing,
            constants::CALL_STATE_ANSWERED => CallState::Answered,
            constants::CALL_STATE_ENDING => CallState::Ending,
            constants::CALL_STATE_ENDED => CallState::Ended,
            other => CallState::Other(other.to_string()),
        }
    }

    /// `true` when this is a terminal call state (`ended`).
    ///
    /// Delegates to [`constants::is_call_terminal`] so the typed and
    /// string predicates can never disagree.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        constants::is_call_terminal(self.as_str())
    }
}

impl fmt::Display for CallState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CallState {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CallState::from_str(s))
    }
}

/// Dial outcome state, as carried by `calling.call.dial` events.
///
/// Distinct from [`CallState`]: a dial reports the *result* of an outbound
/// attempt (`dialing` while in flight, then the terminal `answered` /
/// `failed`), not the lifecycle of an established call. Grounded in the
/// port's [`constants`](super::constants) `DIAL_STATE_*`.
///
/// `#[non_exhaustive]` — server-emitted; unrecognised values parse to
/// [`Other`](DialState::Other).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[must_use]
pub enum DialState {
    /// `dialing` — the outbound attempt is in progress (non-terminal).
    Dialing,
    /// `answered` — a leg answered; the dial succeeded (terminal).
    Answered,
    /// `failed` — no leg answered; the dial failed (terminal).
    Failed,
    /// Any state the server emits that this enum does not (yet) model.
    Other(String),
}

impl DialState {
    /// The canonical wire string for this state (e.g. `"dialing"`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            DialState::Dialing => constants::DIAL_STATE_DIALING,
            DialState::Answered => constants::DIAL_STATE_ANSWERED,
            DialState::Failed => constants::DIAL_STATE_FAILED,
            DialState::Other(s) => s.as_str(),
        }
    }

    /// Parse a wire string into a [`DialState`] (infallible; unknown →
    /// [`Other`](DialState::Other)).
    ///
    /// `clippy::should_implement_trait` is suppressed deliberately: the
    /// [`FromStr`] impl exists below and delegates here, and this inherent
    /// method is the infallible companion that returns a [`DialState`]
    /// directly rather than a `Result`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> DialState {
        match s {
            constants::DIAL_STATE_DIALING => DialState::Dialing,
            constants::DIAL_STATE_ANSWERED => DialState::Answered,
            constants::DIAL_STATE_FAILED => DialState::Failed,
            other => DialState::Other(other.to_string()),
        }
    }

    /// `true` when this is a terminal dial outcome (`answered` or `failed`).
    ///
    /// A dial resolves once it either connects a leg or exhausts every
    /// device, so both `answered` and `failed` are terminal; `dialing` is
    /// not.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, DialState::Answered | DialState::Failed)
    }
}

impl fmt::Display for DialState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DialState {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DialState::from_str(s))
    }
}

/// Message delivery state, as carried by `messaging.state` events.
///
/// Matches `relay/constants.py` `MESSAGE_STATE_*`. The terminal set
/// is `delivered` / `undelivered` / `failed`
/// (`MESSAGE_TERMINAL_STATES`; see [`is_terminal`](MessageState::is_terminal)),
/// matching [`constants::is_message_terminal`]. Distinct from both
/// [`CallState`] and [`DialState`] — `failed` here means *message* failure,
/// not a dial failure.
///
/// `#[non_exhaustive]` — server-emitted; unrecognised values parse to
/// [`Other`](MessageState::Other).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[must_use]
pub enum MessageState {
    /// `queued` — accepted and queued for sending.
    Queued,
    /// `initiated` — sending has begun.
    Initiated,
    /// `sent` — handed off to the carrier.
    Sent,
    /// `delivered` — confirmed delivered (terminal).
    Delivered,
    /// `undelivered` — the carrier reported non-delivery (terminal).
    Undelivered,
    /// `failed` — sending failed (terminal).
    Failed,
    /// `received` — an inbound message was received.
    Received,
    /// Any state the server emits that this enum does not (yet) model.
    Other(String),
}

impl MessageState {
    /// The canonical wire string for this state (e.g. `"delivered"`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            MessageState::Queued => constants::MESSAGE_STATE_QUEUED,
            MessageState::Initiated => constants::MESSAGE_STATE_INITIATED,
            MessageState::Sent => constants::MESSAGE_STATE_SENT,
            MessageState::Delivered => constants::MESSAGE_STATE_DELIVERED,
            MessageState::Undelivered => constants::MESSAGE_STATE_UNDELIVERED,
            MessageState::Failed => constants::MESSAGE_STATE_FAILED,
            MessageState::Received => constants::MESSAGE_STATE_RECEIVED,
            MessageState::Other(s) => s.as_str(),
        }
    }

    /// Parse a wire string into a [`MessageState`] (infallible; unknown →
    /// [`Other`](MessageState::Other)).
    ///
    /// `clippy::should_implement_trait` is suppressed deliberately: the
    /// [`FromStr`] impl exists below and delegates here, and this inherent
    /// method is the infallible companion that returns a [`MessageState`]
    /// directly rather than a `Result`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> MessageState {
        match s {
            constants::MESSAGE_STATE_QUEUED => MessageState::Queued,
            constants::MESSAGE_STATE_INITIATED => MessageState::Initiated,
            constants::MESSAGE_STATE_SENT => MessageState::Sent,
            constants::MESSAGE_STATE_DELIVERED => MessageState::Delivered,
            constants::MESSAGE_STATE_UNDELIVERED => MessageState::Undelivered,
            constants::MESSAGE_STATE_FAILED => MessageState::Failed,
            constants::MESSAGE_STATE_RECEIVED => MessageState::Received,
            other => MessageState::Other(other.to_string()),
        }
    }

    /// `true` when this is a terminal delivery state (`delivered`,
    /// `undelivered`, or `failed`).
    ///
    /// Delegates to [`constants::is_message_terminal`] so the typed and
    /// string predicates can never disagree.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        constants::is_message_terminal(self.as_str())
    }
}

impl fmt::Display for MessageState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MessageState {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(MessageState::from_str(s))
    }
}

// ----------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- CallState ----

    #[test]
    fn call_state_round_trip_known() {
        for (variant, wire) in [
            (CallState::Created, "created"),
            (CallState::Ringing, "ringing"),
            (CallState::Answered, "answered"),
            (CallState::Ending, "ending"),
            (CallState::Ended, "ended"),
        ] {
            assert_eq!(variant.as_str(), wire);
            assert_eq!(CallState::from_str(wire), variant);
            // FromStr trait agrees with the inherent method.
            assert_eq!(wire.parse::<CallState>().unwrap(), variant);
        }
    }

    #[test]
    fn call_state_is_terminal_only_ended() {
        assert!(CallState::Ended.is_terminal());
        for s in [
            CallState::Created,
            CallState::Ringing,
            CallState::Answered,
            CallState::Ending,
        ] {
            assert!(!s.is_terminal(), "{s} must not be terminal");
        }
        // Typed predicate agrees with the string predicate it wraps.
        for wire in ["created", "ringing", "answered", "ending", "ended"] {
            assert_eq!(
                CallState::from_str(wire).is_terminal(),
                constants::is_call_terminal(wire),
            );
        }
    }

    #[test]
    fn call_state_unknown_round_trips_via_other() {
        let s = CallState::from_str("teleported");
        assert_eq!(s, CallState::Other("teleported".to_string()));
        // No panic, lossless round-trip, and not terminal.
        assert_eq!(s.as_str(), "teleported");
        assert!(!s.is_terminal());
        assert_eq!("teleported".parse::<CallState>().unwrap(), s);
    }

    // ---- DialState ----

    #[test]
    fn dial_state_round_trip_known() {
        for (variant, wire) in [
            (DialState::Dialing, "dialing"),
            (DialState::Answered, "answered"),
            (DialState::Failed, "failed"),
        ] {
            assert_eq!(variant.as_str(), wire);
            assert_eq!(DialState::from_str(wire), variant);
            assert_eq!(wire.parse::<DialState>().unwrap(), variant);
        }
    }

    #[test]
    fn dial_state_terminal_is_answered_and_failed() {
        assert!(DialState::Answered.is_terminal());
        assert!(DialState::Failed.is_terminal());
        assert!(!DialState::Dialing.is_terminal());
    }

    #[test]
    fn dial_state_unknown_round_trips_via_other() {
        let s = DialState::from_str("redialing");
        assert_eq!(s, DialState::Other("redialing".to_string()));
        assert_eq!(s.as_str(), "redialing");
        assert!(!s.is_terminal());
    }

    #[test]
    fn dial_state_is_distinct_from_call_state() {
        // Same wire word, different vocabulary: "answered" is terminal for a
        // dial outcome but NOT for a call lifecycle.
        assert!(DialState::from_str("answered").is_terminal());
        assert!(!CallState::from_str("answered").is_terminal());
    }

    // ---- MessageState ----

    #[test]
    fn message_state_round_trip_known() {
        for (variant, wire) in [
            (MessageState::Queued, "queued"),
            (MessageState::Initiated, "initiated"),
            (MessageState::Sent, "sent"),
            (MessageState::Delivered, "delivered"),
            (MessageState::Undelivered, "undelivered"),
            (MessageState::Failed, "failed"),
            (MessageState::Received, "received"),
        ] {
            assert_eq!(variant.as_str(), wire);
            assert_eq!(MessageState::from_str(wire), variant);
            assert_eq!(wire.parse::<MessageState>().unwrap(), variant);
        }
    }

    #[test]
    fn message_state_terminal_set() {
        for s in [
            MessageState::Delivered,
            MessageState::Undelivered,
            MessageState::Failed,
        ] {
            assert!(s.is_terminal(), "{s} must be terminal");
        }
        for s in [
            MessageState::Queued,
            MessageState::Initiated,
            MessageState::Sent,
            MessageState::Received,
        ] {
            assert!(!s.is_terminal(), "{s} must not be terminal");
        }
        // Typed predicate agrees with the string predicate it wraps.
        for wire in [
            "queued",
            "initiated",
            "sent",
            "delivered",
            "undelivered",
            "failed",
            "received",
        ] {
            assert_eq!(
                MessageState::from_str(wire).is_terminal(),
                constants::is_message_terminal(wire),
            );
        }
    }

    #[test]
    fn message_state_unknown_round_trips_via_other() {
        let s = MessageState::from_str("read");
        assert_eq!(s, MessageState::Other("read".to_string()));
        assert_eq!(s.as_str(), "read");
        assert!(!s.is_terminal());
    }

    #[test]
    fn message_failed_is_distinct_from_dial_failed() {
        // Same wire word ("failed") across two vocabularies — both terminal
        // here, but they are different *types* so they can't be mixed up.
        let m: MessageState = "failed".parse::<MessageState>().unwrap();
        let d: DialState = "failed".parse::<DialState>().unwrap();
        assert!(m.is_terminal());
        assert!(d.is_terminal());
        assert_eq!(m.as_str(), d.as_str());
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(CallState::Ringing.to_string(), "ringing");
        assert_eq!(DialState::Failed.to_string(), "failed");
        assert_eq!(MessageState::Delivered.to_string(), "delivered");
        assert_eq!(CallState::Other("x".into()).to_string(), "x");
    }
}
