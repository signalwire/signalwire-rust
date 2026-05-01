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

- `tests/common/mocktest.rs:45` — lock_journal: helper that returns the global serialization MutexGuard
- `tests/common/mocktest.rs:82` — JournalEntry::body_object: accessor on a struct
- `tests/common/mocktest.rs:117` — journal_all: HTTP client to `/__mock__/journal`, panics on transport failure (real exercise)
- `tests/common/mocktest.rs:132` — journal_last: panics if journal empty (real exercise; called from real assertions in test files)
- `tests/common/mocktest.rs:141` — journal_reset: HTTP POST to `/__mock__/journal/reset`
- `tests/common/mocktest.rs:150` — scenario_reset: HTTP POST to `/__mock__/scenarios/reset`
- `tests/common/mocktest.rs:159` — reset_all: composes the two resets above
- `tests/common/mocktest.rs:178` — scenario_set: HTTP POST to `/__mock__/scenarios/<id>`
- `tests/common/mocktest.rs:189` — resolve_port: parses MOCK_SIGNALWIRE_PORT env var
- `tests/common/mocktest.rs:201` — ensure_server: probes `/__mock__/health` and spawns `python -m mock_signalwire` if missing
- `tests/common/mocktest.rs:232` — probe_health: GETs `/__mock__/health` and returns success on 200 + `specs_loaded`
- `tests/common/mocktest.rs:259` — spawn_server: spawns `python -m mock_signalwire` with setsid detach
- `tests/common/mocktest.rs:293` — setsid: extern C binding
- `tests/common/mocktest.rs:296` — libc_setsid: thin wrapper around setsid
- `tests/common/mocktest.rs:304` — decode_journal: serde_json -> Vec<JournalEntry>
- `tests/common/mocktest.rs:312` — decode_entry: serde_json -> single JournalEntry
- `tests/common/mocktest.rs:163` — TestGuard struct + RAII (no body, just storage)
- `tests/common/mocktest.rs:170` — begin: takes the global mutex and resets
- `tests/common/mocktest.rs:175` — begin: takes the global mutex and resets
- `tests/common/mocktest.rs:184` — scenario_set: HTTP POST to `/__mock__/scenarios/<id>`
- `tests/common/mocktest.rs:197` — resolve_port: parses MOCK_SIGNALWIRE_PORT env var
- `tests/common/mocktest.rs:208` — ensure_server: probes `/__mock__/health` and spawns `python -m mock_signalwire` if missing

## tests/common/mod.rs — module marker

This file exists solely to expose the `mocktest` submodule via
`#[path = "common/mod.rs"] mod common;` from each integration-test
binary. It contains no logic.
