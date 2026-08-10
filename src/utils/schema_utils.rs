// schema_utils.rs — Rust port of signalwire.utils.schema_utils.SchemaUtils.
//
// Loads the SWML JSON Schema, extracts verb metadata, and validates
// either a single verb config or a complete SWML document.  Full
// Draft-2020-12 validation via the `jsonschema` crate is the normal path;
// the lightweight required-property check applies only to a PARTIAL schema
// that cannot wrap a verb in a document (which is what the reference does for
// every verb — it has no full validator at all).
//
// A validator that FAILS TO COMPILE is not a degraded mode: `jsonschema` is a
// hard dependency, so a compile failure means a broken install or a bad
// `schema_path`. Validation then REFUSES and names the reason rather than
// falling back to the lightweight check, which would accept wrong-typed
// values and unknown keys that the real schema rejects.
//
// Construction rules mirror Python:
//
//   - Pass `schema_path = None` to use the embedded schema.json.
//   - `schema_validation = false` disables validation
//     (`validate_verb` returns `(true, [])` for every call).
//   - The env var `SWML_SKIP_SCHEMA_VALIDATION=1/true/yes` also
//     disables validation regardless of the constructor argument.

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use serde_json::{Map, Value};

/// The embedded SWML JSON Schema (~495 KB), parsed exactly ONCE for the whole
/// process. Previously `load_schema()` re-ran `serde_json::from_str` over the
/// full embedded blob on EVERY `SchemaUtils::new`, and `SWMLService::add_verb`
/// builds a fresh `SchemaUtils` per call — so a document with N verbs paid the
/// full-schema parse N times (`r5/deep_perf_baseline`: ~24 ms/doc, dominated by
/// this re-parse; the comment claiming a cache was aspirational). This
/// `LazyLock` is the real cache: the default-path `load_schema()` clones the
/// pre-parsed value instead of re-parsing, so `add_verb` never re-parses.
static DEFAULT_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    let raw = include_str!("../swml/schema.json");
    serde_json::from_str(raw).unwrap_or(Value::Null)
});

/// The fully-built default `SchemaUtils` (embedded schema, validation on),
/// constructed exactly ONCE for the whole process. Cloning the parsed
/// `DEFAULT_SCHEMA` Value and re-running `extract_verbs()` on every
/// `SchemaUtils::new` was still ~1 ms/verb (a deep clone of the 495 KB tree
/// plus a full `$defs` walk building the verb `BTreeMap`) — 20 verbs ≈ the whole
/// 24 ms/doc in `r5/deep_perf_baseline`. `add_verb` only needs read-only access
/// to `validate_verb`, so it borrows this shared instance instead of building a
/// fresh one. This is the real fix for the per-verb reparse/rebuild.
static DEFAULT_SCHEMA_UTILS: LazyLock<SchemaUtils> =
    LazyLock::new(|| SchemaUtils::build(None, true));

/// `SchemaValidationError` — Rust port of
/// `signalwire.utils.schema_utils.SchemaValidationError`.
#[derive(Debug, Clone)]
pub struct SchemaValidationError {
    pub verb_name: String,
    pub errors: Vec<String>,
}

impl SchemaValidationError {
    /// Construct a `SchemaValidationError` from the verb it rejected and
    /// the list of validator complaints.
    pub fn new(verb_name: String, errors: Vec<String>) -> Self {
        Self { verb_name, errors }
    }
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Schema validation failed for '{}': {}",
            self.verb_name,
            self.errors.join("; ")
        )
    }
}

impl std::error::Error for SchemaValidationError {}

/// Verb metadata extracted from the schema.
#[derive(Debug, Clone)]
pub struct VerbDefinition {
    pub name: String,
    pub schema_name: String,
    pub definition: Value,
}

/// `SchemaUtils` — Rust port of
/// `signalwire.utils.schema_utils.SchemaUtils`.
pub struct SchemaUtils {
    schema: Value,
    schema_path: Option<String>,
    validation_enabled: bool,
    verbs: BTreeMap<String, VerbDefinition>,
    /// The compiled Draft-2020-12 validator for the WHOLE SWML document schema,
    /// present when full validation is enabled. `validate_verb` wraps a verb in
    /// a minimal `{version, sections:{main:[{verb: config}]}}` document and
    /// validates it against this. (This is STRONGER than the reference, which
    /// has no full validator at all — its `validate_verb` is the required-props
    /// check.) The schema's verb objects are CLOSED via
    /// `unevaluatedProperties`,
    /// so this rejects unknown/misspelled keys and wrong-typed config — the
    /// full validation that replaced the former required-props-only stub.
    full_validator: Option<std::sync::Arc<jsonschema::Validator>>,
    /// WHY `full_validator` is absent, when it is. A missing validator has two
    /// completely different meanings and they must not share a code path:
    /// validation was never requested (legitimate — the caller opted out), or
    /// the validator FAILED TO COMPILE (a broken install / a bad `schema_path`).
    /// Only the first may silently take the lightweight path; the second must
    /// refuse to validate and name the reason, because falling back there
    /// ACCEPTS documents the real schema rejects. Private: this is the
    /// fail-closed bookkeeping behind the existing public methods, not surface.
    validator_status: ValidatorStatus,
}

