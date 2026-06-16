use std::fmt::Write as _;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Default `no_results_message` (mirrors Python's `WebSearchSkill` default).
/// Returned by the snippet fallback when CSE yields nothing at all or the
/// `overall_deadline` fires before any item arrives.
const DEFAULT_NO_RESULTS_MESSAGE: &str = "I couldn't find quality results for '{query}'. The search returned only \
low-quality or inaccessible pages. Try rephrasing your search or asking about \
a different topic.";

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
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web for information using Google Custom Search API"
    }

    fn version(&self) -> &'static str {
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
        let api_key = self
            .sp
            .get_str("api_key")
            .map(std::string::ToString::to_string);
        let cse_id = self
            .sp
            .get_str("search_engine_id")
            .map(std::string::ToString::to_string);

        // Optional prefix/postfix wrapped around every non-empty search
        // result. Use these to give the calling agent a mechanical cue
        // (e.g. "tell the user this came from a public web search") without
        // needing prompt-side rules. Mirrors Python's `response_prefix` /
        // `response_postfix` on `WebSearchSkill`.
        let response_prefix = self.sp.get_str("response_prefix").unwrap_or("").to_string();
        let response_postfix = self
            .sp
            .get_str("response_postfix")
            .unwrap_or("")
            .to_string();

        // Latency-control parameters. The SignalWire kernel times out webhook
        // responses around 55s, so the handler MUST finish under that. Mirrors
        // Python's web_search/skill.py (commits 51101da + 295745b).
        //
        // This Rust port is snippet-only: unlike the Python/Go/TS scrapers it
        // never fetches the candidate pages, it formats the Google CSE result
        // items directly. The single network operation is therefore the CSE
        // call itself, so the latency-control params bind to it:
        //   per_page_timeout: per-request HTTP timeout on the CSE fetch
        //     (ureq `timeout_global`).
        //   overall_deadline: wall-clock budget for the whole tool call. The
        //     fetch runs on a worker thread; if it has not returned by the
        //     deadline we abandon it and fall back to whatever we have (here,
        //     the snippet-fallback / no-results message). THIS IS THE CONTRACT.
        //   parallel_scrape: best-effort and a no-op here — with a single
        //     fetch there is nothing to parallelize — but read and advertised
        //     for Python parity so config carries across cleanly.
        //   snippets_only: format the CSE snippets directly and skip the
        //     fully-formatted "Web search results" rendering. Already the only
        //     mode this port supports; the flag selects the labeled
        //     snippet-only output (Python `_format_snippet_results`).
        let per_page_timeout = self.sp.get_f64("per_page_timeout", 2.0);
        let overall_deadline = self.sp.get_f64("overall_deadline", 10.0);
        let _parallel_scrape = self.sp.get_bool_or("parallel_scrape", true);
        let snippets_only = self.sp.get_bool_or("snippets_only", false);

        // No-results message (Python `no_results_message`, with `{query}`
        // substituted). Used only when CSE returns nothing at all or the
        // deadline fires before any items arrive.
        let no_results_message = self
            .sp
            .get_str("no_results_message")
            .unwrap_or(DEFAULT_NO_RESULTS_MESSAGE)
            .to_string();

        agent.define_tool(
            &tool_name,
            "Search the web for high-quality information using Google Custom Search",
            json!({
                "query": {
                    "type": "string",
                    "description": "The search query",
                    // Not required — Python passes none (web_search/skill.py:707).
                }
            }),
            Box::new(move |args, _raw| {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                if query.is_empty() {
                    let mut r = FunctionResult::new();
                    r.set_response("Error: No search query provided.");
                    return r;
                }

                // overall_deadline is the wall-clock budget for the WHOLE tool
                // call. Start the clock before any network work so the budget
                // covers everything that follows. THIS IS THE CONTRACT: the
                // handler must return by `deadline_at` even if the CSE fetch
                // stalls, so a slow upstream can't blow past the kernel webhook
                // timeout (~55s). Clamp to >= 1.0s to match the schema `min`.
                let overall = if overall_deadline >= 1.0 {
                    overall_deadline
                } else {
                    1.0
                };
                let deadline_at = Instant::now() + Duration::from_secs_f64(overall);

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

                // Run the CSE fetch on a worker thread bounded by
                // per_page_timeout, then wait on it for only the remaining
                // wall-clock budget. If the deadline fires first we ABANDON the
                // in-flight fetch (the detached thread is reaped by its own
                // per-page timeout) and fall back to whatever we have — here,
                // an empty item list -> the snippet fallback's no-results
                // branch. Matches Python's `_scrape_one` + `as_completed`
                // deadline break, adapted to this port's single-fetch model.
                let (tx, rx) = mpsc::channel::<Result<Value, String>>();
                let fetch_timeout = per_page_timeout;
                thread::spawn(move || {
                    let _ = tx.send(http_get_json(&url, fetch_timeout));
                });

                let remaining = deadline_at.saturating_duration_since(Instant::now());
                let fetch_result = match rx.recv_timeout(remaining) {
                    Ok(res) => res,
                    // Deadline fired before the fetch returned. Abandon it.
                    Err(_) => Err("overall_deadline exceeded".to_string()),
                };

                let items = match fetch_result {
                    Ok(body) => body
                        .get("items")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    // Network error OR deadline abandonment: fall through with
                    // no items. The snippet fallback below then returns the
                    // configured no-results message rather than a raw error,
                    // so the model always gets a usable, non-empty reply
                    // before the kernel webhook timeout. (Python parity: the
                    // deadline path falls back to snippet formatting.)
                    Err(_) => Vec::new(),
                };

                // snippets_only / fallback path: format the CSE items as a
                // labeled snippet block (Python `_format_snippet_results`).
                // Used when the caller asked for snippets only, OR whenever the
                // normal render would be empty (deadline fired / no items).
                let formatted = if snippets_only || items.is_empty() {
                    format_snippet_results(
                        query,
                        &items,
                        num_results,
                        &no_results_message,
                        &response_prefix,
                        &response_postfix,
                    )
                } else {
                    format_web_search_response(
                        query,
                        &items,
                        num_results,
                        &response_prefix,
                        &response_postfix,
                    )
                };

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

    /// Advertise the accepted parameters, including the latency-control set
    /// (Python `WebSearchSkill.get_parameter_schema`, commit 295745b). Every
    /// param `register_tools` reads must appear here so it is discoverable;
    /// the inline test module guards against the recurring "read a param but
    /// forgot the schema entry" drift class.
    fn get_parameter_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "swaig_fields": {
                    "type": "array",
                    "description": "Additional SWAIG fields to merge into tool definitions",
                    "default": [],
                },
                "skip_prompt": {
                    "type": "boolean",
                    "description": "If true, skip adding prompt sections for this skill",
                    "default": false,
                },
                "tool_name": {
                    "type": "string",
                    "description": "Custom tool name override for this skill instance",
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of best results to return (clamped 1..10).",
                    "default": 3,
                },
                "no_results_message": {
                    "type": "string",
                    "description": "Message to show when no quality results are found. Use {query} as placeholder.",
                    "default": DEFAULT_NO_RESULTS_MESSAGE,
                },
                "response_prefix": {
                    "type": "string",
                    "description": "Optional text prepended to every non-empty search result.",
                    "default": "",
                },
                "response_postfix": {
                    "type": "string",
                    "description": "Optional text appended to every non-empty search result.",
                    "default": "",
                },
                "per_page_timeout": {
                    "type": "number",
                    "description": "Maximum seconds to wait on a single page fetch (here, the Google CSE request).",
                    "default": 2.0,
                    "min": 0.1,
                },
                "overall_deadline": {
                    "type": "number",
                    "description": "Wall-clock budget in seconds for the whole tool call. In-flight work is abandoned past this so the response beats the kernel webhook timeout.",
                    "default": 10.0,
                    "min": 1.0,
                },
                "parallel_scrape": {
                    "type": "boolean",
                    "description": "Scrape candidate pages concurrently instead of sequentially. Best-effort; a no-op in this snippet-only port (single fetch).",
                    "default": true,
                },
                "snippets_only": {
                    "type": "boolean",
                    "description": "Return Google CSE snippets only, skipping the fully-formatted render. Fastest mode (sub-second).",
                    "default": false,
                },
            },
        })
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
        return format!("No web results for \"{query}\".");
    }
    let lines: Vec<String> = items
        .iter()
        .take(usize::try_from(num_results).unwrap_or(0))
        .map(|it| {
            let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let link = it.get("link").and_then(|v| v.as_str()).unwrap_or("");
            let snippet = it.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            format!("- {title} ({link})\n  {snippet}")
        })
        .collect();
    let mut response = format!(
        "Web search results for \"{}\":\n{}",
        query,
        lines.join("\n")
    );
    if !response_prefix.is_empty() {
        response = format!("{response_prefix}\n\n{response}");
    }
    if !response_postfix.is_empty() {
        response = format!("{response}\n\n{response_postfix}");
    }
    response
}

