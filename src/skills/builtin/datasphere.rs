use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Search knowledge using SignalWire DataSphere RAG stack (handler-based).
pub struct Datasphere {
    sp: SkillParams,
}

impl Datasphere {
    pub fn new(params: Map<String, Value>) -> Self {
        Datasphere {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for Datasphere {
    fn name(&self) -> &str {
        "datasphere"
    }

    fn description(&self) -> &str {
        "Search knowledge using SignalWire DataSphere RAG stack"
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
        let document_id = self.sp.get_str_or("document_id", "");
        let count = self.sp.get_i64("count", 1).max(1).min(10);
        let distance = self.sp.get_f64("distance", 3.0);

        agent.define_tool(
            &tool_name,
            "Search the knowledge base for information on any topic and return relevant results",
            json!({
                "query": {
                    "type": "string",
                    "description": "The search query to find relevant knowledge",
                    "required": true,
                }
            }),
            Box::new(move |args, _raw| {
                let mut result = FunctionResult::new();
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

                if query.is_empty() {
                    result.set_response("Error: No search query provided.");
                    return result;
                }

                result.set_response(&format!(
                    "DataSphere search results for \"{}\": \
                     Searched document \"{}\" in space \"{}\" \
                     with count={} and distance={}. \
                     In production, this would return matching knowledge base chunks.",
                    query, document_id, space_name, count, distance
                ));
                result
            }),
            false,
        );
    }

    fn get_global_data(&self) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("datasphere_enabled".to_string(), Value::Bool(true));
        data.insert(
            "document_id".to_string(),
            Value::String(self.sp.get_str_or("document_id", "")),
        );
        data.insert(
            "knowledge_provider".to_string(),
            Value::String("SignalWire DataSphere".to_string()),
        );
        data
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Knowledge Search Capability",
            "body": "You have access to a knowledge base powered by SignalWire DataSphere.",
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
    fn test_datasphere_metadata() {
        let skill = Datasphere::new(Map::new());
        assert_eq!(skill.name(), "datasphere");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_datasphere_setup_needs_params() {
        let mut skill = Datasphere::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("space_name".to_string(), json!("test.signalwire.com"));
        params.insert("project_id".to_string(), json!("proj-123"));
        params.insert("token".to_string(), json!("tok-456"));
        params.insert("document_id".to_string(), json!("doc-789"));
        let mut skill2 = Datasphere::new(params);
        assert!(skill2.setup());
    }
}
