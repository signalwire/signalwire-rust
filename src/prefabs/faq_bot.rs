use std::fmt::Write as _;

use serde_json::{Map, Value, json};

use crate::agent::{AgentBase, AgentOptions};
use crate::prefabs::PrefabSummaryCallback;
use crate::swaig::FunctionResult;

/// Options for constructing an [`FAQBotAgent`].
///
/// Mirrors the Python reference's `__init__` (`prefabs/faq_bot.py:48-55`)
/// param-for-param. `faqs` is the reference's one REQUIRED positional, so it is
/// the sole argument to [`FAQBotOptions::new`]; every other field carries the
/// reference's default. `FAQBotOptions::new(faqs)` is therefore the exact
/// equivalent of the minimal valid reference program `FAQBotAgent(faqs)`.
#[must_use]
pub struct FAQBotOptions {
    /// FAQ entries, each `{question, answer, categories?}`.
    pub faqs: Vec<Value>,
    /// Whether to suggest related questions (reference default `true`).
    pub suggest_related: bool,
    /// Custom personality description. `None` uses the reference's wording.
    pub persona: Option<String>,
    /// Agent name (reference default `"faq_bot"`).
    pub name: String,
    /// HTTP route (reference default `"/faq"`).
    pub route: String,
}

impl Default for FAQBotOptions {
    fn default() -> Self {
        FAQBotOptions {
            faqs: Vec::new(),
            suggest_related: true,
            persona: None,
            name: "faq_bot".to_string(),
            route: "/faq".to_string(),
        }
    }
}

impl FAQBotOptions {
    /// Options for `faqs`, with every other field at its reference default —
    /// the port of the reference's `FAQBotAgent(faqs)`. `faqs` is required
    /// because the reference declares it as a positional with no default.
    pub fn new(faqs: Vec<Value>) -> Self {
        FAQBotOptions {
            faqs,
            ..Default::default()
        }
    }

    /// Replace the FAQ entries.
    pub fn faqs(mut self, faqs: Vec<Value>) -> Self {
        self.faqs = faqs;
        self
    }

    /// Toggle related-question suggestions (default `true`).
    pub fn suggest_related(mut self, suggest: bool) -> Self {
        self.suggest_related = suggest;
        self
    }

    /// Override the bot's persona description.
    pub fn persona(mut self, persona: &str) -> Self {
        self.persona = Some(persona.to_string());
        self
    }

    /// Set the agent name (default `"faq_bot"`).
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the HTTP route (default `"/faq"`).
    pub fn route(mut self, route: &str) -> Self {
        self.route = route.to_string();
        self
    }
}

/// A pre-built FAQ bot agent that provides answers from a knowledge base.
pub struct FAQBotAgent {
    agent: AgentBase,
    faqs: Vec<Value>,
    suggest_related: bool,
}