/// Format Google CSE result items as a labeled snippet block, WITHOUT scraping
/// the underlying pages.
///
/// Mirrors Python's `GoogleSearchScraper._format_snippet_results` (commit
/// 51101da). Used in two situations:
///   - `snippets_only` is set (the caller wants a fast, sub-second answer); and
///   - as the graceful fallback when the `overall_deadline` fires or CSE
///     returns nothing, so the model always gets a non-empty, useful reply
///     instead of an error and the kernel never sees a webhook timeout.
///
/// When there are no items at all this returns the configured no-results
/// message (with `{query}` substituted) — that branch is deliberately NOT
/// wrapped with prefix/postfix, matching `format_web_search_response`'s
/// empty-path behavior. The non-empty snippet block IS wrapped, matching
/// Python/Go/TS, which apply the wrappers to every non-empty body.
fn format_snippet_results(
    query: &str,
    items: &[Value],
    num_results: i64,
    no_results_message: &str,
    response_prefix: &str,
    response_postfix: &str,
) -> String {
    if items.is_empty() {
        return no_results_message.replace("{query}", query);
    }
    let top = usize::try_from(num_results.max(1)).unwrap_or(1);
    let mut lines: Vec<String> = vec![format!(
        "Snippet-only results for '{}' (page content not scraped):\n",
        query
    )];
    for (i, it) in items.iter().take(top).enumerate() {
        let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
        // CSE exposes the result URL under "link". Python's intermediate dict
        // renames it to "url", so accept either key for robustness.
        let link = it
            .get("link")
            .and_then(|v| v.as_str())
            .or_else(|| it.get("url").and_then(|v| v.as_str()))
            .unwrap_or("");
        let snippet = it
            .get("snippet")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        lines.push(format!("=== RESULT {} ===", i + 1));
        lines.push(format!("Title: {title}"));
        lines.push(format!("URL: {link}"));
        lines.push(format!("Snippet: {snippet}"));
        lines.push(String::new());
    }
    let mut response = lines.join("\n");
    if !response_prefix.is_empty() {
        response = format!("{response_prefix}\n\n{response}");
    }
    if !response_postfix.is_empty() {
        response = format!("{response}\n\n{response_postfix}");
    }
    response
}

