use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Control background file playback (DataMap-based).
pub struct PlayBackgroundFile {
    sp: SkillParams,
}

impl PlayBackgroundFile {
    pub fn new(params: Map<String, Value>) -> Self {
        PlayBackgroundFile {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for PlayBackgroundFile {
    fn name(&self) -> &str {
        "play_background_file"
    }

    fn description(&self) -> &str {
        "Control background file playback"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        let files = self.sp.get_array("files");
        !files.is_empty()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("play_background_file");
        let files = self.sp.get_array("files");

        let mut action_enum = Vec::new();
        let mut expressions = Vec::new();

        for file in &files {
            let key = file.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let url = file.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let description = file
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(key);
            let wait = file
                .get("wait")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if key.is_empty() || url.is_empty() {
                continue;
            }

            let action_key = if wait {
                "play_background_file_wait"
            } else {
                "play_background_file"
            };

            action_enum.push(format!("start_{}", key));

            expressions.push(json!({
                "string": "${args.action}",
                "pattern": format!("start_{}", key),
                "output": {
                    "response": format!("Now playing: {}", description),
                    "action": [{action_key: url}],
                },
            }));
        }

        action_enum.push("stop".to_string());
        expressions.push(json!({
            "string": "${args.action}",
            "pattern": "stop",
            "output": {
                "response": "Stopping background playback.",
                "action": [{"stop_background_file": true}],
            },
        }));

        let action_enum_values: Vec<Value> = action_enum.iter().map(|s| json!(s)).collect();

        let mut func_def = json!({
            "function": tool_name,
            "purpose": format!("Control background file playback for {}", tool_name),
            "argument": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "The playback action to perform",
                        "enum": action_enum_values,
                    }
                },
                "required": ["action"],
            },
            "data_map": {
                "expressions": expressions,
            },
        });

        let swaig_fields = self.get_swaig_fields();
        if let Value::Object(ref mut obj) = func_def {
            for (k, v) in swaig_fields {
                obj.insert(k, v);
            }
        }

        agent.register_swaig_function(func_def);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_play_background_file_metadata() {
        let skill = PlayBackgroundFile::new(Map::new());
        assert_eq!(skill.name(), "play_background_file");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_play_background_file_setup_needs_files() {
        let mut skill = PlayBackgroundFile::new(Map::new());
        assert!(!skill.setup());
    }

    #[test]
    fn test_play_background_file_setup_with_files() {
        let mut params = Map::new();
        params.insert(
            "files".to_string(),
            json!([{"key": "hold", "url": "https://example.com/hold.mp3", "description": "Hold music"}]),
        );
        let mut skill = PlayBackgroundFile::new(params);
        assert!(skill.setup());
    }
}
