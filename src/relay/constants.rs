/// RELAY protocol constants.
///
/// Mirrors the PHP `Constants` class: protocol version, call/dial/message
/// states, terminal-state maps, and per-event-type action terminal states.

/// Protocol version exchanged during `signalwire.connect`.
pub const PROTOCOL_VERSION_MAJOR: u32 = 2;
pub const PROTOCOL_VERSION_MINOR: u32 = 0;
pub const PROTOCOL_VERSION_REVISION: u32 = 0;

// ------------------------------------------------------------------
// Call states
// ------------------------------------------------------------------

pub const CALL_STATE_CREATED: &str = "created";
pub const CALL_STATE_RINGING: &str = "ringing";
pub const CALL_STATE_ANSWERED: &str = "answered";
pub const CALL_STATE_ENDING: &str = "ending";
pub const CALL_STATE_ENDED: &str = "ended";

/// Returns `true` when the call state is terminal (i.e. `"ended"`).
pub fn is_call_terminal(state: &str) -> bool {
    state == CALL_STATE_ENDED
}

// ------------------------------------------------------------------
// Dial states
// ------------------------------------------------------------------

pub const DIAL_STATE_DIALING: &str = "dialing";
pub const DIAL_STATE_ANSWERED: &str = "answered";
pub const DIAL_STATE_FAILED: &str = "failed";

// ------------------------------------------------------------------
// Message states
// ------------------------------------------------------------------

pub const MESSAGE_STATE_QUEUED: &str = "queued";
pub const MESSAGE_STATE_INITIATED: &str = "initiated";
pub const MESSAGE_STATE_SENT: &str = "sent";
pub const MESSAGE_STATE_DELIVERED: &str = "delivered";
pub const MESSAGE_STATE_UNDELIVERED: &str = "undelivered";
pub const MESSAGE_STATE_FAILED: &str = "failed";
pub const MESSAGE_STATE_RECEIVED: &str = "received";

/// Returns `true` when the message state is terminal.
pub fn is_message_terminal(state: &str) -> bool {
    matches!(
        state,
        MESSAGE_STATE_DELIVERED | MESSAGE_STATE_UNDELIVERED | MESSAGE_STATE_FAILED
    )
}

// ------------------------------------------------------------------
// Action terminal states (keyed by event type)
// ------------------------------------------------------------------

