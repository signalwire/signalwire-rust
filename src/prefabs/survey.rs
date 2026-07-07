use serde_json::{Map, Value, json};

use crate::agent::{AgentBase, AgentOptions};
use crate::prefabs::PrefabSummaryCallback;
use crate::swaig::FunctionResult;

/// A pre-built agent for conducting surveys with typed question validation.
pub struct SurveyAgent {
    agent: AgentBase,
    survey_name: String,
    survey_questions: Vec<Value>,
}

impl SurveyAgent {
    /// Create a new `SurveyAgent`.
    ///
    /// # Arguments
    /// - `name` — agent name (defaults to `"survey"` if empty).
    /// - `questions` — list of `{id, text, type, required?, scale?, choices?}` objects.
    /// - `options` — optional map with `survey_name`, `introduction`, `conclusion`,
    ///   `brand_name`, `max_retries`, `route`.
    pub fn new(
        name: &str,
        questions: Vec<Value>,
        options: Option<&serde_json::Map<String, Value>>,
    ) -> Self {
        let empty_map = serde_json::Map::new();
        let opts = options.unwrap_or(&empty_map);

        let survey_name = opts
            .get("survey_name")
            .and_then(|v| v.as_str())
            .unwrap_or(if name.is_empty() { "Survey" } else { name })
            .to_string();

        let introduction = opts
            .get("introduction")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let agent_name = if name.is_empty() { "survey" } else { name };
        let route = opts
            .get("route")
            .and_then(|v| v.as_str())
            .unwrap_or("/survey")
            .to_string();

        let mut agent_opts = AgentOptions::new(agent_name);
        agent_opts.route = Some(route);
        agent_opts.use_pom = true;

        let mut agent = AgentBase::new(agent_opts);

        // Global data
        agent.set_global_data(json!({
            "survey_name": survey_name,
            "questions": questions,
            "question_index": 0,
            "answers": {},
            "completed": false,
        }));

        // Introduction section
        let intro_text = if introduction.is_empty() {
            format!("Welcome to the {survey_name}.")
        } else {
            introduction.clone()
        };

        agent.prompt_add_section(
            "Survey Introduction",
            &intro_text,
            vec![
                "Introduce the survey to the user",
                "Ask each question in sequence",
                "Validate responses based on question type",
                "Thank the user when complete",
            ],
        );

        // Build question descriptions for prompt
        let mut q_bullets: Vec<String> = Vec::new();
        for q in &questions {
            let text = q.get("text").and_then(|v| v.as_str()).unwrap_or("?");
            let qtype = q
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("open_ended");
            let required = q
                .get("required")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let mut desc = format!("Q: {text} (type: {qtype})");
            if required {
                desc.push_str(" [required]");
            }
            q_bullets.push(desc);
        }
        let bullet_refs: Vec<&str> = q_bullets.iter().map(std::string::String::as_str).collect();
        agent.prompt_add_section("Survey Questions", "", bullet_refs);

        // Tool: validate_response
        let q_clone = questions.clone();
        agent.define_tool(
            "validate_response",
            "Validate a survey response against the question type constraints",
            json!({
                "question_id": {"type": "string", "description": "ID of the question"},
                "answer": {"type": "string", "description": "The response to validate"},
            }),
            Box::new(move |args, _raw| {
                let question_id = args
                    .get("question_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let answer = args.get("answer").and_then(|v| v.as_str()).unwrap_or("");

                // Find the question
                let question = q_clone
                    .iter()
                    .find(|q| q.get("id").and_then(|v| v.as_str()) == Some(question_id));

                let Some(question) = question else {
                    return FunctionResult::with_response(&format!(
                        "Unknown question ID: {question_id}"
                    ));
                };

                let qtype = question
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open_ended");

                match qtype {
                    "rating" => {
                        let scale = question
                            .get("scale")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or(5);
                        match answer.parse::<i64>() {
                            Ok(val) if val >= 1 && val <= scale => FunctionResult::with_response(
                                &format!("Valid rating: {val}/{scale}"),
                            ),
                            _ => FunctionResult::with_response(&format!(
                                "Invalid rating. Please provide a number between 1 and {scale}."
                            )),
                        }
                    }
                    "multiple_choice" => {
                        let choices = question
                            .get("choices")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let lower_answer = answer.trim().to_lowercase();
                        for choice in &choices {
                            if let Some(c) = choice.as_str()
                                && c.trim().to_lowercase() == lower_answer
                            {
                                return FunctionResult::with_response(&format!(
                                    "Valid choice: {c}"
                                ));
                            }
                        }
                        let choice_list: Vec<&str> =
                            choices.iter().filter_map(|v| v.as_str()).collect();
                        FunctionResult::with_response(&format!(
                            "Invalid choice. Valid options are: {}",
                            choice_list.join(", ")
                        ))
                    }
                    "yes_no" => {
                        let normalized = answer.trim().to_lowercase();
                        if ["yes", "no", "y", "n"].contains(&normalized.as_str()) {
                            FunctionResult::with_response(&format!("Valid response: {normalized}"))
                        } else {
                            FunctionResult::with_response("Please respond with yes or no.")
                        }
                    }
                    _ => {
                        // open_ended
                        if answer.trim().is_empty() {
                            FunctionResult::with_response("Please provide a non-empty response.")
                        } else {
                            FunctionResult::with_response(&format!("Response accepted: {answer}"))
                        }
                    }
                }
            }),
            false,
        );

        // Tool: log_response
        agent.define_tool(
            "log_response",
            "Log a validated survey response",
            json!({
                "question_id": {"type": "string", "description": "ID of the question"},
                "answer": {"type": "string", "description": "The validated answer"},
            }),
            Box::new(|args, _raw| {
                let question_id = args
                    .get("question_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let answer = args.get("answer").and_then(|v| v.as_str()).unwrap_or("");
                FunctionResult::with_response(&format!("Survey answer for {question_id}: {answer}"))
            }),
            false,
        );

        SurveyAgent {
            agent,
            survey_name,
            survey_questions: questions,
        }
    }

    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    pub fn survey_name(&self) -> &str {
        &self.survey_name
    }

    pub fn survey_questions(&self) -> &[Value] {
        &self.survey_questions
    }

    /// Validate whether a response meets the requirements for a question.
    ///
    /// Ported from Python `SurveyAgent.validate_response`. Reads `question_id` and
    /// `response`; validates per the question's `type` (rating within its `scale`,
    /// `multiple_choice` against its `options`, `yes_no`, and non-empty for a
    /// required `open_ended`). `raw_data` is accepted for handler-signature compatibility
    /// but unused.
    pub fn validate_response(
        &self,
        args: &Map<String, Value>,
        _raw_data: &Map<String, Value>,
    ) -> FunctionResult {
        let question_id = args
            .get("question_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let response = args.get("response").and_then(|v| v.as_str()).unwrap_or("");

        let question = self
            .survey_questions
            .iter()
            .find(|q| q.get("id").and_then(|v| v.as_str()) == Some(question_id));

        let Some(question) = question else {
            return FunctionResult::with_response(&format!(
                "Error: Question with ID '{question_id}' not found."
            ));
        };

        let qtype = question.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let mut message = format!("Response to '{question_id}' is valid.");

        match qtype {
            "rating" => {
                let scale = question
                    .get("scale")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(5);
                match response.trim().parse::<i64>() {
                    Ok(rating) if rating >= 1 && rating <= scale => {}
                    _ => {
                        message = format!(
                            "Invalid rating. Please provide a number between 1 and {scale}."
                        );
                    }
                }
            }
            "multiple_choice" => {
                let options: Vec<String> = question
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|o| o.as_str().map(str::to_lowercase))
                            .collect()
                    })
                    .unwrap_or_default();
                let normalized = response.trim().to_lowercase();
                if !options.iter().any(|o| o == &normalized) {
                    let display: Vec<&str> = question
                        .get("options")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|o| o.as_str()).collect())
                        .unwrap_or_default();
                    message = format!(
                        "Invalid choice. Please select one of: {}.",
                        display.join(", ")
                    );
                }
            }
            "yes_no" => {
                let normalized = response.trim().to_lowercase();
                if !["yes", "no", "y", "n"].contains(&normalized.as_str()) {
                    message = "Please answer with 'yes' or 'no'.".to_string();
                }
            }
            "open_ended" => {
                let required = question
                    .get("required")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
                if response.trim().is_empty() && required {
                    message = "A response is required for this question.".to_string();
                }
            }
            _ => {}
        }

        FunctionResult::with_response(&message)
    }

    /// Log a validated response to a survey question.
    ///
    /// Ported from Python `SurveyAgent.log_response`. Acknowledges the response by
    /// the question's text. Reads `question_id`; `raw_data` is accepted for
    /// handler-signature compatibility but unused.
    pub fn log_response(
        &self,
        args: &Map<String, Value>,
        _raw_data: &Map<String, Value>,
    ) -> FunctionResult {
        let question_id = args
            .get("question_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let question_text = self
            .survey_questions
            .iter()
            .find(|q| q.get("id").and_then(|v| v.as_str()) == Some(question_id))
            .and_then(|q| q.get("text").and_then(|v| v.as_str()))
            .unwrap_or("");

        FunctionResult::with_response(&format!("Response to '{question_text}' has been recorded."))
    }

    /// Register a callback that processes the survey-results summary.
    ///
    /// Delegates to [`AgentBase::on_summary`], matching the Python
    /// `SurveyAgent.on_summary` override point (which logs the summary).
    pub fn on_summary(&mut self, callback: PrefabSummaryCallback) -> &mut Self {
        self.agent.on_summary(callback);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_questions() -> Vec<Value> {
        vec![
            json!({"id": "q1", "text": "Rate our service", "type": "rating", "scale": 5, "required": true}),
            json!({"id": "q2", "text": "Would you recommend us?", "type": "yes_no"}),
            json!({"id": "q3", "text": "Choose a color", "type": "multiple_choice", "choices": ["Red", "Blue", "Green"]}),
        ]
    }

    #[test]
    fn test_survey_construction() {
        let agent = SurveyAgent::new("test_survey", sample_questions(), None);
        assert_eq!(agent.agent().service().name(), "test_survey");
        assert_eq!(agent.agent().service().route(), "/survey");
        assert_eq!(agent.survey_questions().len(), 3);
        assert_eq!(agent.survey_name(), "test_survey");
    }

    #[test]
    fn test_survey_has_tools() {
        let agent = SurveyAgent::new("test", sample_questions(), None);
        let args = serde_json::Map::new();
        let raw = serde_json::Map::new();
        let result = agent
            .agent()
            .on_function_call("validate_response", &args, &raw);
        assert!(result.is_some());
    }

    #[test]
    fn test_survey_validate_rating() {
        let agent = SurveyAgent::new("test", sample_questions(), None);
        let mut args = serde_json::Map::new();
        args.insert("question_id".to_string(), json!("q1"));
        args.insert("answer".to_string(), json!("3"));
        let result =
            agent
                .agent()
                .on_function_call("validate_response", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("Valid rating"));
    }

    #[test]
    fn test_survey_validate_yes_no() {
        let agent = SurveyAgent::new("test", sample_questions(), None);
        let mut args = serde_json::Map::new();
        args.insert("question_id".to_string(), json!("q2"));
        args.insert("answer".to_string(), json!("yes"));
        let result =
            agent
                .agent()
                .on_function_call("validate_response", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("Valid response"));
    }

    // Python-faithful question set: multiple_choice uses `options`, and
    // validate_response/log_response read `response`/`question_id`.
    fn py_questions() -> Vec<Value> {
        vec![
            json!({"id": "q1", "text": "Rate our service", "type": "rating", "scale": 5}),
            json!({"id": "q2", "text": "Recommend us?", "type": "yes_no"}),
            json!({"id": "q3", "text": "Pick a color", "type": "multiple_choice", "options": ["Red", "Blue"]}),
            json!({"id": "q4", "text": "Comments?", "type": "open_ended", "required": true}),
        ]
    }

    #[test]
    fn test_validate_response_rating() {
        let agent = SurveyAgent::new("test", py_questions(), None);
        let raw = Map::new();
        let mut ok = Map::new();
        ok.insert("question_id".to_string(), json!("q1"));
        ok.insert("response".to_string(), json!("4"));
        assert!(
            agent
                .validate_response(&ok, &raw)
                .to_json()
                .contains("is valid")
        );

        let mut bad = Map::new();
        bad.insert("question_id".to_string(), json!("q1"));
        bad.insert("response".to_string(), json!("9"));
        assert!(
            agent
                .validate_response(&bad, &raw)
                .to_json()
                .contains("Invalid rating")
        );
    }

    #[test]
    fn test_validate_response_multiple_choice() {
        let agent = SurveyAgent::new("test", py_questions(), None);
        let raw = Map::new();
        let mut ok = Map::new();
        ok.insert("question_id".to_string(), json!("q3"));
        ok.insert("response".to_string(), json!("blue"));
        assert!(
            agent
                .validate_response(&ok, &raw)
                .to_json()
                .contains("is valid")
        );

        let mut bad = Map::new();
        bad.insert("question_id".to_string(), json!("q3"));
        bad.insert("response".to_string(), json!("purple"));
        assert!(
            agent
                .validate_response(&bad, &raw)
                .to_json()
                .contains("Invalid choice")
        );
    }

    #[test]
    fn test_validate_response_open_ended_required() {
        let agent = SurveyAgent::new("test", py_questions(), None);
        let raw = Map::new();
        let mut empty = Map::new();
        empty.insert("question_id".to_string(), json!("q4"));
        empty.insert("response".to_string(), json!("  "));
        assert!(
            agent
                .validate_response(&empty, &raw)
                .to_json()
                .contains("A response is required")
        );
    }

    #[test]
    fn test_validate_response_unknown_id() {
        let agent = SurveyAgent::new("test", py_questions(), None);
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("question_id".to_string(), json!("nope"));
        args.insert("response".to_string(), json!("x"));
        assert!(
            agent
                .validate_response(&args, &raw)
                .to_json()
                .contains("not found")
        );
    }

    #[test]
    fn test_log_response() {
        let agent = SurveyAgent::new("test", py_questions(), None);
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("question_id".to_string(), json!("q1"));
        args.insert("response".to_string(), json!("4"));
        let json_str = agent.log_response(&args, &raw).to_json();
        assert!(json_str.contains("Rate our service"));
        assert!(json_str.contains("has been recorded"));
    }

    #[test]
    fn test_survey_on_summary_fires() {
        use std::sync::{Arc, Mutex};

        let mut agent = SurveyAgent::new("test", py_questions(), None);
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

        let body = json!({"summary": "Survey done"});
        let (status, _, _) = agent.agent_mut().handle_request(
            "POST",
            "/survey/post_prompt",
            &headers,
            &body.to_string(),
        );
        assert_eq!(status, 200);
        assert_eq!(*captured.lock().unwrap(), "Survey done");
    }
}
