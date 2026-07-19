//! SWAIG (SignalWire AI Gateway) tool definitions and results.
//!
//! Define server-callable tool functions and build their responses with the
//! fluent [`FunctionResult`] (40+ action methods: connect, hangup, say,
//! `send_sms`, …). Media/format options live in [`media_enums`].

pub mod function_result;
pub mod media_enums;
pub mod params_builder;
pub mod swaig_function;
// Generated payload/action modules — exempt from the missing_docs floor (§6.3
// allow-budget): their public surface is schema-derived DTOs whose docs would be
// the spec's field descriptions, not hand-authored prose. Annotated at the
// declaration site so no generated file is edited (keeps GEN-FRESH clean).
#[allow(missing_docs)]
pub mod post_prompt_generated;
#[allow(missing_docs)]
pub mod swaig_actions_generated;
#[allow(missing_docs)]
pub mod swaig_request_generated;

pub use function_result::{FunctionResult, KeysArg};
pub use media_enums::{Codec, ParseMediaEnumError, RecordDirection, RecordFormat, TapDirection};
pub use params_builder::{ParamKind, ParamsBuilder, PropertyBuilder};
pub use swaig_function::{SwaigFunction, SwaigHandler};
