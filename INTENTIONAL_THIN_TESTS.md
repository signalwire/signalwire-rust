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

- `tests/common/relay_mocktest.rs:58` — lock_journal: helper that returns the global serialization MutexGuard
- `tests/common/relay_mocktest.rs:82` — CrossBinaryLock::acquire: opens /tmp lock file and runs flock(LOCK_EX); panics on failure
- `tests/common/relay_mocktest.rs:112` — CrossBinaryLock::drop: explicit flock(LOCK_UN) on file descriptor
- `tests/common/relay_mocktest.rs:126` — extern flock: C binding for the BSD-style file lock
- `tests/common/relay_mocktest.rs:137` — wait_for_no_sessions: polls /__mock__/sessions until count==0 or budget elapses
- `tests/common/relay_mocktest.rs:201` — JournalEntry::frame_params: accessor on a struct
- `tests/common/relay_mocktest.rs:215` — JournalEntry::inner_params: accessor on a struct
- `tests/common/relay_mocktest.rs:228` — JournalEntry::event_params: accessor on a struct
- `tests/common/relay_mocktest.rs:282` — journal_all: HTTP client to `/__mock__/journal`, panics on transport failure
- `tests/common/relay_mocktest.rs:296` — journal_recv: filters journal_all by direction/method
- `tests/common/relay_mocktest.rs:310` — journal_send: filters journal_all by direction/event_type
- `tests/common/relay_mocktest.rs:342` — journal_reset: HTTP POST to `/__mock__/journal/reset`
- `tests/common/relay_mocktest.rs:351` — scenario_reset: HTTP POST to `/__mock__/scenarios/reset`
- `tests/common/relay_mocktest.rs:360` — reset_all: composes the two resets above
- `tests/common/relay_mocktest.rs:370` — arm_method: HTTP POST to `/__mock__/scenarios/<method>`
- `tests/common/relay_mocktest.rs:379` — arm_dial: HTTP POST to `/__mock__/scenarios/dial`
- `tests/common/relay_mocktest.rs:388` — push: HTTP POST to `/__mock__/push`
- `tests/common/relay_mocktest.rs:399` — inbound_call: HTTP POST to `/__mock__/inbound_call`
- `tests/common/relay_mocktest.rs:408` — scenario_play: HTTP POST to `/__mock__/scenario_play`
- `tests/common/relay_mocktest.rs:438` — TestGuard / begin: takes the global mutex + cross-binary file lock + drains sessions, then resets
- `tests/common/relay_mocktest.rs:465` — resolve_ws_port: parses MOCK_RELAY_PORT env var
- `tests/common/relay_mocktest.rs:476` — resolve_http_port: parses MOCK_RELAY_HTTP_PORT env var
- `tests/common/relay_mocktest.rs:488` — ensure_server: probes `/__mock__/health` and spawns mock_relay if missing
- `tests/common/relay_mocktest.rs:529` — probe_health: GETs `/__mock__/health` and returns success on 200 + `schemas_loaded`
- `tests/common/relay_mocktest.rs:601` — discover_porting_sdk_package: walks parents looking for `porting-sdk/test_harness/<name>`
- `tests/common/relay_mocktest.rs:622` — separator (unix): returns `:`
- `tests/common/relay_mocktest.rs:627` — separator (windows): returns `;`
- `tests/common/relay_mocktest.rs:632` — extern setsid: C binding
- `tests/common/relay_mocktest.rs:635` — libc_setsid: thin wrapper around setsid
- `tests/common/relay_mocktest.rs:643` — decode_journal: serde_json -> Vec<JournalEntry>
- `tests/common/relay_mocktest.rs:651` — decode_entry: serde_json -> single JournalEntry
- `tests/common/relay_mocktest.rs:697` — _hashmap_anchor: import-anchor for HashMap

## tests/relay_mock_*.rs — small test-file helpers

The relay mock test files use a few one-line helpers to build common
JSON payloads (state push frames, phone device shapes). These are pure
constructors; the assertions live in the test functions that use them.

- `tests/relay_mock_inbound_call.rs:29` — state_push_frame: builds a calling.call.state push body
- `tests/relay_mock_event_dispatch.rs:63` — bare_event_frame: builds a signalwire.event push body
- `tests/relay_mock_outbound_call.rs:18` — phone_device: builds a `{"type":"phone","params":...}` shape
- `tests/relay_mock_outbound_call.rs:22` — default_device: pre-canned phone_device for canned tests

## tests/webhook_*.rs — webhook-validator test fixture helpers

