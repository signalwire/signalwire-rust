# Third-Party Skills

## Overview

Third-party skills extend the built-in skill system with custom capabilities. A skill is a struct that implements the `SkillBase` trait and registers itself with the global `SkillRegistry`.

## Creating a Custom Skill

### 1. Implement `SkillBase`

```rust
use signalwire::skills::SkillBase;
use signalwire::agent::AgentBase;
use signalwire::swaig::FunctionResult;
use serde_json::{json, Value};

pub struct WeatherSkill {
    api_key: String,
}

impl WeatherSkill {
    pub fn new(config: Option<Value>) -> Self {
        let api_key = config
            .as_ref()
            .and_then(|c| c.get("api_key"))
            .and_then(|v| v.as_str())
            .expect("weather skill requires api_key")
            .to_string();
        WeatherSkill { api_key }
    }
}

impl SkillBase for WeatherSkill {
    fn name(&self) -> &str {
        "weather"
    }

    fn apply(&self, agent: &mut AgentBase) {
        let api_key = self.api_key.clone();

        agent.define_tool(
            "get_weather",
            "Get current weather for a location",
            json!({
                "location": {
                    "type": "string",
                    "description": "City name or zip code"
                }
            }),
            Box::new(move |args, _raw| {
                let location = args.get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                // In production, call the weather API here
                FunctionResult::with_response(
                    &format!("Weather for {location}: sunny, 72F")
                )
            }),
            false,
        );

        agent.prompt_add_section(
            "Weather Skill",
            "You can check the weather using the get_weather tool.",
            vec![],
        );

        agent.add_hints(vec!["weather", "temperature", "forecast"]);
    }
}
```

### 2. Register the Skill

```rust
use signalwire::skills::SkillRegistry;

// At application startup
SkillRegistry::register("weather", |config| {
    Box::new(WeatherSkill::new(config))
});
```

### 3. Use the Skill

```rust
agent.add_skill("weather", Some(json!({"api_key": "your-key"})));
```

## DataMap Skills

Skills can use DataMap for serverless execution:

```rust
impl SkillBase for JokeSkill {
    fn apply(&self, agent: &mut AgentBase) {
        agent.define_datamap_tool(json!({
            "function": self.tool_name,
            "description": "Tell a joke",
            "data_map": {
                "webhooks": [{
                    "url": format!(
                        "https://api.api-ninjas.com/v1/{}",
                        self.joke_type
                    ),
                    "headers": {"X-Api-Key": self.api_key},
                    "output": {
                        "response": "Tell the user: ${array[0].joke}"
                    }
                }]
            }
        }));
    }
}
```

## Skill Design Guidelines

1. **Self-contained** -- a skill should work without requiring the agent to do extra setup
2. **Configurable** -- accept a `Value` config for API keys, options, and customisation
3. **Prompt-aware** -- add relevant prompt sections and hints automatically
4. **Validated** -- check required config fields at construction time, not at call time
5. **Named uniquely** -- skill names must be unique across the registry
6. **Documented** -- include parameter schema docs for users of your skill

## Publishing

Package your skill as a crate and instruct users to register it at startup:

```rust
// In the skill crate
pub fn register() {
    signalwire::skills::SkillRegistry::register("weather", |config| {
        Box::new(WeatherSkill::new(config))
    });
}

// In the user's main.rs
fn main() {
    my_weather_skill::register();

    let mut agent = AgentBase::new(AgentOptions::new("my-agent"));
    agent.add_skill("weather", Some(json!({"api_key": "..."})));
    agent.run();
}
```
