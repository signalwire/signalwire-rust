# SWAIG Reference

## Overview

SWAIG (SignalWire AI Gateway) functions are tools the AI model can invoke during a voice conversation. When the model decides to call a tool, the SignalWire platform POSTs to your agent's webhook endpoint. The SDK routes the request to the registered handler, which returns a `FunctionResult`.

## Defining Tools

```rust
use signalwire::swaig::FunctionResult;
use serde_json::json;

agent.define_tool(
    "get_weather",                           // name
    "Get the current weather for a city",    // description
    json!({                                  // parameters
        "city": {
            "type": "string",
            "description": "City name"
        }
    }),
    Box::new(|args, _raw_data| {             // handler
        let city = args.get("city")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        FunctionResult::with_response(&format!("It is sunny in {city}."))
    }),
    false,                                   // secure (HMAC)
);
```

### Parameters

- **name** -- unique tool name (snake_case recommended)
- **description** -- tells the AI when to use this tool
- **parameters** -- JSON schema for the tool's arguments
- **handler** -- closure receiving `(&Map<String, Value>, &Map<String, Value>)`
- **secure** -- if `true`, the SDK generates an HMAC token for the tool

## FunctionResult

Every handler returns a `FunctionResult`. At minimum it contains a response string that the AI uses to continue the conversation.

### Basic Response

```rust
FunctionResult::with_response("The order has shipped.")
```

### With Actions

Actions tell the platform to do something beyond just returning text:

```rust
let mut result = FunctionResult::with_response("Transferring you now.");
result.add_action(json!({
    "SWML": {
        "sections": {
            "main": [{"connect": {"to": "+15551234567"}}]
        },
        "version": "1.0.0"
    }
}));
```

### Action Helpers

The SDK provides convenience methods for common actions:

```rust
// Transfer the call
FunctionResult::with_response("Connecting you.")
    .connect("+15551234567");

// Send SMS
FunctionResult::with_response("Sending confirmation.")
    .send_sms("+15559876543", "+15551234567", "Your order is confirmed.");

// Start recording
FunctionResult::with_response("Recording started.")
    .record_call();

// Update session data
FunctionResult::with_response("Preferences saved.")
    .update_global_data(json!({"preference": "premium"}));

// Enable/disable tools mid-call
FunctionResult::with_response("Escalating.")
    .toggle_functions(vec![], vec!["escalate"]);

// Hang up
FunctionResult::with_response("Goodbye.")
    .hangup();
```

### Post-Processing

When `post_process` is true, the AI speaks the response to the user before executing the attached actions:

```rust
let mut result = FunctionResult::with_response(
    "I will transfer you to support. Is there anything else?"
);
result.set_post_process(true);
result.connect("+15551234567");
```

## post_data Lifecycle

When the platform POSTs to a SWAIG endpoint, the request body contains:

| Field | Type | Description |
|-------|------|-------------|
| `function` | `string` | Tool name |
| `argument` | `object` | Parsed arguments from the AI |
| `call_id` | `string` | Current call identifier |
| `caller_id_name` | `string` | Caller's name |
| `caller_id_number` | `string` | Caller's phone number |
| `call_display_name` | `string` | Display name |
| `ai_session_id` | `string` | AI session identifier |
| `global_data` | `object` | Current session data |
| `meta_data` | `object` | Call metadata |
| `meta_data_headers` | `object` | SIP headers |

The handler receives the parsed `argument` as the first parameter and the full post body as the second.

## Secure Functions

When a tool is marked `secure: true`, the SDK generates an HMAC-SHA256 token for the function URL. The platform includes this token when calling the function, preventing unauthorized invocations.

```rust
agent.define_tool(
    "transfer_funds",
    "Transfer money between accounts",
    json!({"amount": {"type": "number"}, "to": {"type": "string"}}),
    Box::new(|args, _raw| {
        FunctionResult::with_response("Transfer complete.")
    }),
    true, // secure
);
```

## Function Includes

Include external SWAIG functions from remote servers:

```rust
agent.add_function_include(json!({
    "url": "https://tools.example.com/swaig",
    "functions": ["search_inventory", "check_stock"]
}));
```

## Native Functions

Built-in platform functions that do not require a webhook:

- `transfer` -- transfer the call
- `check_voicemail` -- voicemail access
- `send_digits` -- DTMF tones
- `cdata_storage` -- call data storage

```rust
agent.add_native_functions(vec!["transfer", "check_voicemail"]);
```
