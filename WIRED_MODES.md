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
