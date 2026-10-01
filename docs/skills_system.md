# Skills System

## Overview

Skills are modular, reusable capabilities that can be added to any agent with a single method call. Each skill registers one or more SWAIG tools, adds prompt sections, and optionally configures hints.

<!-- snippet-setup -->
```rust
use signalwire::agent::{AgentBase, AgentOptions};
use serde_json::json;
use std::env;

let mut agent = AgentBase::new(AgentOptions::new("skills-guide"));
```

```rust
// add_skill(name, params) — params is `Option<Value>`; pass `None` for defaults
agent.add_skill("datetime", Some(json!({})));
agent.add_skill("math", Some(json!({})));
agent.add_skill("joke", Some(json!({"api_key": "your-key"})));
```

## Architecture

```
SkillRegistry (singleton)
  └── registers SkillBase implementations
        └── SkillManager (per-agent)
              └── instantiates and applies skills
```

- **SkillBase** -- trait that every skill implements
- **SkillManager** -- manages skill instances for a single agent
- **SkillRegistry** -- global registry of available skill factories

## Built-In Skills

| Skill | Tools Added | Description |
|-------|------------|-------------|
| `datetime` | `get_current_time`, `get_current_date` | Current date/time in configurable timezone |
| `math` | `calculate` | Safe mathematical expression evaluation |
| `joke` | `get_joke` | Jokes via API Ninjas (DataMap, no webhook) |
| `mcp_gateway` | (dynamic) | Bridge MCP server tools into SWAIG |

## Skill Configuration

Skills accept an optional `Value` config object:

```rust
// datetime with custom timezone
agent.add_skill("datetime", Some(json!({"timezone": "America/New_York"})));

// joke with API key
agent.add_skill("joke", Some(json!({"api_key": env::var("API_NINJAS_KEY").unwrap()})));

// mcp_gateway connecting to external MCP server
agent.add_skill("mcp_gateway", Some(json!({
    "gateway_url": "http://localhost:8080",
    "auth_user": "admin",
    "auth_password": "changeme",
    "services": [{"name": "todo"}]
})));
```

## How Skills Work

When `add_skill()` is called:

1. The `SkillRegistry` looks up the skill factory by name
2. The factory creates a `SkillBase` instance with the provided config params
3. The `SkillManager` calls `setup()` to initialise the skill
4. The manager applies the skill to the agent: `register_tools()` registers SWAIG tools,
   and the agent pulls in `get_prompt_sections()`, `get_hints()`, and `get_global_data()`

## Skill Lifecycle

The `SkillBase` trait exposes (among others): `name`, `description`, `setup`,
`register_tools`, `get_hints`, `get_prompt_sections`, `get_global_data`.

```
agent.add_skill("datetime", config)
    │
    ▼
SkillRegistry → DateTimeSkill (params)
    │
    ▼
skill.setup()                       // initialise, validate env vars
skill.register_tools(&mut agent)    // agent.define_tool("get_current_time", ...) etc.
skill.get_prompt_sections()         // merged into the agent prompt
skill.get_hints()                   // merged into the agent's speech hints
```

## Multiple Skill Instances

You can add the same skill type multiple times with different configs:

```rust
let api_key = "your-api-ninjas-key";

agent.add_skill("joke", Some(json!({
    "api_key": api_key,
    "tool_name": "get_regular_joke",
    "default_joke_type": "jokes"
})));

agent.add_skill("joke", Some(json!({
    "api_key": api_key,
    "tool_name": "get_dad_joke",
    "default_joke_type": "dadjokes"
})));
```

## Skills vs Raw Tools

| Aspect | Raw `define_tool` | `add_skill` |
|--------|-------------------|-------------|
| Code | Manual handler, params, prompt | One-liner |
| Reuse | Copy-paste | Automatic |
| Validation | Manual | Built-in |
| Prompt integration | Manual | Automatic |
| Hints | Manual | Automatic |

See [third_party_skills.md](third_party_skills.md) for creating custom skills.
