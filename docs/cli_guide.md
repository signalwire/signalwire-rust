# CLI Guide

## swaig-test

`swaig-test` is a command-line tool for testing agents locally without running a server or making real calls.

## Installation

The tool is built as a binary in the workspace:

```bash
cargo build --bin swaig-test
```

## Commands

### List Tools

Show all SWAIG tools registered by an agent:

```bash
cargo run --bin swaig-test -- --list-tools examples/simple_agent.rs
```

Output:

```
Tools registered by "my-agent":
  get_time        - Get the current time
  check_order     - Look up an order by ID
```

### Dump SWML

Render the complete SWML document the agent would return:

```bash
cargo run --bin swaig-test -- --dump-swml examples/simple_agent.rs
```

Output: pretty-printed JSON SWML document.

### Execute a Function

Call a specific tool with optional JSON arguments:

```bash
# No arguments
cargo run --bin swaig-test -- --exec get_time examples/simple_agent.rs

# With arguments
cargo run --bin swaig-test -- --exec check_order \
    --args '{"order_id": "ORD-123"}' \
    examples/simple_agent.rs
```

Output: the `FunctionResult` JSON.

## Options

| Flag | Description |
|------|-------------|
| `--list-tools` | List all registered SWAIG tools |
| `--dump-swml` | Render and print the SWML document |
| `--exec <name>` | Execute a tool by name |
| `--args <json>` | JSON arguments for `--exec` |
| `--verbose` | Show debug output |

## Examples

```bash
# Test the simple agent
cargo run --bin swaig-test -- --list-tools examples/simple_agent.rs
cargo run --bin swaig-test -- --dump-swml examples/simple_agent.rs
cargo run --bin swaig-test -- --exec get_time examples/simple_agent.rs

# Test the contexts demo
cargo run --bin swaig-test -- --dump-swml examples/contexts_demo.rs

# Test with arguments
cargo run --bin swaig-test -- --exec get_weather \
    --args '{"city": "Austin"}' \
    examples/data_map_demo.rs

# Verbose output for debugging
cargo run --bin swaig-test -- --dump-swml --verbose examples/skills_demo.rs
```

## Troubleshooting

### "No tools found"

The agent may not have any tools defined. Check that `define_tool()` or `add_skill()` is called.

### "Tool not found: <name>"

The tool name does not match any registered tool. Use `--list-tools` to see available names.

### "Failed to parse args"

The `--args` value must be valid JSON. Wrap in single quotes and use double quotes for keys.