impl FAQBotAgent {
    /// Create a new `FAQBotAgent` from [`FAQBotOptions`].
    ///
    /// Every option is defaulted, so `FAQBotAgent::new(FAQBotOptions::default())`
    /// (equivalently `FAQBotAgent::default()`) is the port of the reference's
    /// zero-argument `FAQBotAgent()`.
    pub fn new(options: FAQBotOptions) -> Self {
        let FAQBotOptions {
            faqs,
            suggest_related,
            persona,
            name,
            route,
        } = options;

        let agent_name = if name.is_empty() { "faq_bot" } else { &name };
        let persona_text = persona.unwrap_or_else(|| {
            "You are a helpful FAQ bot that provides accurate answers to common questions."
                .to_string()
        });
        let persona_text = persona_text.as_str();

        let mut opts = AgentOptions::new(agent_name);
        opts.route = Some(if route.is_empty() {
            "/faq".to_string()
        } else {
            route
        });
        opts.use_pom = true;

        let mut agent = AgentBase::new(opts);

        // Global data
        agent.set_global_data(json!({
            "faqs": faqs,
            "suggest_related": suggest_related,
        }));

        // Persona section
        agent.prompt_add_section("Personality", persona_text, vec![]);

        // Build FAQ knowledge section
        let mut faq_bullets: Vec<String> = Vec::new();
        for faq in &faqs {
            let q = faq.get("question").and_then(|v| v.as_str()).unwrap_or("?");
            let a = faq.get("answer").and_then(|v| v.as_str()).unwrap_or("?");
            faq_bullets.push(format!("Q: {q} A: {a}"));
        }
        let bullet_refs: Vec<&str> = faq_bullets
            .iter()
            .map(std::string::String::as_str)
            .collect();
        agent.prompt_add_section(
            "FAQ Knowledge Base",
            "You have knowledge of the following frequently asked questions.",
            bullet_refs,
        );

        // Optional related suggestions section
        if suggest_related {
            agent.prompt_add_section(
                "Related Questions",
                "When appropriate, suggest related questions the user might also be interested in.",
                vec![],
            );
        }

        // Tool: search_faqs
        let faqs_clone = faqs.clone();
        agent.define_tool(
            "search_faqs",
            "Search the FAQ knowledge base by keyword matching and return the best answer",
            json!({
                "query": {"type": "string", "description": "The question or keywords to search"},
            }),
            Box::new(move |args, _raw| {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();

                if query.is_empty() {
                    return FunctionResult::with_response("Please provide a search query.");
                }

                let keywords: Vec<&str> = query.split_whitespace().collect();

                // Score each FAQ by keyword matches
                let mut scored: Vec<(usize, i32, &Value)> = Vec::new();
                for (index, faq) in faqs_clone.iter().enumerate() {
                    let question_lower = faq
                        .get("question")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let mut score = 0i32;

                    // Exact substring match gets highest score
                    if question_lower.contains(&query) {
                        score += 10;
                    }

                    // Individual keyword matches
                    for keyword in &keywords {
                        if !keyword.is_empty() && question_lower.contains(keyword) {
                            score += 1;
                        }
                    }

                    if score > 0 {
                        scored.push((index, score, faq));
                    }
                }

                if scored.is_empty() {
                    return FunctionResult::with_response(&format!(
                        "No FAQ found matching: {}",
                        args.get("query").and_then(|v| v.as_str()).unwrap_or("")
                    ));
                }

                // Sort by score descending
                scored.sort_by_key(|b| std::cmp::Reverse(b.1));

                let best = scored[0].2;
                let mut response = best
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Suggest related questions
                if suggest_related && scored.len() > 1 {
                    let related: Vec<&str> = scored[1..scored.len().min(4)]
                        .iter()
                        .filter_map(|(_, _, faq)| faq.get("question").and_then(|v| v.as_str()))
                        .collect();
                    if !related.is_empty() {
                        response.push_str("\n\nRelated questions: ");
                        response.push_str(&related.join("; "));
                    }
                }

                FunctionResult::with_response(&response)
            }),
            false,
        );

        FAQBotAgent {
            agent,
            faqs,
            suggest_related,
        }
    }

    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    pub fn faqs(&self) -> &[Value] {
        &self.faqs
    }

    pub fn suggest_related(&self) -> bool {
        self.suggest_related
    }

