//! Typed-handler → SWAIG parameter-schema inference.
//!
//! Python's `type_inference` reflects a tool handler's *signature and type
//! hints at runtime* (`inspect.signature` / `typing.get_type_hints`) to derive
//! the SWAIG parameter JSON Schema. Rust erases parameter types at compile time
//! and a closure carries no runtime signature metadata, so the reflection route
//! does not exist. The static-port idiom is the mirror image: instead of the
//! framework *reading* the handler's types, the developer *declares* them once —
//! here, with the fully-typed [`ParamsBuilder`] — and this module derives the
//! same schema tuple from that declaration.
//!
//! This is the same schema inference (the
//! `(parameters, required, description, is_typed, has_raw_data)` tuple), reached
//! through the typed-builder the port already ships rather than runtime
//! reflection.

use serde_json::{Map, Value};

use crate::swaig::FunctionResult;
use crate::swaig::params_builder::ParamsBuilder;

/// The tuple `infer_schema` returns:
///
/// - `parameters`: the `properties` object — a JSON object mapping each
///   parameter name to its JSON-Schema property dict.
/// - `required`: the names of the required parameters.
/// - `description`: the tool description (from the handler's doc), or `None`.
/// - `is_typed`: `true` when the handler declares typed parameters (new style).
/// - `has_raw_data`: `true` when the handler also receives the raw request data.
pub type InferredSchema = (Value, Vec<String>, Option<String>, bool, bool);

/// Derive the SWAIG parameter schema from a typed parameter declaration.
///
/// `params` is the developer's typed [`ParamsBuilder`] — the Rust idiom's
/// stand-in for the reflected handler signature Python inspects. `description`
/// is the tool description (Python reads it from the handler docstring; Rust
/// takes it explicitly since a closure carries no doc). `has_raw_data` records
/// whether the handler also consumes the raw request data.
///
/// Returns the same `(parameters, required, description, is_typed, has_raw_data)`
/// tuple Python's `infer_schema` returns. `is_typed` is `true` whenever any
/// property is declared (a fully-empty builder is the zero-param typed tool,
/// which is still typed — matching Python's zero-param typed path).
#[must_use]
pub fn infer_schema(
    params: &ParamsBuilder,
    description: Option<&str>,
    has_raw_data: bool,
) -> InferredSchema {
    let parameters = params.clone().build();
    let required = params.required_names().to_vec();
    (
        parameters,
        required,
        description.map(str::to_string),
        true,
        has_raw_data,
    )
}

/// The SWAIG calling convention every tool handler is invoked with:
/// `(args, raw_data) -> FunctionResult`.
pub type TypedHandler =
    Box<dyn Fn(&Map<String, Value>, &Map<String, Value>) -> FunctionResult + Send + Sync>;

/// Wrap a typed handler so it is invoked with the standard SWAIG calling
/// convention.
///
/// Python's wrapper unpacks the args dict into the handler's keyword arguments
/// (and passes `raw_data` when the handler declares it). Rust handlers already
/// take `(args, raw_data)` positionally, so the wrapper binds the `has_raw_data`
/// convention: when `false`, the wrapped handler is called with an empty
/// raw-data map, so a handler that ignores raw data never observes it — the same
/// observable behaviour as Python omitting the `raw_data` keyword.
#[must_use]
pub fn create_typed_handler_wrapper(func: TypedHandler, has_raw_data: bool) -> TypedHandler {
    Box::new(move |args, raw_data| {
        if has_raw_data {
            func(args, raw_data)
        } else {
            func(args, &Map::new())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swaig::params_builder::ParamsBuilder;
    use serde_json::json;

    #[test]
    fn infer_schema_from_typed_params_builds_properties_and_required() {
        // Build a schema from a typed params declaration — the Rust idiom's
        // equivalent of Python inferring it from a typed handler signature.
        let params = ParamsBuilder::new()
            .string("query", "The question or keywords to search")
            .integer("limit", "Max results")
            .required(["query"]);

        let (properties, required, description, is_typed, has_raw_data) =
            infer_schema(&params, Some("Search the FAQ knowledge base"), false);

        // Properties are the byte-identical `properties` object the handler would
        // otherwise hand-write.
        assert_eq!(
            properties,
            json!({
                "query": {"type": "string", "description": "The question or keywords to search"},
                "limit": {"type": "integer", "description": "Max results"}
            })
        );
        assert_eq!(required, vec!["query".to_string()]);
        assert_eq!(
            description.as_deref(),
            Some("Search the FAQ knowledge base")
        );
        assert!(is_typed);
        assert!(!has_raw_data);
    }

    #[test]
    fn infer_schema_zero_param_is_still_typed() {
        // An empty typed builder is the zero-param typed tool: no properties, no
        // required, but is_typed == true (matches Python's zero-param typed path).
        let (properties, required, description, is_typed, has_raw_data) =
            infer_schema(&ParamsBuilder::new(), None, true);
        assert_eq!(properties, json!({}));
        assert!(required.is_empty());
        assert!(description.is_none());
        assert!(is_typed);
        assert!(has_raw_data);
    }

    #[test]
    fn typed_handler_wrapper_honors_has_raw_data() {
        // has_raw_data == true: the wrapped handler observes the raw data.
        let with_raw = create_typed_handler_wrapper(
            Box::new(|_args, raw| {
                let seen = raw.get("call_id").and_then(Value::as_str).unwrap_or("none");
                FunctionResult::with_response(seen)
            }),
            true,
        );
        let mut raw = Map::new();
        raw.insert("call_id".to_string(), Value::from("abc"));
        let out = with_raw(&Map::new(), &raw);
        assert_eq!(out.to_value()["response"], "abc");

        // has_raw_data == false: the handler never observes the raw data (called
        // with an empty map), the same observable behaviour as Python omitting
        // the raw_data keyword.
        let without_raw = create_typed_handler_wrapper(
            Box::new(|_args, raw| {
                let seen = raw.get("call_id").and_then(Value::as_str).unwrap_or("none");
                FunctionResult::with_response(seen)
            }),
            false,
        );
        let out = without_raw(&Map::new(), &raw);
        assert_eq!(out.to_value()["response"], "none");
    }
}
