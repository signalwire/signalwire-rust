use serde_json::{json, Value};

use crate::agent::{AgentBase, AgentOptions};
use crate::swaig::FunctionResult;

/// A pre-built FAQ bot agent that provides answers from a knowledge base.
pub struct FAQBotAgent {
    agent: AgentBase,
    faqs: Vec<Value>,
    suggest_related: bool,
}

impl FAQBotAgent {
    /// Create a new FAQBotAgent.
    ///
    /// # Arguments
    /// - `name` — agent name (defaults to `"faq_bot"` if empty).
    /// - `faqs` — list of `{question, answer}` pairs.
    /// - `suggest_related` — whether to suggest related questions.
    /// - `persona` — optional persona description.
    /// - `route` — optional route (defaults to `"/faq"`).
    pub fn new(
        name: &str,
        faqs: Vec<Value>,
        suggest_related: bool,
        persona: Option<&str>,
        route: Option<&str>,
    ) -> Self {
        let agent_name = if name.is_empty() { "faq_bot" } else { name };
        let persona_text = persona.unwrap_or(
            "You are a helpful FAQ bot that provides accurate answers to common questions.",
        );

        let mut opts = AgentOptions::new(agent_name);
        opts.route = Some(route.unwrap_or("/faq").to_string());
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
            let q = faq
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let a = faq
                .get("answer")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            faq_bullets.push(format!("Q: {} A: {}", q, a));
        }
        let bullet_refs: Vec<&str> = faq_bullets.iter().map(|s| s.as_str()).collect();
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
                        args.get("query")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                    ));
                }

                // Sort by score descending
                scored.sort_by(|a, b| b.1.cmp(&a.1));

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
                        .filter_map(|(_, _, faq)| {
                            faq.get("question").and_then(|v| v.as_str())
                        })
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
        let agent = FAQBotAgent::new("test", sample_faqs(), true, None, None);
        assert_eq!(agent.agent().service().name(), "test");
        assert_eq!(agent.agent().service().route(), "/faq");
        assert_eq!(agent.faqs().len(), 3);
        assert!(agent.suggest_related());
    }

    #[test]
    fn test_faq_bot_has_search_tool() {
        let agent = FAQBotAgent::new("test", sample_faqs(), true, None, None);
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
        let agent = FAQBotAgent::new("test", sample_faqs(), false, None, None);
        let mut args = serde_json::Map::new();
        args.insert("query".to_string(), json!("quantum physics"));
        let result = agent
            .agent()
            .on_function_call("search_faqs", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("No FAQ found"));
    }
}
