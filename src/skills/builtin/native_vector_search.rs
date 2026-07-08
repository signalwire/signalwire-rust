use serde_json::{Map, Value, json};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Perform a remote vector search: POST `{query, index_name, count, ...}` to
/// `<remote_url>/search` and return the parsed `results` array.
///
/// Mirrors Python `NativeVectorSearchSkill._search_remote`
/// (`skills/native_vector_search/skill.py`): posts a JSON body to the
/// `/search` sub-path of the configured remote URL and reads the
/// `results: [{content, score, metadata}]` list from the JSON response.
/// On any transport / non-200 / parse error, returns an empty vector.
fn search_remote(remote_url: &str, query: &str, index_name: &str, count: i64) -> Vec<Value> {
    let base = remote_url.trim_end_matches('/');
    let url = format!("{base}/search");

    let payload = json!({
        "query": query,
        "index_name": index_name,
        "count": count,
    });

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();

    let body = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());

    let send = agent
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send(&body);

    let Ok(mut resp) = send else {
        return Vec::new();
    };
    if resp.status().as_u16() != 200 {
        return Vec::new();
    }
    let Ok(body_str) = resp.body_mut().read_to_string() else {
        return Vec::new();
    };
    let Ok(data) = serde_json::from_str::<Value>(&body_str) else {
        return Vec::new();
    };

    data.get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Format a remote `results` list into the response text, mirroring the
