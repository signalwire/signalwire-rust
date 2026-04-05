use serde_json::{json, Map, Value};


/// Fluent builder for DataMap-based SWAIG function definitions.
///
/// A DataMap tool defines its behaviour declaratively (expressions, webhooks)
/// instead of with a code handler.
#[derive(Debug, Clone)]
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

    // ── Fluent setters ───────────────────────────────────────────────────

    pub fn purpose(&mut self, desc: &str) -> &mut Self {
        self.purpose = desc.to_string();
        self
    }

    /// Alias for `purpose`.
    pub fn description(&mut self, desc: &str) -> &mut Self {
        self.purpose(desc)
    }

    /// Add a parameter definition.
    pub fn parameter(
        &mut self,
        name: &str,
        param_type: &str,
        description: &str,
        required: bool,
        enum_values: Vec<&str>,
    ) -> &mut Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), json!(param_type));
        prop.insert("description".to_string(), json!(description));
        if !enum_values.is_empty() {
            prop.insert("enum".to_string(), json!(enum_values));
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
            expr.insert("nomatch_output".to_string(), nm);
        }
        self.expressions.push(Value::Object(expr));
        self
    }

    /// Add a webhook definition.
    pub fn webhook(
        &mut self,
        method: &str,
        url: &str,
        headers: Value,
        form_param: &str,
        input_args_as_params: bool,
        require_args: Vec<&str>,
    ) -> &mut Self {
        let mut wh = Map::new();
        wh.insert("method".to_string(), json!(method));
        wh.insert("url".to_string(), json!(url));

        if let Value::Object(ref h) = headers {
            if !h.is_empty() {
                wh.insert("headers".to_string(), headers.clone());
            }
        }
        if !form_param.is_empty() {
            wh.insert("form_param".to_string(), json!(form_param));
        }
        if input_args_as_params {
            wh.insert("input_args_as_params".to_string(), json!(true));
        }
        if !require_args.is_empty() {
            wh.insert("require_args".to_string(), json!(require_args));
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

    /// Set body on the last webhook.
    pub fn body(&mut self, data: Value) -> &mut Self {
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("body".to_string(), data);
        }
        self
    }

    /// Set params on the last webhook.
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

    /// Set global fallback output.
    pub fn fallback_output(&mut self, result: Value) -> &mut Self {
        self.global_output = Some(Self::resolve_output(result));
        self
    }

    /// Set error_keys on the last webhook.
    pub fn error_keys(&mut self, keys: Vec<&str>) -> &mut Self {
        if let Some(Value::Object(map)) = self.webhooks.last_mut() {
            map.insert("error_keys".to_string(), json!(keys));
        }
        self
    }

    /// Set global error_keys.
    pub fn global_error_keys(&mut self, keys: Vec<&str>) -> &mut Self {
        self.global_error_keys = Some(keys.into_iter().map(|s| s.to_string()).collect());
        self
    }

    // ── Serialisation ────────────────────────────────────────────────────

    /// Serialise to a SWAIG function definition.
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
            data_map.insert("expressions".to_string(), Value::Array(self.expressions.clone()));
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
            builder.parameter(p_name, p_type, p_desc, p_required, p_enum);
        }

        builder.webhook(method, url, headers, "", false, vec![]);
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
            builder.parameter(p_name, p_type, p_desc, p_required, p_enum);
        }

        for expr in expressions {
            let test_str = expr["string"].as_str().unwrap_or("");
            let pattern = expr["pattern"].as_str().unwrap_or("");
            let output = expr.get("output").cloned().unwrap_or(json!(null));
            let nomatch = expr.get("nomatch_output").cloned();
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
        dm.parameter("city", "string", "City name", true, vec![]);
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
        dm.parameter("unit", "string", "Temperature unit", false, vec!["celsius", "fahrenheit"]);
        let val = dm.to_swaig_function();
        let enums = val["argument"]["properties"]["unit"]["enum"].as_array().unwrap();
        assert_eq!(enums.len(), 2);
    }

    #[test]
    fn test_parameter_not_required() {
        let mut dm = DataMap::new("func");
        dm.parameter("opt", "string", "optional", false, vec![]);
        let val = dm.to_swaig_function();
        // required array should not exist if no required params
        assert!(val["argument"].get("required").is_none());
    }

    #[test]
    fn test_expression() {
        let mut dm = DataMap::new("func");
        dm.expression("${args.color}", "red|blue", json!({"response": "matched"}), None);
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
        assert_eq!(val["data_map"]["expressions"][0]["nomatch_output"], "miss");
    }

    #[test]
    fn test_webhook() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api.example.com/data", json!({}), "", false, vec![]);
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
            json!({"Authorization": "Bearer token"}),
            "data",
            true,
            vec!["city"],
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
        dm.webhook("GET", "https://api.example.com", json!({}), "", false, vec![]);
        dm.webhook_expressions(vec![json!({"pattern": "ok", "output": {"response": "good"}})]);
        let val = dm.to_swaig_function();
        let wh_exprs = val["data_map"]["webhooks"][0]["expressions"].as_array().unwrap();
        assert_eq!(wh_exprs.len(), 1);
    }

    #[test]
    fn test_body() {
        let mut dm = DataMap::new("func");
        dm.webhook("POST", "https://api.example.com", json!({}), "", false, vec![]);
        dm.body(json!({"key": "value"}));
        let val = dm.to_swaig_function();
        assert_eq!(val["data_map"]["webhooks"][0]["body"]["key"], "value");
    }

    #[test]
    fn test_params() {
        let mut dm = DataMap::new("func");
        dm.webhook("POST", "https://api.example.com", json!({}), "", false, vec![]);
        dm.params(json!({"q": "${args.query}"}));
        let val = dm.to_swaig_function();
        assert_eq!(val["data_map"]["webhooks"][0]["params"]["q"], "${args.query}");
    }

    #[test]
    fn test_for_each() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api.example.com", json!({}), "", false, vec![]);
        dm.for_each(json!({"input_key": "items", "output_key": "result"}));
        let val = dm.to_swaig_function();
        assert_eq!(val["data_map"]["webhooks"][0]["foreach"]["input_key"], "items");
    }

    #[test]
    fn test_output() {
        let mut dm = DataMap::new("func");
        dm.webhook("GET", "https://api.example.com", json!({}), "", false, vec![]);
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
        dm.webhook("GET", "https://api.example.com", json!({}), "", false, vec![]);
        dm.error_keys(vec!["error", "message"]);
        let val = dm.to_swaig_function();
        let ek = val["data_map"]["webhooks"][0]["error_keys"].as_array().unwrap();
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
            .parameter("city", "string", "City name", true, vec![])
            .webhook("GET", "https://api.weather.com", json!({}), "", false, vec![])
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
            vec![json!({"name": "city", "type": "string", "description": "City name", "required": true})],
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
            vec![json!({"name": "input", "type": "string", "description": "User input", "required": true})],
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
        dm.body(json!({}));
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
        dm.parameter("a", "string", "First", true, vec![])
            .parameter("b", "number", "Second", false, vec![]);
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
        dm.webhook("GET", "https://api1.com", json!({}), "", false, vec![]);
        dm.webhook("POST", "https://api2.com", json!({}), "", false, vec![]);
        let val = dm.to_swaig_function();
        let whs = val["data_map"]["webhooks"].as_array().unwrap();
        assert_eq!(whs.len(), 2);
    }
}
