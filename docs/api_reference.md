# API Reference

## AgentBase

The primary type for building AI voice agents.

### Construction

```rust
AgentBase::new(options: AgentOptions) -> Self
```

### AgentOptions

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | required | Agent identifier |
| `route` | `Option<String>` | `None` | HTTP endpoint path |
| `host` | `Option<String>` | `None` | Bind address |
| `port` | `Option<u16>` | `Some(3000)` | Listen port |
| `basic_auth_user` | `Option<String>` | `None` | Auth username (auto-generated if None) |
| `basic_auth_password` | `Option<String>` | `None` | Auth password (auto-generated if None) |
| `auto_answer` | `bool` | `true` | Auto-answer inbound calls |
| `record_call` | `bool` | `false` | Enable call recording |
| `use_pom` | `bool` | `true` | Use Prompt Object Model |

### Prompt Methods

| Method | Signature |
|--------|-----------|
| `set_prompt_text` | `(&mut self, text: &str) -> &mut Self` |
| `set_post_prompt` | `(&mut self, text: &str) -> &mut Self` |
| `prompt_add_section` | `(&mut self, title: &str, body: &str, bullets: Vec<&str>) -> &mut Self` |
| `prompt_add_subsection` | `(&mut self, parent: &str, title: &str, body: &str) -> &mut Self` |
| `prompt_add_to_section` | `(&mut self, title: &str, body: Option<&str>, bullets: Vec<&str>) -> &mut Self` |
| `prompt_has_section` | `(&self, title: &str) -> bool` |
| `get_prompt` | `(&self) -> Value` |

### Tool Methods

| Method | Signature |
|--------|-----------|
| `define_tool` | `(&mut self, name, description, parameters, handler, secure) -> &mut Self` |
| `define_datamap_tool` | `(&mut self, tool: Value) -> &mut Self` |
| `add_function_include` | `(&mut self, include: Value) -> &mut Self` |

### Language and Voice

| Method | Signature |
|--------|-----------|
| `add_language` | `(&mut self, name: &str, code: &str, voice: &str) -> &mut Self` |
| `add_pronunciation` | `(&mut self, pronunciation: Value) -> &mut Self` |
| `add_hints` | `(&mut self, hints: Vec<&str>) -> &mut Self` |

### Parameters and Data

| Method | Signature |
|--------|-----------|
| `set_params` | `(&mut self, params: Value) -> &mut Self` |
| `set_params_value` | `(&mut self, key: &str, value: Value) -> &mut Self` |
| `set_global_data` | `(&mut self, data: Value) -> &mut Self` |
| `set_global_data_value` | `(&mut self, key: &str, value: Value) -> &mut Self` |

### LLM Parameters

| Method | Signature |
|--------|-----------|
| `set_prompt_llm_params` | `(&mut self, params: Value) -> &mut Self` |
| `set_post_prompt_llm_params` | `(&mut self, params: Value) -> &mut Self` |

### Call Flow

| Method | Signature |
|--------|-----------|
| `add_pre_answer_verb` | `(&mut self, verb: &str, params: Value) -> &mut Self` |
| `add_post_answer_verb` | `(&mut self, verb: &str, params: Value) -> &mut Self` |
| `add_post_ai_verb` | `(&mut self, verb: &str, params: Value) -> &mut Self` |
| `set_answer_config` | `(&mut self, config: Value) -> &mut Self` |

### Callbacks

| Method | Signature |
|--------|-----------|
| `set_dynamic_config_callback` | `(&mut self, cb: Arc<DynamicConfigCallback>) -> &mut Self` |
| `set_summary_callback` | `(&mut self, cb: Arc<SummaryCallback>) -> &mut Self` |
| `set_debug_event_handler` | `(&mut self, cb: Arc<DebugEventCallback>) -> &mut Self` |
| `enable_debug_events` | `(&mut self, level: &str) -> &mut Self` |

### Skills and Contexts

| Method | Signature |
|--------|-----------|
| `add_skill` | `(&mut self, skill: &str, config: Option<Value>) -> &mut Self` |
| `define_contexts` | `(&mut self) -> &mut ContextBuilder` |

### Session and Security

| Method | Signature |
|--------|-----------|
| `get_basic_auth_credentials` | `(&self) -> (&str, &str)` |
| `set_post_prompt_url` | `(&mut self, url: &str) -> &mut Self` |
| `set_webhook_url` | `(&mut self, url: &str) -> &mut Self` |

### Execution

| Method | Signature |
|--------|-----------|
| `run` | `(&self)` |
| `get_app` | `(&self) -> App` |
| `render_swml` | `(&self) -> Value` |

---

## FunctionResult

Returned from SWAIG tool handlers. Serialises to `{"response": "...", "action": [...]}`.

### Construction

| Method | Signature |
|--------|-----------|
| `new` | `() -> Self` |
| `with_response` | `(response: &str) -> Self` |

### Core Methods

| Method | Signature |
|--------|-----------|
| `set_response` | `(&mut self, text: &str) -> &mut Self` |
| `set_post_process` | `(&mut self, val: bool) -> &mut Self` |
| `add_action` | `(&mut self, action: Value) -> &mut Self` |

### Action Helpers

| Method | Description |
|--------|-------------|
| `connect` | Transfer the call |
| `send_sms` | Send SMS during call |
| `record_call` | Start background recording |
| `stop_record_call` | Stop a recording by control ID |
| `play_background_file` | Play audio in background |
| `stop_background_file` | Stop background audio |
| `hold` | Put caller on hold |
| `unhold` | Resume from hold |
| `update_global_data` | Update session key/value pairs |
| `toggle_functions` | Enable/disable tools mid-call |
| `update_settings` | Change AI settings mid-call |
| `add_dynamic_hints` | Add speech recognition hints |
| `set_end_of_speech_timeout` | Adjust silence detection |
| `join_room` | Join a RELAY room |
| `sip_refer` | SIP REFER transfer |
| `set_metadata` | Set call metadata |
| `say` | Speak a message |
| `hangup` | Terminate the call |

### Serialisation

| Method | Signature |
|--------|-----------|
| `to_value` | `(&self) -> Value` |
| `to_json` | `(&self) -> String` |

---

## ContextBuilder / Context / Step

See [contexts_guide.md](contexts_guide.md) for full details.

## DataMap

See [datamap_guide.md](datamap_guide.md) for full details.

## AgentServer

See [agent_guide.md](agent_guide.md) for multi-agent hosting.

## RelayClient

See [relay/docs/client-reference.md](../relay/docs/client-reference.md).

## RestClient

See [rest/docs/client-reference.md](../rest/docs/client-reference.md).
