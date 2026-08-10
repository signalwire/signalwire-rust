use serde_json::{Map, Value, json};

/// Fluent builder for DataMap-based SWAIG function definitions.
///
/// A `DataMap` tool defines its behaviour declaratively (expressions, webhooks)
/// instead of with a code handler.
#[derive(Debug, Clone)]
#[must_use]
pub struct DataMap {
    function_name: String,
    purpose: String,

    /// JSON Schema properties for parameters.
    properties: Map<String, Value>,
    required_params: Vec<String>,

    expressions: Vec<Value>,
    webhooks: Vec<Value>,

    global_output: Option<Value>,
    global_error_keys: Option<Vec<String>>,
}

impl DataMap {
    /// Start building a server-side tool named `function_name`.
    ///
    /// A `DataMap` tool executes on SignalWire's infrastructure rather than
    /// against this agent's `/swaig` webhook, so it needs no handler and no
    /// reachable endpoint. `function_name` is the name the AI calls and is
    /// rendered as the `function` wire key.
    ///
    /// Everything else starts empty — describe the tool with
    /// [`purpose`](DataMap::purpose), declare its arguments with
    /// [`parameter`](DataMap::parameter), add the API call with
    /// [`webhook`](DataMap::webhook), and shape the response with
    /// [`output`](DataMap::output) / [`expression`](DataMap::expression).
    pub fn new(function_name: &str) -> Self {
        DataMap {
            function_name: function_name.to_string(),
            purpose: String::new(),
            properties: Map::new(),
            required_params: Vec::new(),
            expressions: Vec::new(),
            webhooks: Vec::new(),
            global_output: None,
            global_error_keys: None,
        }
    }

    /// The tool name this data-map defines. The reference stores its single ctor
    /// arg as the public attribute `DataMap.function_name` (`data_map.py:72`) and
    /// renders it as the `"function"` wire key (`:436`).
    #[must_use]
    pub fn function_name(&self) -> &str {
        &self.function_name
    }

    // ── Fluent setters ───────────────────────────────────────────────────

    /// Set the LLM-facing tool description (the "purpose"). **Prompt
    /// engineering, not developer documentation.**
    ///
    /// The description string is rendered into the OpenAI tool schema
    /// `description` field on every LLM turn. The model reads it to
    /// decide WHEN to call this tool. A vague `purpose()` is the #1
    /// cause of "the model has the right tool but doesn't call it"
    /// failures with data-map tools.
    ///
    /// # Bad vs good
    ///
    /// ```text
    /// BAD : .purpose("weather api")
    /// GOOD: .purpose("Get the current weather conditions and forecast "
    ///              + "for a specific city. Use this whenever the user "
    ///              + "asks about weather, temperature, rain, or similar "
    ///              + "conditions in a named location.")
    /// ```
    pub fn purpose(&mut self, desc: &str) -> &mut Self {
        self.purpose = desc.to_string();
        self
    }

    /// Alias for [`Self::purpose`]. Sets the LLM-facing tool
    /// description. This string is read by the model to decide WHEN
    /// to call this tool. See [`Self::purpose`] for bad-vs-good
    /// examples.
    pub fn description(&mut self, desc: &str) -> &mut Self {
        self.purpose(desc)
    }

