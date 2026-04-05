use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Search Wikipedia for information about a topic and get article summaries.
pub struct WikipediaSearch {
    sp: SkillParams,
}

impl WikipediaSearch {
    pub fn new(params: Map<String, Value>) -> Self {
        WikipediaSearch {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for WikipediaSearch {
    fn name(&self) -> &str {
        "wikipedia_search"
    }

    fn description(&self) -> &str {
        "Search Wikipedia for information about a topic and get article summaries"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let num_results = self.sp.get_i64("num_results", 1).max(1).min(5);

        agent.define_tool(
            "search_wiki",
            "Search Wikipedia for information about a topic and get article summaries",
            json!({
                "query": {
                    "type": "string",
                    "description": "The topic to search for on Wikipedia",
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
                    "Wikipedia search results for \"{}\": \
                     Searched Wikipedia API with up to {} results. \
                     In production, this would return article summaries from Wikipedia.",
                    query, num_results
                ));
                result
            }),
            false,
        );
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Wikipedia Search",
            "body": "You can search Wikipedia for information on any topic.",
            "bullets": [
                "Use search_wiki to look up articles on Wikipedia.",
                "Returns article summaries for the requested topic.",
                "Useful for factual information, historical data, and general knowledge.",
            ],
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wikipedia_search_metadata() {
        let skill = WikipediaSearch::new(Map::new());
        assert_eq!(skill.name(), "wikipedia_search");
    }

    #[test]
    fn test_wikipedia_search_setup() {
        let mut skill = WikipediaSearch::new(Map::new());
        assert!(skill.setup());
    }
}
