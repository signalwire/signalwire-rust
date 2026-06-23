# CLI Guide

## swaig-test

`swaig-test` is a command-line tool for testing agents locally without running a server or making real calls.

## Installation

The tool is built as a binary in the workspace:

```bash
cargo build --bin swaig-test
```

## Modes

`swaig-test` works in two modes:

- **`--example <NAME>`** — introspect a registered example agent by its example name
  (the file stem under `examples/`, e.g. `simple_agent`). The tool runs the example
  in-process via `cargo run --example`.
- **`--url <URL>`** — point at a running SWAIG endpoint. Basic auth can be embedded in
  the URL (e.g. `https://user:pass@host/swaig`).

### List Tools

Show all SWAIG tools registered by an example agent:

```bash
cargo run --bin swaig-test -- --example simple_agent --list-tools
```

### Dump SWML

Render the complete SWML document the agent would return:

```bash
cargo run --bin swaig-test -- --example simple_agent --dump-swml
```

Output: a JSON SWML document.

### Execute a Function

Call a specific tool. Arguments are passed as repeatable `--param K=V` flags (URL mode):

```bash
# No arguments
cargo run --bin swaig-test -- --url http://localhost:3000 --exec get_time

# With arguments
cargo run --bin swaig-test -- --url http://localhost:3000 \
    --exec check_order --param order_id=ORD-123
```

Output: the `FunctionResult` JSON.

## Options

| Flag | Description |
|------|-------------|
| `--example <NAME>` | Introspect an example agent by name (in-process) |
| `--url <URL>` | Target a running SWAIG endpoint |
| `--list-tools` | List all registered SWAIG tools |
| `--dump-swml` | Render and print the SWML document |
| `--exec <name>` | Execute a tool by name (URL mode) |
| `--param <K=V>` | Parameter for `--exec` (repeatable) |
| `--help` | Print usage |

## Examples

```bash
# Introspect example agents by name
cargo run --bin swaig-test -- --example simple_agent --list-tools
cargo run --bin swaig-test -- --example simple_agent --dump-swml
cargo run --bin swaig-test -- --example contexts_demo --dump-swml

# Execute a tool against a running endpoint with arguments
cargo run --bin swaig-test -- --url http://localhost:3000 \
    --exec get_weather --param city=Austin
```

## Troubleshooting

### "No tools found"

The agent may not have any tools defined. Check that `define_tool()` or `add_skill()` is called.

### "Tool not found: <name>"

The tool name does not match any registered tool. Use `--list-tools` to see available names.

### "Failed to parse args"

The `--args` value must be valid JSON. Wrap in single quotes and use double quotes for keys.