/// Returns `true` when the given `(event_type, action_state)` pair represents
/// a terminal state for an in-flight action.
pub fn is_action_terminal(event_type: &str, state: &str) -> bool {
    match event_type {
        "calling.call.play" => matches!(state, "finished" | "error"),
        "calling.call.record" => matches!(state, "finished" | "no_input"),
        "calling.call.detect" => matches!(state, "finished" | "error"),
        "calling.call.collect" => {
            matches!(state, "finished" | "error" | "no_input" | "no_match")
        }
        "calling.call.fax" => matches!(state, "finished" | "error"),
        "calling.call.tap" => state == "finished",
        "calling.call.stream" => state == "finished",
        "calling.call.transcribe" => state == "finished",
        "calling.call.pay" => matches!(state, "finished" | "error"),
        _ => false,
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        assert_eq!(PROTOCOL_VERSION_MAJOR, 2);
        assert_eq!(PROTOCOL_VERSION_MINOR, 0);
        assert_eq!(PROTOCOL_VERSION_REVISION, 0);
    }

    #[test]
    fn test_call_terminal() {
        assert!(is_call_terminal("ended"));
        assert!(!is_call_terminal("ringing"));
        assert!(!is_call_terminal("answered"));
        assert!(!is_call_terminal("created"));
        assert!(!is_call_terminal("ending"));
    }

    #[test]
    fn test_message_terminal() {
        assert!(is_message_terminal("delivered"));
        assert!(is_message_terminal("undelivered"));
        assert!(is_message_terminal("failed"));
        assert!(!is_message_terminal("queued"));
        assert!(!is_message_terminal("sent"));
        assert!(!is_message_terminal("initiated"));
        assert!(!is_message_terminal("received"));
    }

    #[test]
    fn test_action_terminal_play() {
        assert!(is_action_terminal("calling.call.play", "finished"));
        assert!(is_action_terminal("calling.call.play", "error"));
        assert!(!is_action_terminal("calling.call.play", "playing"));
    }

    #[test]
    fn test_action_terminal_record() {
        assert!(is_action_terminal("calling.call.record", "finished"));
        assert!(is_action_terminal("calling.call.record", "no_input"));
        assert!(!is_action_terminal("calling.call.record", "recording"));
    }

    #[test]
    fn test_action_terminal_collect() {
        assert!(is_action_terminal("calling.call.collect", "finished"));
        assert!(is_action_terminal("calling.call.collect", "error"));
        assert!(is_action_terminal("calling.call.collect", "no_input"));
        assert!(is_action_terminal("calling.call.collect", "no_match"));
        assert!(!is_action_terminal("calling.call.collect", "collecting"));
    }

    #[test]
    fn test_action_terminal_detect() {
        assert!(is_action_terminal("calling.call.detect", "finished"));
        assert!(is_action_terminal("calling.call.detect", "error"));
        assert!(!is_action_terminal("calling.call.detect", "detecting"));
    }

    #[test]
    fn test_action_terminal_fax() {
        assert!(is_action_terminal("calling.call.fax", "finished"));
        assert!(is_action_terminal("calling.call.fax", "error"));
        assert!(!is_action_terminal("calling.call.fax", "sending"));
    }

    #[test]
    fn test_action_terminal_tap() {
        assert!(is_action_terminal("calling.call.tap", "finished"));
        assert!(!is_action_terminal("calling.call.tap", "tapping"));
    }

    #[test]
    fn test_action_terminal_stream() {
        assert!(is_action_terminal("calling.call.stream", "finished"));
        assert!(!is_action_terminal("calling.call.stream", "streaming"));
    }

    #[test]
    fn test_action_terminal_transcribe() {
        assert!(is_action_terminal("calling.call.transcribe", "finished"));
        assert!(!is_action_terminal("calling.call.transcribe", "transcribing"));
    }

    #[test]
    fn test_action_terminal_pay() {
        assert!(is_action_terminal("calling.call.pay", "finished"));
        assert!(is_action_terminal("calling.call.pay", "error"));
        assert!(!is_action_terminal("calling.call.pay", "paying"));
    }

    #[test]
    fn test_action_terminal_unknown_event() {
        assert!(!is_action_terminal("calling.call.unknown", "finished"));
    }

    #[test]
    fn test_call_state_constants() {
        assert_eq!(CALL_STATE_CREATED, "created");
        assert_eq!(CALL_STATE_RINGING, "ringing");
        assert_eq!(CALL_STATE_ANSWERED, "answered");
        assert_eq!(CALL_STATE_ENDING, "ending");
        assert_eq!(CALL_STATE_ENDED, "ended");
    }

    #[test]
    fn test_dial_state_constants() {
        assert_eq!(DIAL_STATE_DIALING, "dialing");
        assert_eq!(DIAL_STATE_ANSWERED, "answered");
        assert_eq!(DIAL_STATE_FAILED, "failed");
    }

    #[test]
    fn test_message_state_constants() {
        assert_eq!(MESSAGE_STATE_QUEUED, "queued");
        assert_eq!(MESSAGE_STATE_INITIATED, "initiated");
        assert_eq!(MESSAGE_STATE_SENT, "sent");
        assert_eq!(MESSAGE_STATE_DELIVERED, "delivered");
        assert_eq!(MESSAGE_STATE_UNDELIVERED, "undelivered");
        assert_eq!(MESSAGE_STATE_FAILED, "failed");
        assert_eq!(MESSAGE_STATE_RECEIVED, "received");
    }
}
