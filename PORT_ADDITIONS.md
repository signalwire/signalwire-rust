# PORT_ADDITIONS.md (signalwire-rust)

Rust port-only symbols not present in the Python reference. Format:

```
<fully.qualified.symbol>: <one-sentence rationale>
```

scripts/diff_port_surface.py reads this file (via --additions) to know
which port-only symbols are intentional. Anything not in this file AND
not in the Python reference fails the diff.

## Categories

### AgentBase methods Rust ships under canonical Python names

These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.

signalwire.core.agent_base.AgentBase.add_function_include: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_hint: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_hints: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_internal_filler: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_internal_filler_for: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_language: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_mcp_server: These methods exist in Python's AgentBase too (via a mixin — here AIConfigMixin.add_mcp_server). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.enable_mcp_server: These methods exist in Python's AgentBase too (via a mixin — here AIConfigMixin.enable_mcp_server). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.contexts: These methods exist in Python's AgentBase too (via a mixin — here PromptMixin.contexts). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.tool: These methods exist in Python's AgentBase too (via a mixin — here ToolMixin.tool). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.as_router: These methods exist in Python's AgentBase too (via a mixin — here WebMixin.as_router). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.get_app: These methods exist in Python's AgentBase too (via a mixin — here WebMixin.get_app). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.enable_debug_routes: These methods exist in Python's AgentBase too (via a mixin — here WebMixin.enable_debug_routes). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.serve: These methods exist in Python's AgentBase too (via a mixin — here WebMixin.serve). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.setup_graceful_shutdown: These methods exist in Python's AgentBase too (via a mixin — here WebMixin.setup_graceful_shutdown). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.register_routing_callback: These methods exist in Python's AgentBase too (via a mixin — here WebMixin.register_routing_callback). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.handle_serverless_request: These methods exist in Python's AgentBase too (via a mixin — here ServerlessMixin.handle_serverless_request). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase in addition to projecting onto the originating mixin. Python has the same surface.
signalwire.core.agent_base.AgentBase.add_pattern_hint: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_pronunciation: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.add_skill: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.build_ai_verb: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.clone_for_request: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.define_contexts: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.define_tools: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.enable_debug_events: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.get_basic_auth_credentials: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.get_language_params: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.get_prompt: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.handle_request: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.has_skill: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.list_skills: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.list_tool_names: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.manual_set_proxy_url: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.prompt_add_section: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.prompt_add_subsection: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.prompt_add_to_section: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.prompt_has_section: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.refresh_context_tools: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.remove_skill: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.render_swml: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.reset_contexts: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.run: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.service: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.service_mut: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_dynamic_config_callback: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_function_includes: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_global_data: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_internal_fillers: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_internal_fillers_map: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_language_params: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_languages: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_native_functions: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_param: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_params: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_post_prompt: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_post_prompt_llm_params: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_prompt_llm_params: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_prompt_text: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_pronunciations: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.set_webhook_url: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.
signalwire.core.agent_base.AgentBase.update_global_data: These methods exist in Python's AgentBase too (often via a mixin). The Rust port hangs them directly off AgentBase, so the per-symbol enumerator emits them under signalwire.core.agent_base.AgentBase rather than under the originating mixin (signalwire.core.mixins.*). Python has the same surface.

### AgentServer Rust-side accessors / methods

Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).

signalwire.agent_server.AgentServer.get_agent_mut: Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).
signalwire.agent_server.AgentServer.handle_request: Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).
signalwire.agent_server.AgentServer.host: Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).
signalwire.agent_server.AgentServer.is_sip_routing_enabled: Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).
signalwire.agent_server.AgentServer.port: Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).
signalwire.agent_server.AgentServer.serve_static: Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).
signalwire.agent_server.AgentServer.sip_username_mapping: Rust idiom of explicit accessor methods (host(), port(), …) and rust-private serve_static (which serve_static_files now aliases for Python parity).

### DataMap Rust helpers

Rust merges Python's create_simple_api_tool / create_expression_tool helpers into DataMap-method form. for_each is the Rust naming of foreach (Python uses snake_case, Rust uses for_each to avoid conflict with the for keyword family).

signalwire.core.data_map.DataMap.create_expression_tool: Rust merges Python's create_simple_api_tool / create_expression_tool helpers into DataMap-method form. for_each is the Rust naming of foreach (Python uses snake_case, Rust uses for_each to avoid conflict with the for keyword family).
signalwire.core.data_map.DataMap.create_simple_api_tool: Rust merges Python's create_simple_api_tool / create_expression_tool helpers into DataMap-method form. for_each is the Rust naming of foreach (Python uses snake_case, Rust uses for_each to avoid conflict with the for keyword family).
signalwire.core.data_map.DataMap.for_each: Rust merges Python's create_simple_api_tool / create_expression_tool helpers into DataMap-method form. for_each is the Rust naming of foreach (Python uses snake_case, Rust uses for_each to avoid conflict with the for keyword family).

### FunctionResult Rust-side conveniences

to_json / to_value are serde serialization helpers; with_response is Rust's idiomatic alternative to Python's `FunctionResult(response=...)` constructor pattern.

signalwire.core.function_result.FunctionResult.to_json: to_json / to_value are serde serialization helpers; with_response is Rust's idiomatic alternative to Python's `FunctionResult(response=...)` constructor pattern.
signalwire.core.function_result.FunctionResult.to_value: to_json / to_value are serde serialization helpers; with_response is Rust's idiomatic alternative to Python's `FunctionResult(response=...)` constructor pattern.
signalwire.core.function_result.FunctionResult.with_response: to_json / to_value are serde serialization helpers; with_response is Rust's idiomatic alternative to Python's `FunctionResult(response=...)` constructor pattern.

### Rust Action common methods (cross-action surface)

Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.

signalwire.relay.call.Action.call_id: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.control_id: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.events: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.execute_subcommand: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.handle_event: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.node_id: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.on_completed: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.payload: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.resolve: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.result: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.set_notify_sender: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.state: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.stop: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.stop_method: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.
signalwire.relay.call.Action.with_stop_method: Rust models all Action variants (PlayAction, RecordAction, …) on top of a single Action struct that exposes shared accessors (call_id, control_id, state, …). Python uses an action-class-per-type hierarchy; the same data is exposed via attribute access on each action.

### Rust BedrockAgent accessor methods

Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.

signalwire.agents.bedrock.BedrockAgent.agent: Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.
signalwire.agents.bedrock.BedrockAgent.agent_mut: Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.
signalwire.agents.bedrock.BedrockAgent.max_tokens: Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.
signalwire.agents.bedrock.BedrockAgent.render_swml: Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.
signalwire.agents.bedrock.BedrockAgent.temperature: Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.
signalwire.agents.bedrock.BedrockAgent.top_p: Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.
signalwire.agents.bedrock.BedrockAgent.voice_id: Read-only / mutable accessors that surface internal Bedrock state. Python uses attribute access (`self._voice_id`) directly.

### Rust Call methods that rename Python equivalents

current_state mirrors Python's `state` attribute under a method name. dispatch_event is the public event-router (private in Python). echo_call / refer_call / pass replace Python's reserved-word-clashing Call.echo / Call.refer / Call.pass_. resolve_all_actions is a Rust convenience for terminal cleanup.

signalwire.relay.call.Call.call_state: Typed Tier-3 accessor returning the call state as a `CallState` enum (created/ringing/answered/ending/ended, `#[non_exhaustive]` with `Other` for unknown server values), exposed ALONGSIDE the string `current_state()` for parity — `call_state().as_str() == current_state()` always. Python's dynamic `state` attribute is a bare string; this is the floor-not-ceiling typed view. Additive; no Python equivalent.
signalwire.relay.call.Call.current_state: current_state mirrors Python's `state` attribute under a method name. dispatch_event is the public event-router (private in Python). echo_call / refer_call / pass replace Python's reserved-word-clashing Call.echo / Call.refer / Call.pass_. resolve_all_actions is a Rust convenience for terminal cleanup.
signalwire.relay.call.Call.dispatch_event: current_state mirrors Python's `state` attribute under a method name. dispatch_event is the public event-router (private in Python). echo_call / refer_call / pass replace Python's reserved-word-clashing Call.echo / Call.refer / Call.pass_. resolve_all_actions is a Rust convenience for terminal cleanup.
signalwire.relay.call.Call.echo_call: current_state mirrors Python's `state` attribute under a method name. dispatch_event is the public event-router (private in Python). echo_call / refer_call / pass replace Python's reserved-word-clashing Call.echo / Call.refer / Call.pass_. resolve_all_actions is a Rust convenience for terminal cleanup.
signalwire.relay.call.Call.pass: current_state mirrors Python's `state` attribute under a method name. dispatch_event is the public event-router (private in Python). echo_call / refer_call / pass replace Python's reserved-word-clashing Call.echo / Call.refer / Call.pass_. resolve_all_actions is a Rust convenience for terminal cleanup.
signalwire.relay.call.Call.refer_call: current_state mirrors Python's `state` attribute under a method name. dispatch_event is the public event-router (private in Python). echo_call / refer_call / pass replace Python's reserved-word-clashing Call.echo / Call.refer / Call.pass_. resolve_all_actions is a Rust convenience for terminal cleanup.
signalwire.relay.call.Call.resolve_all_actions: current_state mirrors Python's `state` attribute under a method name. dispatch_event is the public event-router (private in Python). echo_call / refer_call / pass replace Python's reserved-word-clashing Call.echo / Call.refer / Call.pass_. resolve_all_actions is a Rust convenience for terminal cleanup.

### Rust CrudResource constructor / accessors

Rust CrudResource is a base struct with explicit accessors. Python's _base.CrudResource is internal.

signalwire.rest._base.CrudResource.__init__: Rust CrudResource is a base struct with explicit accessors. Python's _base.CrudResource is internal.
signalwire.rest._base.CrudResource.base_path: Rust CrudResource is a base struct with explicit accessors. Python's _base.CrudResource is internal.
signalwire.rest._base.CrudResource.client: Rust CrudResource is a base struct with explicit accessors. Python's _base.CrudResource is internal.

### Rust Document builder type

Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.

