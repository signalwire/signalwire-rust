use serde_json::{Map, Value, json};

use crate::agent::{AgentBase, AgentOptions};
use crate::prefabs::PrefabSummaryCallback;
use crate::swaig::FunctionResult;

/// A pre-built agent for conducting surveys with typed question validation.
pub struct SurveyAgent {
    agent: AgentBase,
    survey_name: String,
    survey_questions: Vec<Value>,
    /// Resolved brand name — the caller's value or the default
    /// `"Our Company"` (`survey.py:93`). Retained so a caller can read back
    /// what the agent is actually representing; the reference keeps it as
    /// `self.brand_name`.
    brand_name: String,
    /// Resolved retry budget (`survey.py:94`).
    max_retries: i64,
    /// Resolved introduction text (`survey.py:97-100`).
    introduction: String,
    /// Resolved conclusion text (`survey.py:101-103`).
    conclusion: String,
}

/// Options for constructing a [`SurveyAgent`].
///
/// `survey_name` and `questions` are the REQUIRED
/// positionals, so they are the arguments to [`SurveyOptions::new`]; every
/// other field carries the default.
#[must_use]
pub struct SurveyOptions {
    /// Display name of the survey.
    pub survey_name: String,
    /// Questions, each `{id, text, type, required?, scale?, choices?}`.
    pub questions: Vec<Value>,
    /// Optional introduction text; `None` uses a generated welcome line.
    pub introduction: Option<String>,
    /// Optional closing text.
    pub conclusion: Option<String>,
    /// Optional brand name to reference in the prompt.
    pub brand_name: Option<String>,
    /// Re-ask attempts per question (default `2`).
    pub max_retries: i64,
    /// Agent name (default `"survey"`).
    pub name: String,
    /// HTTP route (default `"/survey"`).
    pub route: String,
}

impl SurveyOptions {
    /// Options for the reference's two required positionals, with every other
    /// field at its default — the port of
    /// `SurveyAgent(survey_name, questions)`.
    ///
    /// There is deliberately **no** `Default` impl and no zero-argument
    /// constructor: `survey_name` is a bare `str` positional in the reference
    /// (`survey.py:57`), so omitting it must not compile here either. A caller
    /// who cannot name the survey has no valid `SurveyOptions` to build.
    pub fn new(survey_name: &str, questions: Vec<Value>) -> Self {
        SurveyOptions {
            survey_name: survey_name.to_string(),
            questions,
            introduction: None,
            conclusion: None,
            brand_name: None,
            max_retries: 2,
            name: "survey".to_string(),
            route: "/survey".to_string(),
        }
    }

    /// Replace the survey's display name.
    pub fn survey_name(mut self, survey_name: &str) -> Self {
        self.survey_name = survey_name.to_string();
        self
    }

    /// Set the survey questions.
    pub fn questions(mut self, questions: Vec<Value>) -> Self {
        self.questions = questions;
        self
    }

    /// Set the introduction text.
    pub fn introduction(mut self, introduction: &str) -> Self {
        self.introduction = Some(introduction.to_string());
        self
    }

    /// Set the conclusion text.
    pub fn conclusion(mut self, conclusion: &str) -> Self {
        self.conclusion = Some(conclusion.to_string());
        self
    }

    /// Set the brand name referenced in the prompt.
    pub fn brand_name(mut self, brand_name: &str) -> Self {
        self.brand_name = Some(brand_name.to_string());
        self
    }

    /// Set the per-question retry budget (default `2`).
    pub fn max_retries(mut self, max_retries: i64) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the agent name (default `"survey"`).
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the HTTP route (default `"/survey"`).
    pub fn route(mut self, route: &str) -> Self {
        self.route = route.to_string();
        self
    }
}

