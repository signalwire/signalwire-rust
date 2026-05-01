# INTENTIONAL_THIN_TESTS.md

Allowlist for `audit_no_cheat_tests.py`. Each entry is `path:lineno —
rationale`. Use exactly the path/line the audit reports.

## tests/common/mocktest.rs — shared test harness, not test functions

The mocktest module ships under `tests/common/` so each integration test
binary can pull it in via `#[path = "common/mod.rs"] mod common;`. The
audit walks every `fn` in `tests/` and flags helpers because they don't
have assertion calls — but these aren't `#[test]` items, they are
plumbing for the real tests in `tests/rest_mock_*.rs`. The actual
content-shaped assertions live in those files.

- `tests/common/mocktest.rs:46` — lock_journal: helper that returns the global serialization MutexGuard
- `tests/common/mocktest.rs:83` — JournalEntry::body_object: accessor on a struct
- `tests/common/mocktest.rs:118` — journal_all: HTTP client to `/__mock__/journal`, panics on transport failure
- `tests/common/mocktest.rs:133` — journal_last: panics if journal empty (real exercise)
- `tests/common/mocktest.rs:142` — journal_reset: HTTP POST to `/__mock__/journal/reset`
- `tests/common/mocktest.rs:151` — scenario_reset: HTTP POST to `/__mock__/scenarios/reset`
- `tests/common/mocktest.rs:160` — reset_all: composes the two resets above
- `tests/common/mocktest.rs:176` — begin: takes the global mutex and resets
- `tests/common/mocktest.rs:185` — scenario_set: HTTP POST to `/__mock__/scenarios/<id>`
- `tests/common/mocktest.rs:198` — resolve_port: parses MOCK_SIGNALWIRE_PORT env var
- `tests/common/mocktest.rs:209` — ensure_server: probes `/__mock__/health` and spawns mock_signalwire if missing
- `tests/common/mocktest.rs:235` — probe_health: GETs `/__mock__/health` and returns success on 200 + `specs_loaded`
- `tests/common/mocktest.rs:328` — discover_porting_sdk_package: walks parents looking for `porting-sdk/test_harness/<name>`
- `tests/common/mocktest.rs:352` — separator (unix): returns `:`
- `tests/common/mocktest.rs:357` — separator (windows): returns `;`
- `tests/common/mocktest.rs:364` — extern setsid: C binding
- `tests/common/mocktest.rs:367` — libc_setsid: thin wrapper around setsid
- `tests/common/mocktest.rs:375` — decode_journal: serde_json -> Vec<JournalEntry>
- `tests/common/mocktest.rs:383` — decode_entry: serde_json -> single JournalEntry

## tests/common/mod.rs — module marker

This file exists solely to expose the `mocktest` submodule via
`#[path = "common/mod.rs"] mod common;` from each integration-test
binary. It contains no logic.

## tests/common/relay_mocktest.rs — RELAY harness, not test functions

The `relay_mocktest` module mirrors `mocktest` but for the WebSocket
mock (`mock_relay`). Same plumbing-vs-tests distinction: helpers under
`tests/common/relay_mocktest.rs` aren't `#[test]`-attributed; the real
content-shaped assertions live in the `tests/relay_mock_*.rs` files.

- `tests/common/relay_mocktest.rs:50` — lock_journal: helper that returns the global serialization MutexGuard
- `tests/common/relay_mocktest.rs:96` — JournalEntry::frame_params: accessor on a struct
- `tests/common/relay_mocktest.rs:110` — JournalEntry::inner_params: accessor on a struct
- `tests/common/relay_mocktest.rs:123` — JournalEntry::event_params: accessor on a struct
- `tests/common/relay_mocktest.rs:177` — connected_client: builds a connected RelayClient via env-var redirect
- `tests/common/relay_mocktest.rs:191` — journal_all: HTTP client to `/__mock__/journal`, panics on transport failure
- `tests/common/relay_mocktest.rs:205` — journal_recv: filters journal_all by direction/method
- `tests/common/relay_mocktest.rs:237` — journal_last: panics if journal empty (real exercise)
- `tests/common/relay_mocktest.rs:246` — journal_reset: HTTP POST to `/__mock__/journal/reset`
- `tests/common/relay_mocktest.rs:255` — scenario_reset: HTTP POST to `/__mock__/scenarios/reset`
- `tests/common/relay_mocktest.rs:265` — reset_all: composes the two resets above
- `tests/common/relay_mocktest.rs:274` — arm_method: HTTP POST to `/__mock__/scenarios/<method>`
- `tests/common/relay_mocktest.rs:283` — arm_dial: HTTP POST to `/__mock__/scenarios/dial`
- `tests/common/relay_mocktest.rs:294` — push: HTTP POST to `/__mock__/push`
- `tests/common/relay_mocktest.rs:303` — inbound_call: HTTP POST to `/__mock__/inbound_call`
- `tests/common/relay_mocktest.rs:321` — TestGuard / begin: takes the global mutex and resets
- `tests/common/relay_mocktest.rs:332` — resolve_ws_port: parses MOCK_RELAY_PORT env var
- `tests/common/relay_mocktest.rs:343` — resolve_http_port: parses MOCK_RELAY_HTTP_PORT env var
- `tests/common/relay_mocktest.rs:355` — ensure_server: probes `/__mock__/health` and spawns mock_relay if missing
- `tests/common/relay_mocktest.rs:396` — probe_health: GETs `/__mock__/health` and returns success on 200 + `schemas_loaded`
- `tests/common/relay_mocktest.rs:468` — discover_porting_sdk_package: walks parents looking for `porting-sdk/test_harness/<name>`
- `tests/common/relay_mocktest.rs:489` — separator (unix): returns `:`
- `tests/common/relay_mocktest.rs:494` — separator (windows): returns `;`
- `tests/common/relay_mocktest.rs:499` — extern setsid: C binding
- `tests/common/relay_mocktest.rs:502` — libc_setsid: thin wrapper around setsid
- `tests/common/relay_mocktest.rs:510` — decode_journal: serde_json -> Vec<JournalEntry>
- `tests/common/relay_mocktest.rs:518` — decode_entry: serde_json -> single JournalEntry
- `tests/common/relay_mocktest.rs:564` — _hashmap_anchor: import-anchor for HashMap

## tests/relay_mock_*.rs — small test-file helpers

The relay mock test files use a few one-line helpers to build common
JSON payloads (state push frames, phone device shapes). These are pure
constructors; the assertions live in the test functions that use them.

- `tests/relay_mock_inbound_call.rs:29` — state_push_frame: builds a calling.call.state push body
- `tests/relay_mock_event_dispatch.rs:63` — bare_event_frame: builds a signalwire.event push body
- `tests/relay_mock_outbound_call.rs:18` — phone_device: builds a `{"type":"phone","params":...}` shape
- `tests/relay_mock_outbound_call.rs:22` — default_device: pre-canned phone_device for canned tests