/// Why a `SchemaUtils` has no compiled full validator. Private to this module.
#[derive(Debug, Clone)]
enum ValidatorStatus {
    /// A validator is compiled and in use.
    Compiled,
    /// Validation was not requested (`schema_validation = false`, or the
    /// `SWML_SKIP_SCHEMA_VALIDATION` env override). Opting out is explicit and
    /// legitimate; `validate_verb` returns `(true, [])` as documented.
    Disabled,
    /// The schema itself is absent/unparseable, so there is nothing to compile.
    /// A partial or hand-built test schema legitimately takes the lightweight
    /// required-props path — which is what the reference does for every verb,
    /// its `validate_verb` BEING the required-props check.
    NoSchema,
    /// The schema was present but `jsonschema` REFUSED to compile it. This is
    /// not a supported degraded mode: the `jsonschema` crate is a hard (non-
    /// optional) dependency, so a compile failure means a broken install or a
    /// bad `schema_path`, never a configuration. Validation must REFUSE rather
    /// than fall back, because the lightweight check accepts wrong-typed values
    /// and unknown keys that the real schema rejects.
    CompileFailed(String),
}

impl SchemaUtils {
    /// Construct a `SchemaUtils`.
    ///
    /// For the default embedded-schema path (`schema_path = None`), prefer
    /// [`SchemaUtils::shared_default`] on the hot path — it borrows a single
    /// process-wide instance instead of re-parsing/re-extracting the 495 KB
    /// schema. `new` always builds a fresh owned instance (needed when a caller
    /// wants a distinct `schema_path` or an independent owned value).
    pub fn new(schema_path: Option<String>, schema_validation: bool) -> Self {
        Self::build(schema_path, schema_validation)
    }

