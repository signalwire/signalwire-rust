//! REST API namespace modules.
//!
//! The REST resource surface is GENERATED from the canonical specs + x-sdk-*
//! markup into the `generated` submodule (see `scripts/generate_rest.py`). The
//! only hand-written namespace kept here is `compat` (the Twilio-compatible
//! LAML API), which is NOT part of the generated surface.

pub mod generated;

pub mod compat;
