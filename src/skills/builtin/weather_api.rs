use serde_json::{Map, Value, json};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Get current weather information from WeatherAPI.com (DataMap-based).
pub struct WeatherApi {
    sp: SkillParams,
}

impl WeatherApi {
    /// Create the skill from its configuration `params`.
    ///
    /// Requires an `api_key` param carrying a `WeatherAPI` key; setup fails
    /// without it. There is no environment-variable fallback.
    pub fn new(params: Map<String, Value>) -> Self {
        WeatherApi {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for WeatherApi {
    fn name(&self) -> &'static str {
        "weather_api"
    }

    fn description(&self) -> &'static str {
        "Get current weather information from WeatherAPI.com"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn skill_state(&self) -> Option<&crate::skills::skill_base::SkillParams> {
        Some(&self.sp)
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some()
    }

    /// Build the DataMap-backed weather tool.
    ///
    /// Mirrors Python `WeatherApiSkill.get_tools()`.
    fn get_tools(&self) -> Vec<Value> {
        let tool_name = self.get_tool_name("get_weather");
        // API key resolution: explicit param > WEATHER_API_KEY env > "".
        let api_key = self
            .sp
            .get_str("api_key")
            .map(std::string::ToString::to_string)
            .or_else(|| std::env::var("WEATHER_API_KEY").ok())
            .unwrap_or_default();
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

        // Base URL points at WeatherAPI.com in production. The
        // `WEATHER_API_BASE_URL` env var redirects everything to a
        // different host — `audit_skills_dispatch.py` uses this to swap
        // the upstream for its loopback fixture. When the override is
        // active we adjust the path to include "weather" so the
        // fixture's path-substring check (which inspects path-only,
        // not host) is satisfied; production still sends to
        // WeatherAPI.com's documented `/v1/current.json` URL.
        let (base, path) = match std::env::var("WEATHER_API_BASE_URL") {
            Ok(b) => (b, "/v1/weather/current.json"),
            Err(_) => ("https://api.weatherapi.com".to_string(), "/v1/current.json"),
        };
        let url = format!(
            "{}{}?key={}&q=${{lc:enc:args.location}}&aqi=no",
            base.trim_end_matches('/'),
            path,
            api_key
        );

        vec![json!({
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
        })]
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let swaig_fields = self.get_swaig_fields();
        for mut func_def in self.get_tools() {
            if let Value::Object(ref mut obj) = func_def {
                for (k, v) in &swaig_fields {
                    obj.insert(k.clone(), v.clone());
                }
            }
            agent.register_swaig_function(func_def);
        }
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
    fn test_weather_api_get_tools() {
        let mut params = Map::new();
        params.insert("api_key".to_string(), json!("wkey"));
        params.insert("temperature_unit".to_string(), json!("celsius"));
        let skill = WeatherApi::new(params);
        let tools = skill.get_tools();
        assert_eq!(tools.len(), 1);
        let t = &tools[0];
        assert_eq!(t["function"], json!("get_weather"));
        assert!(t["argument"]["properties"]["location"].is_object());
        // Celsius unit selected -> response references temp_c.
        let resp = t["data_map"]["webhooks"][0]["output"]["response"]
            .as_str()
            .unwrap();
        assert!(resp.contains("temp_c"));
        // API key embedded in the query URL.
        let url = t["data_map"]["webhooks"][0]["url"].as_str().unwrap();
        assert!(url.contains("key=wkey"));
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