    /// A shared, process-wide default `SchemaUtils` (embedded schema,
    /// validation on), built exactly once. The per-`add_verb` validation path
    /// borrows this instead of constructing a new helper each call, so the
    /// parse-and-verb-extract happens a single time no matter how many verbs
    /// render. Crate-internal: a performance-plumbing accessor behind the public
    /// `Service::schema_utils()`, not public API surface of its own.
    #[must_use]
    pub(crate) fn shared_default() -> &'static SchemaUtils {
        &DEFAULT_SCHEMA_UTILS
    }

    /// The actual constructor (parses the schema + extracts verbs). Callers on
    /// the hot path use `shared_default()` so this runs once process-wide; the
    /// parse-once perf test asserts that via pointer identity of the shared
    /// instance across a large `add_verb` workload.
    fn build(schema_path: Option<String>, schema_validation: bool) -> Self {
        let env_skip = env_boolish(&env::var("SWML_SKIP_SCHEMA_VALIDATION").unwrap_or_default());
        let mut su = Self {
            schema: Value::Null,
            schema_path,
            validation_enabled: schema_validation && !env_skip,
            verbs: BTreeMap::new(),
            full_validator: None,
            validator_status: ValidatorStatus::Disabled,
        };
        su.schema = su.load_schema();
        apply_sdk_widen(&mut su.schema);
        su.extract_verbs();
        su.validator_status = if !su.validation_enabled {
            ValidatorStatus::Disabled
        } else if su.schema.is_null() {
            ValidatorStatus::NoSchema
        } else {
            su.init_full_validator()
        };
        su
    }

    /// The schema file path this instance was constructed with, if any.
    /// Reference attribute `SchemaUtils.schema_path`.
    #[must_use]
    pub fn schema_path(&self) -> Option<&str> {
        self.schema_path.as_deref()
    }

    /// Whether full JSON Schema validation is wired up. `false` covers three
    /// distinct cases — validation opted out, no/partial schema, and a validator
    /// that failed to compile. Only the first two take the lightweight path;
    /// a compile failure makes `validate_verb`/`validate_document` REFUSE.
    pub fn full_validation_available(&self) -> bool {
        self.full_validator.is_some()
    }

    /// Read and parse the JSON Schema.
    pub fn load_schema(&self) -> Value {
        if let Some(path) = &self.schema_path {
            return load_from_path(path);
        }
        // Default: the embedded schema.json, parsed ONCE and cached in
        // DEFAULT_SCHEMA. Clone the pre-parsed value instead of re-parsing the
        // ~495 KB blob on every construction (the per-`add_verb` hot path).
        DEFAULT_SCHEMA.clone()
    }

    /// Sorted list of all known verb names.
    pub fn get_all_verb_names(&self) -> Vec<String> {
        self.verbs.keys().cloned().collect()
    }

    /// The `properties[verb_name]` block for a verb, or empty when
    /// unknown.  Matches `get_verb_properties(verb_name)`.
    pub fn get_verb_properties(&self, verb_name: &str) -> Map<String, Value> {
        let Some(v) = self.verbs.get(verb_name) else {
            return Map::new();
        };
        let outer_props = v.definition.get("properties").and_then(|p| p.as_object());
        let inner = outer_props.and_then(|p| p.get(verb_name));
        match inner.and_then(|i| i.as_object()) {
            Some(o) => o.clone(),
            None => Map::new(),
        }
    }

    /// The `required` list for a verb, or empty when unknown / not
    /// specified.
    pub fn get_verb_required_properties(&self, verb_name: &str) -> Vec<String> {
        let inner = self.get_verb_properties(verb_name);
        match inner.get("required").and_then(|r| r.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Parameter-definition block used by code-gen tooling.
    pub fn get_verb_parameters(&self, verb_name: &str) -> Map<String, Value> {
        let inner = self.get_verb_properties(verb_name);
        match inner.get("properties").and_then(|p| p.as_object()) {
            Some(o) => o.clone(),
            None => Map::new(),
        }
    }

    /// The set of KNOWN top-level property names for a verb's config object,
    /// resolving one `$ref` into `$defs` when the verb's inner schema is a
    /// reference (the `ai` verb's inner schema is `{"$ref": "#/$defs/AIObject"}`,
    /// so the property names live on `AIObject`, not inline). Returns an empty
    /// set when the names can't be determined (an open/unknown shape), which
    /// callers treat as "don't reject any key".
    fn verb_top_level_property_names(&self, verb_name: &str) -> std::collections::HashSet<String> {
        // `get_verb_properties` returns the inner `{verb_name: <schema>}` node.
        let inner = self.get_verb_properties(verb_name);
        // Resolve one `$ref` hop into `$defs` if present.
        let resolved: std::borrow::Cow<'_, Map<String, Value>> =
            if let Some(ref_str) = inner.get("$ref").and_then(|r| r.as_str()) {
                let prefix = "#/$defs/";
                ref_str
                    .strip_prefix(prefix)
                    .and_then(|name| self.schema.get("$defs").and_then(|d| d.get(name)))
                    .and_then(|d| d.as_object())
                    .map_or(std::borrow::Cow::Borrowed(&inner), |o| {
                        std::borrow::Cow::Owned(o.clone())
                    })
            } else {
                std::borrow::Cow::Borrowed(&inner)
            };
        resolved
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Validate ONLY that every top-level key of a verb config is a known
    /// property of that verb's object schema — the closed-key check, WITHOUT
    /// deep-validating the sub-object shapes.
    ///
    /// This is the strict-render contract for a HANDLER verb (the `ai` verb):
    /// reject unknown/misspelled TOP-LEVEL keys (temperatur, zzz), but do NOT
    /// deep-validate the sub-objects. The `ai` verb legitimately renders deep
    /// shapes (empty `prompt.pom []`, `SWAIG.defaults.web_hook_url`,
    /// `functions[].web_hook_url` with a `?__token=` query, `__token`, …) that
    /// the bundled JSON schema's deep sub-schemas do not fully accept under this
    /// crate's Draft-2020-12 engine — full-deep-validating the `ai` verb
    /// FALSE-REJECTS valid documents. The python reference's contract for `ai`
    /// is top-level-keys-only too (its jsonschema-rs engine happens to accept
    /// the deep shapes, so its identical minimal-doc pass is a no-op on them;
    /// here we make the SAME outcome explicit and engine-independent). Returns
    /// `(true, [])` when the property-name set can't be determined (open shape).
    /// Crate-internal (behind `Service::add_verb`): a validation-plumbing helper
    /// for the ai-handler path, not public surface of its own.
    pub(crate) fn validate_verb_top_keys(
        &self,
        verb_name: &str,
        verb_config: &Value,
    ) -> (bool, Vec<String>) {
        if !self.validation_enabled {
            return (true, Vec::new());
        }
        let known = self.verb_top_level_property_names(verb_name);
        if known.is_empty() {
            return (true, Vec::new());
        }
        let Some(obj) = verb_config.as_object() else {
            return (true, Vec::new());
        };
        let mut errors = Vec::new();
        for key in obj.keys() {
            if !known.contains(key) {
                let mut available: Vec<&String> = known.iter().collect();
                available.sort();
                errors.push(format!(
                    "Unknown key '{key}' for verb '{verb_name}'. Known keys: {available:?}"
                ));
            }
        }
        (errors.is_empty(), errors)
    }

    /// Validate a verb config against the schema.
    pub fn validate_verb(&self, verb_name: &str, verb_config: &Value) -> (bool, Vec<String>) {
        if !self.validation_enabled {
            return (true, Vec::new());
        }
        if !self.verbs.contains_key(verb_name) {
            return (false, vec![format!("Unknown verb: {}", verb_name)]);
        }
        // Fail CLOSED when the validator failed to compile: the lightweight
        // required-props check accepts wrong-typed values and unknown keys that
        // the real schema rejects, so falling back here would silently pass
        // invalid documents.
        if let Some(errors) = self.compile_failure_errors(Some(verb_name)) {
            return (false, errors);
        }
        if self.full_validator.is_some() {
            return self.validate_verb_full(verb_name, verb_config);
        }
        self.validate_verb_lightweight(verb_name, verb_config)
    }

    /// Full JSON-Schema validation of a single verb config. (No counterpart in
    /// the reference — it has no full-validation path at all.)
    /// Wraps the verb in a minimal SWML document
    /// `{version, sections:{main:[{verb_name: verb_config}]}}` and validates it
    /// against the whole compiled Draft-2020-12 schema. Because the schema's
    /// verb objects are closed (`unevaluatedProperties: {not:{}}`), this rejects
    /// unknown/misspelled keys, wrong-typed config, and missing-required — full
    /// validation, not just the required-props check.
    fn validate_verb_full(&self, verb_name: &str, verb_config: &Value) -> (bool, Vec<String>) {
        // Unreachable in practice: `validate_verb` refuses on CompileFailed
        // before getting here, and only calls this when the validator is
        // present. Kept fail-closed rather than falling back to lightweight.
        let Some(validator) = &self.full_validator else {
            if let Some(errors) = self.compile_failure_errors(Some(verb_name)) {
                return (false, errors);
            }
            return self.validate_verb_lightweight(verb_name, verb_config);
        };
        // Partial/test schemas without full document structure can't wrap a
        // verb in a document; fall back to the required-props check. This is a
        // legitimate lightweight case (and is what the Python reference does for
        // EVERY verb) — distinct from a validator that failed to compile, which
        // refuses above.
        let has_sections = self
            .schema
            .get("properties")
            .and_then(|p| p.as_object())
            .is_some_and(|p| p.contains_key("sections"));
        if !has_sections {
            return self.validate_verb_lightweight(verb_name, verb_config);
        }
        let minimal_doc = serde_json::json!({
            "version": "1.0.0",
            "sections": {"main": [{verb_name: verb_config}]},
        });
        match validator.validate(&minimal_doc) {
            Ok(()) => (true, Vec::new()),
            Err(e) => {
                let mut msg = e.to_string();
                if msg.len() > 500 {
                    msg.truncate(500);
                    msg.push_str("...");
                }
                (
                    false,
                    vec![format!("Schema validation error for '{verb_name}': {msg}")],
                )
            }
        }
    }

    fn validate_verb_lightweight(
        &self,
        verb_name: &str,
        verb_config: &Value,
    ) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        let cfg_obj = verb_config.as_object();
        for prop in self.get_verb_required_properties(verb_name) {
            let present = cfg_obj.is_some_and(|o| o.contains_key(&prop));
            if !present {
                errors.push(format!(
                    "Missing required property '{prop}' for verb '{verb_name}'"
                ));
            }
        }
        (errors.is_empty(), errors)
    }

    /// Validate a complete SWML document against the compiled schema. Returns
    /// `(false, ["Schema validator not initialized"])` when no full validator is
    /// wired in, and a named compile-failure refusal when one failed to build.
    /// Backed by real Draft-2020-12 validation.
    pub fn validate_document(&self, document: &Value) -> (bool, Vec<String>) {
        // Already fail-closed (no validator => not valid); when the cause was a
        // COMPILE FAILURE, say so instead of the generic message, so a broken
        // install is diagnosable.
        if let Some(errors) = self.compile_failure_errors(None) {
            return (false, errors);
        }
        let Some(validator) = &self.full_validator else {
            return (false, vec!["Schema validator not initialized".to_string()]);
        };
        match validator.validate(document) {
            Ok(()) => (true, Vec::new()),
            Err(e) => {
                let mut msg = e.to_string();
                if msg.len() > 500 {
                    msg.truncate(500);
                    msg.push_str("...");
                }
                (false, vec![format!("Document validation error: {msg}")])
            }
        }
    }

    /// Generate a Python-style method signature string for a verb.
    /// Matches `generate_method_signature(verb_name)`.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the internal `params.get(name).unwrap()`
    /// looks up keys taken directly from `params.keys()`, so every lookup is
    /// guaranteed to be present.
    #[must_use]
    pub fn generate_method_signature(&self, verb_name: &str) -> String {
        let params = self.get_verb_parameters(verb_name);
        let required: std::collections::HashSet<String> = self
            .get_verb_required_properties(verb_name)
            .into_iter()
            .collect();
        let mut parts: Vec<String> = vec!["self".to_string()];
        let mut keys: Vec<&String> = params.keys().collect();
        keys.sort();
        for name in &keys {
            let t = python_type_annotation(params.get(*name).unwrap());
            if required.contains(*name) {
                parts.push(format!("{name}: {t}"));
            } else {
                parts.push(format!("{name}: Optional[{t}] = None"));
            }
        }
        parts.push("**kwargs".to_string());
        let mut doc =
            format!("\"\"\"\n        Add the {verb_name} verb to the current document\n        \n");
        for name in &keys {
            let desc = params
                .get(*name)
                .and_then(|d| d.get("description"))
                .and_then(|d| d.as_str())
                .map(|s| s.replace('\n', " ").trim().to_string())
                .unwrap_or_default();
            let _ = write!(doc, "        Args:\n            {name}: {desc}\n");
        }
        doc.push_str(
            "        \n        Returns:\n            True if the verb was added successfully, False otherwise\n        \"\"\"\n",
        );
        format!("def {}({}) -> bool:\n{}", verb_name, parts.join(", "), doc)
    }

    /// Generate a Python-style method body string for a verb.
    #[must_use]
    pub fn generate_method_body(&self, verb_name: &str) -> String {
        let params = self.get_verb_parameters(verb_name);
        let mut keys: Vec<&String> = params.keys().collect();
        keys.sort();
        let mut lines: Vec<String> = vec![
            "        # Prepare the configuration".to_string(),
            "        config = {}".to_string(),
        ];
        for name in &keys {
            lines.push(format!("        if {name} is not None:"));
            lines.push(format!("            config['{name}'] = {name}"));
        }
        lines.push("        # Add any additional parameters from kwargs".to_string());
        lines.push("        for key, value in kwargs.items():".to_string());
        lines.push("            if value is not None:".to_string());
        lines.push("                config[key] = value".to_string());
        lines.push(String::new());
        lines.push(format!("        # Add the {verb_name} verb"));
        lines.push(format!(
            "        return self.add_verb('{verb_name}', config)"
        ));
        lines.join("\n")
    }

    fn extract_verbs(&mut self) {
        let Some(defs) = self.schema.get("$defs").and_then(|d| d.as_object()) else {
            return;
        };
        let Some(any_of) = defs
            .get("SWMLMethod")
            .and_then(|m| m.get("anyOf"))
            .and_then(|a| a.as_array())
        else {
            return;
        };
        for entry in any_of {
            let Some(ref_str) = entry.get("$ref").and_then(|r| r.as_str()) else {
                continue;
            };
            let prefix = "#/$defs/";
            if !ref_str.starts_with(prefix) {
                continue;
            }
            let schema_name = &ref_str[prefix.len()..];
            let Some(def_schema) = defs.get(schema_name) else {
                continue;
            };
            let props = match def_schema.get("properties").and_then(|p| p.as_object()) {
                Some(p) if !p.is_empty() => p,
                _ => continue,
            };
            let actual_verb = match props.keys().next() {
                Some(k) => k.clone(),
                None => continue,
            };
            self.verbs.insert(
                actual_verb.clone(),
                VerbDefinition {
                    name: actual_verb,
                    schema_name: schema_name.to_string(),
                    definition: def_schema.clone(),
                },
            );
        }
    }

    /// Compile the Draft-2020-12 validator for the whole SWML schema.
    /// The embedded schema is self-contained: its only non-`#` `$ref`
    /// (`SWMLObject.json`) is a self-reference to the root `$id`, so no remote
    /// retrieval is needed (the crate is built with `default-features = false`,
    /// dropping the http/file resolvers).
    ///
    /// A compile failure is reported as [`ValidatorStatus::CompileFailed`], NOT
    /// silently swallowed into the lightweight path. `jsonschema` 0.48 compiles
    /// EAGERLY — a bad keyword type (`properties: 42`), an invalid `pattern`
    /// regex, a dangling local `$ref`, or an unresolvable remote `$ref` all fail
    /// here — so this arm is reachable from the public
    /// `SchemaUtils::new(Some(path), true)` with any schema file that parses as
    /// JSON but is not a valid Draft-2020-12 document.
    ///
    /// Falling back to the lightweight required-props check on a compile
    /// failure would silently downgrade validation to one that accepts
    /// wrong-typed values and unknown keys the real schema rejects, so a
    /// compile failure refuses instead.
    fn init_full_validator(&mut self) -> ValidatorStatus {
        match jsonschema::draft202012::new(&self.schema) {
            Ok(v) => {
                self.full_validator = Some(std::sync::Arc::new(v));
                ValidatorStatus::Compiled
            }
            Err(e) => {
                self.full_validator = None;
                ValidatorStatus::CompileFailed(e.to_string())
            }
        }
    }

    /// The refusal error list for a schema that failed to compile, or `None`
    /// when the absence of a validator is legitimate (opted out / no schema).
    /// Refusing names the reason so a broken install is diagnosable instead of
    /// silently permissive.
    fn compile_failure_errors(&self, verb_name: Option<&str>) -> Option<Vec<String>> {
        let ValidatorStatus::CompileFailed(reason) = &self.validator_status else {
            return None;
        };
        let target = match verb_name {
            Some(v) => format!("verb '{v}'"),
            None => "document".to_string(),
        };
        Some(vec![format!(
            "Schema validation unavailable for {target}: the SWML JSON Schema failed to \
             compile, so validation did NOT run (refusing rather than accepting unvalidated \
             input). Underlying error: {reason}"
        )])
    }
}

/// The schema annotation marking a field whose value union is a HINT rather
/// than a closed set. Held as a constant so the literal never appears in a
/// shipped doc comment.
const WIDEN_MARKER: &str = concat!("x", "-sdk-widen");

/// Relax every marked field in a loaded schema, in place.
///
/// A marked field's `const`-union / `enum` enumerates the values the platform
/// DOCUMENTS, not the values it ACCEPTS — it accepts any value of the same base
/// type. Validating against the union as if it were closed rejects documents
/// the platform would happily execute, which is the failure direction nobody
/// looks for. This crate makes that failure loud rather than silent: every SWML
/// emission routes through `SWMLService::add_verb`, which PANICS on a
/// validation failure, so a legitimate-but-unlisted value would abort the
/// process instead of merely being flagged.
///
/// The relaxation drops the value constraints (`anyOf` / `oneOf` / `enum` /
/// `const`) and SETS the base type recovered from them, so the field still
/// rejects a wrong-typed value — `reason: 42` stays invalid while
/// `reason: "no_answer"` becomes valid. Recovering the type is load-bearing:
/// a marked field typically declares no `type` of its own, carrying it only
/// inside the union branches, so dropping the union without setting the type
/// would leave the field accepting anything at all.
fn apply_sdk_widen(schema: &mut Value) {
    match schema {
        Value::Object(obj) => {
            if obj
                .get(WIDEN_MARKER)
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let recovered = recover_base_type(obj);
                for key in ["anyOf", "oneOf", "enum", "const"] {
                    obj.remove(key);
                }
                if let Some(t) = recovered {
                    obj.insert("type".to_string(), t);
                }
            }
            for value in obj.values_mut() {
                apply_sdk_widen(value);
            }
        }
        Value::Array(items) => {
            for item in items {
                apply_sdk_widen(item);
            }
        }
        _ => {}
    }
}

