use serde_json::{json, Value};

use crate::agent::{AgentBase, AgentOptions};
use crate::swaig::FunctionResult;

/// A pre-built agent for conducting surveys with typed question validation.
pub struct SurveyAgent {
    agent: AgentBase,
    survey_name: String,
    survey_questions: Vec<Value>,
}

impl SurveyAgent {
    /// Create a new SurveyAgent.
    ///
    /// # Arguments
    /// - `name` — agent name (defaults to `"survey"` if empty).
    /// - `questions` — list of `{id, text, type, required?, scale?, choices?}` objects.
    /// - `options` — optional map with `survey_name`, `introduction`, `conclusion`,
    ///               `brand_name`, `max_retries`, `route`.
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
        let intro_text = if !introduction.is_empty() {
            introduction.clone()
        } else {
            format!("Welcome to the {}.", survey_name)
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
            let qtype = q.get("type").and_then(|v| v.as_str()).unwrap_or("open_ended");
            let required = q
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mut desc = format!("Q: {} (type: {})", text, qtype);
            if required {
                desc.push_str(" [required]");
            }
            q_bullets.push(desc);
        }
        let bullet_refs: Vec<&str> = q_bullets.iter().map(|s| s.as_str()).collect();
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
                let answer = args
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Find the question
                let question = q_clone.iter().find(|q| {
                    q.get("id").and_then(|v| v.as_str()) == Some(question_id)
                });

                let question = match question {
                    Some(q) => q,
                    None => {
                        return FunctionResult::with_response(&format!(
                            "Unknown question ID: {}",
                            question_id
                        ));
                    }
                };

                let qtype = question
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open_ended");

                match qtype {
                    "rating" => {
                        let scale = question
                            .get("scale")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(5);
                        match answer.parse::<i64>() {
                            Ok(val) if val >= 1 && val <= scale => {
                                FunctionResult::with_response(&format!(
                                    "Valid rating: {}/{}",
                                    val, scale
                                ))
                            }
                            _ => FunctionResult::with_response(&format!(
                                "Invalid rating. Please provide a number between 1 and {}.",
                                scale
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
                            if let Some(c) = choice.as_str() {
                                if c.trim().to_lowercase() == lower_answer {
                                    return FunctionResult::with_response(&format!(
                                        "Valid choice: {}",
                                        c
                                    ));
                                }
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
                            FunctionResult::with_response(&format!(
                                "Valid response: {}",
                                normalized
                            ))
                        } else {
                            FunctionResult::with_response("Please respond with yes or no.")
                        }
                    }
                    _ => {
                        // open_ended
                        if answer.trim().is_empty() {
                            FunctionResult::with_response("Please provide a non-empty response.")
                        } else {
                            FunctionResult::with_response(&format!(
                                "Response accepted: {}",
                                answer
                            ))
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
                let answer = args
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                FunctionResult::with_response(&format!(
                    "Survey answer for {}: {}",
                    question_id, answer
                ))
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
        let result = agent.agent().on_function_call("validate_response", &args, &raw);
        assert!(result.is_some());
    }

    #[test]
    fn test_survey_validate_rating() {
        let agent = SurveyAgent::new("test", sample_questions(), None);
        let mut args = serde_json::Map::new();
        args.insert("question_id".to_string(), json!("q1"));
        args.insert("answer".to_string(), json!("3"));
        let result = agent
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
        let result = agent
            .agent()
            .on_function_call("validate_response", &args, &serde_json::Map::new());
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("Valid response"));
    }
}
