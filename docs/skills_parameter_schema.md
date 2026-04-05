# Skills Parameter Schema

## Overview

Each skill defines a parameter schema that describes its configuration options. The schema is used for validation when a skill is instantiated.

## datetime

```json
{
  "timezone": {
    "type": "string",
    "description": "IANA timezone (e.g. America/New_York)",
    "default": "UTC"
  },
  "format_12h": {
    "type": "boolean",
    "description": "Use 12-hour format instead of 24-hour",
    "default": false
  }
}
```

### Tools Registered

| Tool | Parameters | Description |
|------|-----------|-------------|
| `get_current_time` | none | Returns current time in configured timezone |
| `get_current_date` | none | Returns current date in configured timezone |

---

## math

```json
{
  "precision": {
    "type": "integer",
    "description": "Decimal places for results",
    "default": 6
  }
}
```

### Tools Registered

| Tool | Parameters | Description |
|------|-----------|-------------|
| `calculate` | `expression: string` | Evaluate a mathematical expression |

---

## joke

```json
{
  "api_key": {
    "type": "string",
    "description": "API Ninjas API key",
    "required": true
  },
  "tool_name": {
    "type": "string",
    "description": "Custom tool name",
    "default": "get_joke"
  },
  "default_joke_type": {
    "type": "string",
    "description": "Default joke endpoint (jokes or dadjokes)",
    "default": "jokes"
  }
}
```

### Tools Registered

| Tool | Parameters | Description |
|------|-----------|-------------|
| `get_joke` | `type: string` | Fetch a joke from API Ninjas |

---

## mcp_gateway

```json
{
  "gateway_url": {
    "type": "string",
    "description": "URL of the MCP gateway server",
    "required": true
  },
  "auth_user": {
    "type": "string",
    "description": "Basic auth username",
    "default": ""
  },
  "auth_password": {
    "type": "string",
    "description": "Basic auth password",
    "default": ""
  },
  "services": {
    "type": "array",
    "description": "List of MCP services to expose",
    "items": {
      "type": "object",
      "properties": {
        "name": {"type": "string", "description": "Service name"}
      }
    }
  }
}
```

### Tools Registered

Dynamic -- one tool per exposed MCP service function.

---

## Common Schema Conventions

- `required: true` fields must be provided or the skill will fail to initialise
- Fields with a `default` are optional
- `type` follows JSON Schema types: `string`, `integer`, `boolean`, `array`, `object`
- Skills validate parameters at construction time, not at tool invocation time
