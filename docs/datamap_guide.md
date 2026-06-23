# DataMap Guide

## Overview

DataMap tools execute entirely on SignalWire's servers. Instead of a webhook URL, you provide a `data_map` configuration that tells the platform how to call an external API and transform the response. This means no webhook endpoint is needed -- the platform handles the HTTP call.

## When to Use DataMap

- You want a tool that calls a third-party API but do not want to host a webhook
- The response can be transformed with simple pattern matching
- You need serverless execution without any infrastructure

## DataMap Builder

`DataMap::new(name)` returns a builder you configure with chainable `&mut self`
methods, then serialise with `to_swaig_function()` and register on the agent via
`register_swaig_function`:

```rust
use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;
use serde_json::json;

let mut weather = DataMap::new("get_weather");
weather
    .description("Get the current weather for a city")
    // parameter(name, type, description, required, enum_values)
    .parameter("city", "string", "City name", true, vec![])
    // webhook(method, url, headers, form_param, input_args_as_params, require_args)
    .webhook(
        "GET",
        "https://api.weatherapi.com/v1/current.json",
        json!({}),
        "",
        false,
        vec![],
    )
    // params/body apply to the most recently added webhook
    .params(json!({"key": "YOUR_API_KEY", "q": "${args.city}"}))
    // output() takes a Value — use FunctionResult::with_response(...).to_value()
    .output(
        FunctionResult::with_response(
            "The weather in ${args.city} is ${response.current.condition.text}, \
             temperature ${response.current.temp_f}F.",
        )
        .to_value(),
    );

agent.register_swaig_function(weather.to_swaig_function());
```

## Expressions

Expressions match a string against a regex pattern and return different responses.
`expression(test_value, pattern, output, nomatch_output)` takes `Value` outputs and an
optional `Option<Value>` for the no-match case:

```rust
let mut commands = DataMap::new("command_processor");
commands
    .description("Process user commands")
    .parameter("command", "string", "User command", true, vec![])
    .expression(
        "${args.command}",
        r"^start",
        FunctionResult::with_response("Starting process.").to_value(),
        None,
    )
    .expression(
        "${args.command}",
        r"^stop",
        FunctionResult::with_response("Stopping process.").to_value(),
        None,
    )
    .expression(
        "${args.command}",
        r"^status",
        FunctionResult::with_response("Checking status.").to_value(),
        Some(
            FunctionResult::with_response("Unknown command. Try start, stop, or status.")
                .to_value(),
        ),
    );

agent.register_swaig_function(commands.to_swaig_function());
```

## Webhook Configuration

### Basic Webhook

```rust
let mut search = DataMap::new("search");
search
    .webhook("GET", "https://api.example.com/search", json!({}), "", false, vec![])
    .params(json!({"q": "${args.query}"}))
    .output(FunctionResult::with_response("Results: ${response.data}").to_value());
```

### With Auth Headers

Headers are the third argument to `webhook`; per-request body goes through `body`:

```rust
let mut search = DataMap::new("knowledge_search");
search
    .webhook(
        "POST",
        "https://api.example.com/search",
        json!({
            "Authorization": "Bearer ${env.API_KEY}",
            "Content-Type": "application/json"
        }),
        "",
        false,
        vec![],
    )
    .body(json!({"query": "${args.query}"}))
    .output(FunctionResult::with_response("Found: ${response.results[0].text}").to_value());
```

### Array Processing with foreach

`for_each` takes a JSON config object that is attached to the last webhook:

```rust
let mut list = DataMap::new("list_items");
list
    .webhook("GET", "https://api.example.com/items", json!({}), "", false, vec![])
    .for_each(json!({
        "input_key": "items",
        "output_key": "formatted",
        "append": "Item: ${this.name} - ${this.description}\n"
    }))
    .output(FunctionResult::with_response("${formatted}").to_value());
```

## Advanced Features

### Post-Webhook Expressions

`webhook_expressions` sets expression rules on the most recently added webhook:

```rust
let mut tool = DataMap::new("api_tool");
tool
    .webhook("POST", "https://api.example.com/action", json!({}), "", false, vec![])
    .webhook_expressions(vec![
        json!({
            "string": "${response.status}",
            "pattern": "^success$",
            "output": {"response": "Operation completed."}
        }),
        json!({
            "string": "${response.status}",
            "pattern": "^error$",
            "output": {"response": "Error: ${response.message}"}
        }),
    ]);
```

### Form Parameters

The `form_param` argument (4th positional) names a form field to wrap the body in:

```rust
let mut form_tool = DataMap::new("form_tool");
form_tool.webhook(
    "POST",
    "https://api.example.com/submit",
    json!({}),
    "payload", // form_param name
    false,
    vec![],
);
```

### Input Args as Params

Set `input_args_as_params` (5th positional) to merge all function arguments into the
request parameters; `require_args` (6th positional) lists arguments that must be present:

```rust
let mut passthrough = DataMap::new("passthrough");
passthrough.webhook(
    "POST",
    "https://api.example.com/process",
    json!({}),
    "",
    true,            // input_args_as_params
    vec!["query"],   // require_args
);
```

### Fallback and Error Keys

`fallback_output` sets a global output, `error_keys`/`global_error_keys` mark response
keys that signal an error:

```rust
let mut tool = DataMap::new("resilient_tool");
tool
    .webhook("GET", "https://api.example.com/x", json!({}), "", false, vec![])
    .error_keys(vec!["error", "errorMessage"])
    .fallback_output(FunctionResult::with_response("The service is unavailable.").to_value());
```

## Static Helpers

For the common single-webhook or expression-only case, the `DataMap` associated
functions build the full SWAIG function definition in one call:

```rust
let func = DataMap::create_simple_api_tool(
    "get_joke",
    "Tell a joke",
    vec![json!({"name": "type", "type": "string", "description": "Joke type", "required": false})],
    "GET",
    "https://api.api-ninjas.com/v1/${args.type}",
    FunctionResult::with_response("Tell the user: ${array[0].joke}").to_value(),
    json!({"X-Api-Key": "YOUR_KEY"}),
);
agent.register_swaig_function(func);
```
