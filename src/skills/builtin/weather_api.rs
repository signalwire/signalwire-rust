use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Get current weather information from WeatherAPI.com (DataMap-based).
pub struct WeatherApi {
    sp: SkillParams,
}

impl WeatherApi {
    pub fn new(params: Map<String, Value>) -> Self {
        WeatherApi {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for WeatherApi {
    fn name(&self) -> &str {
        "weather_api"
    }

    fn description(&self) -> &str {
        "Get current weather information from WeatherAPI.com"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("get_weather");
        let api_key = self.sp.get_str_or("api_key", "");
        let unit = self.sp.get_str_or("temperature_unit", "fahrenheit");

        let (temp_field, feels_field, unit_label) = if unit == "celsius" {
            ("${current.temp_c}", "${current.feelslike_c}", "C")
        } else {
            ("${current.temp_f}", "${current.feelslike_f}", "F")
        };

        let output_response = format!(
            "Weather in ${{location.name}}, ${{location.region}}: \
             Temperature: {temp_field}{unit_label}, \
             Feels like: {feels_field}{unit_label}, \
             Condition: ${{current.condition.text}}, \
             Humidity: ${{current.humidity}}%, \
             Wind: ${{current.wind_mph}} mph ${{current.wind_dir}}",
            temp_field = temp_field,
            feels_field = feels_field,
            unit_label = format!("\u{00B0}{}", unit_label),
        );

        let url = format!(
            "https://api.weatherapi.com/v1/current.json?key={}&q=${{lc:enc:args.location}}&aqi=no",
            api_key
        );

        let mut func_def = json!({
            "function": tool_name,
            "purpose": "Get current weather information for any location",
            "argument": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The location to get weather for (city name, zip code, or coordinates)",
                    }
                },
                "required": ["location"],
            },
            "data_map": {
                "webhooks": [{
                    "url": url,
                    "method": "GET",
                    "output": {
                        "response": output_response,
                        "action": [{"say_it": true}],
                    },
                    "error_output": {
                        "response": "Unable to retrieve weather information for the requested location.",
                        "action": [{"say_it": true}],
                    },
                }],
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
    fn test_weather_api_metadata() {
        let skill = WeatherApi::new(Map::new());
        assert_eq!(skill.name(), "weather_api");
    }

    #[test]
    fn test_weather_api_setup_needs_api_key() {
        let mut skill = WeatherApi::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("api_key".to_string(), json!("test-key"));
        let mut skill2 = WeatherApi::new(params);
        assert!(skill2.setup());
    }
}
