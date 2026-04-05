use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Transfer calls between agents based on pattern matching (DataMap-based).
pub struct SwmlTransfer {
    sp: SkillParams,
}

impl SwmlTransfer {
    pub fn new(params: Map<String, Value>) -> Self {
        SwmlTransfer {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for SwmlTransfer {
    fn name(&self) -> &str {
        "swml_transfer"
    }

    fn description(&self) -> &str {
        "Transfer calls between agents based on pattern matching"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp
            .params
            .get("transfers")
            .and_then(|v| v.as_object())
            .map(|o| !o.is_empty())
            .unwrap_or(false)
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("transfer_call");
        let transfers = self.sp.get_object("transfers");
        let description = self.sp.get_str_or("description", "Transfer call based on pattern matching");
        let param_name = self.sp.get_str_or("parameter_name", "transfer_type");
        let param_description =
            self.sp.get_str_or("parameter_description", "The type of transfer to perform");
        let default_message =
            self.sp.get_str_or("default_message", "Transferring your call, please hold.");

        let transfer_keys: Vec<Value> = transfers.keys().map(|k| json!(k)).collect();

        // Build properties
        let mut properties = Map::new();
        properties.insert(
            param_name.clone(),
            json!({
                "type": "string",
                "description": param_description,
                "enum": transfer_keys,
            }),
        );

        let mut required = vec![json!(&param_name)];

        // Required fields
        let required_fields = self.sp.get_array("required_fields");
        for field in &required_fields {
            if let Some(field_name) = field.get("name").and_then(|v| v.as_str()) {
                let field_type = field
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("string");
                let field_desc = field
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(field_name);
                properties.insert(
                    field_name.to_string(),
                    json!({
                        "type": field_type,
                        "description": field_desc,
                    }),
                );
                required.push(json!(field_name));
            }
        }

        // Build DataMap expressions
        let mut expressions = Vec::new();
        for (pattern, config) in &transfers {
            let config_obj = match config.as_object() {
                Some(o) => o,
                None => continue,
            };
            let url = config_obj
                .get("url")
                .or_else(|| config_obj.get("address"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = config_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or(&default_message);
            let post_process = config_obj
                .get("post_process")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut actions = Vec::new();
            if !url.is_empty() {
                if url.starts_with("http://") || url.starts_with("https://") {
                    actions.push(json!({"transfer_uri": url}));
                } else {
                    actions.push(json!({
                        "SWML": {
                            "sections": {
                                "main": [
                                    {"connect": {"to": url}}
                                ]
                            }
                        }
                    }));
                }
            }

            let mut output = json!({
                "response": message,
                "action": actions,
            });
            if post_process {
                output["post_process"] = json!(true);
            }

            expressions.push(json!({
                "string": format!("${{args.{}}}", param_name),
                "pattern": pattern,
                "output": output,
            }));
        }

        let mut func_def = json!({
            "function": tool_name,
            "purpose": description,
            "argument": {
                "type": "object",
                "properties": Value::Object(properties),
                "required": required,
            },
            "data_map": {
                "expressions": expressions,
            },
        });

        let swaig_fields = self.get_swaig_fields();
        if let Value::Object(ref mut obj) = func_def {
            for (k, v) in swaig_fields {
                obj.insert(k, v);
            }
        }

        agent.register_swaig_function(func_def);
    }

    fn get_hints(&self) -> Vec<String> {
        let mut hints = vec![
            "transfer".to_string(),
            "connect".to_string(),
            "speak to".to_string(),
            "talk to".to_string(),
        ];

        let transfers = self.sp.get_object("transfers");
        for key in transfers.keys() {
            for word in key.split(|c: char| c == '_' || c == '-' || c.is_whitespace()) {
                let word = word.trim().to_string();
                if !word.is_empty() && !hints.contains(&word) {
                    hints.push(word);
                }
            }
        }

        hints
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        let transfers = self.sp.get_object("transfers");
        let mut destinations = Vec::new();

        for (pattern, config) in &transfers {
            let message = config
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let entry = if !message.is_empty() {
                format!("{} - {}", pattern, message)
            } else {
                pattern.clone()
            };
            destinations.push(entry);
        }

        let mut sections = vec![json!({
            "title": "Transferring",
            "body": "Available transfer destinations:",
            "bullets": destinations,
        })];

        if !destinations.is_empty() {
            sections.push(json!({
                "title": "Transfer Instructions",
                "body": "When the user wants to be transferred:",
                "bullets": [
                    "Confirm the transfer destination with the user before transferring.",
                    "Use the transfer tool with the appropriate transfer type.",
                ],
            }));
        }

        sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swml_transfer_metadata() {
        let skill = SwmlTransfer::new(Map::new());
        assert_eq!(skill.name(), "swml_transfer");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_swml_transfer_setup_needs_transfers() {
        let mut skill = SwmlTransfer::new(Map::new());
        assert!(!skill.setup());
    }

    #[test]
    fn test_swml_transfer_setup_with_transfers() {
        let mut params = Map::new();
        let mut transfers = Map::new();
        transfers.insert(
            "sales".to_string(),
            json!({"url": "https://example.com/sales", "message": "Transferring to sales"}),
        );
        params.insert("transfers".to_string(), Value::Object(transfers));
        let mut skill = SwmlTransfer::new(params);
        assert!(skill.setup());
    }
}
