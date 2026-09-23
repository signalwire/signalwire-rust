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

## Relationship to the DOC-SURFACE gate

Two different measurements are in play and they do not agree — by design:

- **The DOC-SURFACE gate** (`porting-sdk/scripts/doc_surface.py`, floor in
  `.doc_surface_floor`) counts *declarations*: `pub fn` / `pub struct` /
  `pub enum` / `pub trait` / `pub type` / `pub const` in `src/**/*.rs`,
  excluding generated trees. **That reading is 100.0% (1499/1499) and the
  floor is pinned there** (2026-07-29).
- **rustc's `missing_docs`** counts a *wider* surface: it also wants doc
  comments on public struct FIELDS, enum VARIANTS, and trait ITEMS, which the
  gate's declaration regex never sees. That residue is what this budget still
  covers.

So a module can be at 100% by the gate and still carry an `#[allow]` here.

## Budget (undocumented hand modules) — measured 2026-07-29

Exact counts from `cargo clippy --lib --all-features` with the
`#[allow(missing_docs)]` lines temporarily stripped from `src/lib.rs`. These
are fields / enum variants / trait items only — every counted *declaration*
in these modules is documented.

| module (`src/lib.rs` decl) | undocumented items | was (2026-07-19) |
|---|---|---|
| `relay` | 28 | ~150 |
| `swml` | 26 | ~35 |
| `skills` | 22 | ~30 |
| `logging` | 16 | ~16 |
| `core` | 15 | ~14 |
| `rest` | 14 | ~30 |
| `agent` | 10 | ~49 |
| `web` | 8 | ~8 |
| `utils` | 6 | ~6 |
| `prefabs` | 5 | ~17 |
| `swaig` | 1 | ~17 |
| `serverless` | 1 | ~7 |
| `server` | 1 | ~4 |
| `security` | 1 | ~1 |
| `contexts` | 1 | ~32 |

Total remaining budget: **155** items, down from ~470.

**Ratchet step taken 2026-07-29:** `datamap` reached zero, so its
`#[allow(missing_docs)]` was deleted from `src/lib.rs` and its row from this
table. Adding an undocumented public item to `datamap` now reds LINT.

The next ratchet steps are the five modules sitting at a single item —
`swaig`, `serverless`, `server`, `security`, `contexts` — each needing one
field/variant doc before its `#[allow(missing_docs)]` can be dropped too.
