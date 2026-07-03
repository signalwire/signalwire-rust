use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::agent::{AgentBase, AgentOptions};
use crate::swaig::FunctionResult;

/// Callback for dynamic question configuration.
///
/// Ported from Python `InfoGathererAgent.set_question_callback`: receives
/// `(query_params, body_params, headers)` and returns the list of question
/// objects (`{key_name, question_text, confirm?}`) to gather.
pub type QuestionCallback = Arc<
    dyn Fn(&Map<String, Value>, &Map<String, Value>, &HashMap<String, String>) -> Vec<Value>
        + Send
        + Sync,
>;

/// A pre-built agent that asks a series of questions and collects answers.
///
/// Supports both static mode (questions supplied at construction) and dynamic
/// mode (questions produced per-request by a callback set via
/// [`InfoGathererAgent::set_question_callback`] and resolved in
/// [`InfoGathererAgent::on_swml_request`]).
pub struct InfoGathererAgent {
    agent: AgentBase,
    questions: Vec<Value>,
    /// True when questions were supplied at construction (static mode). In
    /// dynamic mode `on_swml_request` resolves questions per-request.
    static_mode: bool,
    question_callback: Option<QuestionCallback>,
}

impl InfoGathererAgent {
    /// Create a new `InfoGathererAgent`.
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
                let answer = args.get("answer").and_then(|v| v.as_str()).unwrap_or("");
                FunctionResult::with_response(&format!("Answer recorded: {answer}"))
            }),
            false,
        );

        // Static mode when questions are supplied at construction; an empty list
        // means dynamic mode, where `on_swml_request` resolves questions per
        // request (via a callback set by `set_question_callback`, or a fallback).
        let static_mode = !questions.is_empty();

        InfoGathererAgent {
            agent,
            questions,
            static_mode,
            question_callback: None,
        }
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

    /// Set a callback for dynamic (per-request) question configuration.
    ///
    /// Ported from Python `InfoGathererAgent.set_question_callback`. The callback
    /// receives `(query_params, body_params, headers)` and returns a list of
    /// question objects. Only consulted in dynamic mode (no static questions).
    pub fn set_question_callback(&mut self, callback: QuestionCallback) -> &mut Self {
        self.question_callback = Some(callback);
        self
    }

    /// Resolve dynamic configuration when SWML is requested.
    ///
    /// Ported from Python `InfoGathererAgent.on_swml_request`. In static mode
    /// (questions supplied at construction) returns `None`. In dynamic mode it
    /// invokes the question callback (or a name/message fallback when none is set)
    /// and returns a `{"global_data": {questions, question_index, answers}}` map
    /// to seed the agent's global data. Invalid callback output (no questions)
    /// falls back to the default question set.
    pub fn on_swml_request(
        &self,
        request_data: Option<&Map<String, Value>>,
        query_params: &Map<String, Value>,
        headers: &HashMap<String, String>,
    ) -> Option<Value> {
        // Only process in dynamic mode.
        if self.static_mode {
            return None;
        }

        let fallback = || {
            json!([
                {"key_name": "name", "question_text": "What is your name?"},
                {"key_name": "message", "question_text": "How can I help you today?"},
            ])
        };

        let questions: Value = match &self.question_callback {
            None => fallback(),
            Some(cb) => {
                let empty = Map::new();
                let body = request_data.unwrap_or(&empty);
                let result = cb(query_params, body, headers);
                if result.is_empty() {
                    // Mirror Python's validate-then-fallback on invalid output.
                    fallback()
                } else {
                    Value::Array(result)
                }
            }
        };

        Some(json!({
            "global_data": {
                "questions": questions,
                "question_index": 0,
                "answers": [],
            }
        }))
    }

    /// Build the instruction text for asking a question.
    ///
    /// Ported from Python `InfoGathererAgent._generate_question_instruction`.
    fn generate_question_instruction(
        question_text: &str,
        needs_confirmation: bool,
        is_first_question: bool,
    ) -> String {
        let mut instruction = if is_first_question {
            format!("Ask the user to answer the following question: {question_text}\n\n")
        } else {
            format!(
                "Previous Answer recorded. Now ask the user to answer the following question: {question_text}\n\n"
            )
        };

        instruction.push_str(
            "Make sure the answer fits the scope and context of the question before submitting it. ",
        );

        if needs_confirmation {
            instruction.push_str(
                "Insist that the user confirms the answer as many times as needed until they say it is correct.",
            );
        } else {
            instruction.push_str("You don't need the user to confirm the answer to this question.");
        }

        instruction
    }

    /// Start the question sequence by returning the first question.
    ///
    /// Ported from Python `InfoGathererAgent.start_questions`. Reads `questions`
    /// and `question_index` from `raw_data["global_data"]` and returns the
    /// instruction for the current question. `args` is accepted for
    /// handler-signature parity but unused.
    pub fn start_questions(
        &self,
        _args: &Map<String, Value>,
        raw_data: &Map<String, Value>,
    ) -> FunctionResult {
        let global_data = raw_data.get("global_data");
        let questions = global_data
            .and_then(|g| g.get("questions"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let question_index = usize::try_from(
            global_data
                .and_then(|g| g.get("question_index"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        )
        .unwrap_or(0);

        if questions.is_empty() || question_index >= questions.len() {
            return FunctionResult::with_response("I don't have any questions to ask.");
        }

        let current = &questions[question_index];
        let question_text = current
            .get("question_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let needs_confirmation = current
            .get("confirm")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let instruction =
            Self::generate_question_instruction(question_text, needs_confirmation, true);

        let mut result = FunctionResult::with_response(&instruction);
        result.replace_in_history(Some("Welcome! Let me ask you a few questions."));
        result
    }

    /// Submit an answer to the current question and advance to the next.
    ///
    /// Ported from Python `InfoGathererAgent.submit_answer`. Records the answer
    /// under the current question's `key_name`, increments `question_index`, and
    /// returns either the next question's instruction or a completion message,
    /// with the updated `answers`/`question_index` pushed via
    /// `update_global_data`. State is read from `raw_data["global_data"]`.
    pub fn submit_answer(
        &self,
        args: &Map<String, Value>,
        raw_data: &Map<String, Value>,
    ) -> FunctionResult {
        let answer = args.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        let global_data = raw_data.get("global_data");
        let questions = global_data
            .and_then(|g| g.get("questions"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let question_index = usize::try_from(
            global_data
                .and_then(|g| g.get("question_index"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        )
        .unwrap_or(0);
        let mut answers = global_data
            .and_then(|g| g.get("answers"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if question_index >= questions.len() {
            return FunctionResult::with_response("All questions have already been answered.");
        }

        let key_name = questions[question_index]
            .get("key_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        answers.push(json!({"key_name": key_name, "answer": answer}));

        let new_question_index = question_index + 1;

        if new_question_index < questions.len() {
            let next = &questions[new_question_index];
            let next_text = next
                .get("question_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let needs_confirmation = next
                .get("confirm")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);

            let instruction =
                Self::generate_question_instruction(next_text, needs_confirmation, false);

            let mut result = FunctionResult::with_response(&instruction);
            result.replace_in_history(None);
            result.update_global_data(json!({
                "answers": answers,
                "question_index": new_question_index,
            }));
            result
        } else {
            let mut result = FunctionResult::with_response(
                "Thank you! All questions have been answered. You can now summarize the information collected or ask if there's anything else the user would like to discuss.",
            );
            result.replace_in_history(None);
            result.update_global_data(json!({
                "answers": answers,
                "question_index": new_question_index,
            }));
            result
        }
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
        let result = agent
            .agent()
            .on_function_call("start_questions", &args, &raw);
        assert!(result.is_some());
    }

    #[test]
    fn test_info_gatherer_default_name() {
        let agent = InfoGathererAgent::new("", sample_questions(), None);
        assert_eq!(agent.agent().service().name(), "info_gatherer");
    }

    fn global_data(questions: &[Value], index: u64, answers: Value) -> Map<String, Value> {
        let mut raw = Map::new();
        raw.insert(
            "global_data".to_string(),
            json!({
                "questions": questions,
                "question_index": index,
                "answers": answers,
            }),
        );
        raw
    }

    #[test]
    fn test_start_questions_returns_first() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        let qs = sample_questions();
        let raw = global_data(&qs, 0, json!([]));
        let json_str = agent.start_questions(&Map::new(), &raw).to_json();
        assert!(json_str.contains("What is your name?"));
        assert!(json_str.contains("don't need the user to confirm"));
    }

    #[test]
    fn test_start_questions_empty() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        let raw = global_data(&[], 0, json!([]));
        let json_str = agent.start_questions(&Map::new(), &raw).to_json();
        assert!(json_str.contains("don't have any questions"));
    }

    #[test]
    fn test_submit_answer_advances() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        let qs = sample_questions();
        let raw = global_data(&qs, 0, json!([]));
        let mut args = Map::new();
        args.insert("answer".to_string(), json!("Alice"));
        let result = agent.submit_answer(&args, &raw);
        let json_str = result.to_json();
        // Advances to the second (confirm=true) question.
        assert!(json_str.contains("What is your email?"));
        assert!(json_str.contains("Insist that the user confirms"));
        // The recorded answer + new index are pushed via update_global_data.
        assert!(json_str.contains("\"answer\":\"Alice\""));
        assert!(json_str.contains("\"key_name\":\"name\""));
    }

    #[test]
    fn test_submit_answer_completes() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        let qs = sample_questions();
        // Answer the final question (index 1 of 2).
        let raw = global_data(&qs, 1, json!([{"key_name": "name", "answer": "Alice"}]));
        let mut args = Map::new();
        args.insert("answer".to_string(), json!("a@b.com"));
        let json_str = agent.submit_answer(&args, &raw).to_json();
        assert!(json_str.contains("All questions have been answered"));
    }

    #[test]
    fn test_submit_answer_out_of_bounds() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        let qs = sample_questions();
        let raw = global_data(&qs, 5, json!([]));
        let mut args = Map::new();
        args.insert("answer".to_string(), json!("x"));
        let json_str = agent.submit_answer(&args, &raw).to_json();
        assert!(json_str.contains("already been answered"));
    }

    #[test]
    fn test_on_swml_request_static_mode_returns_none() {
        let agent = InfoGathererAgent::new("test", sample_questions(), None);
        assert!(
            agent
                .on_swml_request(None, &Map::new(), &HashMap::new())
                .is_none()
        );
    }

    #[test]
    fn test_on_swml_request_dynamic_fallback() {
        // Empty questions => dynamic mode; no callback => fallback questions.
        let agent = InfoGathererAgent::new("test", vec![], None);
        let out = agent
            .on_swml_request(None, &Map::new(), &HashMap::new())
            .expect("dynamic mode returns global_data");
        let questions = out["global_data"]["questions"].as_array().unwrap();
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0]["key_name"], "name");
    }

    #[test]
    fn test_on_swml_request_dynamic_callback() {
        let mut agent = InfoGathererAgent::new("test", vec![], None);
        agent.set_question_callback(Arc::new(|query, _body, _headers| {
            let set = query
                .get("set")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            if set == "support" {
                vec![
                    json!({"key_name": "name", "question_text": "Name?"}),
                    json!({"key_name": "issue", "question_text": "Issue?"}),
                ]
            } else {
                vec![json!({"key_name": "name", "question_text": "Name?"})]
            }
        }));

        let mut query = Map::new();
        query.insert("set".to_string(), json!("support"));
        let out = agent
            .on_swml_request(None, &query, &HashMap::new())
            .expect("dynamic mode returns global_data");
        let questions = out["global_data"]["questions"].as_array().unwrap();
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[1]["key_name"], "issue");
    }

    #[test]
    fn test_on_swml_request_dynamic_callback_empty_falls_back() {
        let mut agent = InfoGathererAgent::new("test", vec![], None);
        agent.set_question_callback(Arc::new(|_q, _b, _h| vec![]));
        let out = agent
            .on_swml_request(None, &Map::new(), &HashMap::new())
            .expect("dynamic mode returns global_data");
        let questions = out["global_data"]["questions"].as_array().unwrap();
        assert_eq!(questions.len(), 2); // fallback
    }
}