    /// Search for FAQs matching a specific query and/or category.
    ///
    /// Ported from Python `FAQBotAgent.search_faqs`. Scores each FAQ by
    /// substring/prefix match on the `query` (exact match 100, substring 50, a
    /// prefix bonus of 25) plus 30 for a matching `category`, then returns the
    /// top three matching questions. `raw_data` is accepted for
    /// handler-signature compatibility but unused.
    pub fn search_faqs(
        &self,
        args: &Map<String, Value>,
        _raw_data: &Map<String, Value>,
    ) -> FunctionResult {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let category = args
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        // (score, question) accumulator for positive-scoring FAQs.
        let mut results: Vec<(i64, String)> = Vec::new();

        for faq in &self.faqs {
            let question = faq.get("question").and_then(|v| v.as_str()).unwrap_or("");
            let question_lower = question.to_lowercase();
            let categories: Vec<String> = faq
                .get("categories")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| c.as_str().map(str::to_lowercase))
                        .collect()
                })
                .unwrap_or_default();

            let mut match_score = 0i64;

            // Match on query.
            if !query.is_empty() && question_lower.contains(&query) {
                if query == question_lower {
                    match_score += 100;
                } else {
                    match_score += 50;
                }
                if question_lower.starts_with(&query) {
                    match_score += 25;
                }
            }

            // Match on category.
            if !category.is_empty() && categories.iter().any(|c| c == &category) {
                match_score += 30;
            }

            if match_score > 0 {
                results.push((match_score, question.to_string()));
            }
        }

        // Sort by score descending, then take the top three.
        results.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        let top: Vec<&(i64, String)> = results.iter().take(3).collect();

        if top.is_empty() {
            return FunctionResult::with_response("No matching FAQs found.");
        }

        let mut result_text = String::from("Here are the most relevant FAQs:\n\n");
        for (i, (_, question)) in top.iter().enumerate() {
            let _ = writeln!(result_text, "{}. {question}", i + 1);
        }
        FunctionResult::with_response(&result_text)
    }

    /// Register a callback that processes the interaction summary.
    ///
    /// Delegates to [`AgentBase::on_summary`], matching the Python
    /// `FAQBotAgent.on_summary` override point (which logs the summary).
    pub fn on_summary(&mut self, callback: PrefabSummaryCallback) -> &mut Self {
        self.agent.on_summary(callback);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_faqs() -> Vec<Value> {
        vec![
            json!({"question": "What are your hours?", "answer": "We are open 9am to 5pm."}),
            json!({"question": "Where are you located?", "answer": "123 Main Street."}),
            json!({"question": "Do you offer refunds?", "answer": "Yes, within 30 days."}),
        ]
    }

    #[test]
    fn test_faq_bot_construction() {
        let agent = FAQBotAgent::new(FAQBotOptions::new(sample_faqs()).name("test"));
        assert_eq!(agent.agent().service().name(), "test");
        assert_eq!(agent.agent().service().route(), "/faq");
        assert_eq!(agent.faqs().len(), 3);
        assert!(agent.suggest_related());
    }

    #[test]
    fn test_faq_bot_has_search_tool() {
        let agent = FAQBotAgent::new(FAQBotOptions::new(sample_faqs()).name("test"));
        let mut args = serde_json::Map::new();
        args.insert("query".to_string(), json!("hours"));
        let result = agent
            .agent()
            .on_function_call("search_faqs", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("9am to 5pm"));
    }

    #[test]
    fn test_faq_bot_no_match() {
        let agent = FAQBotAgent::new(
            FAQBotOptions::new(sample_faqs())
                .name("test")
                .suggest_related(false),
        );
        let mut args = serde_json::Map::new();
        args.insert("query".to_string(), json!("quantum physics"));
        let result = agent
            .agent()
            .on_function_call("search_faqs", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("No FAQ found"));
    }

    fn categorized_faqs() -> Vec<Value> {
        vec![
            json!({"question": "What are your hours?", "answer": "9-5", "categories": ["general"]}),
            json!({"question": "How do I get a refund?", "answer": "Within 30 days", "categories": ["billing"]}),
            json!({"question": "Where are you located?", "answer": "123 Main", "categories": ["general"]}),
        ]
    }

    #[test]
    fn test_search_faqs_query_match() {
        let agent = FAQBotAgent::new(FAQBotOptions::new(categorized_faqs()).name("test"));
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("query".to_string(), json!("hours"));
        let json_str = agent.search_faqs(&args, &raw).to_json();
        assert!(json_str.contains("most relevant FAQs"));
        assert!(json_str.contains("What are your hours?"));
    }

    #[test]
    fn test_search_faqs_category_match() {
        let agent = FAQBotAgent::new(FAQBotOptions::new(categorized_faqs()).name("test"));
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("category".to_string(), json!("billing"));
        let json_str = agent.search_faqs(&args, &raw).to_json();
        assert!(json_str.contains("How do I get a refund?"));
    }

    #[test]
    fn test_search_faqs_no_match() {
        let agent = FAQBotAgent::new(FAQBotOptions::new(categorized_faqs()).name("test"));
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("query".to_string(), json!("nonexistent topic"));
        let json_str = agent.search_faqs(&args, &raw).to_json();
        assert!(json_str.contains("No matching FAQs found"));
    }

    #[test]
    fn test_faq_bot_on_summary_fires() {
        use std::sync::{Arc, Mutex};

        let mut agent = FAQBotAgent::new(FAQBotOptions::new(sample_faqs()).name("test"));
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_clone = Arc::clone(&captured);
        agent.on_summary(Box::new(move |summary, _data, _headers| {
            *captured_clone.lock().unwrap() = summary.to_string();
        }));

        let (user, pass) = agent.agent().service().basic_auth_credentials();
        let auth = {
            use base64::Engine;
            use base64::engine::general_purpose::STANDARD as BASE64;
            format!("Basic {}", BASE64.encode(format!("{user}:{pass}")))
        };
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), auth);

        let body = json!({"summary": "FAQ answered"});
        let (status, _, _) = agent.agent_mut().handle_request(
            "POST",
            "/faq/post_prompt",
            &headers,
            &body.to_string(),
        );
        assert_eq!(status, 200);
        assert_eq!(*captured.lock().unwrap(), "FAQ answered");
    }
}