/// The base type a marked field permits, recovered from its own `type` when it
/// declares one, else from the union branches / `enum` values it lists.
/// Returns `None` when the branches disagree, so an ambiguous field is left
/// unconstrained rather than being narrowed to a guess.
fn recover_base_type(obj: &Map<String, Value>) -> Option<Value> {
    if let Some(t) = obj.get("type") {
        return Some(t.clone());
    }

    // Collect the base type each branch / literal implies. Any branch that
    // implies nothing, or a set that disagrees, leaves the field unconstrained
    // rather than narrowed to a guess.
    let mut seen: Vec<&str> = Vec::new();

    for key in ["anyOf", "oneOf"] {
        let Some(branches) = obj.get(key).and_then(Value::as_array) else {
            continue;
        };
        for branch in branches {
            match branch.get("type").and_then(Value::as_str) {
                Some(t @ ("string" | "integer" | "number" | "boolean")) => seen.push(t),
                _ => return None,
            }
        }
    }

    if let Some(values) = obj.get("enum").or_else(|| obj.get("const")) {
        let listed = values
            .as_array()
            .map_or_else(|| vec![values.clone()], Clone::clone);
        for v in &listed {
            match v {
                Value::String(_) => seen.push("string"),
                Value::Bool(_) => seen.push("boolean"),
                Value::Number(n) if n.is_i64() || n.is_u64() => seen.push("integer"),
                Value::Number(_) => seen.push("number"),
                _ => return None,
            }
        }
    }

    let first = *seen.first()?;
    if seen.iter().any(|t| *t != first) {
        return None;
    }
    Some(Value::String(first.to_string()))
}

