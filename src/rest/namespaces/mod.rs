//! REST API namespace modules.
//!
//! The REST resource surface is GENERATED from the canonical specs markup into the
//! `generated` submodule (see `scripts/generate_rest.py`).

// Generated REST resource + typed-I/O layer — exempt from the missing_docs floor
// (§6.3 allow-budget): the whole subtree is spec-derived (resources, request/
// response DTOs). Annotated at the declaration site so no generated file is
// edited (GEN-FRESH stays clean).
#[allow(missing_docs)]
pub mod generated;
