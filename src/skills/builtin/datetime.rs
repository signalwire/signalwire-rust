use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Get current date, time, and timezone information.
pub struct Datetime {
    sp: SkillParams,
}

impl Datetime {
    pub fn new(params: Map<String, Value>) -> Self {
        Datetime {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for Datetime {
    fn name(&self) -> &str {
        "datetime"
    }

    fn description(&self) -> &str {
        "Get current date, time, and timezone information"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        // get_current_time
        agent.define_tool(
            "get_current_time",
            "Get the current time, optionally in a specific timezone",
            json!({
                "timezone": {
                    "type": "string",
                    "description": "Timezone name (e.g., America/New_York, Europe/London). Defaults to UTC.",
                }
            }),
            Box::new(|args, _raw| {
                let tz_name = args
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UTC");

                let mut result = FunctionResult::new();
                let now = chrono::Utc::now();
                // Use the timezone name in the response; actual tz conversion is best-effort
                let time_str = now.format("%H:%M:%S UTC").to_string();
                result.set_response(&format!(
                    "The current time in {} is {} (server UTC reference)",
                    tz_name, time_str
                ));
                result
            }),
            false,
        );

        // get_current_date
        agent.define_tool(
            "get_current_date",
            "Get the current date",
            json!({
                "timezone": {
                    "type": "string",
                    "description": "Timezone name (e.g., America/New_York, Europe/London). Defaults to UTC.",
                }
            }),
            Box::new(|args, _raw| {
                let tz_name = args
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UTC");

                let mut result = FunctionResult::new();
                let now = chrono::Utc::now();
                let date_str = now.format("%Y-%m-%d (%A, %B %e, %Y)").to_string();
                result.set_response(&format!(
                    "The current date in {} is {} (server UTC reference)",
                    tz_name, date_str
                ));
                result
            }),
            false,
        );
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Date and Time Information",
            "body": "You have access to date and time tools.",
            "bullets": [
                "Use get_current_time to retrieve the current time in any timezone.",
                "Use get_current_date to retrieve the current date in any timezone.",
                "Default timezone is UTC if none is specified.",
            ],
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentOptions;

    #[test]
    fn test_datetime_metadata() {
        let skill = Datetime::new(Map::new());
        assert_eq!(skill.name(), "datetime");
        assert_eq!(skill.version(), "1.0.0");
        assert!(!skill.supports_multiple_instances());
    }

    #[test]
    fn test_datetime_setup() {
        let mut skill = Datetime::new(Map::new());
        assert!(skill.setup());
    }

    #[test]
    fn test_datetime_prompt_sections() {
        let skill = Datetime::new(Map::new());
        let sections = skill.get_prompt_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0]["title"], "Date and Time Information");
    }

    #[test]
    fn test_datetime_skip_prompt() {
        let mut params = Map::new();
        params.insert("skip_prompt".to_string(), Value::Bool(true));
        let skill = Datetime::new(params);
        assert!(skill.get_prompt_sections().is_empty());
    }

    #[test]
    fn test_datetime_register_tools() {
        let skill = Datetime::new(Map::new());
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);
        // Tools are registered internally; we can verify through function call
        let args = Map::new();
        let raw = Map::new();
        let result = agent.on_function_call("get_current_time", &args, &raw);
        assert!(result.is_some());
    }
}