The three webhook test files (`webhook_validator.rs`,
`webhook_middleware.rs`, `webhook_agent_base.rs`) use a small set of
non-`#[test]` helpers to build canonical signing inputs and test
fixtures. These are constructors / signers with no assertions of
their own; the content-shaped assertions live in the `#[test]`
functions that consume them.

- `tests/webhook_validator.rs:36` — vector_b_params: builds the canonical Vector B param list (sorted-by-key concat input)
- `tests/webhook_validator.rs:46` — vector_b_form_body: builds the wire-shape form-encoded body for Vector B
- `tests/webhook_validator.rs:157` — b64_sig: HMAC-SHA1 base64 signer used by the URL port-normalization tests
- `tests/webhook_middleware.rs:51` — HitCounter::count: returns the AtomicUsize hit count read by the assertions
- `tests/webhook_middleware.rs:56` — echo_handler: axum handler that bumps the hit count and echoes the buffered body
- `tests/webhook_middleware.rs:62` — build_router: assembles a Router with the WebhookLayer for each test
- `tests/webhook_middleware.rs:69` — read_body: drains an axum response body into Vec<u8> for byte comparison
- `tests/webhook_agent_base.rs:28` — make_agent: AgentBase factory parameterised by signing-key option
- `tests/webhook_agent_base.rs:42` — auth_headers: builds a Basic-auth HashMap for the agent's check_auth gate
- `tests/webhook_agent_base.rs:49` — hex_sig: Scheme-A HMAC-SHA1 hex signer for known url+body pairs

## tests/common/tls_support.rs — TLS capability-test harness, not test functions

Shared plumbing for the three TLS capability tests (`tls_wss_relay.rs`,
`tls_https_rest.rs`, `tls_https_server.rs`): cert discovery, `--tls` mock
spawning on dedicated ports, a CA-trusting HTTPS agent, and a cross-binary
`flock`. Same plumbing-vs-tests distinction as the other `common/` helpers —
these aren't `#[test]` items; the content-shaped assertions (real `wss://` /
`https://` round-trips plus untrusted-CA negative controls) live in the three
`tls_*` test files. The flagged helpers below have no assertion of their own.

- `tests/common/tls_support.rs:53` — certs_dir: walks to porting-sdk/test_harness/tls and runs the idempotent gen_certs.sh
- `tests/common/tls_support.rs:79` — ca_file: returns the certs/ca.crt path
- `tests/common/tls_support.rs:111` — discover_harness_pkg: walks parents looking for `porting-sdk/test_harness/<name>`
- `tests/common/tls_support.rs:134` — TlsMockProc::drop: kills + reaps the spawned `--tls` mock subprocess
- `tests/common/tls_support.rs:179` — spawn_tls_mock_relay: starts `mock_relay --tls` on the dedicated WS/HTTP ports, waits for health
- `tests/common/tls_support.rs:205` — spawn_tls_mock_signalwire: starts `mock_signalwire --tls` on the dedicated port, waits for HTTPS health
- `tests/common/tls_support.rs:280` — RelayTlsLock::acquire: opens /tmp lock file and runs flock(LOCK_EX); panics on failure
- `tests/common/tls_support.rs:301` — RelayTlsLock::drop: explicit flock(LOCK_UN) on file descriptor
- `tests/common/tls_support.rs:312` — extern setsid: C binding for session detach
- `tests/common/tls_support.rs:315` — libc_setsid: thin wrapper around setsid

## tests/tls_*.rs — TLS capability-test file helpers

A couple of one-off helpers local to the TLS test files read the mock journal
over the control plane. They panic on transport/decode failure but carry no
assertion of their own; the content-shaped assertions (protocol string set,
`signalwire.connect` recorded, untrusted-CA rejection) live in the `#[test]`
functions that consume them.

- `tests/tls_wss_relay.rs:138` — journal_recv_methods: GETs the plain-HTTP `/__mock__/journal` and returns the recv-direction method names

## tests/relay_mock_typed_errors.rs + tests/server_typed_errors.rs — typed-error test-file helpers

Two helper factories local to the typed-error test files. They construct test fixtures (a relay client pointed at the mock with caller-chosen creds; an agent built through the fluent `AgentOptions` builder) and carry no assertion of their own; the content-shaped assertions — that each genuine failure surfaces the right `RelayError` / `ServerError` *variant* with the expected carried data — live in the `#[test]` functions that consume them.

- `tests/relay_mock_typed_errors.rs:27` — client_with_creds: builds a RelayClient at the mock host with caller-chosen project/token (to drive the auth-rejection path); mirrors `connected_client` minus the connect-must-succeed assertion
- `tests/server_typed_errors.rs:17` — agent: AgentBase factory built via the fluent `AgentOptions` builder (also exercises the builder item under test)
