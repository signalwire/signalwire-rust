# INTENTIONAL_NON_IMPLEMENTATION.md (signalwire-rust)

Allow-list for `scripts/audit_stubs.py` (in porting-sdk). Each entry justifies why a flagged line is NOT a stub bug. See porting-sdk's `INTENTIONAL_NON_IMPLEMENTATION.md` template for the four legitimate categories.

Format: `- <file:line> — <one-sentence justification>`

---

## Allow-listed entries

- src/swml/service.rs:505 — handle_swml_request returns the rendered SWML doc identically for GET and POST per Python's SWMLService base behavior; AgentBase wraps for dynamic-config (request-body-driven rendering) but the SWMLService base correctly ignores `_method` / `_request_data` / `_headers` and serves the static doc. Category 4 — extension-point default with the documented behavior.
- src/relay/action.rs:159 — should_handle_event() default returns true ("accept all events") per the documented contract (see docstring just above the function). Subclasses override to filter. Category 4 — extension-point default.
- src/cli/main.rs:619 — false positive: the audit's canned-error-string regex matches the literal text "HTTP transport not available" inside a TEST DOCSTRING that explains what the function previously returned (it's now fixed; the comment narrates history). The test itself drives a real HTTP round-trip. Category: false positive.
