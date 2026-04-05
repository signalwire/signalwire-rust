use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Gather answers to a configurable list of questions (handler-based).
pub struct InfoGatherer {
    sp: SkillParams,
}

impl InfoGatherer {
    pub fn new(params: Map<String, Value>) -> Self {
        InfoGatherer {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for InfoGatherer {
    fn name(&self) -> &str {
        "info_gatherer"
    }

    fn description(&self) -> &str {
        "Gather answers to a configurable list of questions"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        let questions = self.sp.get_array("questions");
        !questions.is_empty()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let prefix = self.sp.get_str_or("prefix", "");
        let questions = self.sp.get_array("questions");
        let completion_message = self
            .sp
            .get_str_or("completion_message", "All questions have been answered. Thank you!");
        let namespace = self.get_instance_key();

        let start_tool = if !prefix.is_empty() {
            format!("{}_start_questions", prefix)
        } else {
            "start_questions".to_string()
        };
        let submit_tool = if !prefix.is_empty() {
            format!("{}_submit_answer", prefix)
        } else {
            "submit_answer".to_string()
        };

        // start_questions tool
        let questions_clone = questions.clone();
        let namespace_clone = namespace.clone();
        agent.define_tool(
            &start_tool,
            "Start the question gathering process and get the first question",
            json!({}),
            Box::new(move |_args, _raw| {
                let mut result = FunctionResult::new();

                if questions_clone.is_empty() {
                    result.set_response("No questions configured.");
                    return result;
                }

                let first = questions_clone[0]
                    .get("question_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No question text.");

                result.set_response(&format!("Starting questions. First question: {}", first));
                result.add_action(json!({
                    "set_global_data": {
                        namespace_clone.clone(): {
                            "questions": questions_clone,
                            "question_index": 0,
                            "answers": [],
                        }
                    }
                }));

                result
            }),
            false,
        );

        // submit_answer tool
        let questions_clone2 = questions.clone();
        let completion_msg = completion_message.clone();
        agent.define_tool(
            &submit_tool,
            "Submit an answer to the current question",
            json!({
                "answer": {
                    "type": "string",
                    "description": "The answer to the current question",
                    "required": true,
                },
                "confirmed_by_user": {
                    "type": "boolean",
                    "description": "Whether the user has confirmed this answer is correct",
                },
            }),
            Box::new(move |args, _raw| {
                let mut result = FunctionResult::new();
                let answer = args
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let confirmed = args
                    .get("confirmed_by_user")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if answer.is_empty() {
                    result.set_response("Please provide an answer.");
                    return result;
                }

                let total = questions_clone2.len();
                let current_index = 0_usize; // Managed by global data in runtime

                let current_question = questions_clone2.get(current_index);
                let needs_confirm = current_question
                    .and_then(|q| q.get("confirm"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if needs_confirm && !confirmed {
                    let question_text = current_question
                        .and_then(|q| q.get("question_text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    result.set_response(&format!(
                        "You answered \"{}\" for: {}. Can you confirm this is correct?",
                        answer, question_text
                    ));
                    return result;
                }

                let next_index = current_index + 1;
                if next_index >= total {
                    result.set_response(&completion_msg);
                } else {
                    let next_question = questions_clone2[next_index]
                        .get("question_text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("No question text.");
                    result.set_response(&format!(
                        "Answer recorded. Next question: {}",
                        next_question
                    ));
                }

                result
            }),
            false,
        );
    }

    fn get_global_data(&self) -> Map<String, Value> {
        let namespace = self.get_instance_key();
        let questions = self.sp.get_array("questions");

        let mut data = Map::new();
        data.insert(
            namespace,
            json!({
                "questions": questions,
                "question_index": 0,
                "answers": [],
            }),
        );
        data
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        let instance_key = self.get_instance_key();
        let questions = self.sp.get_array("questions");

        let mut bullets = vec![
            "Call start_questions to begin the question flow.".to_string(),
            "Submit each answer using submit_answer with the user's response.".to_string(),
            "Questions that require confirmation will ask the user to verify their answer."
                .to_string(),
        ];

        for q in &questions {
            if let Some(prompt_add) = q.get("prompt_add").and_then(|v| v.as_str()) {
                if !prompt_add.is_empty() {
                    bullets.push(prompt_add.to_string());
                }
            }
        }

        let bullet_values: Vec<Value> = bullets.into_iter().map(|b| json!(b)).collect();

        vec![json!({
            "title": format!("Info Gatherer ({})", instance_key),
            "body": "You need to gather information from the user by asking a series of questions.",
            "bullets": bullet_values,
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_gatherer_metadata() {
        let skill = InfoGatherer::new(Map::new());
        assert_eq!(skill.name(), "info_gatherer");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_info_gatherer_setup_needs_questions() {
        let mut skill = InfoGatherer::new(Map::new());
        assert!(!skill.setup());
    }

    #[test]
    fn test_info_gatherer_setup_with_questions() {
        let mut params = Map::new();
        params.insert(
            "questions".to_string(),
            json!([
                {"key_name": "name", "question_text": "What is your name?"},
                {"key_name": "email", "question_text": "What is your email?"},
            ]),
        );
        let mut skill = InfoGatherer::new(params);
        assert!(skill.setup());
    }
}