    /// Add a parameter definition — the `description` is **LLM-FACING**.
    ///
    /// Each parameter description is rendered into the OpenAI tool
    /// schema under `parameters.properties.<name>.description` and
    /// sent to the model. The model uses it to decide HOW to fill in
    /// the argument from user speech. It is prompt engineering, not
    /// developer FYI.
    ///
    /// # Bad vs good
    ///
    /// ```text
    /// BAD : .parameter("city", "string", "the city", ...)
    /// GOOD: .parameter("city", "string",
    ///         "The name of the city to get weather for, e.g. "
    ///         "'San Francisco'. Ask the user if they did not "
    ///         "provide one. Include the state or country if the "
    ///         "city name is ambiguous.", ...)
    /// ```
    pub fn parameter(
        &mut self,
        name: &str,
        param_type: &str,
        description: &str,
        required: Option<bool>,
        enum_values: Option<Vec<&str>>,
    ) -> &mut Self {
        let required = required.unwrap_or(false);
        let mut prop = Map::new();
        prop.insert("type".to_string(), json!(param_type));
        prop.insert("description".to_string(), json!(description));
        if let Some(e) = enum_values.filter(|e| !e.is_empty()) {
            prop.insert("enum".to_string(), json!(e));
        }
        self.properties
            .insert(name.to_string(), Value::Object(prop));

        if required && !self.required_params.contains(&name.to_string()) {
            self.required_params.push(name.to_string());
        }
        self
    }

    /// Add an expression rule.
    pub fn expression(
        &mut self,
        test_value: &str,
        pattern: &str,
        output: Value,
        nomatch_output: Option<Value>,
    ) -> &mut Self {
        let mut expr = Map::new();
        expr.insert("string".to_string(), json!(test_value));
        expr.insert("pattern".to_string(), json!(pattern));
        expr.insert("output".to_string(), output);
        if let Some(nm) = nomatch_output {
            // HYPHENATED wire key, matching the reference (data_map.py:202) and the
            // behavioral manifest. An underscore is a key the server does not
            // recognise, so the no-match branch would never fire.
            expr.insert("nomatch-output".to_string(), nm);
        }
        self.expressions.push(Value::Object(expr));
        self
    }

    /// Add a webhook definition.
    pub fn webhook(
        &mut self,
        method: &str,
        url: &str,
        headers: Option<Value>,
        form_param: Option<&str>,
        input_args_as_params: Option<bool>,
        require_args: Option<Vec<&str>>,
    ) -> &mut Self {
        let input_args_as_params = input_args_as_params.unwrap_or(false);
        let mut wh = Map::new();
        // The reference upper-cases the method on the wire (core/data_map.py:230,
        // `"method": method.upper()`), so the same program emits byte-identical SWML in
        // both languages. The engine itself compares case-insensitively.
        wh.insert("method".to_string(), json!(method.to_uppercase()));
        wh.insert("url".to_string(), json!(url));

        if let Some(Value::Object(h)) = headers
            && !h.is_empty()
        {
            wh.insert("headers".to_string(), Value::Object(h));
        }
        if let Some(f) = form_param.filter(|f| !f.is_empty()) {
            wh.insert("form_param".to_string(), json!(f));
        }
        if input_args_as_params {
            wh.insert("input_args_as_params".to_string(), json!(true));
        }
        if let Some(r) = require_args.filter(|r| !r.is_empty()) {
            wh.insert("require_args".to_string(), json!(r));
        }
        self.webhooks.push(Value::Object(wh));
        self
    }