signalwire.core.swml_builder.Document: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.__init__: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.add_raw_verb: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.add_section: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.add_verb: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.add_verb_to_section: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.clear_section: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.get_verbs: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.has_section: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.render: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.render_pretty: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.reset: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.to_value: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.
signalwire.core.swml_builder.Document.version: Rust's swml_builder ships a Document struct (Python uses dict shapes throughout). The methods are the build-side helpers a user would compose into a SWML doc.

### Rust Event struct methods

Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.

signalwire.relay.event.Event.__init__: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.call_id: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.control_id: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.event_type: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.node_id: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.params: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.parse: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.state: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.tag: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.timestamp: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.
signalwire.relay.event.Event.to_value: Rust models Event as a struct with typed accessors (call_id, control_id, event_type, …) and helpers (parse, to_value). Python uses an Event class hierarchy with attribute access — the data is the same; the accessor names differ.

### Rust Message struct accessors

Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.

signalwire.relay.message.Message.body: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.context: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.direction: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.dispatch_event: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.from_number: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.media: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.message_id: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.message_state: Typed Tier-3 accessor returning the delivery state as an `Option<MessageState>` enum (queued/initiated/sent/delivered/undelivered/failed/received, `#[non_exhaustive]` with `Other` for unknown server values), exposed ALONGSIDE the string `state()` for parity — when set, `message_state().unwrap().as_str() == state().unwrap()`. Floor-not-ceiling typed view over Python's bare-string `state`. Additive; no Python equivalent.
signalwire.relay.message.Message.on_completed: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.reason: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.resolve: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.state: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.tags: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.
signalwire.relay.message.Message.to_number: Rust ships typed accessors on Message (body, context, direction, dispatch_event, from_number, …) where Python exposes the same data via attribute access. Python's RelayClient.send_message wraps Message construction.

### Rust typed RELAY state enums (Tier-3 typed objects)

Tier-3 idiom pass: three `#[non_exhaustive]` enums giving the server-emitted RELAY state vocabularies a typed view over the bare strings Python carries dynamically (floor-not-ceiling). DELIBERATELY three distinct types — CallState ≠ DialState ≠ MessageState — so the vocabularies can't be conflated even where wire words coincide (`answered` is terminal for a dial but not a call; `failed` is both a dial and a message state). Each: `as_str()` (wire string, incl. the captured `Other` value), `from_str()` (infallible — unknown server values become `Other`, never panic, also via `FromStr`), `is_terminal()` (delegates to the matching `relay::constants::is_*_terminal` so typed and string predicates can't disagree). Grounded in Python `relay/constants.py` (CALL_STATE_*/MESSAGE_STATE_*/MESSAGE_TERMINAL_STATES) + the port's `relay::constants` (DIAL_STATE_*). Additive — `constants` keeps the raw consts + predicates. No Python equivalent (Python has no state enums).

signalwire.relay.state_enums.CallState: Typed call-lifecycle state (created/ringing/answered/ending/ended), `#[non_exhaustive]` + `Other(String)` for unknown server values. Terminal = ended. See section preamble.
signalwire.relay.state_enums.CallState.as_str: Canonical wire string for the state (the captured raw string for `Other`), so `CallState::from_str(s).as_str() == s`. See section preamble.
signalwire.relay.state_enums.CallState.from_str: Infallible parse of a wire string to CallState (unknown → `Other`); also exposed via `impl FromStr`. See section preamble.
signalwire.relay.state_enums.CallState.is_terminal: `true` iff terminal (ended); delegates to `relay::constants::is_call_terminal`. See section preamble.
signalwire.relay.state_enums.DialState: Typed dial-outcome state (dialing/answered/failed), `#[non_exhaustive]` + `Other(String)`. Terminal = answered or failed. Distinct from CallState. See section preamble.
signalwire.relay.state_enums.DialState.as_str: Canonical wire string for the dial state (raw for `Other`). See section preamble.
signalwire.relay.state_enums.DialState.from_str: Infallible parse to DialState (unknown → `Other`); also via `impl FromStr`. See section preamble.
signalwire.relay.state_enums.DialState.is_terminal: `true` iff terminal (answered or failed). See section preamble.
signalwire.relay.state_enums.MessageState: Typed message-delivery state (queued/initiated/sent/delivered/undelivered/failed/received), `#[non_exhaustive]` + `Other(String)`. Terminal = delivered/undelivered/failed. Distinct from DialState. See section preamble.
signalwire.relay.state_enums.MessageState.as_str: Canonical wire string for the delivery state (raw for `Other`). See section preamble.
signalwire.relay.state_enums.MessageState.from_str: Infallible parse to MessageState (unknown → `Other`); also via `impl FromStr`. See section preamble.
signalwire.relay.state_enums.MessageState.is_terminal: `true` iff terminal (delivered/undelivered/failed); delegates to `relay::constants::is_message_terminal`. See section preamble.

### Rust typed RELAY Device struct (Tier-3 typed object)

Tier-3 idiom pass: a typed `{type, params}` Device shape for the device object that recurs as a raw `serde_json::Value` across `connect`/`refer`/`dial`/`tap` (and the serial/parallel matrix `[[device]]`). Types the SHAPE only — `type` stays a `String` because the discriminant (phone/sip/webrtc/rtp/…) is NOT enumerated in `relay-protocol/calling.{dial,connect,refer,tap}.params.json`. Additive: every raw-`Value` entry point is unchanged, and `Device::to_value()` serialises byte-identical to the hand-written `json!({"type":…,"params":…})`. No Python equivalent (Python passes a raw dict).

signalwire.relay.device.Device: Typed RELAY device descriptor (`device_type: String` + `params: Value`); types the wire shape only. See section preamble.
signalwire.relay.device.Device.__init__: `Device::new(type, params)` constructor; non-object params normalise to `{}` on the wire. See section preamble.
signalwire.relay.device.Device.phone: Convenience constructor for a `phone` device (`{to_number, from_number}`). See section preamble.
signalwire.relay.device.Device.sip: Convenience constructor for a `sip` device (`{to, from}`). See section preamble.
signalwire.relay.device.Device.to_value: Serialise to the wire device object `{"type":…,"params":{…}}`, byte-identical to the hand-written form. See section preamble.
signalwire.relay.device.Device.matrix: Build the serial/parallel device matrix (`[[device,…],…]`) that dial/connect take, from rows of Devices. See section preamble.

### Rust REST HTTP transport types

Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.

signalwire.rest.http_client.HttpClient: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.__init__: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.auth_header: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.base_url: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.delete: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.get: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.list_all: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.patch: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.post: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.project_id: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.put: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.token: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.HttpClient.with_stub: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.StubTransport: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.StubTransport.__init__: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.StubTransport.set_response: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.UreqTransport: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.
signalwire.rest.http_client.UreqTransport.__init__: Rust splits the HTTP transport into a public HttpClient with two pluggable transport types (UreqTransport for production, StubTransport for tests). Python uses requests directly.

### Rust REST error type

Rust uses Result<_, SignalWireRestError> for REST failures. Python uses a custom Exception class with the same name. The fields (status_code, message, response_body) match.

signalwire.rest.error.SignalWireRestError.__init__: Rust uses Result<_, SignalWireRestError> for REST failures. Python uses a custom Exception class with the same name. The fields (status_code, message, response_body) match.
signalwire.rest.error.SignalWireRestError.message: Rust uses Result<_, SignalWireRestError> for REST failures. Python uses a custom Exception class with the same name. The fields (status_code, message, response_body) match.
signalwire.rest.error.SignalWireRestError.response_body: Rust uses Result<_, SignalWireRestError> for REST failures. Python uses a custom Exception class with the same name. The fields (status_code, message, response_body) match.
signalwire.rest.error.SignalWireRestError.status_code: Rust uses Result<_, SignalWireRestError> for REST failures. Python uses a custom Exception class with the same name. The fields (status_code, message, response_body) match.

### Rust REST namespace methods

Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.

signalwire.rest.namespaces.calling.Calling: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.__init__: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.ai_hold: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.ai_message: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.ai_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.ai_unhold: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.base_path: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.client: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.collect: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.collect_start_input_timers: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.collect_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.denoise: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.denoise_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.detect: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.detect_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.dial: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.disconnect: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.end: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.live_transcribe: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.live_translate: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.play: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.play_pause: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.play_resume: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.play_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.play_volume: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.project_id: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.receive_fax_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.record: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.record_pause: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.record_resume: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.record_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.refer: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.send_fax_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.stream: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.stream_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.tap: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.tap_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.transcribe: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.transcribe_stop: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.transfer: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.update_call: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.calling.Calling.user_event: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.__init__: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.addresses: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.ai_agents: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.call_flows: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.call_queues: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.client: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.conference_rooms: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.conversations: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.dial_plans: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.freeclimb_apps: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.phone_numbers: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.sip_endpoints: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.sip_profiles: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.subscribers: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.
signalwire.rest.namespaces.fabric.Fabric.swml_scripts: Rust REST namespaces (Calling, Fabric, …) ship explicit methods for every documented operation (Calling.play, Calling.dial, Fabric.subscribers, …). These are the typed equivalents of Python's dynamic-attribute access on the same namespace classes.

### Rust RelayClient methods

Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.

signalwire.relay.client.RelayClient.authenticate: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.authenticate_blocking: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.bump_reconnect_delay: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.connect_fresh: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.from_env: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.get_call: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.get_message: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.handle_event: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.handle_message: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.is_connected: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.is_running: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.on_event: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.reconnect: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.register_dial: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.register_pending: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.remove_pending_dial: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.send: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.send_ack: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.send_request: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.
signalwire.relay.client.RelayClient.track_message: Rust splits the RelayClient surface into explicit methods (authenticate, send_request, …) where Python uses dynamic dispatch (RelayClient.execute, send_message). These are the equivalent typed methods.

### Rust RestClient namespace accessors and helpers

Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.

signalwire.rest.client.RestClient.addresses: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.base_url: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.calling: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.chat: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.datasphere: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.fabric: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.from_env: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.http: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.imported_numbers: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.logs: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.lookup: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.mfa: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.number_groups: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.phone_numbers: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.project: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.project_id: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.pubsub: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.queues: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.recordings: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.registry: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.short_codes: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.sip_profile: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.space: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.token: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.verified_callers: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.video: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.with_base_url: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.
signalwire.rest.client.RestClient.with_http: Rust ships every REST namespace as a method on RestClient (calling, fabric, phone_numbers, …) — these methods are required by users to access the namespaces. Python users access namespaces via attribute access on RestClient. The data and behaviour are equivalent.

