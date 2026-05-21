use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Search the web using Google Custom Search API.
///
/// Mirrors Python's `signalwire.skills.web_search`: the SDK issues a
/// real HTTP GET to Google CSE (`https://www.googleapis.com/customsearch/v1`)
/// with the query, key, and CSE ID in the query string, and parses the
/// JSON response. The base URL can be overridden via the
/// `WEB_SEARCH_BASE_URL` env var (used by `audit_skills_dispatch.py`'s
/// fixture). Without that override, the URL points at Google.
pub struct WebSearch {
    sp: SkillParams,
}

impl WebSearch {
    pub fn new(params: Map<String, Value>) -> Self {
        WebSearch {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for WebSearch {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information using Google Custom Search API"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        // Either explicit params or env vars must supply the credentials.
        // The handler reads the same names at call time, so setup just
        // verifies they exist somewhere.
        let key_present = self.sp.get_str("api_key").is_some()
            || std::env::var("GOOGLE_API_KEY").is_ok()
            || std::env::var("GOOGLE_SEARCH_API_KEY").is_ok();
        let cx_present = self.sp.get_str("search_engine_id").is_some()
            || std::env::var("GOOGLE_CSE_ID").is_ok()
            || std::env::var("GOOGLE_SEARCH_ENGINE_ID").is_ok();
        key_present && cx_present
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("web_search");
        let num_results = self.sp.get_i64("num_results", 3).clamp(1, 10);
        let api_key = self.sp.get_str("api_key").map(|s| s.to_string());
        let cse_id = self.sp.get_str("search_engine_id").map(|s| s.to_string());

        // Optional prefix/postfix wrapped around every non-empty search
        // result. Use these to give the calling agent a mechanical cue
        // (e.g. "tell the user this came from a public web search") without
        // needing prompt-side rules. Mirrors Python's `response_prefix` /
        // `response_postfix` on `WebSearchSkill`.
        let response_prefix = self
            .sp
            .get_str("response_prefix")
            .unwrap_or("")
            .to_string();
        let response_postfix = self
            .sp
            .get_str("response_postfix")
            .unwrap_or("")
            .to_string();

        agent.define_tool(
            &tool_name,
            "Search the web for high-quality information using Google Custom Search",
            json!({
                "query": {
                    "type": "string",
                    "description": "The search query",
                    "required": true,
                }
            }),
            Box::new(move |args, _raw| {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                if query.is_empty() {
                    let mut r = FunctionResult::new();
                    r.set_response("Error: No search query provided.");
                    return r;
                }

                // Resolve credentials at call time so env-var overrides
                // (notably from the audit harness) take effect.
                let key = api_key
                    .clone()
                    .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
                    .or_else(|| std::env::var("GOOGLE_SEARCH_API_KEY").ok())
                    .unwrap_or_default();
                let cx = cse_id
                    .clone()
                    .or_else(|| std::env::var("GOOGLE_CSE_ID").ok())
                    .or_else(|| std::env::var("GOOGLE_SEARCH_ENGINE_ID").ok())
                    .unwrap_or_default();
                let base = std::env::var("WEB_SEARCH_BASE_URL")
                    .unwrap_or_else(|_| "https://www.googleapis.com".to_string());

                let url = format!(
                    "{}/customsearch/v1?key={}&cx={}&q={}&num={}",
                    base.trim_end_matches('/'),
                    url_encode(&key),
                    url_encode(&cx),
                    url_encode(query),
                    num_results,
                );

                let body = match http_get_json(&url) {
                    Ok(v) => v,
                    Err(e) => {
                        let mut r = FunctionResult::new();
                        r.set_response(&format!("Web search error: {}", e));
                        return r;
                    }
                };

                let items = body.get("items").and_then(|v| v.as_array()).cloned()
                    .unwrap_or_default();

                let formatted = format_web_search_response(
                    query,
                    &items,
                    num_results,
                    &response_prefix,
                    &response_postfix,
                );

                let mut r = FunctionResult::new();
                r.set_response(&formatted);
                r
            }),
            false,
        );
    }

    fn get_global_data(&self) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("web_search_enabled".to_string(), Value::Bool(true));
        data.insert(
            "search_provider".to_string(),
            Value::String("Google Custom Search".to_string()),
        );
        data
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Web Search Capability",
            "body": "You can search the web for information.",
            "bullets": [
                "Use the web search tool to find current information on any topic.",
                "Results come from Google Custom Search.",
            ],
        })]
    }
}

/// Format the search-results body that goes back to the model.
///
/// The empty-results path always returns the plain "No web results" string
/// with no wrapping — matches Python's behavior, which only applies
/// `response_prefix` / `response_postfix` to the successful response. This
/// keeps the model's "no results" handling crisp (no mechanical postamble
/// like "tell the user this came from the web" when nothing was found).
///
/// Exposed at module scope so the test module can exercise it without a
/// live HTTP call.
fn format_web_search_response(
    query: &str,
    items: &[Value],
    num_results: i64,
    response_prefix: &str,
    response_postfix: &str,
) -> String {
    if items.is_empty() {
        return format!("No web results for \"{}\".", query);
    }
    let lines: Vec<String> = items
        .iter()
        .take(num_results as usize)
        .map(|it| {
            let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let link = it.get("link").and_then(|v| v.as_str()).unwrap_or("");
            let snippet = it.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            format!("- {} ({})\n  {}", title, link, snippet)
        })
        .collect();
    let mut response = format!("Web search results for \"{}\":\n{}", query, lines.join("\n"));
    if !response_prefix.is_empty() {
        response = format!("{}\n\n{}", response_prefix, response);
    }
    if !response_postfix.is_empty() {
        response = format!("{}\n\n{}", response, response_postfix);
    }
    response
}

