# WIRED_MODES.md — load-bearing run-ci modes (signalwire-rust)

The merge-coherence guard for `scripts/run-ci.sh`. Each entry below names a
line that MUST remain present in run-ci because it is the ENV/MODE context that
makes a gate actually *check* something — not a gate itself. The strict-mocks ×
Part-5 merge race silently dropped exactly this class of line from several ports
(a gate stayed green but vacuous). `check_wired_modes.py` greps run-ci for each
pattern and fails loud if one goes missing, so a future merge can't silently
un-wire a mode.

Format (consumed by `porting-sdk/scripts/check_wired_modes.py`): each required
entry is a list item of the form `` - `PATTERN` `` followed by an em-dash and a
reason. The `## Required patterns` section below holds the real entries.

## Required patterns

- `MOCK_RELAY_STRICT=1` — RELAY strict mode: the mock 400s an unknown frame field / duplicate command-id instead of tolerantly journaling it, so a RELAY wire-shape regression fails loud. Exported/passed to the TEST + STRICT-MOCKS + SNIPPET-RUN + EXAMPLES-RUN gates; without it those gates run against a tolerant mock and are vacuous.
- `MOCK_SIGNALWIRE_STRICT` — REST 400-on-unknown-key strict default: the REST mock rejects an unknown body key (STRICT-MOCKS §2.2c), which is what makes REST-COVERAGE + DOC-WIRE catch a wrong wire key. Set at the top of run-ci and inherited by every REST-facing gate.
- `assert_no_wire_violations` — the STRICT-MOCKS journal read in the REST-COVERAGE gate: after the wire-test suites run, it reads the live mock's wire_violations journal and reds the gate on ANY offender (respelling-proof). Dropping this line makes REST-COVERAGE pass even when the port emits an unknown/duplicate field the strict mock flagged.
- `diff_port_secure_default.py` — the SECURE-DEFAULT (A1) gate: proves `define_tool`'s secure state reaches the wire as the per-tool `__token`. Without this line the port can silently render tools as unauthenticated (rust DID, until 2026-07-26) and no other gate notices — the static checks only see the `secure` field being stored, never whether the render consumes it.
- `diff_port_secret_scrub.py` — the SECRET-SCRUB-LIVE gate: drives a real debug-level RELAY connect + re-auth and asserts no credential sentinel reaches the log. The STATIC `secret_scrub.py` leg cannot replace it — the leak found here was the project echoed inside the connect response's `identity` field, invisible to a frame-log-site grep.
- `scripts/secret_scrub.py" --port rust` — the SECRET-SCRUB static leg: greps the relay/skill source for the raw-frame credential-log shape. Cheap per-PR companion to the nightly LIVE gate; without it a reintroduced raw-frame log waits until the next nightly to surface. (Spelled with the `--port` suffix so this pattern cannot be satisfied by the `diff_port_secret_scrub.py` line, which contains `secret_scrub.py` as a substring.)
- `--native-names` — the DOC-AUDIT native-name sidecar: `port_surface.json` holds the FOLDED surface (reference spellings), so without this flag every member the enumerator folds — accessor renames, options-struct fields like `ServiceOptions::basic_auth` / `AgentOptions::use_pom` — becomes unresolvable in this crate's own docs, which are correct compiling code. Dropping the flag does not make DOC-AUDIT vacuous, it makes it FALSELY RED, and the tempting "fix" is to launder real names into DOC_AUDIT_IGNORE.md. (The failure mode this pattern guards is the reverse of the others here, which is why it belongs in the same manifest.)
- `tls_verify.py` — the TLS-VERIFY gate: no hardcoded TLS-verify-off construct in the builtin-skill / HTTP-client source. Nothing else in the suite would notice a `danger_accept_invalid_certs`-style regression.
- `ca_var_parity.py` — the CA-VAR gate: the REST source reads `SIGNALWIRE_REST_CA_FILE` and the RELAY source reads `SIGNALWIRE_RELAY_CA_FILE` under the exact fleet names. A rename would silently strand operators' custom-CA config.