impl SurveyAgent {
    /// Create a new `SurveyAgent` from [`SurveyOptions`].
    ///
    /// `SurveyAgent::new(SurveyOptions::new(survey_name, questions))` ports
    /// the reference's minimal `SurveyAgent(survey_name, questions)`.
    pub fn new(options: SurveyOptions) -> Self {
        let SurveyOptions {
            survey_name,
            questions,
            introduction,
            conclusion,
            brand_name,
            max_retries,
            name,
            route,
        } = options;

        // Reference defaults for the optional text params (`survey.py:93-103`).
        let brand_name = brand_name.unwrap_or_else(|| "Our Company".to_string());
        let conclusion = conclusion.unwrap_or_else(|| {
            "Thank you for completing our survey. Your feedback is valuable to us.".to_string()
        });
        // The reference resolves the introduction ONCE, at construction, and the
        // resolved value is both what it renders and what `self.introduction`
        // reads back (`survey.py:97-100`). Resolving it here rather than at the
        // render site keeps the reader and the prompt in agreement.
        let introduction = introduction.unwrap_or_else(|| {
            format!("Welcome to our {survey_name}. We appreciate your participation.")
        });

        let agent_name = if name.is_empty() { "survey" } else { &name };

        let mut agent_opts = AgentOptions::new(agent_name);
        agent_opts.route = Some(if route.is_empty() {
            "/survey".to_string()
        } else {
            route
        });
        agent_opts.use_pom = true;

        let mut agent = AgentBase::new(agent_opts);

        // Global data. `brand_name` and `max_retries` are exposed to the AI
        // here exactly as the reference does (`survey.py:242-248`).
        agent.set_global_data(json!({
            "survey_name": survey_name,
            "brand_name": brand_name,
            "questions": questions,
            "max_retries": max_retries,
            "question_index": 0,
            "answers": {},
            "completed": false,
        }));

        // Personality section names the brand (reference `survey.py:147-150`).
        agent.prompt_add_section(
            "Personality",
            &format!("You are a friendly and professional survey agent representing {brand_name}."),
            vec![],
        );

        // Introduction section
        let intro_text = introduction.clone();

        let retry_instruction =
            format!("If a response is invalid, explain and retry up to {max_retries} times.");
        agent.prompt_add_section(
            "Survey Introduction",
            &intro_text,
            vec![
                "Introduce the survey to the user",
                "Ask each question in sequence",
                "Validate responses based on question type",
                &retry_instruction,
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

        // Conclusion section (reference `survey.py:196-199`).
        agent.prompt_add_section(
            "Conclusion",
            &format!("End with this conclusion: {conclusion}"),
            vec![],
        );

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
            brand_name,
            max_retries,
            introduction,
            conclusion,
        }
    }

    /// Borrow the underlying [`AgentBase`] this prefab wraps.
    ///
    /// `SurveyAgent` composes an agent rather than inheriting from one, so
    /// this is how you read the configured prompt, tools, and skills.
    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    /// Mutably borrow the underlying [`AgentBase`].
    ///
    /// Use this to layer extra configuration — additional tools, skills,
    /// hints, or verbs — on top of what the prefab already set up.
    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    /// The survey's name, as configured. Woven into the agent's prompt and
    /// used to label the collected responses.
    pub fn survey_name(&self) -> &str {
        &self.survey_name
    }

    /// The survey's questions, as configured. Mirrors the reference's
    /// `self.questions` (`survey.py:92`).
    pub fn questions(&self) -> &[Value] {
        &self.survey_questions
    }

    /// The brand the agent represents — the caller's `brand_name` or the
    /// default `"Our Company"` (`survey.py:93`).
    pub fn brand_name(&self) -> &str {
        &self.brand_name
    }

    /// How many times an invalid answer is re-asked (`survey.py:94`).
    pub fn max_retries(&self) -> i64 {
        self.max_retries
    }

    /// The resolved introduction text (`survey.py:97-100`).
    pub fn introduction(&self) -> &str {
        &self.introduction
    }

    /// The resolved conclusion text (`survey.py:101-103`).
    pub fn conclusion(&self) -> &str {
        &self.conclusion
    }

    /// Validate whether a response meets the requirements for a question.
    ///
    /// Reads `question_id` and
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
    /// Acknowledges the response by
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

    /// `survey_name` is a bare `str` positional in the reference
    /// (`survey.py:57`) — genuinely REQUIRED. The port previously shipped an
    /// `impl Default for SurveyOptions` seeding it with the invented literal
    /// `"Survey"`, so a caller who never named the survey silently got an
    /// agent literally called "Survey" instead of the compile error the
    /// reference and every other port give them.
    ///
    /// `SurveyOptions::new` is now the ONLY constructor and it takes
    /// `survey_name`, so omitting it does not compile. The compile-time half of
    /// that guarantee is enforced by the build itself (there is no
    /// zero-argument path to reach); this test is the runtime half: whatever
    /// the caller passes is the value the agent uses EVERYWHERE it surfaces,
    /// with no fallback literal reachable on any of those paths.
    #[test]
    fn test_survey_name_is_required_and_the_callers_value_is_used_throughout() {
        // A name that could not possibly be produced by a default.
        let agent = SurveyAgent::new(SurveyOptions::new(
            "Q3 Onboarding Experience",
            sample_questions(),
        ));

        // Readback.
        assert_eq!(agent.survey_name(), "Q3 Onboarding Experience");

        // Global data — what the AI actually sees.
        assert_eq!(
            agent.agent().get_global_data()["survey_name"],
            "Q3 Onboarding Experience"
        );

        // The derived introduction is built FROM survey_name, so a leaked
        // default would show up here even if the field itself were right.
        assert_eq!(
            agent.introduction(),
            "Welcome to our Q3 Onboarding Experience. We appreciate your participation."
        );

        // And nothing anywhere carries the old invented default.
        let prompt = agent.agent().get_prompt().to_string();
        let global = agent.agent().get_global_data().to_string();
        assert!(
            !prompt.contains("Welcome to our Survey."),
            "invented default survey_name leaked into the prompt: {prompt}"
        );
        assert!(
            !global.contains("\"survey_name\":\"Survey\""),
            "invented default survey_name leaked into global data: {global}"
        );
    }

    #[test]
    fn test_survey_construction() {
        let agent = SurveyAgent::new(
            SurveyOptions::new("test_survey", sample_questions()).name("test_survey"),
        );
        assert_eq!(agent.agent().service().name(), "test_survey");
        assert_eq!(agent.agent().service().route(), "/survey");
        assert_eq!(agent.questions().len(), 3);
        assert_eq!(agent.survey_name(), "test_survey");
    }

    #[test]
    fn test_survey_config_is_retained_and_rendered() {
        let agent = SurveyAgent::new(
            SurveyOptions::new("test_survey", sample_questions())
                .name("test")
                .brand_name("Acme")
                .max_retries(5)
                .introduction("Hi there")
                .conclusion("All done"),
        );
        // READBACK: the reference keeps all four (`survey.py:92-103`); the port
        // consumed each into prompt text and kept none.
        assert_eq!(agent.brand_name(), "Acme");
        assert_eq!(agent.max_retries(), 5);
        assert_eq!(agent.introduction(), "Hi there");
        assert_eq!(agent.conclusion(), "All done");

        // And they reach the rendered prompt / global data, not just the fields.
        let prompt = agent.agent().get_prompt().to_string();
        assert!(prompt.contains("Acme"), "brand_name missing: {prompt}");
        assert!(
            prompt.contains("Hi there"),
            "introduction missing: {prompt}"
        );
        assert!(prompt.contains("All done"), "conclusion missing: {prompt}");
        assert!(
            prompt.contains("retry up to 5 times"),
            "max_retries missing: {prompt}"
        );
        let gd = agent.agent().get_global_data();
        assert_eq!(gd["brand_name"], "Acme");
        assert_eq!(gd["max_retries"], 5);
    }

    #[test]
    fn test_survey_defaults_resolve_once_like_the_reference() {
        // The reference resolves each default IN the constructor, so the value it
        // renders is the value it reads back (`survey.py:93-103`). The port used
        // to leave `introduction` empty and substitute DIFFERENT text at the
        // render site, so reader and prompt disagreed.
        let agent =
            SurveyAgent::new(SurveyOptions::new("Customer Poll", sample_questions()).name("test"));
        assert_eq!(agent.brand_name(), "Our Company");
        assert_eq!(agent.max_retries(), 2);
        assert_eq!(
            agent.introduction(),
            "Welcome to our Customer Poll. We appreciate your participation."
        );
        let prompt = agent.agent().get_prompt().to_string();
        assert!(
            prompt.contains(agent.introduction()),
            "the rendered introduction is not the one the reader returns: {prompt}"
        );
    }

    #[test]
    fn test_survey_has_tools() {
        let agent = SurveyAgent::new(SurveyOptions::new("test", sample_questions()).name("test"));
        let args = serde_json::Map::new();
        let raw = serde_json::Map::new();
        let result = agent
            .agent()
            .on_function_call("validate_response", &args, Some(&raw));
        assert!(result.is_some());
    }

    #[test]
    fn test_survey_validate_rating() {
        let agent = SurveyAgent::new(SurveyOptions::new("test", sample_questions()).name("test"));
        let mut args = serde_json::Map::new();
        args.insert("question_id".to_string(), json!("q1"));
        args.insert("answer".to_string(), json!("3"));
        let result = agent.agent().on_function_call(
            "validate_response",
            &args,
            Some(&serde_json::Map::new()),
        );
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("Valid rating"));
    }

    #[test]
    fn test_survey_validate_yes_no() {
        let agent = SurveyAgent::new(SurveyOptions::new("test", sample_questions()).name("test"));
        let mut args = serde_json::Map::new();
        args.insert("question_id".to_string(), json!("q2"));
        args.insert("answer".to_string(), json!("yes"));
        let result = agent.agent().on_function_call(
            "validate_response",
            &args,
            Some(&serde_json::Map::new()),
        );
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
        let agent = SurveyAgent::new(SurveyOptions::new("test", py_questions()).name("test"));
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
        let agent = SurveyAgent::new(SurveyOptions::new("test", py_questions()).name("test"));
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
        let agent = SurveyAgent::new(SurveyOptions::new("test", py_questions()).name("test"));
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
        let agent = SurveyAgent::new(SurveyOptions::new("test", py_questions()).name("test"));
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
        let agent = SurveyAgent::new(SurveyOptions::new("test", py_questions()).name("test"));
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

        let mut agent = SurveyAgent::new(SurveyOptions::new("test", py_questions()).name("test"));
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
            Some(&body.to_string()),
        );
        assert_eq!(status, 200);
        assert_eq!(*captured.lock().unwrap(), "Survey done");
    }
}