/// Issue a real HTTP GET via ureq and parse the JSON response.
///
/// `per_page_timeout` (seconds) bounds the whole request — connect + send +
/// receive — via ureq's `timeout_global`, matching Python's `per_page_timeout`
/// applied to the underlying fetch. A non-positive value falls back to a fixed
/// 15s bound so a misconfiguration can never produce an unbounded request.
fn http_get_json(url: &str, per_page_timeout: f64) -> Result<Value, String> {
    let timeout = if per_page_timeout > 0.0 {
        Duration::from_secs_f64(per_page_timeout)
    } else {
        Duration::from_secs(15)
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .get(url)
        .header("User-Agent", "signalwire-agents-rust-skills/1.0")
        .call()
        .map_err(|e| format!("HTTP GET {url} failed: {e}"))?;
    let status = resp.status().as_u16();
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("HTTP GET {url} body read failed: {e}"))?;
    if !(200..300).contains(&status) {
        return Err(format!("HTTP GET {url} returned {status}: {body}"));
    }
    serde_json::from_str(&body).map_err(|e| format!("HTTP GET {url} returned non-JSON: {e}"))
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
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
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
        let out = format_web_search_response("rust", &items, 3, "PREFIX_LINE", "POSTFIX_LINE");
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

    // ===================================================================
    // Latency-control params — per_page_timeout / overall_deadline /
    // parallel_scrape / snippets_only + snippet fallback.
    //
    // Ports Python 51101da + 295745b. overall_deadline + per_page_timeout
    // are the CONTRACT: a slow upstream must not blow past the kernel
    // webhook timeout (~55s). This port is snippet-only (it never scrapes
    // candidate pages), so the single network op is the Google CSE call;
    // the deadline tests point that call at a local black-hole TCP server
    // that accepts the connection and never replies, so the deadline path
    // is exercised deterministically and fast.
    // ===================================================================

    use std::io::Read as _;
    use std::net::TcpListener;
    use std::sync::{Mutex, OnceLock};

    use crate::agent::{AgentBase, AgentOptions};

    /// Serialize every test that mutates the process-global `WEB_SEARCH_BASE_URL`
    /// env var. The CI gate runs `--test-threads=1`, but a bare `cargo test`
    /// runs a binary's unit tests in parallel — without this guard two of
    /// these tests would clobber each other's base-URL override.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// RAII override of `WEB_SEARCH_BASE_URL`, restored on drop. Holds the
    /// global env lock for its whole lifetime so concurrent tests don't race.
    struct BaseUrlGuard {
        prev: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl BaseUrlGuard {
        fn set(url: &str) -> Self {
            let lock = env_lock()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let prev = std::env::var("WEB_SEARCH_BASE_URL").ok();
            unsafe {
                std::env::set_var("WEB_SEARCH_BASE_URL", url);
            }
            BaseUrlGuard { prev, _lock: lock }
        }
    }

    impl Drop for BaseUrlGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var("WEB_SEARCH_BASE_URL", v),
                    None => std::env::remove_var("WEB_SEARCH_BASE_URL"),
                }
            }
        }
    }

    /// A local TCP server that accepts every connection and NEVER sends a
    /// response (it reads the request bytes, then holds the socket open).
    /// ureq's per-request `timeout_global` (`per_page_timeout`) and the
    /// handler's `recv_timeout` (`overall_deadline`) are the only things that
    /// can end a fetch against it — exactly what the deadline tests need.
    /// Returns the bound `http://127.0.0.1:<port>` base URL.
    fn spawn_blackhole_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind blackhole");
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            // Accept connections forever; never write a byte back. Sockets are
            // dropped only when this thread (and the whole test process) ends.
            for mut s in listener.incoming().flatten() {
                thread::spawn(move || {
                    let mut buf = [0u8; 1024];
                    // One read to consume the request line, then stall.
                    let _ = s.read(&mut buf);
                    // Hold the socket open well past any test deadline.
                    #[allow(clippy::duration_suboptimal_units)] // 60s reads clearer than from_mins
                    thread::sleep(Duration::from_secs(60));
                });
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    /// A local TCP server that answers EVERY request with a fixed HTTP/1.1
    /// 200 carrying `json_body`. Stands in for Google CSE so the happy-path
    /// and `snippets_only` tests get real `items` back without hitting the
    /// network. Returns the bound base URL.
    fn spawn_cse_server(json_body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind cse");
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            for mut s in listener.incoming().flatten() {
                thread::spawn(move || {
                    use std::io::Write as _;
                    let mut buf = [0u8; 2048];
                    let _ = s.read(&mut buf);
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                        json_body.len(),
                        json_body
                    );
                    let _ = s.write_all(resp.as_bytes());
                    let _ = s.flush();
                });
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    /// Build + register the `web_search` skill on a throwaway agent and invoke
    /// its tool, returning the response string the model would see.
    fn run_web_search(params: Map<String, Value>, query: &str) -> String {
        let skill = WebSearch::new(params);
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);
        let mut args = Map::new();
        args.insert("query".to_string(), json!(query));
        let result = agent
            .on_function_call("web_search", &args, &Map::new())
            .expect("web_search tool should be registered");
        result
            .to_value()
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn creds() -> Map<String, Value> {
        let mut p = Map::new();
        p.insert("api_key".to_string(), json!("k"));
        p.insert("search_engine_id".to_string(), json!("id"));
        p
    }

    const CSE_TWO_ITEMS: &str = r#"{"items":[
        {"title":"Rust Lang","link":"https://rust-lang.org","snippet":"A systems language about widgets."},
        {"title":"Tokio","link":"https://tokio.rs","snippet":"Async runtime for Rust widgets."}
    ]}"#;

    // ---- Schema advertisement (Python 295745b) ----------------------------

    #[test]
    fn test_schema_advertises_all_six_latency_response_params() {
        let skill = WebSearch::new(creds());
        let schema = skill.get_parameter_schema();
        let props = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("schema.properties");

        // The four latency params + response_prefix/postfix must all appear.
        for key in [
            "response_prefix",
            "response_postfix",
            "per_page_timeout",
            "overall_deadline",
            "parallel_scrape",
            "snippets_only",
        ] {
            assert!(
                props.contains_key(key),
                "register_tools reads {key:?} but schema omits it"
            );
        }

        // Type + default fidelity vs Python.
        assert_eq!(props["per_page_timeout"]["type"], json!("number"));
        assert_eq!(props["per_page_timeout"]["default"], json!(2.0));
        assert_eq!(props["overall_deadline"]["type"], json!("number"));
        assert_eq!(props["overall_deadline"]["default"], json!(10.0));
        assert_eq!(props["parallel_scrape"]["type"], json!("boolean"));
        assert_eq!(props["parallel_scrape"]["default"], json!(true));
        assert_eq!(props["snippets_only"]["type"], json!("boolean"));
        assert_eq!(props["snippets_only"]["default"], json!(false));
        assert_eq!(props["response_prefix"]["default"], json!(""));
        assert_eq!(props["response_postfix"]["default"], json!(""));
    }

    // ---- format_snippet_results unit behavior -----------------------------

    #[test]
    fn test_format_snippet_results_labels_and_carries_snippets() {
        let items = vec![json!({
            "title": "Rust Lang",
            "link": "https://rust-lang.org",
            "snippet": "A systems language.",
        })];
        let out = format_snippet_results("rust", &items, 3, DEFAULT_NO_RESULTS_MESSAGE, "", "");
        assert!(out.contains("Snippet-only results for 'rust' (page content not scraped):"));
        assert!(out.contains("=== RESULT 1 ==="));
        assert!(out.contains("Title: Rust Lang"));
        assert!(out.contains("URL: https://rust-lang.org"));
        assert!(out.contains("Snippet: A systems language."));
    }

    #[test]
    fn test_format_snippet_results_empty_returns_no_results_message_unwrapped() {
        // No items -> the configured no-results message with {query} filled,
        // and NOT wrapped by prefix/postfix (matches the scraped empty path).
        let out = format_snippet_results(
            "rust",
            &[],
            3,
            DEFAULT_NO_RESULTS_MESSAGE,
            "PREFIX",
            "POSTFIX",
        );
        assert!(out.contains("I couldn't find quality results for 'rust'"));
        assert!(!out.contains("PREFIX"));
        assert!(!out.contains("POSTFIX"));
        assert!(!out.contains("Snippet-only results"));
    }

    #[test]
    fn test_format_snippet_results_wraps_nonempty_body() {
        let items = vec![json!({"title":"T","link":"https://x.com","snippet":"s"})];
        let out = format_snippet_results("q", &items, 3, DEFAULT_NO_RESULTS_MESSAGE, "PRE", "POST");
        assert!(out.starts_with("PRE\n\n"));
        assert!(out.ends_with("\n\nPOST"));
    }

    // ---- snippets_only selects the labeled snippet render ------------------

    #[test]
    fn test_snippets_only_returns_snippet_block() {
        let _g = BaseUrlGuard::set(&spawn_cse_server(CSE_TWO_ITEMS));
        let mut params = creds();
        params.insert("snippets_only".to_string(), json!(true));
        let resp = run_web_search(params, "rust widgets");

        // snippets_only must yield the labeled snippet-only format, NOT the
        // default "Web search results for ..." render.
        assert!(
            resp.contains("Snippet-only results for 'rust widgets' (page content not scraped):"),
            "snippets_only should produce labeled snippet output; got: {resp:.200}"
        );
        assert!(resp.contains("Snippet: A systems language about widgets."));
        assert!(!resp.contains("Web search results for"));
    }

    #[test]
    fn test_default_path_returns_full_render_not_snippets() {
        // With snippets_only unset (default false) and CSE returning items,
        // the handler uses the fully-formatted render, not the snippet block.
        let _g = BaseUrlGuard::set(&spawn_cse_server(CSE_TWO_ITEMS));
        let resp = run_web_search(creds(), "rust widgets");
        assert!(
            resp.starts_with("Web search results for \"rust widgets\":"),
            "default path should produce the full render; got: {resp:.200}"
        );
        assert!(resp.contains("Rust Lang"));
        assert!(!resp.contains("Snippet-only results"));
    }

    // ---- overall_deadline CONTRACT: truncate -> snippet/no-results fallback

    #[test]
    fn test_overall_deadline_truncates_to_fallback() {
        // Black-hole CSE: accepts but never replies. With a 1s overall budget
        // and a large 30s per-page timeout, the DEADLINE (not the per-page
        // timeout) must end the call. The handler abandons the in-flight fetch
        // and returns the snippet fallback (no items -> the configured
        // no-results message), all within ~deadline + slack — NOT 30s.
        let _g = BaseUrlGuard::set(&spawn_blackhole_server());
        let mut params = creds();
        params.insert("overall_deadline".to_string(), json!(1.0));
        params.insert("per_page_timeout".to_string(), json!(30.0));

        let start = Instant::now();
        let resp = run_web_search(params, "kubernetes");
        let elapsed = start.elapsed();

        // Returned at the deadline, not after the per-page timeout / hang.
        assert!(
            elapsed < Duration::from_secs(3),
            "overall_deadline (1s) not enforced: call took {elapsed:?}"
        );
        // Non-empty fallback, NOT a raw error or an empty string.
        assert!(
            !resp.trim().is_empty(),
            "deadline fallback response must be non-empty"
        );
        assert!(
            resp.contains("I couldn't find quality results for 'kubernetes'"),
            "deadline path must fall back to the no-results message; got: {resp:.200}"
        );
        assert!(
            !resp.contains("Web search error"),
            "deadline path must NOT surface a raw HTTP error; got: {resp:.200}"
        );
    }

    #[test]
    fn test_per_page_timeout_bounds_the_fetch() {
        // Black-hole CSE again, but now the PER-PAGE timeout (0.3s) is the
        // tight bound and the overall budget is generous (8s). The fetch must
        // error out near 0.3s, yielding no items and the snippet fallback —
        // crucially WELL before the 8s overall budget would expire.
        let _g = BaseUrlGuard::set(&spawn_blackhole_server());
        let mut params = creds();
        params.insert("per_page_timeout".to_string(), json!(0.3));
        params.insert("overall_deadline".to_string(), json!(8.0));

        let start = Instant::now();
        let resp = run_web_search(params, "elixir");
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(3),
            "per_page_timeout (0.3s) not honored: call took {elapsed:?}"
        );
        assert!(
            resp.contains("I couldn't find quality results for 'elixir'"),
            "all-fetch-timed-out should fall back to the no-results message; got: {resp:.200}"
        );
    }

    #[test]
    fn test_fast_cse_under_deadline_returns_real_results() {
        // Happy path: a fast CSE server with default latency params returns the
        // fully-formatted results (proving the deadline machinery doesn't
        // degrade the normal case to a fallback).
        let _g = BaseUrlGuard::set(&spawn_cse_server(CSE_TWO_ITEMS));
        let resp = run_web_search(creds(), "postgres");
        assert!(
            resp.starts_with("Web search results for \"postgres\":"),
            "fast path under the deadline should produce full results; got: {resp:.200}"
        );
        assert!(!resp.contains("I couldn't find quality results"));
    }
}
