use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Search knowledge using SignalWire DataSphere with serverless DataMap execution.
pub struct DatasphereServerless {
    sp: SkillParams,
}

impl DatasphereServerless {
    pub fn new(params: Map<String, Value>) -> Self {
        DatasphereServerless {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for DatasphereServerless {
    fn name(&self) -> &str {
        "datasphere_serverless"
    }

    fn description(&self) -> &str {
        "Search knowledge using SignalWire DataSphere with serverless DataMap execution"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        for key in &["space_name", "project_id", "token", "document_id"] {
            if self.sp.get_str(key).is_none() {
                return false;
            }
        }
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("search_knowledge");
        let space_name = self.sp.get_str_or("space_name", "");
        let project_id = self.sp.get_str_or("project_id", "");
        let token = self.sp.get_str_or("token", "");
        let document_id = self.sp.get_str_or("document_id", "");
        let count = self.sp.get_i64("count", 1).max(1).min(10);
        let distance = self.sp.get_f64("distance", 3.0);
        let no_results_msg = self.sp.get_str_or(
            "no_results_message",
            "No results found in the knowledge base for the given query.",
        );

        let auth_string = BASE64.encode(format!("{}:{}", project_id, token));

        let mut body_payload = json!({
            "document_id": document_id,
            "query_string": "${args.query}",
            "count": count,
            "distance": distance,
        });

        if let Some(tags) = self.sp.params.get("tags") {
            body_payload["tags"] = tags.clone();
        }
        if let Some(language) = self.sp.params.get("language") {
            body_payload["language"] = language.clone();
        }

        let mut func_def = json!({
            "function": tool_name,
            "purpose": "Search the knowledge base for information on any topic and return relevant results",
            "argument": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to find relevant knowledge",
                    }
                },
                "required": ["query"],
            },
            "data_map": {
                "webhooks": [{
                    "url": format!("https://{}/api/datasphere/documents/search", space_name),
                    "method": "POST",
                    "headers": {
                        "Content-Type": "application/json",
                        "Authorization": format!("Basic {}", auth_string),
                    },
                    "body": body_payload,
                    "foreach": {
                        "input_key": "chunks",
                        "output_key": "formatted_results",
                        "template": "${this.document_id}: ${this.text}",
                    },
                    "output": {
                        "response": "I found results for \"${args.query}\":\n\n${formatted_results}",
                        "action": [{"say_it": true}],
                    },
                    "error_output": {
                        "response": no_results_msg,
                        "action": [{"say_it": true}],
                    },
                }],
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

    fn get_global_data(&self) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert(
            "datasphere_serverless_enabled".to_string(),
            Value::Bool(true),
        );
        data.insert(
            "document_id".to_string(),
            Value::String(self.sp.get_str_or("document_id", "")),
        );
        data.insert(
            "knowledge_provider".to_string(),
            Value::String("SignalWire DataSphere (Serverless)".to_string()),
        );
        data
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Knowledge Search Capability (Serverless)",
            "body": "You have access to a knowledge base powered by SignalWire DataSphere (serverless mode).",
            "bullets": [
                "Use the search tool to look up information in the knowledge base.",
                "Always search the knowledge base before saying you do not know something.",
                "Provide accurate answers based on the search results.",
            ],
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datasphere_serverless_metadata() {
        let skill = DatasphereServerless::new(Map::new());
        assert_eq!(skill.name(), "datasphere_serverless");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_datasphere_serverless_setup_needs_params() {
        let mut skill = DatasphereServerless::new(Map::new());
        assert!(!skill.setup());
    }
}
