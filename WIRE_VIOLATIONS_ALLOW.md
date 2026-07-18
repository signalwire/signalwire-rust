# WIRE_VIOLATIONS_ALLOW.md — signed exceptions to the STRICT-MOCKS wire-truth gate

The STRICT-MOCKS consumer (`porting-sdk/scripts/assert_no_wire_violations.py`, wired
into REST-COVERAGE / EXAMPLES-RUN / SNIPPET-RUN) reads the mock journal after a run
and fails on ANY `wire_violation` — a request/frame that put a shape on the wire the
OpenAPI/RELAY spec does not declare (an undeclared query param, an unknown body key,
an unknown frame field). A wire violation is a spec bug or a real defect; the fix is
to make the wire match the spec, NOT to allowlist it.

This file exists for the rare, genuinely-justified exception, and each entry needs a
human-signed reason. Format (one per line):

    - <kind>:<name> — reason (approver, date)

where `<kind>` is the violation kind (`unknown_query_param`, `unknown_body_key`,
`unknown_frame_field`, `duplicate_command_id`) and `<name>` is the offending
key/param name. A bare `kind:name` with no ` — reason` is NOT matched, so it cannot
silently widen the allowlist.

## Currently empty

No entries. The wired gates (REST-COVERAGE / EXAMPLES-RUN / SNIPPET-RUN) run wire-clean
against this port.

Two known spec gaps exist upstream (recordings `page_size` on
`relay-rest.list_recordings`, and `cursor` on `fabric.list_fabric_addresses`) — both
owner-parked pending prime-rails confirmation of the server-side param, tracked at the
porting-sdk level, not per-port here. They are NOT allowlisted in this file: rust's
generated REST wire-test suite (the only suite that feeds the REST-COVERAGE journal)
does not exercise the hand-authored regression-probe shape that would surface them, so
no entry is needed to keep this gate green. Do not add a name-only token for either —
it would over-broadly mask any future real violation on the many endpoints that
legitimately declare `page_size`/`cursor`.
