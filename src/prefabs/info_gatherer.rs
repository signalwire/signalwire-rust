use serde_json::{json, Value};

use crate::agent::{AgentBase, AgentOptions};
use crate::swaig::FunctionResult;

/// A pre-built agent that asks a series of questions and collects answers.
pub struct InfoGathererAgent {
    agent: AgentBase,
    questions: Vec<Value>,
}

impl InfoGathererAgent {
    /// Create a new InfoGathererAgent.
    ///
    /// # Arguments
    /// - `name` — agent name (defaults to `"info_gatherer"` if empty).
    /// - `questions` — list of `{key_name, question_text, confirm?}` objects.
    /// - `route` — optional route (defaults to `"/info_gatherer"`).
    pub fn new(name: &str, questions: Vec<Value>, route: Option<&str>) -> Self {
        let agent_name = if name.is_empty() {
            "info_gatherer"
        } else {
            name
        };

        let mut opts = AgentOptions::new(agent_name);
        opts.route = Some(route.unwrap_or("/info_gatherer").to_string());
        opts.use_pom = true;

        let mut agent = AgentBase::new(opts);

        // Global data tracks question index and answers
        agent.set_global_data(json!({
            "questions": questions,
            "question_index": 0,
            "answers": [],
        }));

        // Prompt section
        agent.prompt_add_section(
            "Information Gathering",
            "You are an information-gathering assistant. Your job is to ask the user a series of questions and collect their answers.",
            vec![
                "Ask questions one at a time in order",
                "Wait for the user to answer before asking the next question",
                "Confirm answers when the question requires confirmation",
                "Use start_questions to begin and submit_answer for each response",
            ],
        );

        // Tool: start_questions
        let q_clone = questions.clone();
        agent.define_tool(
            "start_questions",
            "Start the question-gathering process and return the first question",
            json!({}),
            Box::new(move |_args, _raw| {
                let first = q_clone
                    .first()
                    .and_then(|q| q.get("question_text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("No questions configured");
                FunctionResult::with_response(first)
            }),
            false,
        );

        // Tool: submit_answer
        agent.define_tool(
            "submit_answer",
            "Submit an answer to the current question",
            json!({
                "answer": {
                    "type": "string",
                    "description": "The answer",
                },
                "confirmed_by_user": {
                    "type": "boolean",
                    "description": "User confirmed this answer",
                },
            }),
            Box::new(|args, _raw| {
                let answer = args
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                FunctionResult::with_response(&format!("Answer recorded: {}", answer))
            }),
            false,
        );

        InfoGathererAgent { agent, questions }
    }

    /// Access the underlying `AgentBase`.
    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    /// Access the underlying `AgentBase` mutably.
    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    /// Get the configured questions.
    pub fn questions(&self) -> &[Value] {
        &self.questions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_questions() -> Vec<Value> {
        vec![
            json!({"key_name": "name", "question_text": "What is your name?"}),
            json!({"key_name": "email", "question_text": "What is your email?", "confirm": true}),
        ]
    }

    #[test]
    fn test_info_gatherer_construction() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        assert_eq!(agent.agent().service().name(), "test");
        assert_eq!(agent.agent().service().route(), "/info_gatherer");
        assert_eq!(agent.questions().len(), 2);
    }

    #[test]
    fn test_info_gatherer_has_tools() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        let args = serde_json::Map::new();
        let raw = serde_json::Map::new();
        let result = agent.agent().on_function_call("start_questions", &args, &raw);
        assert!(result.is_some());
    }

    #[test]
    fn test_info_gatherer_default_name() {
        let agent = InfoGathererAgent::new("", sample_questions(), None);
        assert_eq!(agent.agent().service().name(), "info_gatherer");
    }
}
