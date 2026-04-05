# DataMap Guide

## Overview

DataMap tools execute entirely on SignalWire's servers. Instead of a webhook URL, you provide a `data_map` configuration that tells the platform how to call an external API and transform the response. This means no webhook endpoint is needed -- the platform handles the HTTP call.

## When to Use DataMap

- You want a tool that calls a third-party API but do not want to host a webhook
- The response can be transformed with simple pattern matching
- You need serverless execution without any infrastructure

## DataMap Builder

The `DataMap` struct provides a fluent builder API:

```rust
use signalwire::datamap::DataMap;
use signalwire::swaig::FunctionResult;

let tool = DataMap::new("get_weather")
    .description("Get current weather for a city")
    .parameter("city", "string", "City name", true)
    .webhook(
        "GET",
        "https://api.weatherapi.com/v1/current.json",
        json!({"key": "YOUR_API_KEY", "q": "${args.city}"}),
        json!({"Accept": "application/json"}),
    )
    .output(FunctionResult::with_response(
        "The weather in ${args.city} is ${response.current.condition.text}, \
         temperature ${response.current.temp_f}F."
    ))
    .build();

agent.define_datamap_tool(tool);
```

## Expressions

Expressions match a string against a regex pattern and return different responses:

```rust
let tool = DataMap::new("command_processor")
    .description("Process user commands")
    .parameter("command", "string", "User command", true)
    .expression(
        "${args.command}",
        r"^start",
        FunctionResult::with_response("Starting process."),
    )
    .expression(
        "${args.command}",
        r"^stop",
        FunctionResult::with_response("Stopping process."),
    )
    .expression_with_nomatch(
        "${args.command}",
        r"^status",
        FunctionResult::with_response("Checking status."),
        FunctionResult::with_response("Unknown command. Try start, stop, or status."),
    )
    .build();
```

## Webhook Configuration

### Basic Webhook

```rust
DataMap::new("search")
    .webhook("GET", "https://api.example.com/search", json!({"q": "${args.query}"}), json!({}))
    .output(FunctionResult::with_response("Results: ${response.data}"))
```

### With Auth Headers

```rust
DataMap::new("knowledge_search")
    .webhook(
        "POST",
        "https://api.example.com/search",
        json!({"query": "${args.query}"}),
        json!({
            "Authorization": "Bearer ${env.API_KEY}",
            "Content-Type": "application/json"
        }),
    )
    .output(FunctionResult::with_response("Found: ${response.results[0].text}"))
```

### Array Processing with foreach

```rust
DataMap::new("list_items")
    .webhook("GET", "https://api.example.com/items", json!({}), json!({}))
    .foreach("response.items", "item", FunctionResult::with_response(
        "Item: ${item.name} - ${item.description}"
    ))
```

## Advanced Features

### Post-Webhook Expressions

Apply pattern matching on the API response:

```rust
DataMap::new("api_tool")
    .webhook("POST", "https://api.example.com/action", json!({}), json!({}))
    .webhook_expression(
        "${response.status}",
        "^success$",
        FunctionResult::with_response("Operation completed."),
    )
    .webhook_expression(
        "${response.status}",
        "^error$",
        FunctionResult::with_response("Error: ${response.message}"),
    )
```

### Form Parameters

```rust
DataMap::new("form_tool")
    .webhook_with_form(
        "POST",
        "https://api.example.com/submit",
        json!({}),
        json!({}),
        "payload",  // form_param name
    )
```

### Input Args as Params

Merge all function arguments into the request parameters:

```rust
DataMap::new("passthrough")
    .webhook_with_options(
        "POST",
        "https://api.example.com/process",
        json!({}),
        json!({}),
        true,   // input_args_as_params
        None,   // form_param
        None,   // require_args
    )
```

## Raw DataMap JSON

You can also provide raw data_map JSON for full control:

```rust
agent.define_datamap_tool(json!({
    "function": "get_joke",
    "description": "Tell a joke",
    "data_map": {
        "webhooks": [{
            "url": "https://api.api-ninjas.com/v1/${args.type}",
            "headers": {"X-Api-Key": "YOUR_KEY"},
            "output": {"response": "Tell the user: ${array[0].joke}"}
        }]
    }
}));
```