/// Python skill's `_search_handler`: a "Found N relevant results" header,
/// then one `**Result i**` block per hit (from filename, optional section,
/// optional tags, relevance score, then the content).
fn format_results(query: &str, results: &[Value]) -> String {
    use std::fmt::Write as _;
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        "Found {} relevant results for '{query}':\n",
        results.len()
    ));

    for (i, result) in results.iter().enumerate() {
        let metadata = result.get("metadata");
        let filename = metadata
            .and_then(|m| m.get("filename"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let section = metadata
            .and_then(|m| m.get("section"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let score = result.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let content = result.get("content").and_then(|v| v.as_str()).unwrap_or("");

        let tags: Vec<String> = result
            .get("tags")
            .or_else(|| metadata.and_then(|m| m.get("tags")))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let mut text = format!("**Result {}** (from {filename}", i + 1);
        if !section.is_empty() {
            let _ = write!(text, ", section: {section}");
        }
        if !tags.is_empty() {
            let _ = write!(text, ", tags: {}", tags.join(", "));
        }
        let _ = write!(text, ", relevance: {score:.2})\n{content}\n");
        parts.push(text);
    }

    parts.join("\n")
}

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
    fn name(&self) -> &'static str {
        "native_vector_search"
    }

    fn description(&self) -> &'static str {
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
        let tool_description = self.sp.get_str_or(
            "description",
            "Search the local knowledge base for information",
        );
        let default_count = self.sp.get_i64("count", 5).max(1);
        let remote_url = self.sp.get_str_or("remote_url", "");
        let index_name = self.sp.get_str_or("index_name", "");
        let no_results_message = self
            .sp
            .get_str_or("no_results_message", "No information found for '{query}'");

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
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(default_count);

                if query.is_empty() {
                    result.set_response("Error: No search query provided.");
                    return result;
                }

                if remote_url.is_empty() {
                    // Local (in-process) index mode is not shipped in the Rust
                    // port — the native search engine (sentence-transformers /
                    // sqlite-vec) is Python-only. Only network mode performs a
                    // real search here.
                    result.set_response(&format!(
                        "Vector search results for \"{query}\": \
                         Searched index \"{index_name}\" with count={count}. \
                         In production, this would return vector similarity search results."
                    ));
                } else {
                    // Network mode: POST to <remote_url>/search and format the
                    // returned results (parity with Python's _search_remote +
                    // _search_handler).
                    let results = search_remote(&remote_url, query, &index_name, count);
                    if results.is_empty() {
                        result.set_response(&no_results_message.replace("{query}", query));
                    } else {
                        result.set_response(&format_results(query, &results));
                    }
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

    // ===================================================================
    // Tier-2 behavioral contract #4: native_vector_search REMOTE HTTP.
    // Configure remote_url → a mock HTTP server on a FREE port; invoke the
    // search tool; assert a real POST to <remote_url>/search carried the
    // query, and the mock's results are formatted into the response
    // (NOT the old "In production this would…" stub string).
    // ===================================================================

    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    use crate::agent::{AgentBase, AgentOptions};

    /// A local TCP server that answers the first request with a fixed 200
    /// JSON body and sends the received request path+body back over `tx`
    /// so the test can assert the client actually POSTed. Binds a free
    /// ephemeral port; the socket is released when the process ends.
    fn spawn_search_server(json_body: &'static str, tx: mpsc::Sender<String>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind search mock");
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Some(mut s) = listener.incoming().flatten().next() {
                // Read until we have the full request: headers + a body of the
                // advertised Content-Length. ureq may split headers and body
                // across TCP segments, so a single read can miss the body.
                let mut raw = String::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = s.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if let Some(hdr_end) = raw.find("\r\n\r\n") {
                        let content_len = raw
                            .lines()
                            .find_map(|l| {
                                let l = l.to_ascii_lowercase();
                                l.strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                            })
                            .unwrap_or(0);
                        let body_have = raw.len() - (hdr_end + 4);
                        if body_have >= content_len {
                            break;
                        }
                    }
                }
                let _ = tx.send(raw);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    json_body.len(),
                    json_body
                );
                let _ = s.write_all(resp.as_bytes());
                let _ = s.flush();
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    const REMOTE_RESULTS: &str = r#"{"results":[
        {"content":"Widgets are configured via the dashboard.","score":0.91,
         "metadata":{"filename":"widgets.md","section":"Setup"}},
        {"content":"Restart the service to apply changes.","score":0.72,
         "metadata":{"filename":"ops.md"}}
    ]}"#;

    #[test]
    fn test_search_remote_posts_query_and_formats_results() {
        let (tx, rx) = mpsc::channel();
        let base = spawn_search_server(REMOTE_RESULTS, tx);

        let mut params = Map::new();
        params.insert("remote_url".to_string(), json!(base.clone()));
        params.insert("index_name".to_string(), json!("kb"));

        let skill = NativeVectorSearch::new(params);
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);

        let mut args = Map::new();
        args.insert("query".to_string(), json!("how do I configure widgets"));
        let result = agent
            .on_function_call("search_knowledge", &args, &Map::new())
            .expect("search_knowledge tool should be registered");
        let response = result
            .to_value()
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // (a) A real HTTP request reached the mock, POSTed to /search, and
        //     carried the query in its body.
        let raw = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("mock server should have received a request");
        assert!(
            raw.starts_with("POST /search "),
            "expected POST /search, got: {raw}"
        );
        assert!(
            raw.contains("how do I configure widgets"),
            "request body should carry the query, got: {raw}"
        );

        // (b) The mock's results are formatted into the FunctionResult — NOT
        //     the old stub string.
        assert!(
            !response.contains("In production"),
            "must not return the stub string; got: {response}"
        );
        assert!(response.contains("Found 2 relevant results for 'how do I configure widgets'"));
        assert!(
            response.contains("**Result 1** (from widgets.md, section: Setup, relevance: 0.91)")
        );
        assert!(response.contains("Widgets are configured via the dashboard."));
        assert!(response.contains("**Result 2** (from ops.md, relevance: 0.72)"));
    }

    #[test]
    fn test_search_remote_no_results_message() {
        let (tx, _rx) = mpsc::channel();
        let base = spawn_search_server(r#"{"results":[]}"#, tx);

        let mut params = Map::new();
        params.insert("remote_url".to_string(), json!(base));
        let skill = NativeVectorSearch::new(params);
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);

        let mut args = Map::new();
        args.insert("query".to_string(), json!("nonexistent topic"));
        let result = agent
            .on_function_call("search_knowledge", &args, &Map::new())
            .expect("tool registered");
        let response = result
            .to_value()
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        assert_eq!(response, "No information found for 'nonexistent topic'");
    }
}