### Rust SWMLService methods Python doesn't expose at the same name

These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.

signalwire.core.swml_service.SWMLService.basic_auth_credentials: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.define_tool: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.document: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.document_mut: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.get_proxy_url_base: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.handle_request: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.has_tool: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.host: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.list_tool_names: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.name: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.on_function_call: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.port: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.register_swaig_function: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.render: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.render_pretty: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.route: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.run: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.sleep: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.
signalwire.core.swml_service.SWMLService.tool_definition: These are Rust-idiomatic accessors / module helpers (basic_auth_credentials, document, document_mut, get_proxy_url_base, handle_request, has_tool, host, list_tool_names, name, on_function_call, port, register_swaig_function, render, render_pretty, route, run, sleep, tool_definition, define_tool). Python's SWMLService surface relies on dynamic attribute lookup and FastAPI router methods; Rust types them explicitly.

### Rust SkillParams typed-params helper

Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.

signalwire.skills.skill_base.SkillParams: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.__init__: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.empty: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_array: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_bool: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_bool_or: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_f64: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_i64: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_object: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_str: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.SkillParams.get_str_or: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.
signalwire.skills.skill_base.value_to_map: Rust ships SkillParams + value_to_map as typed helpers for parameter unpacking. Python uses Dict[str, Any] directly.

### Rust SkillName closed-set enum

