# Skills System

## Overview

Skills are modular, reusable capabilities that can be added to any agent with a single method call. Each skill registers one or more SWAIG tools, adds prompt sections, and optionally configures hints.

```rust
agent.add_skill("datetime", None);
agent.add_skill("math", None);
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
2. The factory creates a `SkillBase` instance with the provided config
3. The `SkillManager` calls `apply()` on the skill instance
4. `apply()` registers tools, adds prompt sections, and sets hints on the agent

## Skill Lifecycle

```
agent.add_skill("datetime", config)
    │
    ▼
SkillRegistry::get("datetime") → DateTimeSkill::new(config)
    │
    ▼
DateTimeSkill::apply(&mut agent)
  ├── agent.define_tool("get_current_time", ...)
  ├── agent.define_tool("get_current_date", ...)
  ├── agent.prompt_add_section("Available Skills", ...)
  └── agent.add_hints(vec!["current time", "what time is it"])
```

## Multiple Skill Instances

You can add the same skill type multiple times with different configs:

```rust
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
