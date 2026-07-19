# DOC_SURFACE_ALLOW.md — the shrinking missing-docs budget (plan §6.3)

The crate enables `#![warn(missing_docs)]` (in `src/lib.rs`), and LINT promotes
warnings to errors (`clippy … -D warnings`). To keep LINT green while the
hand-written item-level surface is documented incrementally, each module that
still has undocumented public items carries an `#[allow(missing_docs)]` at its
declaration site in `src/lib.rs`. **This file is the ledger of that budget.**

The budget is a RATCHET: it may only shrink. When a module's public items all
gain doc comments, delete its `#[allow(missing_docs)]` in `src/lib.rs` and its
row here. Adding a new undocumented public item to an *un-allowed* module reds
LINT — that is the floor working.

What is already documented (NOT in the budget):
- The crate-level `//!` landing page (`src/lib.rs`) — the docs.rs front page.
- Every module's own `//!` header (renders as each module's docs.rs page).
- `pom` — fully documented (no allow).
- All generated modules — exempt at their declaration sites (schema-derived; a
  separate concern from this hand-surface budget), see the `#[allow(missing_docs)]`
  in `src/swaig/mod.rs`, `src/swml/mod.rs`, `src/relay/mod.rs`,
  `src/rest/namespaces/mod.rs`.

## Budget (undocumented hand modules) — snapshot 2026-07-19

The item counts are approximate (from `cargo clippy --lib -- -W missing_docs`)
and exist to show relative size / track the ratchet, not as a hard gate.

| module (`src/lib.rs` decl) | ~undocumented items | notes |
|---|---|---|
| `relay` | ~150 | the RELAY "Simple-RPC" block (action.rs/call.rs/client.rs — the 57+ calling verbs) + constants/event; the largest cluster |
| `agent` | ~49 | AgentBase builder methods + fields |
| `contexts` | ~32 | ContextBuilder / Context / Step |
| `swml` | ~35 | service.rs / renderer.rs / document.rs items |
| `swaig` | ~17 | FunctionResult action methods |
| `skills` | ~30 | skill_base + builtin skill structs |
| `rest` | ~30 | http_client / client / request_options / pagination fields |
| `prefabs` | ~17 | archetype builder methods |
| `security` | ~1 | session/util items |
| `server` | ~4 | agent_server items |
| `serverless` | ~7 | adapter items |
| `web` | ~8 | web_service items |
| `core` | ~14 | security_config fields |
| `datamap` | ~1 | datamap items |
| `logging` | ~16 | Logger/Level items |
| `utils` | ~6 | schema_utils items |

Total starting budget: ~470 hand items (down from ~5950 before the generated
exemptions). The next ratchet step is documenting the `relay` Simple-RPC block
and dropping its allow.