    /// Set expressions on the last webhook.
    pub fn webhook_expressions(&mut self, expressions: Vec<Value>) -> &mut Self {
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("expressions".to_string(), Value::Array(expressions));
        }
        self
    }

    /// Set params on the last webhook — the method for POST/PUT request data.
    ///
    /// `params` is part of the webhook contract: `schema.json` `$defs/Webhook` lists
    /// it among the ten permitted properties and forbids everything else, and the
    /// engine's webhook readers look it up. There is deliberately no `body` setter —
    /// a `body` key is schema-forbidden and read by no engine reader, so writing one
    /// produced an invalid document and silently discarded the caller's payload.
    pub fn params(&mut self, data: Value) -> &mut Self {
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("params".to_string(), data);
        }
        self
    }

    /// Set foreach on the last webhook.
    pub fn for_each(&mut self, config: Value) -> &mut Self {
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("foreach".to_string(), config);
        }
        self
    }

    /// Set output on the last webhook.
    pub fn output(&mut self, result: Value) -> &mut Self {
        let resolved = Self::resolve_output(result);
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("output".to_string(), resolved);
        }
        self
    }

    /// Process an array from the webhook response using the foreach mechanism.
    ///
    /// `foreach_config` is an object with keys `input_key`, `output_key`,
    /// `append` (all required) and an optional `max`. Attaches to the most
    /// recent webhook.
    ///
    /// # Panics
    ///
    /// Panics (matching `ValueError`) if no webhook has been added,
    /// if `foreach_config` is not an object, or if a required key is missing.
    pub fn foreach(&mut self, foreach_config: Value) -> &mut Self {
        assert!(
            !self.webhooks.is_empty(),
            "Must add webhook before setting foreach"
        );
        let Value::Object(cfg) = &foreach_config else {
            panic!("foreach_config must be a dictionary");
        };
        let missing: Vec<&str> = ["input_key", "output_key", "append"]
            .into_iter()
            .filter(|k| !cfg.contains_key(*k))
            .collect();
        assert!(
            missing.is_empty(),
            "foreach config missing required keys: {missing:?}"
        );
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("foreach".to_string(), foreach_config);
        }
        self
    }

    /// Set global fallback output.
    pub fn fallback_output(&mut self, result: Value) -> &mut Self {
        self.global_output = Some(Self::resolve_output(result));
        self
    }

    /// Set `error_keys` on the last webhook.
    pub fn error_keys(&mut self, keys: Vec<&str>) -> &mut Self {
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("error_keys".to_string(), json!(keys));
        }
        self
    }

    /// Set global `error_keys`.
    pub fn global_error_keys(&mut self, keys: Vec<&str>) -> &mut Self {
        self.global_error_keys = Some(
            keys.into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
        );
        self
    }

    // ── Serialisation ────────────────────────────────────────────────────

    /// Serialise to a SWAIG function definition.
    #[must_use]
    pub fn to_swaig_function(&self) -> Value {
        let mut func = Map::new();
        func.insert("function".to_string(), json!(self.function_name));

        if !self.purpose.is_empty() {
            func.insert("purpose".to_string(), json!(self.purpose));
        }

        if !self.properties.is_empty() {
            let mut argument = Map::new();
            argument.insert("type".to_string(), json!("object"));
            argument.insert(
                "properties".to_string(),
                Value::Object(self.properties.clone()),
            );
            if !self.required_params.is_empty() {
                argument.insert("required".to_string(), json!(self.required_params));
            }
            func.insert("argument".to_string(), Value::Object(argument));
        }

        let mut data_map = Map::new();

        if !self.expressions.is_empty() {
            data_map.insert(
                "expressions".to_string(),
                Value::Array(self.expressions.clone()),
            );
        }

        if !self.webhooks.is_empty() {
            data_map.insert("webhooks".to_string(), Value::Array(self.webhooks.clone()));
        }

        if let Some(ref output) = self.global_output {
            data_map.insert("output".to_string(), output.clone());
        }

        if let Some(ref keys) = self.global_error_keys {
            data_map.insert("error_keys".to_string(), json!(keys));
        }

        if !data_map.is_empty() {
            func.insert("data_map".to_string(), Value::Object(data_map));
        }

        Value::Object(func)
    }

    // ── Static Helpers ───────────────────────────────────────────────────

    /// Build a complete SWAIG function definition with a single webhook.
    pub fn create_simple_api_tool(
        name: &str,
        purpose: &str,
        parameters: Vec<Value>,
        method: &str,
        url: &str,
        output: Value,
        headers: Value,
    ) -> Value {
        let mut builder = DataMap::new(name);
        builder.purpose(purpose);

        for param in parameters {
            let p_name = param["name"].as_str().unwrap_or("");
            let p_type = param["type"].as_str().unwrap_or("string");
            let p_desc = param["description"].as_str().unwrap_or("");
            let p_required = param["required"].as_bool().unwrap_or(false);
            let p_enum: Vec<&str> = param["enum"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            builder.parameter(p_name, p_type, p_desc, Some(p_required), Some(p_enum));
        }

        builder.webhook(method, url, Some(headers), None, None, None);
        builder.output(output);

        builder.to_swaig_function()
    }

    /// Build a complete SWAIG function definition with expressions only.
    pub fn create_expression_tool(
        name: &str,
        purpose: &str,
        parameters: Vec<Value>,
        expressions: Vec<Value>,
    ) -> Value {
        let mut builder = DataMap::new(name);
        builder.purpose(purpose);

        for param in parameters {
            let p_name = param["name"].as_str().unwrap_or("");
            let p_type = param["type"].as_str().unwrap_or("string");
            let p_desc = param["description"].as_str().unwrap_or("");
            let p_required = param["required"].as_bool().unwrap_or(false);
            let p_enum: Vec<&str> = param["enum"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            builder.parameter(p_name, p_type, p_desc, Some(p_required), Some(p_enum));
        }

        for expr in expressions {
            let test_str = expr["string"].as_str().unwrap_or("");
            let pattern = expr["pattern"].as_str().unwrap_or("");
            let output = expr.get("output").cloned().unwrap_or(json!(null));
            // Accept either spelling on INPUT (these maps are caller-supplied), but
            // expression() always EMITS the hyphenated reference key.
            let nomatch = expr
                .get("nomatch-output")
                .or_else(|| expr.get("nomatch_output"))
                .cloned();
            builder.expression(test_str, pattern, output, nomatch);
        }

        builder.to_swaig_function()
    }

    // ── Private ──────────────────────────────────────────────────────────

    fn resolve_output(result: Value) -> Value {
        // If it looks like a FunctionResult serialisation, pass through
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construction() {
        let dm = DataMap::new("test_func");
        let val = dm.to_swaig_function();
        assert_eq!(val["function"], "test_func");
    }

    #[test]
    fn test_purpose() {
        let mut dm = DataMap::new("func");
        dm.purpose("Lookup weather");
        assert_eq!(dm.to_swaig_function()["purpose"], "Lookup weather");
    }

    #[test]
    fn test_description_alias() {
        let mut dm = DataMap::new("func");
        dm.description("Lookup weather");
        assert_eq!(dm.to_swaig_function()["purpose"], "Lookup weather");
    }

    #[test]
    fn test_parameter() {
        let mut dm = DataMap::new("func");
        dm.parameter("city", "string", "City name", Some(true), None);
        let val = dm.to_swaig_function();
        let props = &val["argument"]["properties"];
        assert_eq!(props["city"]["type"], "string");
        assert_eq!(props["city"]["description"], "City name");
        let required = val["argument"]["required"].as_array().unwrap();
        assert!(required.contains(&json!("city")));
    }

    #[test]
    fn test_parameter_with_enum() {
        let mut dm = DataMap::new("func");
        dm.parameter(
            "unit",
            "string",
            "Temperature unit",
            None,
            Some(vec!["celsius", "fahrenheit"]),
        );
        let val = dm.to_swaig_function();
        let enums = val["argument"]["properties"]["unit"]["enum"]
            .as_array()
            .unwrap();
        assert_eq!(enums.len(), 2);
    }

    #[test]
    fn test_parameter_not_required() {
        let mut dm = DataMap::new("func");
        dm.parameter("opt", "string", "optional", None, None);
        let val = dm.to_swaig_function();
        // required array should not exist if no required params
        assert!(val["argument"].get("required").is_none());
    }

    #[test]
    fn test_expression() {
        let mut dm = DataMap::new("func");
        dm.expression(
            "${args.color}",
            "red|blue",
            json!({"response": "matched"}),
            None,
        );
        let val = dm.to_swaig_function();
        let exprs = val["data_map"]["expressions"].as_array().unwrap();
        assert_eq!(exprs.len(), 1);
        assert_eq!(exprs[0]["string"], "${args.color}");
        assert_eq!(exprs[0]["pattern"], "red|blue");
    }

    #[test]
    fn test_expression_with_nomatch() {
        let mut dm = DataMap::new("func");
        dm.expression("${args.x}", "y", json!("hit"), Some(json!("miss")));
        let val = dm.to_swaig_function();
        let expr = &val["data_map"]["expressions"][0];
        // HYPHENATED key per the reference (data_map.py:202). An underscored key is
        // one the server ignores, so the no-match branch would never fire.
        assert_eq!(expr["nomatch-output"], "miss");
        assert!(expr.get("nomatch_output").is_none());
    }

    #[test]
    fn test_webhook() {
        let mut dm = DataMap::new("func");
        dm.webhook(
            "GET",
            "https://api.example.com/data",
            None,
            None,
            None,
            None,
        );
        let val = dm.to_swaig_function();
        let wh = &val["data_map"]["webhooks"][0];
        assert_eq!(wh["method"], "GET");
        assert_eq!(wh["url"], "https://api.example.com/data");
    }

    #[test]
    fn test_webhook_with_options() {
        let mut dm = DataMap::new("func");
        dm.webhook(
            "POST",
            "https://api.example.com",
            Some(json!({"Authorization": "Bearer token"})),
            Some("data"),
            Some(true),
            Some(vec!["city"]),
        );
        let val = dm.to_swaig_function();
        let wh = &val["data_map"]["webhooks"][0];
        assert_eq!(wh["headers"]["Authorization"], "Bearer token");
        assert_eq!(wh["form_param"], "data");
        assert_eq!(wh["input_args_as_params"], true);
        assert_eq!(wh["require_args"][0], "city");
    }

    #[test]
    fn test_webhook_expressions() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api.example.com", None, None, None, None);
        dm.webhook_expressions(vec![
            json!({"pattern": "ok", "output": {"response": "good"}}),
        ]);
        let val = dm.to_swaig_function();
        let wh_exprs = val["data_map"]["webhooks"][0]["expressions"]
            .as_array()
            .unwrap();
        assert_eq!(wh_exprs.len(), 1);
    }

    #[test]
    fn test_params() {
        let mut dm = DataMap::new("func");
        dm.webhook("POST", "https://api.example.com", None, None, None, None);
        dm.params(json!({"q": "${args.query}"}));
        let val = dm.to_swaig_function();
        assert_eq!(
            val["data_map"]["webhooks"][0]["params"]["q"],
            "${args.query}"
        );
    }

    #[test]
    fn test_for_each() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api.example.com", None, None, None, None);
        dm.for_each(json!({"input_key": "items", "output_key": "result"}));
        let val = dm.to_swaig_function();
        assert_eq!(
            val["data_map"]["webhooks"][0]["foreach"]["input_key"],
            "items"
        );
    }

    #[test]
    fn test_output() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api.example.com", None, None, None, None);
        dm.output(json!({"response": "Weather is ${temp}"}));
        let val = dm.to_swaig_function();
        assert_eq!(
            val["data_map"]["webhooks"][0]["output"]["response"],
            "Weather is ${temp}"
        );
    }

    #[test]
    fn test_fallback_output() {
        let mut dm = DataMap::new("func");
        dm.fallback_output(json!({"response": "Default output"}));
        let val = dm.to_swaig_function();
        assert_eq!(val["data_map"]["output"]["response"], "Default output");
    }

    #[test]
    fn test_error_keys() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api.example.com", None, None, None, None);
        dm.error_keys(vec!["error", "message"]);
        let val = dm.to_swaig_function();
        let ek = val["data_map"]["webhooks"][0]["error_keys"]
            .as_array()
            .unwrap();
        assert_eq!(ek.len(), 2);
    }

    #[test]
    fn test_global_error_keys() {
        let mut dm = DataMap::new("func");
        dm.global_error_keys(vec!["error"]);
        let val = dm.to_swaig_function();
        let gek = val["data_map"]["error_keys"].as_array().unwrap();
        assert_eq!(gek.len(), 1);
    }

    #[test]
    fn test_no_data_map_when_empty() {
        let mut dm = DataMap::new("func");
        dm.purpose("Test");
        let val = dm.to_swaig_function();
        assert!(val.get("data_map").is_none());
    }

    #[test]
    fn test_chaining() {
        let mut dm = DataMap::new("weather");
        dm.purpose("Get weather")
            .parameter("city", "string", "City name", Some(true), None)
            .webhook("GET", "https://api.weather.com", None, None, None, None)
            .output(json!({"response": "Weather: ${temp}"}));

        let val = dm.to_swaig_function();
        assert_eq!(val["function"], "weather");
        assert_eq!(val["purpose"], "Get weather");
        assert!(val["argument"]["properties"]["city"].is_object());
        assert!(val["data_map"]["webhooks"].is_array());
    }

    // ── Static helper tests ──────────────────────────────────────────────

    #[test]
    fn test_create_simple_api_tool() {
        let tool = DataMap::create_simple_api_tool(
            "weather",
            "Get weather",
            vec![
                json!({"name": "city", "type": "string", "description": "City name", "required": true}),
            ],
            "GET",
            "https://api.weather.com",
            json!({"response": "Temperature: ${temp}"}),
            json!({"Authorization": "Bearer token"}),
        );
        assert_eq!(tool["function"], "weather");
        assert_eq!(tool["purpose"], "Get weather");
        assert!(tool["argument"]["properties"]["city"].is_object());
        assert_eq!(tool["data_map"]["webhooks"][0]["method"], "GET");
    }

    #[test]
    fn test_create_expression_tool() {
        let tool = DataMap::create_expression_tool(
            "classify",
            "Classify input",
            vec![
                json!({"name": "input", "type": "string", "description": "User input", "required": true}),
            ],
            vec![json!({
                "string": "${args.input}",
                "pattern": "yes|ok",
                "output": {"response": "Positive"},
                "nomatch_output": {"response": "Negative"}
            })],
        );
        assert_eq!(tool["function"], "classify");
        let exprs = tool["data_map"]["expressions"].as_array().unwrap();
        assert_eq!(exprs.len(), 1);
        assert_eq!(exprs[0]["pattern"], "yes|ok");
    }

    #[test]
    fn test_webhook_on_empty_list_is_noop() {
        let mut dm = DataMap::new("func");
        // These should not panic when no webhooks exist
        dm.webhook_expressions(vec![]);
        dm.params(json!({}));
        dm.for_each(json!({}));
        dm.output(json!("test"));
        dm.error_keys(vec!["err"]);
        // Should produce no data_map webhooks
        let val = dm.to_swaig_function();
        assert!(val.get("data_map").is_none());
    }

    #[test]
    fn test_multiple_parameters() {
        let mut dm = DataMap::new("func");
        dm.parameter("a", "string", "First", Some(true), None)
            .parameter("b", "number", "Second", None, None);
        let val = dm.to_swaig_function();
        assert!(val["argument"]["properties"]["a"].is_object());
        assert!(val["argument"]["properties"]["b"].is_object());
        let required = val["argument"]["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert!(required.contains(&json!("a")));
    }

    #[test]
    fn test_multiple_webhooks() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api1.com", None, None, None, None);
        dm.webhook("POST", "https://api2.com", None, None, None, None);
        let val = dm.to_swaig_function();
        let whs = val["data_map"]["webhooks"].as_array().unwrap();
        assert_eq!(whs.len(), 2);
    }

    /// The reference emits the method upper-cased on the wire
    /// (`core/data_map.py:230`, `"method": method.upper()`), so a caller writing a
    /// lower-case method must still produce byte-identical SWML across languages.
    #[test]
    fn test_webhook_upper_cases_method_on_the_wire() {
        let mut dm = DataMap::new("case_fn");
        dm.webhook("get", "https://api.example.com", None, None, None, None);
        dm.webhook("post", "https://api2.example.com", None, None, None, None);
        let val = dm.to_swaig_function();
        let whs = val["data_map"]["webhooks"].as_array().unwrap();
        assert_eq!(whs[0]["method"], "GET");
        assert_eq!(whs[1]["method"], "POST");
        // An already-upper-case method is unchanged.
        let mut dm2 = DataMap::new("case_fn2");
        dm2.webhook("DELETE", "https://api3.example.com", None, None, None, None);
        let val2 = dm2.to_swaig_function();
        assert_eq!(val2["data_map"]["webhooks"][0]["method"], "DELETE");
    }

    /// `DataMap::body()` is GONE — the key it wrote is invalid, not merely ignored.
    ///
    /// Owner-ruled 2026-07-29, extending the `create_simple_api_tool` ruling ("if the
    /// server doesn't read them, remove them") to the public builder method. Three
    /// independent sources condemn it:
    ///
    /// * The SWML schema's `$defs/Webhook` declares exactly ten properties —
    ///   `error_keys`, `expressions`, `foreach`, `headers`, `input_args_as_params`,
    ///   `method`, `output`, `params`, `require_args`, `url` — under
    ///   `unevaluatedProperties: {"not": {}}`. `body` is not among them, so emitting
    ///   it is a SCHEMA VIOLATION.
    /// * `mod_openai/actions.c:735-739` and `bedrock.c:4920-4926` read `url`, `method`,
    ///   `form_param`, `params` and `headers` and nothing else; `grep -n '"body"'`
    ///   across both returns ZERO matches.
    /// * So the method's only possible effect was producing an invalid document while
    ///   silently discarding the caller's payload.
    ///
    /// [`params`](DataMap::params) is the correct method for POST/PUT request data — it
    /// writes the `params` key, which IS in the contract and IS read.
    ///
    /// The builder surface offers no way to reach the forbidden key: `dm.body(..)` is a
    /// hard compile error after the removal, so the compiler gates the METHOD. This test
    /// gates the WIRE — driving every setter the builder still exposes and asserting the
    /// emitted webhook carries no `body` and nothing outside the ten allowed properties.
    #[test]
    fn test_body_key_is_never_emitted() {
        /// Every key the `$defs/Webhook` contract permits.
        const ALLOWED: &[&str] = &[
            "error_keys",
            "expressions",
            "foreach",
            "headers",
            "input_args_as_params",
            "method",
            "output",
            "params",
            "require_args",
            "url",
        ];

        // Drive every webhook-key setter the builder still exposes.
        let mut dm = DataMap::new("no_body");
        dm.webhook(
            "POST",
            "https://api.example.com",
            Some(json!({"Content-Type": "application/json"})),
            None,
            None,
            None,
        );
        dm.params(json!({"query": "${args.q}"}));
        dm.webhook_expressions(vec![json!({"pattern": "ok", "output": {"response": "y"}})]);
        dm.for_each(json!({"input_key": "r", "output_key": "o", "append": "x"}));
        dm.output(json!({"response": "ok"}));
        let val = dm.to_swaig_function();
        let wh = val["data_map"]["webhooks"][0].as_object().unwrap();
        assert!(
            !wh.contains_key("body"),
            "the `body` webhook key is forbidden by schema.json $defs/Webhook and is read \
             by no engine reader — use params() instead; got keys {:?}",
            wh.keys().collect::<Vec<_>>()
        );
        // Positive control: the replacement still writes the contract key.
        assert_eq!(wh["params"]["query"], "${args.q}");
        // Every key present is one of the ten the contract allows.
        for k in wh.keys() {
            assert!(
                ALLOWED.contains(&k.as_str()),
                "webhook key {k:?} is not one of the ten schema.json $defs/Webhook properties"
            );
        }
    }
}
