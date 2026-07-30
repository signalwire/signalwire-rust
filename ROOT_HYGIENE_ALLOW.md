# ROOT_HYGIENE_ALLOW.md

Repo-root files excused from the `root_hygiene` gate. Each is a load-bearing
porting-audit contract file that porting-sdk's shared audit scripts (and this
repo's `scripts/run-ci.sh` / `scripts/enumerate_*.py`) read at the repo root by
relative path; moving them under `eng/` would break the shared audit pipeline
(which this repo cannot edit). Verified reader per file below.

- CHECKLIST.md — required audit-contract file read by porting-sdk audit scripts (orchestrator, 2026-07-06)
- DOC_AUDIT_IGNORE.md — required audit-contract file read by porting-sdk audit scripts (audit_docs.py, ignore_ledger_verify.py) (orchestrator, 2026-07-06)
- INTENTIONAL_NON_IMPLEMENTATION.md — required audit-contract file read by porting-sdk audit scripts (audit_stubs.py, audit_no_cheat_tests.py) (orchestrator, 2026-07-06)
- INTENTIONAL_THIN_TESTS.md — required audit-contract file read by porting-sdk audit scripts (audit_no_cheat_tests.py) (orchestrator, 2026-07-06)
- PORT_ADDITIONS.md — required audit-contract file read by porting-sdk audit scripts (diff_port_signatures.py) (orchestrator, 2026-07-06)
- PORT_OMISSIONS.md — required audit-contract file read by porting-sdk audit scripts (diff_port_signatures.py) (orchestrator, 2026-07-06)
- PORT_SIGNATURE_OMISSIONS.md — required audit-contract file read by porting-sdk audit scripts (diff_port_signatures.py) (orchestrator, 2026-07-06)
- PORT_TEST_OMISSIONS.md — required audit-contract file read by porting-sdk audit scripts (orchestrator, 2026-07-06)
- PROGRESS.md — required porting-process progress file referenced by CLAUDE.md and the artifact_deny ledger (orchestrator, 2026-07-06)
- REST_COVERAGE_GAPS.md — required audit-contract file read by porting-sdk audit scripts (orchestrator, 2026-07-06)
- audit_coverage.json — required audit-contract file read by porting-sdk audit scripts (audit_coverage_map.py) (orchestrator, 2026-07-06)
- audit_coverage_baseline.json — required audit-contract file read by porting-sdk audit scripts (audit_coverage_map.py) (orchestrator, 2026-07-06)
- perf_baseline.json — committed per-port PERF-BASELINE ratchet baseline (r5 P2 SWML-render), read at repo root by scripts/run-ci.sh's PERF-BASELINE gate + porting-sdk perf_baseline.py; parallel to port_signatures.baseline.json (c2-rust, 2026-07-22)
- port_signatures.json — regenerated + read at root by scripts/run-ci.sh, scripts/enumerate_signatures.py, and porting-sdk diff_port_signatures.py (orchestrator, 2026-07-06)
- port_signatures.baseline.json — load-bearing SEMVER-DIFF release-floor file; mirrors port_signatures.json; must be at root (read by porting-sdk semver_diff.py), must not ship (Cargo.toml exclude) (orchestrator, 2026-07-13)
- port_surface.json — regenerated + read at root by scripts/run-ci.sh, scripts/enumerate_surface.py, and porting-sdk audit_docs.py/ignore_ledger_verify.py (orchestrator, 2026-07-06)
- port_surface_native.json — NATIVE-name sidecar written by scripts/enumerate_surface.py and read at exactly this path by porting-sdk suites/_doc_audit.py (`repo / "port_surface_native.json"` -> audit_docs --native-names); the path is the contract, so it must be at root; must not ship (Cargo.toml exclude) (b2-rust-tail, 2026-07-26)
- ROOT_HYGIENE_ALLOW.md — this allowlist itself, required at root by porting-sdk root_hygiene.py (orchestrator, 2026-07-06)
- SUPPRESSION_LEDGER.md — required audit-contract file read at repo root by porting-sdk suppression_ledger.py (`repo / "SUPPRESSION_LEDGER.md"`); ledgers the crate's 49 file-level #![allow] (orchestrator, 2026-07-09)
- WIRE_VIOLATIONS_ALLOW.md — STRICT-MOCKS signed-exception ledger read by porting-sdk assert_no_wire_violations.py / examples_run.py / snippet_run.py at repo root (mike@signalwire.com, 2026-07-18)
- WIRED_MODES.md — wired-modes manifest read by porting-sdk check_wired_modes.py (WIRED-MODES merge-coherence guard) at repo root (mike@signalwire.com, 2026-07-19)
- DOC_SURFACE_ALLOW.md — missing_docs allow-budget ledger read by the 6.3 doc-surface floor at repo root (mike@signalwire.com, 2026-07-19)
- .doc_surface_floor — DOC-SURFACE ratchet floor read at exactly this root path by porting-sdk doc_surface.py (`FLOOR_FILE = ".doc_surface_floor"`, used as `repo / FLOOR_FILE` at :379 to read and :415 to ratchet) and asserted by a_bar.py:63; the constant has no flag or override, so the path IS the contract and moving it to eng/ would break the shared audit pipeline this repo cannot edit. Same grounds as port_surface_native.json above. Written by scripts/run-ci.sh DOC-SURFACE (mike@signalwire.com, 2026-07-29)
