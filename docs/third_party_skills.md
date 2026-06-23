# Third-Party Skills

## Overview

Third-party skills extend the built-in skill system with custom capabilities. A skill is a struct that implements the `SkillBase` trait and registers itself with the global `SkillRegistry`.

## Creating a Custom Skill

### 1. Implement `SkillBase`

A skill stores its config in `SkillParams`, registers tools in `register_tools`, and
exposes prompt sections and hints via `get_prompt_sections` / `get_hints`. There is no
`apply()` method — the manager calls `setup()` then `register_tools(&mut agent)`.

```rust
use signalwire::agent::AgentBase;
use signalwire::skills::skill_base::{SkillBase, SkillParams};
use signalwire::swaig::FunctionResult;
use serde_json::{json, Map, Value};

pub struct WeatherSkill {
    sp: SkillParams,
}

impl WeatherSkill {
    pub fn new(params: Map<String, Value>) -> Self {
        WeatherSkill { sp: SkillParams::new(params) }
    }
}

impl SkillBase for WeatherSkill {
    fn name(&self) -> &'static str {
        "weather"
    }

    fn description(&self) -> &'static str {
        "Get current weather for a location"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    // Return false to abort registration (e.g. missing required config)
    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
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
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        vec![json!({
            "title": "Weather Skill",
            "body": "You can check the weather using the get_weather tool.",
        })]
    }

    fn get_hints(&self) -> Vec<String> {
        vec!["weather".into(), "temperature".into(), "forecast".into()]
    }
}
```

### 2. Register the Skill

`SkillRegistry::register_skill(name, factory)` takes a factory of type
`Box<dyn Fn(Map<String, Value>) -> Box<dyn SkillBase>>`:

```rust
use signalwire::skills::SkillRegistry;

// At application startup
SkillRegistry::register_skill(
    "weather",
    Box::new(|params| Box::new(WeatherSkill::new(params))),
);
```

### 3. Use the Skill

```rust
agent.add_skill("weather", json!({"api_key": "your-key"}));
```

## DataMap Skills

Skills can use DataMap for serverless execution:

```rust
impl SkillBase for JokeSkill {
    fn register_tools(&self, agent: &mut AgentBase) {
        // Build the DataMap and register its serialised SWAIG function.
        agent.register_swaig_function(json!({
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
    signalwire::skills::SkillRegistry::register_skill(
        "weather",
        Box::new(|params| Box::new(WeatherSkill::new(params))),
    );
}

// In the user's main.rs
fn main() {
    my_weather_skill::register();

    let mut agent = AgentBase::new(AgentOptions::new("my-agent"));
    agent.add_skill("weather", json!({"api_key": "..."}));
    agent.run();
}
```
