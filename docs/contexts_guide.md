# Contexts and Steps Guide

## Overview

Contexts are separate conversation flows within a single agent. Each context can have its own persona, prompt sections, and available tools. The AI switches between contexts using the built-in `change_context` tool.

Steps are sequential stages within a context. Each step has prompt sections, criteria for advancement, and navigation rules.

## Core Concepts

- **Context** -- a self-contained conversation flow (e.g. a department)
- **Step** -- a sequential stage within a context (e.g. greeting, qualification, closing)
- **Navigation** -- the AI moves between steps and contexts using built-in tools
- **Isolation** -- isolated contexts have independent prompts; non-isolated inherit the parent

## Defining Contexts

```rust
let ctx_builder = agent.define_contexts();

let ctx = ctx_builder.add_context("sales");
ctx.set_isolated(true);
ctx.add_section("Persona", "You are a sales specialist named Sarah.", vec![]);
ctx.add_enter_filler("Welcome to our sales department! How can I help you?");
```

## Defining Steps

```rust
let step1 = ctx.add_step("greeting");
step1.set_text("Greet the customer and ask what product they are interested in.");
step1.set_valid_steps(vec!["qualification"]);

let step2 = ctx.add_step("qualification");
step2.set_text("Ask qualifying questions about their needs and budget.");
step2.set_step_criteria("Customer has described their needs and budget range.");
step2.set_valid_steps(vec!["recommendation"]);
step2.set_functions(vec!["check_inventory", "get_pricing"]);

let step3 = ctx.add_step("recommendation");
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
let step = ctx.add_step("demographics");
step.set_text("Collect the patient's basic information.");
step.set_gather_info("patient_demographics", "Please provide the following information.");

step.add_gather_question("full_name", "What is your full name?", "string", false);
step.add_gather_question("phone", "What is your phone number?", "string", true);
step.add_gather_question("email", "What is your email address?", "string", false);

step.set_valid_steps(vec!["symptoms"]);
```

### GatherQuestion Fields

| Field | Type | Description |
|-------|------|-------------|
| `key_name` | `&str` | Key in global_data for the answer |
| `question_text` | `&str` | Question spoken to the caller |
| `type` | `&str` | Expected type (`string`, `integer`, etc.) |
| `confirm` | `bool` | Repeat back for confirmation |

## Multi-Persona Example

```rust
let ctx_builder = agent.define_contexts();

// Franklin - greeter
let greeter = ctx_builder.add_context("greeter");
greeter.set_isolated(true);
greeter.add_section("Persona", "You are Franklin, a friendly greeter.", vec![]);
greeter.add_enter_filler("Hey there! I'm Franklin. Welcome!");

let g_step = greeter.add_step("intro");
g_step.set_text("Welcome the caller and ask what department they need.");
g_step.set_valid_contexts(vec!["sales", "support"]);

// Rachael - sales
let sales = ctx_builder.add_context("sales");
sales.set_isolated(true);
sales.add_section("Persona", "You are Rachael, a sales expert.", vec![]);
sales.add_enter_filler("Hi! This is Rachael from sales.");

// Dwight - support
let support = ctx_builder.add_context("support");
support.set_isolated(true);
support.add_section("Persona", "You are Dwight, a tech support specialist.", vec![]);
support.add_enter_filler("Dwight here. Let me help you troubleshoot.");
```

## Context Entry Parameters

When switching contexts, these parameters control how the transition works:

| Parameter | Description |
|-----------|-------------|
| `system_prompt` | Override the context's system prompt |
| `consolidate` | Summarise prior conversation before switching |
| `full_reset` | Clear conversation history on entry |
| `user_prompt` | Inject a user message on entry |
