use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Fast web scraping and crawling capabilities (handler-based).
pub struct Spider {
    sp: SkillParams,
}

impl Spider {
    pub fn new(params: Map<String, Value>) -> Self {
        Spider {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for Spider {
    fn name(&self) -> &str {
        "spider"
    }

    fn description(&self) -> &str {
        "Fast web scraping and crawling capabilities"
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
        let prefix = self.sp.get_str_or("tool_prefix", "");
        let max_length = self.sp.get_i64("max_text_length", 5000);
        let extract_type = self.sp.get_str_or("extract_type", "clean_text");
        let max_pages = self.sp.get_i64("max_pages", 10);
        let max_depth = self.sp.get_i64("max_depth", 3);

        let scrape_name = format!("{}scrape_url", prefix);
        let crawl_name = format!("{}crawl_site", prefix);
        let extract_name = format!("{}extract_structured_data", prefix);

        agent.define_tool(
            &scrape_name,
            "Scrape content from a web page URL",
            json!({
                "url": {
                    "type": "string",
                    "description": "The URL of the web page to scrape",
                    "required": true,
                }
            }),
            Box::new(move |args, _raw| {
                let mut result = FunctionResult::new();
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.is_empty() {
                    result.set_response("Error: No URL provided.");
                    return result;
                }
                result.set_response(&format!(
                    "Scraped content from \"{}\" (extract type: {}, max length: {}). \
                     In production, this would return the parsed text content of the page.",
                    url, extract_type, max_length
                ));
                result
            }),
            false,
        );

        agent.define_tool(
            &crawl_name,
            "Crawl a website starting from a URL and collect content from multiple pages",
            json!({
                "start_url": {
                    "type": "string",
                    "description": "The starting URL to begin crawling from",
                    "required": true,
                }
            }),
            Box::new(move |args, _raw| {
                let mut result = FunctionResult::new();
                let start_url = args.get("start_url").and_then(|v| v.as_str()).unwrap_or("");
                if start_url.is_empty() {
                    result.set_response("Error: No start URL provided.");
                    return result;
                }
                result.set_response(&format!(
                    "Crawled site starting from \"{}\" (max pages: {}, max depth: {}). \
                     In production, this would return collected content from multiple pages.",
                    start_url, max_pages, max_depth
                ));
                result
            }),
            false,
        );

        agent.define_tool(
            &extract_name,
            "Extract structured data from a web page",
            json!({
                "url": {
                    "type": "string",
                    "description": "The URL to extract structured data from",
                    "required": true,
                }
            }),
            Box::new(|args, _raw| {
                let mut result = FunctionResult::new();
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.is_empty() {
                    result.set_response("Error: No URL provided.");
                    return result;
                }
                result.set_response(&format!(
                    "Extracted structured data from \"{}\". \
                     In production, this would return structured data extracted using CSS selectors or schema.org markup.",
                    url
                ));
                result
            }),
            false,
        );
    }

    fn get_hints(&self) -> Vec<String> {
        vec![
            "scrape".to_string(),
            "crawl".to_string(),
            "extract".to_string(),
            "web page".to_string(),
            "website".to_string(),
            "spider".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spider_metadata() {
        let skill = Spider::new(Map::new());
        assert_eq!(skill.name(), "spider");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_spider_setup() {
        let mut skill = Spider::new(Map::new());
        assert!(skill.setup());
    }

    #[test]
    fn test_spider_hints() {
        let skill = Spider::new(Map::new());
        let hints = skill.get_hints();
        assert!(hints.contains(&"scrape".to_string()));
        assert!(hints.contains(&"crawl".to_string()));
    }
}
