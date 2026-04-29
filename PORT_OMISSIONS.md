# PORT_OMISSIONS.md (signalwire-rust)

Python symbols deliberately not implemented in this port. Format:

```
- <fully.qualified.symbol>: <one-sentence rationale>
```

`scripts/diff_port_surface.py` reads this to know which Python symbols
to ignore when checking parity. Anything not in this file AND not
implemented in the port fails the diff.

---

## Skip-list categories (already documented in porting-sdk skip rules)

The Rust port deliberately omits:

- **Search-related modules.** Python's `signalwire/search/`, `signalwire/cli/build_search.py`, `signalwire/cli/dokku.py` and the search-skill family are Python-only per the SignalWire SDK skip list. Vector embedding indexing requires the Python ML stack; the Rust port doesn't ship it.
- **Sigmond integration.** Python ships sigmond-* helpers tied to internal infrastructure not exposed via the Rust port.
- **Generative test/example helpers in `signalwire.cli.*`** (CustomArgumentParser, init_project, build_search) — Python-specific scaffolding not portable.

(The diff_port_surface.py categorical-skip pattern would simplify this — currently each symbol must be enumerated individually; a known limitation we accept.)

## Known real omissions to fix

- `signalwire.agents.bedrock.BedrockAgent` and its methods — Bedrock prefab is required per the parity-with-Python rule (memory feedback_bedrock_deprioritized.md). Currently not shipped in Rust. **TO FIX:** port the Python `prefabs/bedrock_agent.py` to Rust. Tracking task #48 (Go has the same gap; Rust needs its own port too).

## Documented per-symbol omissions

> The full ~1177-symbol gap surfaced by diff_port_surface.py is being triaged as a separate task. This file currently captures the broad-categorical omissions above. Per-symbol entries below are populated as each gap is reviewed.

(empty pending triage)