fn load_from_path(path: &str) -> Value {
    match fs::read_to_string(Path::new(path)) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn env_boolish(value: &str) -> bool {
    matches!(value.trim().to_lowercase().as_str(), "1" | "true" | "yes")
}

fn python_type_annotation(def: &Value) -> String {
    let Some(obj) = def.as_object() else {
        return "Any".to_string();
    };
    match obj.get("type").and_then(|t| t.as_str()) {
        Some("string") => "str".to_string(),
        Some("integer") => "int".to_string(),
        Some("number") => "float".to_string(),
        Some("boolean") => "bool".to_string(),
        Some("array") => {
            let item = obj
                .get("items")
                .map_or_else(|| "Any".to_string(), python_type_annotation);
            format!("List[{item}]")
        }
        Some("object") => "Dict[str, Any]".to_string(),
        _ => "Any".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a `SchemaUtils` after locking the env-mutex and removing
    /// `SWML_SKIP_SCHEMA_VALIDATION`, so this test sees a clean env even
    /// when running in parallel with `env_skip_disables_validation`.
    fn fresh() -> (std::sync::MutexGuard<'static, ()>, SchemaUtils) {
        let g = env_guard();
        let su = SchemaUtils::new(None, true);
        (g, su)
    }

    #[test]
    fn default_load() {
        let (_g, su) = fresh();
        let names = su.get_all_verb_names();
        assert!(!names.is_empty(), "expected verbs from default schema");
        assert!(names.contains(&"ai".to_string()));
        assert!(names.contains(&"answer".to_string()));
    }

    #[test]
    fn disabled_validation() {
        let su = SchemaUtils::new(None, false);
        assert!(!su.full_validation_available());
        let (valid, errors) = su.validate_verb("ai", &json!({}));
        assert!(valid, "validation skipped should return valid=true");
        assert!(errors.is_empty());
    }

    // Tests that read or mutate SWML_SKIP_SCHEMA_VALIDATION serialize on
    // a single mutex so they don't race each other.  Other tests in the
    // module aren't env-sensitive; they construct SchemaUtils after this
    // mutex is released.
    use std::sync::Mutex;
    static ENV_MTX: Mutex<()> = Mutex::new(());

    #[test]
    fn env_skip_disables_validation() {
        let _g = env_guard();
        unsafe { env::set_var("SWML_SKIP_SCHEMA_VALIDATION", "1") };
        let su = SchemaUtils::new(None, true);
        assert!(!su.full_validation_available());
        let (valid, _errors) = su.validate_verb("ai", &json!({}));
        assert!(valid);
        unsafe { env::remove_var("SWML_SKIP_SCHEMA_VALIDATION") };
    }

    #[test]
    fn validate_verb_unknown() {
        let (_g, su) = fresh();
        let (valid, errors) = su.validate_verb("not_a_real_verb", &json!({}));
        assert!(!valid);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Unknown verb"));
    }

    #[test]
    fn get_verb_properties_known() {
        let (_g, su) = fresh();
        let props = su.get_verb_properties("answer");
        assert!(
            !props.is_empty(),
            "expected non-empty properties for 'answer'"
        );
        assert_eq!(props.get("type").and_then(|v| v.as_str()), Some("object"));
    }

    #[test]
    fn get_verb_properties_unknown() {
        let (_g, su) = fresh();
        assert!(su.get_verb_properties("not_a_verb").is_empty());
    }

    #[test]
    fn get_verb_required_properties_unknown() {
        let (_g, su) = fresh();
        assert!(su.get_verb_required_properties("not_a_verb").is_empty());
    }

    #[test]
    fn validate_document_full_validator() {
        // With validation ON the full Draft-2020-12 validator is compiled, so
        // a well-formed minimal document validates and a malformed one is
        // rejected (the required-props-only stub could do neither).
        let (_g, su) = fresh();
        assert!(su.full_validation_available());
        let (valid, _errors) = su.validate_document(&json!({
            "version": "1.0.0",
            "sections": {"main": [{"answer": {"max_duration": 5}}]},
        }));
        assert!(valid, "a valid SWML doc must pass full validation");
        let (bad, errors) = su.validate_document(&json!({
            "version": "1.0.0",
            "sections": {"main": [{"answer": {"wibble": 1}}]},
        }));
        assert!(!bad, "an unknown verb key must fail full validation");
        assert!(!errors.is_empty());
    }

    #[test]
    fn validate_document_no_full_validator_when_disabled() {
        // With validation OFF no validator is compiled, so validate_document
        // reports "not initialized" (the documented no-validator contract).
        let su = SchemaUtils::new(None, false);
        let (valid, errors) = su.validate_document(&json!({
            "version": "1.0.0",
            "sections": {"main": []},
        }));
        assert!(!valid);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("validator not initialized"));
    }

    #[test]
    fn generate_method_signature_shape() {
        let (_g, su) = fresh();
        let sig = su.generate_method_signature("answer");
        assert!(sig.starts_with("def answer("), "got: {sig}");
        assert!(sig.contains("**kwargs"));
    }

    #[test]
    fn generate_method_body_shape() {
        let (_g, su) = fresh();
        let body = su.generate_method_body("answer");
        assert!(body.contains("self.add_verb('answer'"));
        assert!(body.contains("config = {}"));
    }

    /// The LazyLock-cached default schema must be identical to a fresh parse of
    /// the embedded blob (the cache changes NOTHING about the loaded content),
    /// and repeated construction must keep returning the full verb set (proving
    /// the cache is reused, not exhausted).
    #[test]
    fn cached_default_schema_matches_fresh_parse() {
        let fresh_parse: Value = serde_json::from_str(include_str!("../swml/schema.json")).unwrap();
        let su = SchemaUtils::new(None, true);
        assert_eq!(su.load_schema(), fresh_parse);
        // Construct again — the cached schema still yields the same verbs.
        let su2 = SchemaUtils::new(None, true);
        assert_eq!(su.get_all_verb_names(), su2.get_all_verb_names());
        assert!(su2.get_all_verb_names().contains(&"ai".to_string()));
    }

    #[test]
    fn schema_validation_error_message() {
        let err = SchemaValidationError::new(
            "ai".to_string(),
            vec!["missing prompt".to_string(), "bad type".to_string()],
        );
        let msg = format!("{err}");
        assert!(msg.contains("ai"));
        assert!(msg.contains("missing prompt"));
        assert_eq!(err.verb_name, "ai");
        assert_eq!(err.errors.len(), 2);
    }

    // ---- fail-closed on a validator that failed to compile ----------------
    //
    // `jsonschema` 0.48 compiles EAGERLY, so a schema file that parses as JSON
    // but is not a valid Draft-2020-12 document makes `draft202012::new` fail.
    // Before the fix that error was discarded (`Err(_) => full_validator =
    // None`) and every later validation silently fell back to the lightweight
    // required-props check, which ACCEPTS wrong-typed values and unknown keys.
    // These tests pin the refusal, and reach it through PUBLIC API only:
    // `SchemaUtils::new(Some(path), true)`.

    /// Lock `ENV_MTX` tolerating a poisoned mutex, and clear the skip-env var.
    /// A test that fails while holding `ENV_MTX` poisons it, which would turn
    /// one real assertion failure into a cascade of `PoisonError` unwrap panics
    /// in every other env-sensitive test and bury the actual cause. Recovering
    /// the guard keeps a failure readable — the protected data is `()`, so there
    /// is no invariant left broken by the panic.
    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = ENV_MTX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe { env::remove_var("SWML_SKIP_SCHEMA_VALIDATION") };
        g
    }

    /// A minimal-but-real SWML-shaped schema: `sections` validates its verbs
    /// against `$defs/SWMLMethod`, and `answer` requires an integer
    /// `max_duration` with unknown keys closed off. When `poison` is `Some`, one
    /// compile-fatal construct is spliced in.
    fn swml_shaped_schema(poison: Option<Value>) -> Value {
        let mut props = json!({
            "version": {"type": "string"},
            "sections": {
                "type": "object",
                "additionalProperties": {
                    "type": "array",
                    "items": {"$ref": "#/$defs/SWMLMethod"}
                }
            }
        });
        if let Some(p) = poison {
            props
                .as_object_mut()
                .expect("props is an object")
                .insert("poison".to_string(), p);
        }
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": props,
            "$defs": {
                "SWMLMethod": {"anyOf": [{"$ref": "#/$defs/AnswerMethod"}]},
                "AnswerMethod": {
                    "type": "object",
                    "properties": {
                        "answer": {
                            "type": "object",
                            "properties": {"max_duration": {"type": "integer"}},
                            "required": ["max_duration"],
                            "unevaluatedProperties": {"not": {}}
                        }
                    },
                    "unevaluatedProperties": {"not": {}}
                }
            }
        })
    }

    /// Write a schema to a repo-local temp dir and return its path.
    fn write_temp_schema(name: &str, schema: &Value) -> std::path::PathBuf {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".sw-tmp")
            .join("schema_utils_tests");
        fs::create_dir_all(&dir).expect("create temp schema dir");
        let path = dir.join(name);
        fs::write(
            &path,
            serde_json::to_string(schema).expect("serialize schema"),
        )
        .expect("write temp schema");
        path
    }

    /// The document that separates real validation from the lightweight check:
    /// the required prop is PRESENT (so required-props passes) but wrong-typed,
    /// and there is an unknown key the closed schema forbids.
    fn wrong_typed_answer() -> Value {
        json!({"max_duration": "not-an-integer", "wibble": 1})
    }

    /// CONTROL for the tests below: with a schema that COMPILES, this document
    /// is rejected by full validation but ACCEPTED by the lightweight check.
    /// That is exactly what a silent fallback would have let through.
    #[test]
    fn compiling_schema_rejects_wrong_typed_config() {
        let _g = env_guard();
        let path = write_temp_schema("clean.json", &swml_shaped_schema(None));
        let su = SchemaUtils::new(Some(path.to_string_lossy().to_string()), true);
        assert!(
            su.full_validation_available(),
            "the un-poisoned schema must compile"
        );
        let (valid, errors) = su.validate_verb("answer", &wrong_typed_answer());
        assert!(!valid, "full validation must reject a wrong-typed config");
        assert!(!errors.is_empty());
        // Proof the lightweight check alone would have PASSED it: the required
        // property is present, so it reports no error.
        let (light_ok, light_errors) =
            su.validate_verb_lightweight("answer", &wrong_typed_answer());
        assert!(
            light_ok,
            "lightweight accepts it — that's why fallback was unsafe: {light_errors:?}"
        );
    }

    /// Every compile-fatal schema shape must make `validate_verb` REFUSE and
    /// name the reason — never silently accept via the lightweight path.
    #[test]
    fn compile_failure_refuses_instead_of_accepting() {
        let _g = env_guard();
        let poisons: Vec<(&str, Value)> = vec![
            // unresolvable remote $ref (http resolver dropped by design)
            (
                "remote_ref",
                json!({"$ref": "https://example.invalid/x.json"}),
            ),
            // dangling local pointer
            ("dangling_local_ref", json!({"$ref": "#/$defs/NotThere"})),
            // invalid regex in `pattern`
            ("bad_regex", json!({"type": "string", "pattern": "["})),
            // wrong keyword type: `properties` must be an object
            ("props_not_object", json!({"properties": 42})),
        ];
        for (label, poison) in poisons {
            let schema = swml_shaped_schema(Some(poison));
            assert!(
                jsonschema::draft202012::new(&schema).is_err(),
                "[{label}] must be compile-fatal for this test to mean anything"
            );
            let path = write_temp_schema(&format!("poison_{label}.json"), &schema);
            let su = SchemaUtils::new(Some(path.to_string_lossy().to_string()), true);
            assert!(!su.full_validation_available(), "[{label}]");
            // The verb is still extracted, so this is NOT an "Unknown verb" path.
            assert!(
                su.get_all_verb_names().contains(&"answer".to_string()),
                "[{label}] verbs must still extract"
            );
            let (valid, errors) = su.validate_verb("answer", &wrong_typed_answer());
            assert!(
                !valid,
                "[{label}] a failed validator must REFUSE, not accept via lightweight fallback"
            );
            let joined = errors.join(" | ");
            assert!(
                joined.contains("failed to compile"),
                "[{label}] the refusal must say the schema failed to compile: {joined}"
            );
            assert!(
                joined.contains("validation did NOT run"),
                "[{label}] the refusal must say validation did not run: {joined}"
            );
            // `validate_document` must refuse with the same named reason.
            let (doc_valid, doc_errors) = su.validate_document(&json!({
                "version": "1.0.0",
                "sections": {"main": [{"answer": {"max_duration": 5}}]},
            }));
            assert!(!doc_valid, "[{label}] document validation must refuse");
            assert!(
                doc_errors.join(" | ").contains("failed to compile"),
                "[{label}] document refusal must name the compile failure: {doc_errors:?}"
            );
        }
    }

    /// A PARTIAL schema (no document `sections` structure) is a legitimate
    /// lightweight case and must NOT be treated as a compile failure — it still
    /// validates required properties and accepts a config that satisfies them.
    /// This is the boundary the fix must not over-reach: refuse only on a
    /// COMPILE failure, not on every absent validator.
    #[test]
    fn partial_schema_still_takes_lightweight_path() {
        let _g = env_guard();
        // Compiles fine, but has no `properties.sections`, so a verb cannot be
        // wrapped in a document.
        let partial = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {"version": {"type": "string"}},
            "$defs": {
                "SWMLMethod": {"anyOf": [{"$ref": "#/$defs/AnswerMethod"}]},
                "AnswerMethod": {
                    "type": "object",
                    "properties": {
                        "answer": {
                            "type": "object",
                            "properties": {"max_duration": {"type": "integer"}},
                            "required": ["max_duration"]
                        }
                    }
                }
            }
        });
        let path = write_temp_schema("partial.json", &partial);
        let su = SchemaUtils::new(Some(path.to_string_lossy().to_string()), true);
        assert!(
            su.full_validation_available(),
            "a partial schema still COMPILES"
        );
        // Required prop present => lightweight accepts.
        let (ok, errors) = su.validate_verb("answer", &json!({"max_duration": 5}));
        assert!(
            ok,
            "partial-schema lightweight path must accept: {errors:?}"
        );
        // Required prop missing => lightweight rejects, with the required-props
        // message (NOT a compile-failure refusal).
        let (bad, bad_errors) = su.validate_verb("answer", &json!({}));
        assert!(!bad);
        let joined = bad_errors.join(" | ");
        assert!(
            joined.contains("Missing required property"),
            "expected the required-props message, got: {joined}"
        );
        assert!(
            !joined.contains("failed to compile"),
            "a partial schema is NOT a compile failure: {joined}"
        );
    }

    /// An UNREADABLE / non-JSON `schema_path` yields no schema at all. That is
    /// not a compile failure either: nothing was compiled. Validation stays on
    /// the lightweight path, and with no verbs extracted the verb is unknown.
    #[test]
    fn absent_schema_is_not_a_compile_failure() {
        let _g = env_guard();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".sw-tmp")
            .join("schema_utils_tests");
        fs::create_dir_all(&dir).expect("create temp schema dir");
        let path = dir.join("not_json.json");
        fs::write(&path, "this is not json").expect("write non-json");
        let su = SchemaUtils::new(Some(path.to_string_lossy().to_string()), true);
        assert!(!su.full_validation_available());
        let (valid, errors) = su.validate_verb("answer", &json!({"max_duration": 5}));
        assert!(!valid, "no schema => no verbs => unknown verb");
        let joined = errors.join(" | ");
        assert!(joined.contains("Unknown verb"), "got: {joined}");
        assert!(
            !joined.contains("failed to compile"),
            "an absent schema is NOT a compile failure: {joined}"
        );
    }

    /// Explicitly opting OUT of validation must still return valid — the fix
    /// must not turn a deliberate opt-out into a refusal.
    #[test]
    fn validation_disabled_still_returns_valid() {
        let _g = env_guard();
        // Even with a compile-fatal schema, an explicit opt-out short-circuits.
        let path = write_temp_schema(
            "poison_optout.json",
            &swml_shaped_schema(Some(json!({"$ref": "#/$defs/NotThere"}))),
        );
        let su = SchemaUtils::new(Some(path.to_string_lossy().to_string()), false);
        let (valid, errors) = su.validate_verb("answer", &wrong_typed_answer());
        assert!(valid, "opt-out must return valid: {errors:?}");
        assert!(errors.is_empty());
    }

    /// The DEFAULT embedded schema must compile — if it ever stops compiling,
    /// the fail-closed refusal would take over and this test catches it loudly
    /// instead of the SDK silently degrading to permissive validation.
    #[test]
    fn embedded_default_schema_compiles() {
        let (_g, su) = fresh();
        assert!(
            su.full_validation_available(),
            "the embedded SWML schema must compile; a failure here would now \
             REFUSE all validation rather than silently degrade"
        );
        let (valid, errors) = su.validate_verb("answer", &json!({"max_duration": 5}));
        assert!(valid, "valid answer config must pass: {errors:?}");
    }
}