rust_enum_idiom: typed closed set of the 18 built-in skill names. add_skill/remove_skill/has_skill keep their &str parameter (parity with Python's bare str + custom skills); SkillName plugs in via as_str()/AsRef<str>/Display so a typo like add_skill("datetiem") that Python only catches at the server fails at the Rust call site instead, with editor autocomplete and exhaustive matching. Wire behaviour is identical (normalizes to the same snake_case string).

signalwire.skills.skill_name.SkillName: rust_enum_idiom: typed closed set of the 18 built-in skill names; add_skill/remove_skill/has_skill keep their &str param (Python parity + custom skills) and accept this enum via as_str()/AsRef<str>. Wire behaviour identical.
signalwire.skills.skill_name.SkillName.as_str: rust_enum_idiom: returns the canonical snake_case wire name (see SkillName) — the exact string add_skill(&str) expects, so the enum and string paths load the identical skill.
signalwire.skills.skill_name.SkillName.all: rust_enum_idiom: &'static slice of every built-in SkillName for exhaustive iteration (see SkillName).
signalwire.skills.skill_name.SkillName.from_str: rust_enum_idiom: parse a wire name back to a SkillName, None for custom/third-party names (see SkillName).

### Rust SWML media-action closed-set enums

rust_enum_idiom: typed closed sets for FunctionResult media-action parameters that the Python reference validates against a fixed list and rejects with ValueError otherwise — record_call(format in [wav,mp3,mp4]; direction in [speak,listen,both]) and tap(direction in [speak,hear,both]; codec in [PCMU,PCMA]). As of the wave-1 closed-set adoption the oracle emits these four params as `enum<…>` and the audit REQUIRES a typed port form (not a bare `string`); FunctionResult.record_call/tap therefore take `format: impl Into<MediaArg<RecordFormat>>` etc. — a generic that accepts BOTH the typed enum (`RecordFormat::Mp3`) AND a raw wire string (`"mp3"`, for Python parity / forward-compat). The raw arm carries the string verbatim into the method body, where the unchanged closed-set check rejects an out-of-set value with the reference's exact ValueError text; the typed arm is always valid by construction. Wire/SWML output is byte-identical between the two call styles. record_call's direction (listen) and tap's direction (hear) are deliberately two enums mirroring the reference's two separate validation lists. (The adapter unwraps `impl Into<MediaArg<E>>` to the inner enum, so the param surfaces as `class:…RecordFormat` — the typed closed set the oracle's `enum<…>` describes.)

signalwire.swaig.media_enums.MediaArg: rust_enum_idiom: the typed-or-raw parameter wrapper behind record_call(format,direction) / tap(direction,codec). `MediaArg<E>` is `Typed(E)` (the closed-set enum) OR `Raw(String)` (a raw wire string carried verbatim, validated in the method body exactly as Python validates its str arg). Lets a single `impl Into<MediaArg<E>>` param accept both `RecordFormat::Wav` and `"wav"` with no overloads and byte-identical wire output. No Python equivalent (Python's param is a bare `str`); this is the typed-form the wave-1 oracle's `enum<…>` contract requires while keeping the str path for forward-compat.
signalwire.swaig.media_enums.MediaArg.wire: rust_enum_idiom: the canonical wire string this arg resolves to — `as_str()` for the Typed arm, the string verbatim for the Raw arm. The method body validates this against the closed set before emitting, so both call styles yield identical SWML (see MediaArg).
signalwire.swaig.function_result.KeysArg: rust_union_idiom: the `Union[str, List[str]]` argument wrapper behind FunctionResult.remove_global_data / remove_metadata (Python's `keys: Union[str, List[str]]`). `KeysArg` is `One(String)` (a single key) OR `Many(Vec<String>)` (a key list); a single `impl Into<KeysArg>` param accepts both `"plan"` and `vec!["plan","chips"]` with no overloads. Restores Python's pass-through emission: the One arm emits the BARE KEY STRING (`{"unset_global_data":"plan"}`), the Many arm emits the ARRAY (`{"unset_global_data":["plan","chips"]}`) — byte-identical to the reference per arm (the prior bare-`Vec<&str>` signature could only emit the array, wrongly wrapping a single key; the cross-port emission differ caught it). No Python equivalent symbol (Python uses an untagged union on a bare `str`/`list`); this is the Rust typed-form that models it.
signalwire.swaig.function_result.KeysArg.into_value: rust_union_idiom: the JSON wire value this arg resolves to — a bare string for the One arm, an array for the Many arm — matching Python's verbatim pass-through of its `Union[str, List[str]]` value (see KeysArg).
signalwire.swaig.media_enums.RecordFormat: rust_enum_idiom: typed {wav,mp3,mp4} for FunctionResult.record_call(format) — mirrors Python's `format in ["wav","mp3","mp4"]` validation. record_call accepts this directly via `impl Into<MediaArg<RecordFormat>>` (and the raw `&str` still works). Wire output identical.
signalwire.swaig.media_enums.RecordFormat.as_str: rust_enum_idiom: canonical wire string for the format (see RecordFormat) — the exact string record_call(&str) expects.
signalwire.swaig.media_enums.RecordFormat.all: rust_enum_idiom: &'static slice of every RecordFormat for exhaustive iteration (see RecordFormat).
signalwire.swaig.media_enums.RecordFormat.from_str: rust_enum_idiom: parse a wire string to a RecordFormat, None for anything the reference would reject (see RecordFormat).
signalwire.swaig.media_enums.RecordDirection: rust_enum_idiom: typed {speak,listen,both} for FunctionResult.record_call(direction) — mirrors Python's `direction in ["speak","listen","both"]` validation. Distinct from TapDirection (uses hear, not listen). record_call keeps &str. Wire output identical.
signalwire.swaig.media_enums.RecordDirection.as_str: rust_enum_idiom: canonical wire string for the direction (see RecordDirection) — the exact string record_call(&str) expects.
signalwire.swaig.media_enums.RecordDirection.all: rust_enum_idiom: &'static slice of every RecordDirection for exhaustive iteration (see RecordDirection).
signalwire.swaig.media_enums.RecordDirection.from_str: rust_enum_idiom: parse a wire string to a RecordDirection, None for anything the reference would reject — including `hear`, which is valid only for tap (see RecordDirection).
signalwire.swaig.media_enums.TapDirection: rust_enum_idiom: typed {speak,hear,both} for FunctionResult.tap(direction) — mirrors Python's `valid_directions = ["speak","hear","both"]` validation. Distinct from RecordDirection (uses listen, not hear). tap keeps &str. Wire output identical.
signalwire.swaig.media_enums.TapDirection.as_str: rust_enum_idiom: canonical wire string for the direction (see TapDirection) — the exact string tap(&str) expects.
signalwire.swaig.media_enums.TapDirection.all: rust_enum_idiom: &'static slice of every TapDirection for exhaustive iteration (see TapDirection).
signalwire.swaig.media_enums.TapDirection.from_str: rust_enum_idiom: parse a wire string to a TapDirection, None for anything the reference would reject — including `listen`, which is valid only for record_call (see TapDirection).
signalwire.swaig.media_enums.Codec: rust_enum_idiom: typed {PCMU,PCMA} for FunctionResult.tap(codec) — mirrors Python's `valid_codecs = ["PCMU","PCMA"]` validation. tap keeps &str, accepts this via as_str()/AsRef<str>. Wire output identical (upper-case strings).
signalwire.swaig.media_enums.Codec.as_str: rust_enum_idiom: canonical upper-case wire string for the codec (see Codec) — the exact string tap(&str) expects.
signalwire.swaig.media_enums.Codec.all: rust_enum_idiom: &'static slice of every Codec for exhaustive iteration (see Codec).
signalwire.swaig.media_enums.Codec.from_str: rust_enum_idiom: parse a wire string to a Codec (case-sensitive, mirroring the reference's literal list), None otherwise (see Codec).

### Typed SWAIG tool-parameter builder (Tier-2 flagship idiom pass)

`define_tool(parameters: Value)` takes the SWAIG argument schema as an untyped, hand-written `json!({ ... })` `properties` blob (Python passes the same as a `Dict[str,Any]`). `ParamsBuilder`/`PropertyBuilder`/`ParamKind` are an ADDITIVE typed convenience over the EXACT SAME wire output: `build()` returns the byte-identical `properties` object `define_tool` already accepts, `build_schema()` returns the byte-identical full `{"type":"object","properties":{…},"required":[…]}` schema (Python's `_ensure_parameter_structure` output) for the `register_swaig_function` / DataMap path. The untyped path is unchanged; no Python-reference symbol corresponds (this is pure Rust idiom — fluent + `#[must_use]`). Closed-set properties integrate the Tier-1 media enums via their `all()`/`AsRef<str>`.

signalwire.swaig.params_builder.ParamKind: rust-builder-idiom: JSON-Schema primitive-type enum ({string,number,integer,boolean,array,object}) used by the typed param builder; renders the literal `"type"` value. No Python equivalent (Python writes the type string inline).
signalwire.swaig.params_builder.ParamKind.as_str: rust-builder-idiom: canonical JSON-Schema `"type"` string for the kind (e.g. `"integer"`) — what lands in the schema, byte-identical to the hand-written literal.
signalwire.swaig.params_builder.ParamsBuilder: rust-builder-idiom: fluent typed builder for a SWAIG tool's parameter schema; produces the SAME untyped `serde_json::Value` `define_tool` already accepts. Additive convenience, untyped path unchanged.
signalwire.swaig.params_builder.ParamsBuilder.__init__: rust-builder-idiom: `ParamsBuilder::new()` — start an empty parameter schema (enumerator maps Rust `new` → `__init__`).
signalwire.swaig.params_builder.ParamsBuilder.array: rust-builder-idiom: add an `array` property with a `ParamKind` element type, emitting `{"type":"array","items":{"type":…}}` — byte-identical to the hand-written form.
signalwire.swaig.params_builder.ParamsBuilder.boolean: rust-builder-idiom: add a `boolean` property with description — byte-identical to the hand-written `{"type":"boolean","description":…}`.
signalwire.swaig.params_builder.ParamsBuilder.build: rust-builder-idiom: render the `properties` object the unchanged `define_tool(parameters)` accepts; byte-identical to the hand-written `json!({…})` properties blob.
signalwire.swaig.params_builder.ParamsBuilder.build_schema: rust-builder-idiom: render the full `{"type":"object","properties":{…},"required":[…]}` schema (Python's `_ensure_parameter_structure` output); byte-identical to the hand-written full schema used with register_swaig_function / DataMap.
signalwire.swaig.params_builder.ParamsBuilder.enum_of: rust-builder-idiom: add a closed-set (`enum`) property from any `impl AsRef<str>` iterator (the Tier-1 media enums plug in via `all()`), emitting `{"type":"string","enum":[…]}` — byte-identical to the hand-written enum schema.
signalwire.swaig.params_builder.ParamsBuilder.integer: rust-builder-idiom: add an `integer` property with description — byte-identical to the hand-written `{"type":"integer","description":…}`.
signalwire.swaig.params_builder.ParamsBuilder.number: rust-builder-idiom: add a `number` (float) property with description — byte-identical to the hand-written `{"type":"number","description":…}`.
signalwire.swaig.params_builder.ParamsBuilder.object: rust-builder-idiom: add a nested `object` property whose shape is another ParamsBuilder, emitting `{"type":"object","properties":{…}}` (plus nested required) — byte-identical to the hand-written nested schema.
signalwire.swaig.params_builder.ParamsBuilder.property: rust-builder-idiom: add a fully-customised property built via PropertyBuilder (escape hatch for default/format/per-property required); inserts its rendered object verbatim.
signalwire.swaig.params_builder.ParamsBuilder.required: rust-builder-idiom: declare the top-level required-parameter list (JSON-Schema sibling of `properties`, == Python's `required=[…]` arg); surfaces in build_schema()'s `"required":[…]`.
signalwire.swaig.params_builder.ParamsBuilder.string: rust-builder-idiom: add a `string` property with description — byte-identical to the hand-written `{"type":"string","description":…}`.
signalwire.swaig.params_builder.PropertyBuilder: rust-builder-idiom: per-property typed builder for options the one-line ParamsBuilder helpers don't cover (default/format/per-property required/nesting); renders the same `{"type":…,"description":…,…}` object as hand-written.
signalwire.swaig.params_builder.PropertyBuilder.__init__: rust-builder-idiom: `PropertyBuilder::new(kind, description)` — start a property of the given ParamKind with an LLM-facing description (enumerator maps `new` → `__init__`).
signalwire.swaig.params_builder.PropertyBuilder.build: rust-builder-idiom: finish the property, yielding its rendered schema object; byte-identical to the hand-written property object.
signalwire.swaig.params_builder.PropertyBuilder.default: rust-builder-idiom: attach a `"default"` value to the property — byte-identical to the hand-written `"default":…`.
signalwire.swaig.params_builder.PropertyBuilder.enum_values: rust-builder-idiom: constrain the property to a closed set (`"enum":[…]`) from any `impl AsRef<str>` iterator — how the Tier-1 media enums integrate.
signalwire.swaig.params_builder.PropertyBuilder.extra: rust-builder-idiom: escape hatch to insert an arbitrary extra schema key (e.g. `"minimum"`, `"pattern"`) without a dedicated helper.
signalwire.swaig.params_builder.PropertyBuilder.format: rust-builder-idiom: attach a JSON-Schema `"format"` hint (`"date"`/`"email"`/…); the format vocabulary is open so this stays a `&str`.
signalwire.swaig.params_builder.PropertyBuilder.items: rust-builder-idiom: set the array element schema, emitting `"items":{"type":…}` — byte-identical to the hand-written array items.
signalwire.swaig.params_builder.PropertyBuilder.properties: rust-builder-idiom: set the nested `properties` (and nested `required`) for an object property from another ParamsBuilder — byte-identical to the hand-written nested object schema.
signalwire.swaig.params_builder.PropertyBuilder.required: rust-builder-idiom: per-property `"required": true` flag (the style some skills use, e.g. the datasphere skill); distinct from ParamsBuilder.required's top-level array.

### Rust SkillRegistry methods

external_paths returns the directories registered via add_skill_directory (mirroring Python's _external_paths attribute). get_factory returns a closure-wrapped factory (Rust replacement for Python's get_skill / get_skill_class).

signalwire.skills.registry.SkillRegistry.external_paths: external_paths returns the directories registered via add_skill_directory (mirroring Python's _external_paths attribute). get_factory returns a closure-wrapped factory (Rust replacement for Python's get_skill / get_skill_class).
signalwire.skills.registry.SkillRegistry.get_factory: external_paths returns the directories registered via add_skill_directory (mirroring Python's _external_paths attribute). get_factory returns a closure-wrapped factory (Rust replacement for Python's get_skill / get_skill_class).

### Rust action module helper

Rust action.rs file-private helpers; module-level identifiers that don't have a Python equivalent.

signalwire.relay.action.__init__: Rust action.rs file-private helpers; module-level identifiers that don't have a Python equivalent.
signalwire.relay.action.action: Rust action.rs file-private helpers; module-level identifiers that don't have a Python equivalent.

### Rust constants module helpers

Rust ships free functions in relay::constants for terminal-state checks. Python exposes the same predicates as static methods on the Action/Call/Message classes.

signalwire.relay.constants.is_action_terminal: Rust ships free functions in relay::constants for terminal-state checks. Python exposes the same predicates as static methods on the Action/Call/Message classes.
signalwire.relay.constants.is_call_terminal: Rust ships free functions in relay::constants for terminal-state checks. Python exposes the same predicates as static methods on the Action/Call/Message classes.
signalwire.relay.constants.is_message_terminal: Rust ships free functions in relay::constants for terminal-state checks. Python exposes the same predicates as static methods on the Action/Call/Message classes.

### Rust constructor-options struct

Idiomatic Rust uses options/builder structs (AgentOptions) rather than long kwargs. Python has the same parameters as keyword arguments to AgentBase.__init__.

signalwire.agent.agent_base.AgentOptions: Idiomatic Rust uses options/builder structs (AgentOptions) rather than long kwargs. Python has the same parameters as keyword arguments to AgentBase.__init__.
signalwire.agent.agent_base.AgentOptions.__init__: Idiomatic Rust uses options/builder structs (AgentOptions) rather than long kwargs. Python has the same parameters as keyword arguments to AgentBase.__init__.

### Rust constructor-options struct for BedrockAgent

Same idiom as AgentOptions — Rust prefers options structs over long kwarg lists.

signalwire.agents.bedrock.BedrockOptions: Same idiom as AgentOptions — Rust prefers options structs over long kwarg lists.
signalwire.agents.bedrock.BedrockOptions.with_name: Same idiom as AgentOptions — Rust prefers options structs over long kwarg lists.

### Rust context-builder accessor methods

Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.

signalwire.core.contexts.Context.get_step_mut: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Context.name: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Context.set_prompt_text: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Context.step_order: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Context.steps: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Context.to_value: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.ContextBuilder.attach_tool_name_supplier: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.ContextBuilder.get_context_mut: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.ContextBuilder.has_contexts: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.ContextBuilder.to_value: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.GatherInfo.completion_action: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.GatherInfo.questions: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.GatherInfo.to_value: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.GatherQuestion.key: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.GatherQuestion.to_value: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Step.gather_info: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Step.name: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Step.to_value: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Step.valid_contexts: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.
signalwire.core.contexts.Step.valid_steps: Rust uses to_value() for serialization rather than to_dict(). The other names are typed accessors that mirror Python's attribute access on the same data shape.

### Rust contexts::context_builder helper

Module-path artefact of Rust's mod hierarchy. Python ships the same helper at signalwire.core.contexts.create_simple_context.

signalwire.contexts.context_builder.create_simple_context: Module-path artefact of Rust's mod hierarchy. Python ships the same helper at signalwire.core.contexts.create_simple_context.

### Rust custom_skills skill

Rust ships custom_skills as a registered skill. Python doesn't have it as a separate module.

signalwire.skills.custom_skills.skill.CustomSkillsSkill: Rust ships custom_skills as a registered skill. Python doesn't have it as a separate module.

### Rust logging::Logger and Level types

Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).

signalwire.logging.Level: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Level.all: rust_enum_idiom: &'static slice of every Level (debug/info/warn/error) in ascending-severity order for exhaustive iteration. Python uses the stdlib logging module; this is the Rust enum's closed-set helper.
signalwire.logging.Level.as_str: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Level.from_str: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger.__init__: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger.debug: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger.error: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger.info: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger.log: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger.should_log: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.Logger.warn: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).
signalwire.logging.init: Rust ships an explicit Logger type and Level enum with associated helpers. Python uses the standard logging module with helper wrappers in signalwire.core.logging_config (which is in PORT_OMISSIONS.md).

### Rust per-action accessors

Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.

signalwire.relay.call.CollectAction.action: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.CollectAction.collect_result: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.CollectAction.handle_event_filtered: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.DetectAction.detect_result: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.FaxAction.action: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.FaxAction.fax_type: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.RecordAction.duration: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.RecordAction.size: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.
signalwire.relay.call.RecordAction.url: Specific accessors on the per-action Rust subtypes (CollectAction.collect_result, DetectAction.detect_result, FaxAction.action / fax_type, RecordAction.duration / size / url). Python exposes the same data via attribute access.

### Rust port-only addition

Rust-idiomatic API; no direct Python equivalent in the auto-generated python_surface.json.

signalwire.relay.event.Event: Rust-idiomatic API; no direct Python equivalent in the auto-generated python_surface.json.
signalwire.rest.error.SignalWireRestError: Rust-idiomatic API; no direct Python equivalent in the auto-generated python_surface.json.

### Rust prefab accessors

Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.

signalwire.prefabs.concierge.ConciergeAgent.agent: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.concierge.ConciergeAgent.agent_mut: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.concierge.ConciergeAgent.amenities: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.concierge.ConciergeAgent.services: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.concierge.ConciergeAgent.venue_name: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.faq_bot.FAQBotAgent.agent: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.faq_bot.FAQBotAgent.agent_mut: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.faq_bot.FAQBotAgent.faqs: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.faq_bot.FAQBotAgent.suggest_related: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.info_gatherer.InfoGathererAgent.agent: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.info_gatherer.InfoGathererAgent.agent_mut: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.info_gatherer.InfoGathererAgent.questions: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.receptionist.ReceptionistAgent.agent: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.receptionist.ReceptionistAgent.agent_mut: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.receptionist.ReceptionistAgent.departments: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.receptionist.ReceptionistAgent.greeting: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.survey.SurveyAgent.agent: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.survey.SurveyAgent.agent_mut: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.survey.SurveyAgent.survey_name: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.
signalwire.prefabs.survey.SurveyAgent.survey_questions: Rust prefabs (InfoGathererAgent, SurveyAgent, ReceptionistAgent, FAQBotAgent, ConciergeAgent) wrap an AgentBase via composition + Deref. The agent / agent_mut accessors expose the wrapped AgentBase explicitly. Python prefabs subclass AgentBase so attribute access is direct.

### Rust serverless adapter

Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.

signalwire.serverless.adapter.Adapter: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.
signalwire.serverless.adapter.Adapter.detect: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.
signalwire.serverless.adapter.Adapter.handle_azure: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.
signalwire.serverless.adapter.Adapter.handle_lambda: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.
signalwire.serverless.adapter.Adapter.serve_detect: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.
signalwire.serverless.adapter.Adapter.status_text: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.
signalwire.serverless.adapter.RuntimeEnvironment: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.
signalwire.serverless.adapter.RuntimeEnvironment.as_str: Rust ships a typed Adapter / RuntimeEnvironment pair for serverless mode detection and event handling. Python uses a flatter signalwire/serverless module.

### Rust skill constructors

Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.

signalwire.skills.claude_skills.skill.ClaudeSkillsSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.custom_skills.skill.CustomSkillsSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.datasphere.skill.DataSphereSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.datetime.skill.DateTimeSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.google_maps.skill.GoogleMapsSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.info_gatherer.skill.InfoGathererSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.joke.skill.JokeSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.math.skill.MathSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.swml_transfer.skill.SWMLTransferSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.web_search.skill.WebSearchSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.__init__: Each Rust skill exposes a constructor (mapped to __init__). Python skills also have constructors but the per-symbol enumerator may not list them under the same path.

### Rust swml::schema introspection helpers

Rust ships free functions for schema introspection. Python provides equivalents via signalwire.utils.schema_utils.SchemaUtils (which is in PORT_OMISSIONS.md).

signalwire.swml.schema.get_verb: Rust ships free functions for schema introspection. Python provides equivalents via signalwire.utils.schema_utils.SchemaUtils (which is in PORT_OMISSIONS.md).
signalwire.swml.schema.get_verb_names: Rust ships free functions for schema introspection. Python provides equivalents via signalwire.utils.schema_utils.SchemaUtils (which is in PORT_OMISSIONS.md).
signalwire.swml.schema.is_valid_verb: Rust ships free functions for schema introspection. Python provides equivalents via signalwire.utils.schema_utils.SchemaUtils (which is in PORT_OMISSIONS.md).
signalwire.swml.schema.verb_count: Rust ships free functions for schema introspection. Python provides equivalents via signalwire.utils.schema_utils.SchemaUtils (which is in PORT_OMISSIONS.md).

### SessionManager Rust constructors / accessors

with_defaults is Rust's named-constructor for SessionManager::new() with the standard 3600s expiry; the others surface internal state / one-off helpers that the Python class hides as private.

signalwire.core.security.session_manager.SessionManager.create_token: with_defaults is Rust's named-constructor for SessionManager::new() with the standard 3600s expiry; the others surface internal state / one-off helpers that the Python class hides as private.
signalwire.core.security.session_manager.SessionManager.token_expiry_secs: with_defaults is Rust's named-constructor for SessionManager::new() with the standard 3600s expiry; the others surface internal state / one-off helpers that the Python class hides as private.
signalwire.core.security.session_manager.SessionManager.with_defaults: with_defaults is Rust's named-constructor for SessionManager::new() with the standard 3600s expiry; the others surface internal state / one-off helpers that the Python class hides as private.

### SkillManager Rust API

Rust SkillManager exposes list_skills (mirrors Python's list_loaded_skills) and load_skill_instance (factory-driven instantiation, since Rust does not load .py files at runtime).

signalwire.core.skill_manager.SkillManager.list_skills: Rust SkillManager exposes list_skills (mirrors Python's list_loaded_skills) and load_skill_instance (factory-driven instantiation, since Rust does not load .py files at runtime).
signalwire.core.skill_manager.SkillManager.load_skill_instance: Rust SkillManager exposes list_skills (mirrors Python's list_loaded_skills) and load_skill_instance (factory-driven instantiation, since Rust does not load .py files at runtime).

### Top-level re-export at the crate root

Rust idiom: pub use exposes types under signalwire::Foo so users can `use signalwire::AgentBase` directly. Python lists these too but the dotted-path enumerator records them differently.

signalwire.AgentBase: Rust idiom: pub use exposes types under signalwire::Foo so users can `use signalwire::AgentBase` directly. Python lists these too but the dotted-path enumerator records them differently.
signalwire.AgentOptions: Rust idiom: pub use exposes types under signalwire::Foo so users can `use signalwire::AgentBase` directly. Python lists these too but the dotted-path enumerator records them differently.
signalwire.AgentServer: Rust idiom: pub use exposes types under signalwire::Foo so users can `use signalwire::AgentBase` directly. Python lists these too but the dotted-path enumerator records them differently.
signalwire.SWMLService: Rust idiom: pub use exposes types under signalwire::Foo so users can `use signalwire::AgentBase` directly. Python lists these too but the dotted-path enumerator records them differently.

### Rust function-field hook setters (no method overriding via inheritance)

Rust has no method overriding via embedded structs alone. Where Python exposes a subclass-overridable method on WebMixin (on_swml_request), the Rust port exposes a typed set_<name>_hook setter that registers a closure. This is the idiomatic Rust override pattern; the function-field hook is invoked from the corresponding accessor on Service. The hook setter has no Python equivalent because Python overrides via subclassing — the *capability* is mirrored, but the binding shape is port-native.

signalwire.core.swml_service.SWMLService.set_on_swml_request_hook: Rust function-field hook setter used in place of subclass override; Python's WebMixin.on_swml_request is overridden via subclassing, but Rust has no method inheritance so the hook is registered as a closure on Service.

signalwire.utils.schema_utils.SchemaUtils.generate_method_signature: Python-source codegen helper; canonical Python signatures filter this method out (Python-only output shape)
signalwire.utils.schema_utils.SchemaUtils.generate_method_body: Python-source codegen helper; canonical Python signatures filter this method out (Python-only output shape)
signalwire.utils.schema_utils.SchemaUtils.full_validation_available: @property in Python (filtered as bool-returning attribute); ports expose it as an explicit method per spec

### REST namespaces — explicit CRUD where Python uses inheritance

The Python SDK derives DatasphereDocuments /
NumberGroupsResource / QueuesResource from CrudResource and uses class
inheritance to hand off list / create / get / update / delete. Rust has
no class inheritance; the port emits each method explicitly on every
struct so the user-facing surface matches one-to-one. This is the same
"flatten-the-MRO" pattern the diff already excuses for the Agent / Skill
side.

signalwire.rest.namespaces.datasphere.DatasphereDocuments.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.datasphere.DatasphereDocuments.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.datasphere.DatasphereDocuments.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.datasphere.DatasphereDocuments.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.datasphere.DatasphereDocuments.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.number_groups.NumberGroupsResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.number_groups.NumberGroupsResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.number_groups.NumberGroupsResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.number_groups.NumberGroupsResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.number_groups.NumberGroupsResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.queues.QueuesResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.queues.QueuesResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.queues.QueuesResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.queues.QueuesResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.queues.QueuesResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.

### BedrockAgent methods present in Python's surface but not in its signatures inventory

Python's surface inventory (`python_surface.json`) lists these BedrockAgent methods, but the canonical `python_signatures.json` does not (the Python adapter could not import boto3 to enumerate them). These are NOT port-only additions — they are the same methods Python ships, with corresponding Rust implementations.

signalwire.agents.bedrock.BedrockAgent.__init__: Python ships this in BedrockAgent's source surface; the Python signatures adapter could not enumerate it (missing boto3 in the audit env). Rust matches Python's surface.
signalwire.agents.bedrock.BedrockAgent.set_inference_params: Python ships this in BedrockAgent's source surface; the Python signatures adapter could not enumerate it (missing boto3 in the audit env). Rust matches Python's surface.
signalwire.agents.bedrock.BedrockAgent.set_llm_model: Python ships this in BedrockAgent's source surface; the Python signatures adapter could not enumerate it (missing boto3 in the audit env). Rust matches Python's surface.
signalwire.agents.bedrock.BedrockAgent.set_llm_temperature: Python ships this in BedrockAgent's source surface; the Python signatures adapter could not enumerate it (missing boto3 in the audit env). Rust matches Python's surface.
signalwire.agents.bedrock.BedrockAgent.set_post_prompt_llm_params: Python ships this in BedrockAgent's source surface; the Python signatures adapter could not enumerate it (missing boto3 in the audit env). Rust matches Python's surface.
signalwire.agents.bedrock.BedrockAgent.set_prompt_llm_params: Python ships this in BedrockAgent's source surface; the Python signatures adapter could not enumerate it (missing boto3 in the audit env). Rust matches Python's surface.
signalwire.agents.bedrock.BedrockAgent.set_voice: Python ships this in BedrockAgent's source surface; the Python signatures adapter could not enumerate it (missing boto3 in the audit env). Rust matches Python's surface.

### Rust AgentBase / SWMLService extras

signalwire.core.agent_base.AgentBase.create_tool_token: Rust convenience method that mints a one-shot tool-call token for use in subsequent SWAIG callbacks. Python users hit StateMixin.validate_tool_token for the inverse half; Rust ships both halves on AgentBase.
signalwire.core.swml_service.SWMLService.get_basic_auth_credentials_with_source: Rust variant of get_basic_auth_credentials that returns a 3-tuple (user, pass, source). Python uses the same method's `include_source=True` flag for this; Rust splits into two methods to keep the return type monomorphic.

### Rust Action subclass accessors

signalwire.relay.call.DetectAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.
signalwire.relay.call.PlayAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.
signalwire.relay.call.AIAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.
signalwire.relay.call.PayAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.
signalwire.relay.call.StreamAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.
signalwire.relay.call.TapAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.
signalwire.relay.call.TranscribeAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.
signalwire.relay.call.RecordAction.action: Rust Action subclasses expose an ``action()`` accessor that returns the underlying Action struct (the inner state shared across all action variants). Python uses inheritance, so this accessor does not exist as a method.

### Rust RelayClient blocking variants

These are blocking-IO siblings of Python's async RelayClient methods. Python's reference is async-only because the SDK lives inside FastAPI's event loop; the Rust port additionally exposes a blocking API for synchronous calling-code paths.

signalwire.relay.client.RelayClient.dial_blocking: Rust ships blocking-IO variants of the async dial/execute/send_message methods so synchronous Rust code can invoke RELAY without spinning up a tokio runtime. Python's RelayClient is async-only.
signalwire.relay.client.RelayClient.execute_blocking: Rust ships blocking-IO variants of the async dial/execute/send_message methods so synchronous Rust code can invoke RELAY without spinning up a tokio runtime. Python's RelayClient is async-only.
signalwire.relay.client.RelayClient.send_message_blocking: Rust ships blocking-IO variants of the async dial/execute/send_message methods so synchronous Rust code can invoke RELAY without spinning up a tokio runtime. Python's RelayClient is async-only.

### Rust top-level re-exports

signalwire.SkillSpec: top-level re-export: Rust exposes SkillSpec at the crate root for ergonomic `signalwire::SkillSpec` access; Python's equivalent is internal to the skill registry. The struct itself is a Rust idiom — Python uses raw class objects passed to `register_skill(...)`.
signalwire.SkillSpec.__init__: top-level re-export: Rust exposes SkillSpec at the crate root for ergonomic `signalwire::SkillSpec` access; Python's equivalent is internal to the skill registry. The struct itself is a Rust idiom — Python uses raw class objects passed to `register_skill(...)`.

### AgentBase prompt_mixin / state_mixin lifted methods

prompt_mixin_lifted / state_mixin_lifted: Rust folds Python's PromptMixin and StateMixin onto AgentBase directly so callers don't reach into a sub-object — same pattern as the documented tool_mixin_lifted bucket above. Python keeps these methods on the originating mixins; Rust hangs them on AgentBase.

signalwire.core.agent_base.AgentBase.get_contexts: prompt_mixin_lifted: Rust AgentBase exposes a get_contexts() accessor; Python's equivalent lives on PromptMixin (mirrors tool_mixin_lifted pattern).
signalwire.core.agent_base.AgentBase.get_post_prompt: prompt_mixin_lifted: Rust rolls up PromptMixin onto AgentBase; Python keeps these on PromptMixin (mirrors tool_mixin_lifted pattern).
signalwire.core.agent_base.AgentBase.get_raw_prompt: prompt_mixin_lifted: Rust rolls up PromptMixin onto AgentBase; Python keeps these on PromptMixin (mirrors tool_mixin_lifted pattern).
signalwire.core.agent_base.AgentBase.pom: prompt_mixin_lifted: Rust accessor returning the underlying PromptObjectModel; Python keeps the POM private inside PromptMixin (mirrors tool_mixin_lifted pattern).
signalwire.core.agent_base.AgentBase.set_prompt_pom: prompt_mixin_lifted: Rust rolls up PromptMixin onto AgentBase; Python keeps these on PromptMixin (mirrors tool_mixin_lifted pattern).
signalwire.core.agent_base.AgentBase.validate_tool_token: state_mixin_lifted: Rust rolls up StateMixin onto AgentBase; Python keeps validate_tool_token on StateMixin (mirrors tool_mixin_lifted pattern).

### SWMLService tool_mixin / web_mixin / auth_mixin lifted methods

tool_mixin_lifted / web_mixin_lifted / auth_mixin_lifted: Rust folds Python's ToolMixin / WebMixin / AuthMixin onto SWMLService directly so subclasses (notably AgentBase) inherit them without a separate composition step — same pattern as the documented tool_mixin_lifted bucket above.

signalwire.core.swml_service.SWMLService.get_all_functions: tool_mixin_lifted: Rust exposes the tool registry's accessors directly on SWMLService; Python keeps these on ToolRegistry (accessed via agent.tool_registry.get_all_functions()).
signalwire.core.swml_service.SWMLService.get_function: tool_mixin_lifted: Rust exposes the tool registry's accessors directly on SWMLService; Python keeps these on ToolRegistry (mirrors tool_mixin_lifted pattern).
signalwire.core.swml_service.SWMLService.has_function: tool_mixin_lifted: Rust exposes the tool registry's accessors directly on SWMLService; Python keeps these on ToolRegistry (mirrors tool_mixin_lifted pattern).
signalwire.core.swml_service.SWMLService.on_swml_request: web_mixin_lifted: Rust rolls up WebMixin onto SWMLService so subclasses (notably AgentBase) can override the SWML-request hook directly; Python keeps on_swml_request on WebMixin (mirrors tool_mixin_lifted pattern).
signalwire.core.swml_service.SWMLService.remove_function: tool_mixin_lifted: Rust exposes the tool registry's mutators directly on SWMLService; Python keeps these on ToolRegistry (mirrors tool_mixin_lifted pattern).
signalwire.core.swml_service.SWMLService.schema_utils: tool_mixin_lifted: Rust exposes a schema_utils() accessor on SWMLService for the SWML schema validator; Python imports `signalwire.utils.schema_utils` directly (mirrors tool_mixin_lifted pattern).
signalwire.core.swml_service.SWMLService.validate_basic_auth: auth_mixin_lifted: Rust rolls up AuthMixin onto SWMLService; Python keeps validate_basic_auth on AuthMixin and accesses it via the mixin chain (mirrors tool_mixin_lifted pattern).

### PaginatedIterator Rust-side field accessors

idiomatic_getter: Rust models PaginatedIterator as a struct whose fields are exposed via accessor functions (`pub fn data_key(&self) -> &str`, etc.). Python uses plain attribute access on the iterator instance; the same data is reachable, just without explicit getters.

signalwire.rest._pagination.PaginatedIterator.data_key: idiomatic_getter: Rust accessor for the underlying field; Python uses attribute access on the iterator object.
signalwire.rest._pagination.PaginatedIterator.http: idiomatic_getter: Rust accessor for the underlying field; Python uses attribute access on the iterator object.
signalwire.rest._pagination.PaginatedIterator.index: idiomatic_getter: Rust accessor for the underlying field; Python uses attribute access on the iterator object.
signalwire.rest._pagination.PaginatedIterator.is_done: idiomatic_getter: Rust accessor for the underlying field; Python uses attribute access on the iterator object (Python's equivalent is `not iter._has_more`).
signalwire.rest._pagination.PaginatedIterator.items: idiomatic_getter: Rust accessor for the underlying field; Python uses attribute access on the iterator object.
signalwire.rest._pagination.PaginatedIterator.next_item: idiomatic_getter: Rust accessor for the next-item helper; Python uses `next(iter)` directly via the iterator protocol.
signalwire.rest._pagination.PaginatedIterator.params: idiomatic_getter: Rust accessor for the underlying field; Python uses attribute access on the iterator object.
signalwire.rest._pagination.PaginatedIterator.path: idiomatic_getter: Rust accessor for the underlying field; Python uses attribute access on the iterator object.

### REST namespace field accessors (base_path / client / project_id / sub-resource getters)

namespace_field_accessor: Rust REST namespaces are structs whose fields are exposed via accessor functions (`pub fn base_path(&self) -> &str`, `pub fn client(&self) -> &Client`, `pub fn subscribers(&self) -> &Subscribers`, etc.). Python keeps the equivalent state as private attributes accessed via `__dict__` / `self.subscribers`; Rust ships explicit getters so the surface is one-to-one with the C-style API the rest of the language expects.

signalwire.rest.namespaces.calling.CallingNamespace.base_path: namespace_field_accessor: Rust accessor for the namespace's base path; Python uses a private class attribute.
signalwire.rest.namespaces.calling.CallingNamespace.client: namespace_field_accessor: Rust accessor for the parent client reference; Python uses a private attribute.
signalwire.rest.namespaces.calling.CallingNamespace.project_id: namespace_field_accessor: Rust accessor for the project_id field; Python uses an instance attribute.
signalwire.rest.namespaces.chat.ChatResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.datasphere.DatasphereDocuments.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.datasphere.DatasphereNamespace.documents: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricAddresses.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.FabricNamespace.addresses: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.ai_agents: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.call_flows: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.call_queues: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.client: namespace_field_accessor: Rust accessor for the parent client reference; Python uses a private attribute.
signalwire.rest.namespaces.fabric.FabricNamespace.conference_rooms: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.conversations: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.cxml_applications: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.cxml_scripts: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.cxml_webhooks: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.dial_plans: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.freeclimb_apps: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.freeswitch_connectors: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.phone_numbers: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.relay_applications: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.sip_gateways: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.swml_webhooks: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.resources: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.sip_endpoints: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.sip_profiles: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.subscribers: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.swml_scripts: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.fabric.FabricNamespace.tokens: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.logs.LogsNamespace.client: namespace_field_accessor: Rust accessor for the parent client reference; Python uses a private attribute.
signalwire.rest.namespaces.logs.LogsNamespace.conferences: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.logs.LogsNamespace.fax: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.logs.LogsNamespace.messages: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.logs.LogsNamespace.voice: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.mfa.MfaResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.number_groups.NumberGroupsResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.client: namespace_field_accessor: Rust accessor for the parent client reference; Python uses a private attribute.
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.project.ProjectNamespace.tokens: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.project.ProjectTokens.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.pubsub.PubSubResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.queues.QueuesResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.registry.RegistryNamespace.brands: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.registry.RegistryNamespace.campaigns: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.registry.RegistryNamespace.client: namespace_field_accessor: Rust accessor for the parent client reference; Python uses a private attribute.
signalwire.rest.namespaces.registry.RegistryNamespace.numbers: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.registry.RegistryNamespace.orders: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.sip_profile.SipProfileResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.client: namespace_field_accessor: Rust accessor for the parent client reference; Python uses a private attribute.
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoNamespace.client: namespace_field_accessor: Rust accessor for the parent client reference; Python uses a private attribute.
signalwire.rest.namespaces.video.VideoNamespace.conference_tokens: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.video.VideoNamespace.conferences: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.video.VideoNamespace.room_recordings: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.video.VideoNamespace.room_sessions: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.video.VideoNamespace.room_tokens: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.video.VideoNamespace.rooms: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.
signalwire.rest.namespaces.video.VideoNamespace.streams: namespace_field_accessor: Rust sub-resource getter for the namespace; Python uses attribute access on the namespace instance.

### Rust REST resource constructors and explicit CRUD (flatten-the-MRO)

Rust port emits explicit `__init__` constructors and CRUD methods on each resource struct since Rust has no class inheritance — same flatten-the-MRO pattern documented above for DatasphereDocuments / NumberGroupsResource / etc. These resources extend the existing list with newly-added Fabric / Logs / Registry / Video resources whose entries weren't in the file yet.

signalwire.rest.namespaces.fabric.CallFlowsResource.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.CallFlowsResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.CallFlowsResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CallFlowsResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CallFlowsResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CallFlowsResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CallFlowsResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.list_addresses: crud_with_addresses_lifted: Rust folds Python's CrudWithAddresses.list_addresses mixin onto the CxmlApplicationsResource directly so callers don't reach into a parent class; Python keeps it on the abstract CrudWithAddresses parent.
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricAddresses.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.FabricResource.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.FabricResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.FabricResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResource.list_addresses: crud_with_addresses_lifted: Rust folds Python's CrudWithAddresses.list_addresses mixin onto the FabricResource directly so callers don't reach into a parent class; Python keeps it on the abstract CrudWithAddresses parent.
signalwire.rest.namespaces.fabric.FabricResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResourcePUT.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.FabricResourcePUT.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.FabricResourcePUT.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResourcePUT.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResourcePUT.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResourcePUT.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.FabricResourcePUT.list_addresses: crud_with_addresses_lifted: Rust folds Python's CrudWithAddresses.list_addresses mixin onto the FabricResourcePUT directly so callers don't reach into a parent class; Python keeps it on the abstract CrudWithAddresses parent.
signalwire.rest.namespaces.fabric.FabricResourcePUT.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.GenericResources.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.GenericResources.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.SubscribersResource.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.fabric.SubscribersResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.SubscribersResource.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.SubscribersResource.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.SubscribersResource.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.SubscribersResource.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.fabric.SubscribersResource.list_addresses: crud_with_addresses_lifted: Rust folds Python's CrudWithAddresses.list_addresses mixin onto the SubscribersResource directly so callers don't reach into a parent class; Python keeps it on the abstract CrudWithAddresses parent (same pattern Perl documents).
signalwire.rest.namespaces.fabric.SubscribersResource.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.logs.ConferenceLogs.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.logs.ConferenceLogs.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.logs.FaxLogs.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.logs.FaxLogs.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.logs.MessageLogs.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.logs.MessageLogs.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.logs.VoiceLogs.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.logs.VoiceLogs.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.registry.RegistryBrands.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.registry.RegistryBrands.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.registry.RegistryCampaigns.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.registry.RegistryCampaigns.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.registry.RegistryNumbers.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.registry.RegistryNumbers.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.registry.RegistryOrders.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.registry.RegistryOrders.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.video.VideoConferenceTokens.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.video.VideoConferenceTokens.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.video.VideoConferences.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.video.VideoConferences.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.video.VideoConferences.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoConferences.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoConferences.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoConferences.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoConferences.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoRoomRecordings.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.video.VideoRoomRecordings.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.video.VideoRoomSessions.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.video.VideoRoomSessions.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.video.VideoRoomTokens.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.video.VideoRoomTokens.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.video.VideoRooms.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.video.VideoRooms.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.video.VideoRooms.create: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoRooms.delete: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoRooms.get: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoRooms.list: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoRooms.update: Rust port emits explicit CRUD where Python inherits via CrudResource.
signalwire.rest.namespaces.video.VideoStreams.__init__: Rust port emits an explicit constructor; Python's BaseResource.__init__ is inherited.
signalwire.rest.namespaces.video.VideoStreams.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.

### Rust module-level helpers projected under `<file>::mod`

The Rust adapter projects free functions defined in `mod.rs` files under the module path `signalwire.<parent>.mod.<fn>`; Python flattens these to `signalwire.<parent>.<fn>`. The helpers themselves match Python; only the path differs.

signalwire.utils.mod.is_serverless_mode: rust-path-projection: Rust ships this as a free function in `signalwire/src/utils/mod.rs`; the Rust adapter emits the `mod` segment in the qualified path. Python's equivalent is `signalwire.utils.is_serverless_mode` and is functionally identical.
signalwire.utils.url_validator._set_resolver: port-only test helper: Rust exposes a `_set_resolver` function so the audit harness can inject DNS-resolver mocks for url_validator tests; Python's equivalent test path patches the resolver via `unittest.mock.patch`.

### Webhook signing-key getter / setter on AgentBase

Rust adds an explicit `signing_key()` accessor and a `set_signing_key()` setter on AgentBase so callers can introspect or override the resolved Signing Key after construction. Python uses attribute access (`agent.signing_key = ...`) which doesn't show up as a method in the signature audit.

signalwire.core.agent_base.AgentBase.set_signing_key: rust-explicit-setter — Rust exposes ``set_signing_key(Option<&str>)`` so callers can configure the webhook signature key after construction; Python relies on attribute assignment which doesn't surface as a method in the signature inventory.
signalwire.core.agent_base.AgentBase.signing_key: rust-explicit-getter — Rust exposes ``signing_key() -> Option<&str>`` for the resolved Signing Key; Python uses attribute access (``agent.signing_key``) which the audit infrastructure doesn't model as a method.
signalwire.security.webhook.validate_request: rust_idiom: validator functions live under signalwire::security::webhook (Python uses signalwire.core.security.webhook_validator); see PORT_OMISSIONS for the missing-port projection
signalwire.security.webhook.validate_webhook_signature: rust_idiom: see validate_request entry
signalwire.security.webhook_layer.WebhookLayer: rust_idiom: tower::Layer for axum/hyper (Python uses FastAPI dependency factory)
signalwire.security.webhook_layer.WebhookLayer.__init__: rust_idiom: see WebhookLayer entry
signalwire.security.webhook_layer.WebhookLayer.with_url_base: rust_idiom_builder: builder method for proxy URL base (Rust idiom, no Python equivalent)

### Typed RELAY / server error enums (Tier-2 idiom pass — match the REST exemplar)

The Tier-2 idiom pass replaced the relay/server `Result<_, String>` failure channel with proper Rust error enums, exactly as the REST layer already does with `SignalWireRestError`. `RelayError`/`ServerError` carry their failure data in variants (callers `match`, not call getters) and impl `Display` + `std::error::Error`. Python models all failures as a single stringly raised exception, so these typed enums + their constructor helpers are port-only. `ServerError` carries data only in variants (no `pub fn`), so it does NOT appear here; `RelayError` ships two ergonomic constructor helpers. The Err type is invisible to the signature audit (`Result<T,E>` → `T`), so this is surface-only and drift-0 on Layer A.

signalwire.relay.error.RelayError: rust-typed-error — closed RELAY failure set replacing `Result<_, String>` on the relay client surface; mirrors the REST `SignalWireRestError` exemplar. Python raises a single stringly exception.
signalwire.relay.error.RelayError.transport: rust-error-ctor — convenience constructor for the `Transport` variant from a context + any `Display` cause; used by the client's `map_err` sites. No Python equivalent.
signalwire.relay.error.RelayError.missing_env: rust-error-ctor — convenience constructor for the `MissingEnv` variant naming the unset variable. No Python equivalent.

### Typed `FromStr` parse-error structs for the closed-set enums (Tier-2 idiom pass)

The Tier-2 idiom pass added `impl std::str::FromStr` to the closed-set enums so callers can write the idiomatic `"wav".parse::<RecordFormat>()`. The trait's `Err` is a small typed parse-error struct per std convention (`ParseIntError`-style), carrying the offending input and a diagnostic `Display`. These are the typed analogue of Python's `ValueError`; Python validates with a bare `raise ValueError` (no named error type), so the structs + their `input()` accessor are port-only.

signalwire.swaig.media_enums.ParseMediaEnumError: rust-parse-error — `FromStr::Err` for RecordFormat/RecordDirection/TapDirection/Codec; the typed analogue of Python's `ValueError` on the media-action closed sets.
signalwire.swaig.media_enums.ParseMediaEnumError.input: rust-error-accessor — returns the string that failed to parse. Python's `ValueError` carries only a message.
signalwire.skills.skill_name.ParseSkillNameError: rust-parse-error — `FromStr::Err` for SkillName (`"datetime".parse::<SkillName>()`); the open-set inherent `from_str` still returns `Option` for custom names. No Python equivalent.
signalwire.skills.skill_name.ParseSkillNameError.input: rust-error-accessor — returns the string that was not a built-in skill name. No Python equivalent.
signalwire.logging.ParseLevelError: rust-parse-error — `FromStr::Err` for the log `Level` enum (`"debug".parse::<Level>()`), case-insensitive. No Python equivalent.
signalwire.logging.ParseLevelError.input: rust-error-accessor — returns the string that was not a valid log level. No Python equivalent.

### Fluent builder methods on the options structs (Tier-2 idiom pass — agent/service builder)

The Tier-2 idiom pass added a fluent `with_*` builder face to the `AgentOptions` / `ServiceOptions` construction structs (each method takes and returns `self`), mirroring the existing `BedrockOptions::with_name` precedent. Python configures these as keyword arguments to `AgentBase.__init__` / `SWMLService.__init__`; the Rust port carries them on the options struct, so the builder methods are port-only. `ServiceOptions` gains a `new` + the methods, so the whole struct now surfaces.

signalwire.agent.agent_base.AgentOptions.route: rust-builder-method — fluent setter for the agent's HTTP route; Python passes `route=` to `AgentBase.__init__`.
signalwire.agent.agent_base.AgentOptions.host: rust-builder-method — fluent setter for the bind host; Python passes `host=` to `AgentBase.__init__`.
signalwire.agent.agent_base.AgentOptions.port: rust-builder-method — fluent setter for the bind port; Python passes `port=` to `AgentBase.__init__`.
signalwire.agent.agent_base.AgentOptions.basic_auth: rust-builder-method — fluent setter for Basic-Auth credentials; Python passes `basic_auth=(user,pass)` to `AgentBase.__init__`.
signalwire.agent.agent_base.AgentOptions.auto_answer: rust-builder-method — fluent setter for auto-answer; Python passes `auto_answer=` to `AgentBase.__init__`.
signalwire.agent.agent_base.AgentOptions.record_call: rust-builder-method — fluent setter for call recording; Python passes `record_call=` to `AgentBase.__init__`.
signalwire.agent.agent_base.AgentOptions.use_pom: rust-builder-method — fluent setter for POM prompt rendering; Python passes `use_pom=` to `AgentBase.__init__`.
signalwire.agent.agent_base.AgentOptions.signing_key: rust-builder-method — fluent setter for the webhook signing key; Python passes `agent.signing_key = ...` (attribute) or relies on the env var.
signalwire.swml.service.ServiceOptions: rust-options-builder — construction options for `Service` (Python's `SWMLService`); now exposes a fluent builder, so the struct surfaces. Python uses `SWMLService.__init__` keyword arguments.
signalwire.swml.service.ServiceOptions.__init__: rust-options-builder — name-only constructor for the service options builder. Python uses `SWMLService.__init__` directly.
signalwire.swml.service.ServiceOptions.route: rust-builder-method — fluent setter for the service route; Python passes `route=` to `SWMLService.__init__`.
signalwire.swml.service.ServiceOptions.host: rust-builder-method — fluent setter for the bind host; Python passes `host=` to `SWMLService.__init__`.
signalwire.swml.service.ServiceOptions.port: rust-builder-method — fluent setter for the bind port; Python passes `port=` to `SWMLService.__init__`.
signalwire.swml.service.ServiceOptions.basic_auth: rust-builder-method — fluent setter for Basic-Auth credentials; Python passes `basic_auth=(user,pass)` to `SWMLService.__init__`.

### Pre-existing relay/server transport helpers (Layer-B only; not gated by run-ci)

These two Rust-only transport entry points predate this pass but were never documented because run-ci only gates Layer A (signatures), not Layer B (surface). Documented here so the surface diff is clean. Both are internal transport plumbing with no Python equivalent.

signalwire.relay.client.ws_connect: rust-transport-helper — opens a verified WebSocket (plain `ws://` or rustls `wss://`, optionally trusting a private CA) for the relay client. Python's websocket connect is internal to `RelayClient`.
signalwire.server.tls.bind_server: rust-transport-helper — `pub(crate)` HTTP/HTTPS listener bind shared by the server entry points (selects TLS via `SWML_SSL_*`). Python uses uvicorn's `ssl_*`.

### Rust REST spec-parity helpers (narrow top-level resources + shared util)

The narrow top-level resources (addresses/recordings/short_codes/imported_numbers,
in `namespaces::simple_resources`) and the collapsed `FabricResource` each ship a
Rust `base_path()` field accessor and, in `FabricResource`, a `new_put`
constructor (the PUT-update variant of the PATCH-default `new`); Python keeps the
base path as a class attribute and has no constructor-variant. `rest::util` holds
the two `pub(crate)` path helpers shared across namespaces (factored out of nine
copy-pasted definitions); Python's equivalents are module-private. None add a
route or change the wire contract.

signalwire.rest.namespaces.addresses.AddressesResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.recordings.RecordingsResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.short_codes.ShortCodesResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.imported_numbers.ImportedNumbersResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.lookup.LookupResource.base_path: namespace_field_accessor: Rust accessor for the resource's base path; Python uses a class-level attribute.
signalwire.rest.namespaces.fabric.FabricResource.new_put: Rust constructor variant that builds the PUT-update fabric resource (Python FabricResourcePUT); Python expresses the PATCH/PUT split as two classes, Rust as one struct with two constructors.
signalwire.rest.util.join: rust-helper — pub(crate) path-segment join shared across REST namespaces; Python uses module-private equivalents.
signalwire.rest.util.params_to_string_map: rust-helper — pub(crate) JSON-object-to-query-map helper shared across REST namespaces; Python uses module-private equivalents.

### SkillBase trait accessors (Rust method for Python class/instance attribute)

The Rust `SkillBase` trait exposes as accessor methods what Python's `SkillBase`
exposes as class/instance attributes (`SKILL_NAME`/`SKILL_DESCRIPTION`/
`SKILL_VERSION`/`self.params`/etc.). Same surface, different idiom — Rust has no
class attributes, so a trait method is the faithful expression. (Now emitted by
enumerate_surface.py, which captures trait-body methods.)

signalwire.core.skill_base.SkillBase.name: Rust accessor for Python's SKILL_NAME class attribute.
signalwire.core.skill_base.SkillBase.description: Rust accessor for Python's SKILL_DESCRIPTION class attribute.
signalwire.core.skill_base.SkillBase.version: Rust accessor for Python's SKILL_VERSION class attribute.
signalwire.core.skill_base.SkillBase.params: Rust accessor for Python's self.params instance attribute.
signalwire.core.skill_base.SkillBase.required_env_vars: Rust accessor for the skill's required env-var list; Python exposes the equivalent via REQUIRED_ENV_VARS / validate_env_vars.
signalwire.core.skill_base.SkillBase.supports_multiple_instances: Rust accessor for Python's SUPPORTS_MULTIPLE_INSTANCES class attribute.
signalwire.core.skill_base.SkillBase.get_swaig_fields: Rust accessor for Python's self.swaig_fields instance attribute (extracted from params).
signalwire.core.skill_base.SkillBase.get_tool_name: Rust accessor that builds the instance-scoped tool name; Python computes the equivalent inline in define_tool.

### Rust-internal abstraction traits (no public Python counterpart)

signalwire.rest.http_client.HttpTransport: Rust-only trait abstracting the blocking HTTP transport (ureq) so tests can inject a recording transport; Python calls requests directly.
signalwire.rest.http_client.HttpTransport.execute: method of the Rust-only HttpTransport trait.
signalwire.serverless.adapter.RequestHandler: Rust-only trait abstracting a serverless request handler; Python uses duck-typed callables.
signalwire.serverless.adapter.RequestHandler.handle_request: method of the Rust-only RequestHandler trait.

## Flattened RELAY action mixin methods (§H abstract-action-base surface analog)

signalwire.relay.call.PlayAction.pause: port-only: Rust flattens the abstract PausableAction/StoppableAction/VolumeAction mixin methods onto the concrete PlayAction (the reference declares pause on the abstract base)
signalwire.relay.call.PlayAction.resume: port-only: Rust flattens the abstract mixin methods onto the concrete PlayAction (the reference declares resume on the abstract base)
signalwire.relay.call.PlayAction.volume: port-only: Rust flattens the abstract mixin methods onto the concrete PlayAction (the reference declares volume on the abstract base)
signalwire.relay.call.RecordAction.pause: port-only: Rust flattens the abstract mixin methods onto the concrete RecordAction (the reference declares pause on the abstract base)
signalwire.relay.call.RecordAction.resume: port-only: Rust flattens the abstract mixin methods onto the concrete RecordAction (the reference declares resume on the abstract base)
signalwire.skills.claude_skills.skill.ClaudeSkillsSkill.get_prompt_sections: port-only: Rust's claude_skills skill overrides get_prompt_sections; the Python reference's ClaudeSkillsSkill does not declare it (relies on the SkillBase default)
signalwire.skills.custom_skills.skill.CustomSkillsSkill.register_tools: port-only: method of the Rust-only custom_skills skill (Python has no custom_skills module)
signalwire.skills.custom_skills.skill.CustomSkillsSkill.setup: port-only: method of the Rust-only custom_skills skill (Python has no custom_skills module)
signalwire.skills.info_gatherer.skill.InfoGathererSkill.get_prompt_sections: port-only: Rust's info_gatherer skill overrides get_prompt_sections; the Python reference's InfoGathererSkill does not declare it (relies on the SkillBase default)
signalwire.core.agent_base.AgentBase.set_multilingual: These methods exist in Python's AgentBase too (via the AIConfigMixin). The Rust port hangs set_multilingual directly off AgentBase (and projects it onto AIConfigMixin), so the per-symbol enumerator also emits it under signalwire.core.agent_base.AgentBase. Python has the same surface.
