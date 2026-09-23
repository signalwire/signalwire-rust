//! `token_interop_mint` — the Rust port's TOKEN-INTEROP mint fixture for the cross-port
//! checker (`porting-sdk/scripts/diff_port_token_interop.py`).
//!
//! The contract being proven is property 3 of the SWAIG tool-token contract: a token
//! this port MINTS must validate under the REFERENCE's own decoder. The other two
//! properties (that a token is minted at all; that the HMAC is keyed with the
//! `secret_key` STRING's bytes) already had coverage — this one did not, and a port can
//! pass both and still emit a token no other implementation accepts, in which case
//! every secure tool call fails authentication in production. This port is where the
//! defect class was first proven: it used to mint with `URL_SAFE_NO_PAD`, and the
//! reference's `urlsafe_b64decode` RAISES on a stripped `=`.
//!
//! Protocol: read the FIXED mint inputs from the environment (the checker owns them, so
//! this fixture cannot drift from the values it is verified against), construct a
//! `SessionManager` with that secret key, mint ONE token, and print JUST the token on
//! stdout. Anything else belongs on stderr.
//!
//! Run from the signalwire-rust repo root:
//!
//! ```text
//! cargo run --quiet --example token_interop_mint
//! ```

use signalwire::security::SessionManager;

/// Read a required fixed mint input from the environment, or fail loud.
fn required(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => value,
        _ => {
            eprintln!(
                "{name} is not set — the TOKEN-INTEROP checker supplies the fixed mint \
                 inputs in the environment; run this via diff_port_token_interop.py \
                 --mint-cmd."
            );
            std::process::exit(1);
        }
    }
}

fn main() {
    let secret_key = required("SW_TOKEN_INTEROP_SECRET_KEY");
    let call_id = required("SW_TOKEN_INTEROP_CALL_ID");
    let function_name = required("SW_TOKEN_INTEROP_FUNCTION_NAME");

    // Default expiry — the token must carry a FUTURE expiry, which the checker verifies.
    // `with_secret` takes the reference's `secret_key` STRING, whose UTF-8 bytes key the
    // HMAC (NOT 32 raw bytes hex-decoded from it).
    let manager = SessionManager::with_secret(900, &secret_key);
    println!("{}", manager.generate_token(&function_name, &call_id));
}