/// Issue a real HTTP GET via ureq and parse the JSON response.
fn http_get_json(url: &str) -> Result<Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(15)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .get(url)
        .header("User-Agent", "signalwire-agents-rust-skills/1.0")
        .call()
        .map_err(|e| format!("HTTP GET {} failed: {}", url, e))?;
    let status = resp.status().as_u16();
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("HTTP GET {} body read failed: {}", url, e))?;
    if status < 200 || status >= 300 {
        return Err(format!("HTTP GET {} returned {}: {}", url, status, body));
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("HTTP GET {} returned non-JSON: {}", url, e))
}

/// Minimal URL-encode for query-string values. Encodes the small set
/// of characters that matter for our usage; we don't need a full
/// percent-encoder because the inputs are plain strings.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_metadata() {
        let skill = WebSearch::new(Map::new());
        assert_eq!(skill.name(), "web_search");
        assert_eq!(skill.version(), "2.0.0");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_web_search_setup_needs_credentials() {
        // Without keys/cx in either params or env, setup() returns false.
        let prev_key = std::env::var("GOOGLE_API_KEY").ok();
        let prev_search_key = std::env::var("GOOGLE_SEARCH_API_KEY").ok();
        let prev_cx = std::env::var("GOOGLE_CSE_ID").ok();
        let prev_search_cx = std::env::var("GOOGLE_SEARCH_ENGINE_ID").ok();
        unsafe {
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_SEARCH_API_KEY");
            std::env::remove_var("GOOGLE_CSE_ID");
            std::env::remove_var("GOOGLE_SEARCH_ENGINE_ID");
        }

        let mut skill = WebSearch::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("api_key".to_string(), json!("k"));
        params.insert("search_engine_id".to_string(), json!("id"));
        let mut skill2 = WebSearch::new(params);
        assert!(skill2.setup());

        unsafe {
            if let Some(v) = prev_key {
                std::env::set_var("GOOGLE_API_KEY", v);
            }
            if let Some(v) = prev_search_key {
                std::env::set_var("GOOGLE_SEARCH_API_KEY", v);
            }
            if let Some(v) = prev_cx {
                std::env::set_var("GOOGLE_CSE_ID", v);
            }
            if let Some(v) = prev_search_cx {
                std::env::set_var("GOOGLE_SEARCH_ENGINE_ID", v);
            }
        }
    }

    #[test]
    fn test_url_encode_safe_chars() {
        assert_eq!(url_encode("hello"), "hello");
        assert_eq!(url_encode("foo bar"), "foo%20bar");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
    }

    // -------------------------------------------------------------------
    // response_prefix / response_postfix — mirrors the Python wrap logic.
    // -------------------------------------------------------------------

    fn sample_items() -> Vec<Value> {
        vec![json!({
            "title": "Rust Programming",
            "link": "https://example.com/rust",
            "snippet": "A systems programming language.",
        })]
    }

    #[test]
    fn test_format_no_prefix_no_postfix_passes_through() {
        let items = sample_items();
        let out = format_web_search_response("rust", &items, 3, "", "");
        assert!(out.starts_with("Web search results for \"rust\":\n- Rust Programming"));
        // No wrapping at all when both are empty.
        assert!(!out.contains("\n\n"));
    }

    #[test]
    fn test_format_prefix_only_wraps_top() {
        let items = sample_items();
        let out = format_web_search_response("rust", &items, 3, "PREFIX_LINE", "");
        assert!(out.starts_with("PREFIX_LINE\n\nWeb search results for"));
        assert!(!out.ends_with("PREFIX_LINE"));
    }

    #[test]
    fn test_format_postfix_only_wraps_bottom() {
        let items = sample_items();
        let out = format_web_search_response("rust", &items, 3, "", "POSTFIX_LINE");
        assert!(out.starts_with("Web search results for"));
        assert!(out.ends_with("\n\nPOSTFIX_LINE"));
    }

    #[test]
    fn test_format_both_prefix_and_postfix_wrap() {
        let items = sample_items();
        let out =
            format_web_search_response("rust", &items, 3, "PREFIX_LINE", "POSTFIX_LINE");
        assert!(out.starts_with("PREFIX_LINE\n\nWeb search results for"));
        assert!(out.ends_with("\n\nPOSTFIX_LINE"));
        // Both wrappers must appear exactly once.
        assert_eq!(out.matches("PREFIX_LINE").count(), 1);
        assert_eq!(out.matches("POSTFIX_LINE").count(), 1);
    }

    #[test]
    fn test_format_empty_results_never_wrapped() {
        // Matches Python: the no-results path is returned verbatim and is
        // NOT wrapped with prefix/postfix.
        let out = format_web_search_response("rust", &[], 3, "PREFIX", "POSTFIX");
        assert_eq!(out, "No web results for \"rust\".");
        assert!(!out.contains("PREFIX"));
        assert!(!out.contains("POSTFIX"));
    }
}
