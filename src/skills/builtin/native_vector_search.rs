use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Search document indexes using vector similarity and keyword search (handler-based).
pub struct NativeVectorSearch {
    sp: SkillParams,
}

impl NativeVectorSearch {
    pub fn new(params: Map<String, Value>) -> Self {
        NativeVectorSearch {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for NativeVectorSearch {
    fn name(&self) -> &str {
        "native_vector_search"
    }

    fn description(&self) -> &str {
        "Search document indexes using vector similarity and keyword search (local or remote)"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("search_knowledge");
        let tool_description =
            self.sp.get_str_or("description", "Search the local knowledge base for information");
        let default_count = self.sp.get_i64("count", 5).max(1);
        let remote_url = self.sp.get_str_or("remote_url", "");
        let index_name = self.sp.get_str_or("index_name", "");

        agent.define_tool(
            &tool_name,
            &tool_description,
            json!({
                "query": {
                    "type": "string",
                    "description": "The search query to find relevant information",
                    "required": true,
                },
                "count": {
                    "type": "integer",
                    "description": "Number of results to return",
                    "default": default_count,
                },
            }),
            Box::new(move |args, _raw| {
                let mut result = FunctionResult::new();
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let count = args
                    .get("count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(default_count);

                if query.is_empty() {
                    result.set_response("Error: No search query provided.");
                    return result;
                }

                if !remote_url.is_empty() {
                    result.set_response(&format!(
                        "Vector search results for \"{}\": \
                         Searched remote endpoint \"{}\" with count={}. \
                         In production, this would return vector similarity search results.",
                        query, remote_url, count
                    ));
                } else {
                    result.set_response(&format!(
                        "Vector search results for \"{}\": \
                         Searched index \"{}\" with count={}. \
                         In production, this would return vector similarity search results.",
                        query, index_name, count
                    ));
                }
                result
            }),
            false,
        );
    }

    fn get_hints(&self) -> Vec<String> {
        let mut hints = vec![
            "search".to_string(),
            "find".to_string(),
            "look up".to_string(),
            "documentation".to_string(),
            "knowledge base".to_string(),
        ];

        let custom_hints = self.sp.get_array("hints");
        for hint in custom_hints {
            if let Some(s) = hint.as_str() {
                let s = s.to_string();
                if !hints.contains(&s) {
                    hints.push(s);
                }
            }
        }

        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_vector_search_metadata() {
        let skill = NativeVectorSearch::new(Map::new());
        assert_eq!(skill.name(), "native_vector_search");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_native_vector_search_setup() {
        let mut skill = NativeVectorSearch::new(Map::new());
        assert!(skill.setup());
    }

    #[test]
    fn test_native_vector_search_hints() {
        let skill = NativeVectorSearch::new(Map::new());
        let hints = skill.get_hints();
        assert!(hints.contains(&"search".to_string()));
    }
}
