# Contexts and Steps Guide

## Overview

Contexts are separate conversation flows within a single agent. Each context can have its own system prompt, enter/exit fillers, and steps. The AI switches between contexts using the built-in `change_context` tool.

Steps are sequential stages within a context. Each step has prompt text (or POM sections), criteria for advancement, and navigation rules.

## Core Concepts

- **Context** -- a self-contained conversation flow (e.g. a department or persona)
- **Step** -- a sequential stage within a context (e.g. greeting, qualification, closing)
- **Navigation** -- the AI moves between steps and contexts using built-in tools
- **System prompt** -- a context that calls `set_system_prompt` replaces the agent's
  top-level prompt; a context that does not inherits the agent-level prompt sections

## Defining Contexts

`agent.define_contexts()` returns a `&mut ContextBuilder`. Add contexts with
`add_context(name)`, which returns a `&mut Context`:

```rust
let ctx_builder = agent.define_contexts();

let sales = ctx_builder.add_context("sales");
sales.set_system_prompt("You are a sales specialist named Sarah.");
sales.set_enter_fillers(serde_json::json!([
    "Welcome to our sales department! How can I help you?"
]));
```

> A context configures its prompt via `set_system_prompt` / `set_prompt_text`. POM
> `add_section` is a **step-level** method, not a context-level one.

## Defining Steps

`context.add_step(name)` returns a `&mut Step`. Each step uses `set_text` *or* POM
`add_section(title, body)` — the two are mutually exclusive on a single step.
`set_functions` takes a JSON `Value` (e.g. an array of tool names):

```rust
let step1 = sales.add_step("greeting");
step1.set_text("Greet the customer and ask what product they are interested in.");
step1.set_valid_steps(vec!["qualification"]);

let step2 = sales.add_step("qualification");
step2.set_text("Ask qualifying questions about their needs and budget.");
step2.set_step_criteria("Customer has described their needs and budget range.");
step2.set_valid_steps(vec!["recommendation"]);
step2.set_functions(serde_json::json!(["check_inventory", "get_pricing"]));

let step3 = sales.add_step("recommendation");
step3.set_text("Recommend a product based on their needs. Offer to transfer to support if needed.");
step3.set_valid_contexts(vec!["support"]);
```

## Step Navigation

Steps support two built-in navigation tools:

- **`next_step`** -- move to a step listed in `set_valid_steps()`
- **`change_context`** -- switch to a context listed in `set_valid_contexts()`

If `set_valid_steps()` is not called, the AI cannot advance from that step. The same applies to `set_valid_contexts()`.

## Gather Info Mode

Gather info mode presents questions one at a time with zero tool-call entries in conversation history. Answers are stored in `global_data`.

```rust
let step = symptoms_ctx.add_step("demographics");
step.set_text("Collect the patient's basic information.");
// set_gather_info(completion_key, completion_prompt)
step.set_gather_info("patient_demographics", "Please provide the following information.");

// add_gather_question(key_name, question_text, type, confirm)
step.add_gather_question("full_name", "What is your full name?", "string", false);
step.add_gather_question("phone", "What is your phone number?", "string", true);
step.add_gather_question("email", "What is your email address?", "string", false);

step.set_valid_steps(vec!["symptoms"]);
```

### Gather Question Fields

| Argument | Type | Description |
|-------|------|-------------|
| `key_name` | `&str` | Key in global_data for the answer |
| `question_text` | `&str` | Question spoken to the caller |
| `type` | `&str` | Expected type (`string`, `integer`, etc.) |
| `confirm` | `bool` | Repeat back for confirmation |

## Multi-Persona Example

Each context sets its own system prompt and enter filler. Top-level agent prompt sections
remain available to any context that does not override the system prompt:

```rust
// Top-level prompt — available to contexts that don't set their own system prompt
agent.prompt_add_section("Company", "You work for TechCo, a premium computer retailer.", vec![]);

let ctx_builder = agent.define_contexts();

// Franklin — greeter
let greeter = ctx_builder.add_context("greeter");
greeter.set_system_prompt("You are Franklin, a friendly greeter.");
greeter.set_enter_fillers(serde_json::json!(["Hey there! I'm Franklin. Welcome!"]));

let g_step = greeter.add_step("intro");
g_step.set_text("Welcome the caller and ask what department they need.");
g_step.set_valid_contexts(vec!["sales", "support"]);

// Rachael — sales
let sales = ctx_builder.add_context("sales");
sales.set_system_prompt("You are Rachael, a sales expert.");
sales.set_enter_fillers(serde_json::json!(["Hi! This is Rachael from sales."]));

// Dwight — support
let support = ctx_builder.add_context("support");
support.set_system_prompt("You are Dwight, a tech support specialist.");
support.set_enter_fillers(serde_json::json!(["Dwight here. Let me help you troubleshoot."]));
```

## Context Configuration Methods

| Method | Description |
|-----------|-------------|
| `set_system_prompt(&str)` | Replace the agent's top-level prompt for this context |
| `set_prompt_text(&str)` | Set raw prompt text for the context |
| `set_enter_fillers(Value)` | Phrases spoken when entering the context |
| `set_exit_fillers(Value)` | Phrases spoken when leaving the context |
| `set_initial_step(&str)` | Name the step the context starts on |
