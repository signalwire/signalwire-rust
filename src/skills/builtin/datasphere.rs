use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Map, Value, json};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Search knowledge using SignalWire DataSphere RAG stack.
///
/// Mirrors Python's `signalwire.skills.datasphere`: real HTTP POST to
/// `https://{space_name}.signalwire.com/api/datasphere/documents/search`
/// with project-id/token Basic auth and a JSON body containing the
/// document ID, query, distance, and count.
///
/// The base URL can be overridden by setting `DATASPHERE_BASE_URL` —
/// `audit_skills_dispatch.py` uses this to point at its loopback
/// fixture. Token can also come from `DATASPHERE_TOKEN`.
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
    fn name(&self) -> &'static str {
        "datasphere"
    }

    fn description(&self) -> &'static str {
        "Search knowledge using SignalWire DataSphere RAG stack"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        for key in &["space_name", "project_id", "document_id"] {
            if self.sp.get_str(key).is_none() {
                return false;
            }
        }
        // Token can come from params OR DATASPHERE_TOKEN env var.
        if self.sp.get_str("token").is_none() && std::env::var("DATASPHERE_TOKEN").is_err() {
            return false;
        }
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("search_knowledge");
        let space_name = self.sp.get_str_or("space_name", "");
        let project_id = self.sp.get_str_or("project_id", "");
        let token_param = self
            .sp
            .get_str("token")
            .map(std::string::ToString::to_string);
        let document_id = self.sp.get_str_or("document_id", "");
        let count = self.sp.get_i64("count", 1).clamp(1, 10);
        let distance = self.sp.get_f64("distance", 3.0);

        agent.define_tool(
            &tool_name,
            "Search the knowledge base for information on any topic and return relevant results",
            json!({
                "query": {
                    "type": "string",
                    "description": "The search query to find relevant knowledge",
                    // Not required — Python passes none (datasphere/skill.py:171).
                }
            }),
            Box::new(move |args, _raw| {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                if query.is_empty() {
                    let mut r = FunctionResult::new();
                    r.set_response("Error: No search query provided.");
                    return r;
                }

                let token = token_param
                    .clone()
                    .or_else(|| std::env::var("DATASPHERE_TOKEN").ok())
                    .unwrap_or_default();

                let base = std::env::var("DATASPHERE_BASE_URL")
                    .unwrap_or_else(|_| format!("https://{space_name}.signalwire.com"));
                let url = format!(
                    "{}/api/datasphere/documents/search",
                    base.trim_end_matches('/')
                );

                let payload = json!({
                    "document_id": document_id,
                    "query_string": query,
                    "distance": distance,
                    "count": count,
                });

                let body = match http_post_json(&url, &project_id, &token, &payload) {
                    Ok(v) => v,
                    Err(e) => {
                        let mut r = FunctionResult::new();
                        r.set_response(&format!("DataSphere error: {e}"));
                        return r;
                    }
                };

                // Parse `chunks` (real API) OR `results` (audit fixture
                // shape) — both are real-shape upstream responses.
                let entries = body
                    .get("chunks")
                    .and_then(|v| v.as_array())
                    .or_else(|| body.get("results").and_then(|v| v.as_array()))
                    .cloned()
                    .unwrap_or_default();

                let formatted = if entries.is_empty() {
                    format!("No DataSphere knowledge results for \"{query}\".")
                } else {
                    let lines: Vec<String> = entries
                        .iter()
                        .take(usize::try_from(count).unwrap_or(0))
                        .enumerate()
                        .map(|(i, e)| {
                            let text = e
                                .get("text")
                                .or_else(|| e.get("content"))
                                .or_else(|| e.get("chunk"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            format!("=== RESULT {} ===\n{}", i + 1, text)
                        })
                        .collect();
                    format!(
                        "I found {} result(s) for '{}':\n\n{}",
                        entries.len(),
                        query,
                        lines.join("\n\n")
                    )
                };

                let mut r = FunctionResult::new();
                r.set_response(&formatted);
                r
            }),
            true,
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

/// Issue an HTTP POST with Basic auth and a JSON body, parse JSON response.
fn http_post_json(url: &str, project: &str, token: &str, payload: &Value) -> Result<Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();

    let auth = format!("Basic {}", BASE64.encode(format!("{project}:{token}")));
    let body = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());

    let mut resp = agent
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", &auth)
        .header("User-Agent", "signalwire-agents-rust-skills/1.0")
        .send(&body)
        .map_err(|e| format!("POST {url} failed: {e}"))?;

    let status = resp.status().as_u16();
    let body_str = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("POST {url} body read failed: {e}"))?;

    if !(200..300).contains(&status) {
        return Err(format!("POST {url} returned {status}: {body_str}"));
    }

    serde_json::from_str(&body_str).map_err(|e| format!("POST {url} returned non-JSON: {e}"))
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
        let prev = std::env::var("DATASPHERE_TOKEN").ok();
        unsafe {
            std::env::remove_var("DATASPHERE_TOKEN");
        }

        let mut skill = Datasphere::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("space_name".to_string(), json!("test.signalwire.com"));
        params.insert("project_id".to_string(), json!("proj-123"));
        params.insert("token".to_string(), json!("tok-456"));
        params.insert("document_id".to_string(), json!("doc-789"));
        let mut skill2 = Datasphere::new(params);
        assert!(skill2.setup());

        unsafe {
            if let Some(v) = prev {
                std::env::set_var("DATASPHERE_TOKEN", v);
            }
        }
    }
}
