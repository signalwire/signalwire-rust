# COUNT-CLAIM allowlist

Format: `- <noun>:<claimed> — reason (approver, date)`

The COUNT-CLAIM gate (`porting-sdk/scripts/count_claim.py`) counts REST namespaces
via the glob `src/rest/namespaces/*.rs` (non-recursive, excluding `_generated` in
the *filename*). Rust ships **zero** hand-written per-namespace modules: every REST
namespace is generated into `src/rest/namespaces/generated/*_resources_generated.rs`
(14 generated resource modules + a client-tree facade), so the gate's glob matches
only `src/rest/namespaces/mod.rs` (the module index) and reports a count of **1**.

The documented "21 namespaces" claim is TRUE — it is the count of user-facing REST
namespace accessors on `RestClient` (verified against the authoritative
`CHECKLIST.md` "All 21 REST namespaces" list). The gate simply cannot see rust's
generated-namespace layout; this is a gate-glob-vs-idiom gap, not a stale doc.

NOTE FOR REVIEW: this entry documents a gate gap and needs orchestrator/human
ratification (or a gate fix that points the rust `ns_glob` at
`src/rest/namespaces/generated/*_resources_generated.rs`). It is not a laundered
finding — the 21-namespace surface is real and provable.

- namespaces:21 — gate ns_glob cannot count rust's generated namespace modules; 21 is the true RestClient namespace-accessor count per CHECKLIST.md (wave3-rust agent, 2026-07-11; PENDING human ratification)
- namespaces:21+ — same gate-glob gap; "21+" floor claim is likewise true (wave3-rust agent, 2026-07-11; PENDING human ratification)
