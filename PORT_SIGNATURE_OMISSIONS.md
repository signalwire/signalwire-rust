# PORT_SIGNATURE_OMISSIONS.md (signalwire-rust)

Documented signature divergences between this Rust port and the Python
reference for symbols that exist in BOTH inventories. The symbol-level
NAME parity is checked separately by `diff_port_surface.py` against
`PORT_OMISSIONS.md` / `PORT_ADDITIONS.md`; this file covers cases where
the symbol exists everywhere but its parameter list / return type
differs because of a deliberate Rust idiom.

`scripts/diff_port_signatures.py` reads this file (via `--omissions`) to
know which signature divergences are intentional. Anything not in this
file fails the diff.

Format:

```
<fully.qualified.symbol>: <one-sentence rationale>
```

The rationale should explain *why* the divergence exists — usually a
Rust language idiom (Result<T,E>, &self vs self, builder/options
constructors, no inheritance / no var_keyword, etc.).

---

## Adapter projection: trailing `params: serde_json::Value` ≡ Python `**kwargs`

The Rust adapter (`scripts/enumerate_signatures.py`) projects Rust REST
namespace methods and Call methods that take a trailing `params:
serde_json::Value` onto Python's `**kwargs` shape *when the Python
reference also has var_keyword at the same position*. That projection
fixes ~125 cases automatically. The entries below cover the residual
cases where the projection cannot be applied:

  - Python's signature does NOT have `**kwargs` at the same FQN
    (e.g. `Call.bind_digit(self, digits, bind_method, ...)` — Python
    takes explicit args, Rust collapses to a single `params: Value`).
  - The method is on a CrudResource base class and the projection
    rule excludes base classes to avoid duplicating with subclasses.
  - The trailing-arg name is something other than `params` / `kwargs` /
    `options`.

---

## Subclass return-type collapse — Rust uses tagged-union Action / generic CrudResource

Python returns specific subclasses (`PlayAction`, `RecordAction`,
`PhoneNumbersResource`, ...) where the corresponding Rust method
returns the parent type. Rust does not have inheritance, so it uses a
single `Action` struct with an internal kind tag (or a generic
`CrudResource<T>` type alias) and dispatches on that tag at runtime.
The contract is functionally equivalent — callers issue the same
RPC/REST call and observe the same fields.

---

## Constructor shapes — Rust uses options-builder / `params: &Value`

Many Rust constructors take a single `options: SomeOptions` builder
struct or a `params: &Value` JSON object instead of Python's explicit
keyword-argument list. This is the canonical Rust idiom for
many-argument or polymorphic constructors and aligns with how the
RELAY/SWML server protocols actually deliver the data (a single JSON
object on the wire).

---

## Documented divergences

signalwire.agent_server.AgentServer.__init__: Rust AgentServer constructor accepts (host, port) where Python accepts (host, port, log_level) — Rust uses RUST_LOG env-var convention for log levels rather than constructor arg
signalwire.agent_server.AgentServer.get_agents: Rust ``get_agents()`` returns ``list<string>`` of agent names where Python returns ``list<tuple<string, AgentBase>>`` — Rust returns lookup keys; users call ``get(name)`` for the agent itself
signalwire.agent_server.AgentServer.register_global_routing_callback: Rust ``register_global_routing_callback`` takes a typed ``GlobalRoutingCallback`` struct where Python takes an arbitrary callable signature — Rust trades flexibility for type safety
signalwire.agent_server.AgentServer.run: Rust ``run(host, port)`` takes 2 args where Python takes (event, context, host, port) — Rust does not have AWS Lambda event/context routing in its run() variant
signalwire.agent_server.AgentServer.setup_sip_routing: Rust ``setup_sip_routing()`` takes no args where Python takes (route, auto_map) — Rust SIP routing is configured separately via env/config rather than via this entry-point
signalwire.core.agent.prompt.manager.PromptManager.define_contexts: Rust ``define_contexts()`` returns a builder struct (chainable) where Python returns void and takes a `contexts` dict — different pattern (chainable builder vs eager dict)
signalwire.core.agent.prompt.manager.PromptManager.prompt_add_section: Rust ``prompt_add_section`` takes (title, body, bullets) — fewer args than Python's full (title, body, bullets, numbered, numbered_bullets, subsections)
signalwire.core.agent.prompt.manager.PromptManager.prompt_add_subsection: Rust ``prompt_add_subsection`` takes (parent_title, title, body) where Python adds an extra `bullets` arg
signalwire.core.agent.prompt.manager.PromptManager.prompt_add_to_section: Rust ``prompt_add_to_section`` takes (title, body, bullets) where Python takes (title, body, bullet, bullets) — Rust collapses singular `bullet` into the `bullets` parameter
signalwire.core.agent.tools.registry.ToolRegistry.define_tool: Rust ToolRegistry.define_tool takes 6 args (no fillers/wait_file/wait_file_loops/webhook_url/required/is_typed_handler/swaig_fields) — Rust uses a builder on ToolDef for those fields rather than a 13-arg constructor
signalwire.core.agent.tools.registry.ToolRegistry.get_all_functions: Rust ToolRegistry uses ``ToolDef`` (concrete struct) where Python uses ``SWAIGFunction|dict<string,any>`` union — Rust has a single canonical tool descriptor type
signalwire.core.agent.tools.registry.ToolRegistry.get_function: Rust ToolRegistry uses ``ToolDef`` (concrete struct) where Python uses ``SWAIGFunction|dict<string,any>`` union — Rust has a single canonical tool descriptor type
signalwire.core.agent_base.AgentBase.__init__: Rust AgentBase constructor takes a single ``options: AgentBaseOptions`` builder struct where Python takes 21 explicit kwargs — Rust uses an options-builder pattern (idiomatic for many-arg constructors)
signalwire.core.agent_base.AgentBase.enable_sip_routing: Rust ``enable_sip_routing()``/``register_sip_username()`` shapes differ — Rust does not take a `path` route override (auto_map is fixed) and register_sip_username uses (username, route) instead of (sip_username)
signalwire.core.agent_base.AgentBase.on_summary: Rust ``on_summary(callback)`` registers a single callback function where Python takes (summary, raw_data) override-style hook
signalwire.core.agent_base.AgentBase.pom: Rust ``pom()`` returns ``optional<list<any>>`` (the section list) where Python returns the full ``PromptObjectModel`` object
signalwire.core.agent_base.AgentBase.register_sip_username: Rust ``enable_sip_routing()``/``register_sip_username()`` shapes differ — Rust does not take a `path` route override (auto_map is fixed) and register_sip_username uses (username, route) instead of (sip_username)
signalwire.core.contexts.Context.add_step: Rust ``Context.add_step`` takes only (name) where Python takes the full (name, task, bullets, criteria, functions, valid_steps) — Rust returns a builder for the rest
signalwire.core.contexts.ContextBuilder.__init__: Rust ``ContextBuilder::new`` takes no args where Python takes (agent) — Rust uses fluent API attached to agent later
signalwire.core.contexts.GatherInfo.add_question: Rust ``GatherInfo.add_question`` takes (key, question, question_type, confirm, prompt, functions) explicitly where Python takes (key, question, **kwargs)
signalwire.core.data_map.DataMap.expression: Rust takes a string pattern where Python takes ``union<class:Pattern,string>`` — Rust users compile patterns at site rather than passing pre-compiled Pattern objects
signalwire.core.function_result.FunctionResult.__init__: Rust ``FunctionResult::new()`` takes no args where Python takes (response, post_process) — Rust uses builder methods for those
signalwire.core.function_result.FunctionResult.add_action: Rust ``add_action(action: Value)`` takes a single value param where Python takes (name, data) — Rust users build the action JSON externally
signalwire.core.function_result.FunctionResult.create_payment_action: Rust ``create_payment_*`` shapes differ — Rust takes (action_type/name/value, ... text/language/voice) where Python uses different parameter naming/ordering
signalwire.core.function_result.FunctionResult.create_payment_parameter: Rust ``create_payment_*`` shapes differ — Rust takes (action_type/name/value, ... text/language/voice) where Python uses different parameter naming/ordering
signalwire.core.function_result.FunctionResult.create_payment_prompt: Rust ``create_payment_*`` shapes differ — Rust takes (action_type/name/value, ... text/language/voice) where Python uses different parameter naming/ordering
signalwire.core.function_result.FunctionResult.execute_rpc: Rust ``execute_rpc(method, params)`` takes only the immediate method+params where Python adds (call_id, node_id) — Rust passes those via params
signalwire.core.function_result.FunctionResult.pay: Rust ``pay(payment_connector_url, ...)`` takes 6 args where Python takes 20 — Rust takes a smaller required-args set; rest go via Value config
signalwire.core.function_result.FunctionResult.record_call: Rust ``record_call`` takes 5 args (control_id, stereo, format, direction, terminators) where Python takes 12 — rest via Value config
signalwire.core.function_result.FunctionResult.remove_global_data: Rust takes ``list<string>`` / ``optional<string>`` / ``dict<string,bool>`` where Python accepts a union of types — Rust prefers a single concrete type per method (no union dispatch)
signalwire.core.function_result.FunctionResult.remove_metadata: Rust takes ``list<string>`` / ``optional<string>`` / ``dict<string,bool>`` where Python accepts a union of types — Rust prefers a single concrete type per method (no union dispatch)
signalwire.core.function_result.FunctionResult.replace_in_history: Rust takes ``list<string>`` / ``optional<string>`` / ``dict<string,bool>`` where Python accepts a union of types — Rust prefers a single concrete type per method (no union dispatch)
signalwire.core.function_result.FunctionResult.rpc_ai_message: Rust ``rpc_ai_message(call_id, message_text)`` omits Python's optional `role` parameter — defaults to ``user``
signalwire.core.function_result.FunctionResult.rpc_dial: Rust ``rpc_dial(to, from, dest_swml, call_timeout, region)`` uses different param names/ordering than Python's (to_number, from_number, dest_swml, device_type)
signalwire.core.function_result.FunctionResult.send_sms: Rust ``send_sms`` takes (to, from, body, media, tags) where Python uses (to_number, from_number, body, media, tags, region) — Rust drops the optional region
signalwire.core.function_result.FunctionResult.switch_context: Rust ``switch_context`` adds an extra `isolate_data` arg vs Python — Rust supports an extra isolation flag
signalwire.core.function_result.FunctionResult.swml_transfer: Rust ``swml_transfer(dest, ai_response)`` omits Python's `final` parameter — Rust does not support transfer finalization
signalwire.core.function_result.FunctionResult.tap: Rust ``tap`` omits Python's `rtp_ptime`, `status_url` parameters — Rust takes (uri, control_id, direction, codec)
signalwire.core.function_result.FunctionResult.toggle_functions: Rust takes ``list<string>`` / ``optional<string>`` / ``dict<string,bool>`` where Python accepts a union of types — Rust prefers a single concrete type per method (no union dispatch)
signalwire.core.mixins.auth_mixin.AuthMixin.get_basic_auth_credentials: Rust ``get_basic_auth_credentials()`` takes no `include_source` arg — always returns the 2-tuple form
signalwire.core.mixins.tool_mixin.ToolMixin.define_tool: Rust ToolMixin.define_tool takes (name, description, parameters, handler, secure) — same reduction as ToolRegistry.define_tool
signalwire.core.mixins.tool_mixin.ToolMixin.define_tools: Rust ToolMixin.define_tools takes a tool_defs argument while Python takes none — Rust accepts batch tool registration
signalwire.core.mixins.web_mixin.WebMixin.on_swml_request: Rust ``on_swml_request`` takes (request_data, callback_path) where Python additionally takes a `request` parameter (FastAPI Request object passed for context)
signalwire.core.security.session_manager.SessionManager.__init__: Rust ``SessionManager::new(token_expiry_secs)`` takes one arg where Python takes (token_expiry_secs, secret_key) — Rust generates secret key internally
signalwire.core.skill_manager.SkillManager.__init__: Rust ``SkillManager::new()`` takes no args where Python takes (agent) — Rust attaches agent later via setter
signalwire.core.skill_manager.SkillManager.load_skill: Rust ``load_skill(name, skill_class, params)`` argument types diverge — Rust takes JSON params/factory dict where Python takes a class object + params dict
signalwire.core.swml_service.SWMLService.__init__: Rust SWMLService constructor takes a single ``options: SwmlServiceOptions`` builder struct where Python takes 9 explicit kwargs — Rust uses an options-builder pattern
signalwire.core.swml_service.SWMLService.add_verb: Rust ``add_verb(verb, section, config)`` takes (verb, section, config) where Python takes (verb_name, config) — Rust requires explicit section placement; Python infers section. Return ``void`` vs ``bool``.
signalwire.core.swml_service.SWMLService.get_basic_auth_credentials: Rust ``get_basic_auth_credentials`` returns ``tuple<string,string>`` where Python returns a 2-or-3-tuple union — Rust always returns the 2-tuple form (no source-provenance flag)
signalwire.prefabs.concierge.ConciergeAgent.__init__: Rust prefab agent constructors use builder/options pattern — argument list and ordering differ from Python (Rust always takes name first, then prefab-specific args, then route)
signalwire.prefabs.faq_bot.FAQBotAgent.__init__: Rust prefab agent constructors use builder/options pattern — argument list and ordering differ from Python (Rust always takes name first, then prefab-specific args, then route)
signalwire.prefabs.info_gatherer.InfoGathererAgent.__init__: Rust prefab agent constructors use builder/options pattern — argument list and ordering differ from Python (Rust always takes name first, then prefab-specific args, then route)
signalwire.prefabs.receptionist.ReceptionistAgent.__init__: Rust prefab agent constructors use builder/options pattern — argument list and ordering differ from Python (Rust always takes name first, then prefab-specific args, then route)
signalwire.prefabs.survey.SurveyAgent.__init__: Rust prefab agent constructors use builder/options pattern — argument list and ordering differ from Python (Rust always takes name first, then prefab-specific args, then route)
signalwire.relay.call.AIAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.Action.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id, terminal_event, terminal_states) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.Call.__init__: Rust constructor takes a single ``params: &Value`` JSON object where Python takes explicit arguments — uniform Rust idiom for constructing relay objects from server event payloads
signalwire.relay.call.Call.ai: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.ai_hold: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.ai_message: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.ai_unhold: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.amazon_bedrock: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.answer: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.bind_digit: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.clear_digit_bindings: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.collect: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.connect: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.detect: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.detect_answering_machine: Rust collapses Python's keyword-only AMD args (initial_timeout/end_silence_timeout/machine_voice_threshold/machine_words_threshold/detect_interruptions/detect_message_end/timeout/on_completed) into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of DetectAction; emits the same {"type":"machine","params":{...only-provided...}} detect media over Call::detect
signalwire.relay.call.Call.detect_digit: Rust collapses Python's keyword-only digits/timeout/on_completed into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of DetectAction; emits the same {"type":"digit","params":{digits?}} detect media over Call::detect
signalwire.relay.call.Call.detect_fax: Rust collapses Python's keyword-only tone/timeout/on_completed into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of DetectAction; emits the same {"type":"fax","params":{tone?}} detect media over Call::detect
signalwire.relay.call.Call.hangup: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.join_conference: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.join_room: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.leave_conference: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.leave_room: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.live_transcribe: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.live_translate: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.on: Rust ``Call.on(event_type, cb)`` takes a callback function where Python takes (event_type, handler) — equivalent contract, different param naming
signalwire.relay.call.Call.pay: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.play: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.play_and_collect: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.play_audio: Rust collapses Python's keyword-only volume/on_completed into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of PlayAction; emits the same [{"type":"audio","params":{"url":...}}] play media over Call::play
signalwire.relay.call.Call.play_ringtone: Rust collapses Python's keyword-only duration/volume/on_completed into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of PlayAction; emits the same [{"type":"ringtone","params":{"name":...,duration?}}] play media over Call::play
signalwire.relay.call.Call.play_silence: Rust drops Python's keyword-only on_completed (no functional callback variant) and returns the unified ``class:Action`` instead of PlayAction; emits the same [{"type":"silence","params":{"duration":...}}] play media over Call::play
signalwire.relay.call.Call.play_tts: Rust collapses Python's keyword-only language/gender/voice/volume/on_completed into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of PlayAction; emits the same [{"type":"tts","params":{"text":...,language?,gender?,voice?}}] play media over Call::play
signalwire.relay.call.Call.prompt_audio: Rust collapses Python's keyword-only volume/on_completed into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of CollectAction; emits the same [{"type":"audio","params":{"url":...}}] play_and_collect media over Call::play_and_collect
signalwire.relay.call.Call.prompt_tts: Rust collapses Python's keyword-only language/gender/voice/volume/on_completed into a trailing ``opts: serde_json::Value`` bag and returns the unified ``class:Action`` instead of CollectAction; emits the same [{"type":"tts","params":{"text":...,language?,gender?,voice?}}] play_and_collect media over Call::play_and_collect
signalwire.relay.call.Call.queue_enter: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.queue_leave: Rust method has no optional kwargs forwarding (Python's ``**kwargs`` allows arbitrary extra args; Rust accepts no extras)
signalwire.relay.call.Call.receive_fax: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.record: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.send_digits: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.send_fax: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.stream: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.tap: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.transcribe: Rust returns ``class:Action`` where Python returns the specific subclass (PlayAction, RecordAction, etc.) — Rust uses a single Action struct with internal kind tag rather than inheritance hierarchy
signalwire.relay.call.Call.transfer: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.Call.user_event: Rust method takes a single ``params: serde_json::Value`` where Python takes explicit kwargs (the var_keyword projection only fires when Python's same-FQN method also has **kwargs at the trailing position)
signalwire.relay.call.CollectAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.CollectAction.start_input_timers: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.DetectAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.FaxAction.__init__: Rust FaxAction constructor takes (control_id, call_id, node_id, fax_type) where Python takes (call, control_id, method_prefix) — Rust uses an explicit FaxType enum instead of method-name prefix
signalwire.relay.call.PayAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.PlayAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.PlayAction.pause: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.PlayAction.resume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.PlayAction.volume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.RecordAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.RecordAction.pause: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.RecordAction.resume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.StreamAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.TapAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.call.TranscribeAction.__init__: Rust Action constructor takes (control_id, call_id, node_id) where Python takes (call, control_id) — Rust passes IDs explicitly rather than holding a Call reference
signalwire.relay.client.RelayClient.__init__: Rust ``Client::new(project, token, host)`` takes 3 args where Python takes 7 (project, token, jwt_token, host, contexts, max_active_calls) — Rust uses RELAY_HOST env or builder for the rest
signalwire.relay.client.RelayClient.on_call: Rust ``on_call/on_message`` return a CallHandler/MessageHandler registration object where Python returns void
signalwire.relay.client.RelayClient.on_message: Rust ``on_call/on_message`` return a CallHandler/MessageHandler registration object where Python returns void
signalwire.relay.message.Message.__init__: Rust constructor takes a single ``params: &Value`` JSON object where Python takes explicit arguments — uniform Rust idiom for constructing relay objects from server event payloads
signalwire.rest._base.CrudResource.create: Rust CrudResource.create/update take a positional ``params: Value`` argument where Python's CrudResource expects ``**kwargs`` — base classes are excluded from the var_keyword projection because their concrete subclasses go through it instead
signalwire.rest._base.CrudResource.update: Rust CrudResource.create/update take a positional ``params: Value`` argument where Python's CrudResource expects ``**kwargs`` — base classes are excluded from the var_keyword projection because their concrete subclasses go through it instead
signalwire.rest.client.RestClient.addresses: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.chat: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.imported_numbers: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.lookup: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.phone_numbers: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.pubsub: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.recordings: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.short_codes: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.verified_callers: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.namespaces.calling.CallingNamespace.__init__: Rust REST namespace constructor takes (client, project_id|base_path) where Python takes (http) — Rust factors HTTP client + scoping path explicitly while Python wraps both in a single http object
signalwire.rest.namespaces.compat.CompatAccounts.__init__: Rust REST namespace constructor takes (client, project_id|base_path) where Python takes (http) — Rust factors HTTP client + scoping path explicitly while Python wraps both in a single http object
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.create: Rust ``CxmlApplicationsResource.create(kwargs)`` takes a single Value param where Python takes **kwargs — same rationale as CrudResource base
signalwire.rest.namespaces.fabric.FabricNamespace.ai_agents: Rust returns ``class:CrudResource`` (generic) where Python returns FabricResource/FabricResourcePUT/SwmlWebhooksResource subclasses — Rust uses generic CrudResource for fabric sub-resources
signalwire.rest.namespaces.fabric.FabricNamespace.sip_endpoints: Rust returns ``class:CrudResource`` (generic) where Python returns FabricResource/FabricResourcePUT/SwmlWebhooksResource subclasses — Rust uses generic CrudResource for fabric sub-resources
signalwire.rest.namespaces.fabric.FabricNamespace.swml_scripts: Rust returns ``class:CrudResource`` (generic) where Python returns FabricResource/FabricResourcePUT/SwmlWebhooksResource subclasses — Rust uses generic CrudResource for fabric sub-resources
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.__init__: Rust skill constructor takes (params) where Python takes (agent, params) — Rust attaches the agent in a separate setup phase rather than via constructor
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.__init__: Rust skill constructor takes (params) where Python takes (agent, params) — Rust attaches the agent in a separate setup phase rather than via constructor
signalwire.skills.registry.SkillRegistry.add_skill_directory: Rust ``add_skill_directory(path)`` is a free function (no self) — Rust attaches the registry as a static singleton
signalwire.skills.registry.SkillRegistry.list_skills: Rust ``SkillRegistry::list_skills()`` is a free function (no self) returning richer info; Rust attaches the registry as a static singleton
signalwire.skills.registry.SkillRegistry.register_skill: Rust ``register_skill(skill_class)`` takes a SkillFactory (function pointer) where Python takes a SkillBase class — Rust does not have inheritance
signalwire.skills.spider.skill.SpiderSkill.__init__: Rust skill constructor takes (params) where Python takes (agent, params) — Rust attaches the agent in a separate setup phase rather than via constructor
signalwire.skills.weather_api.skill.WeatherApiSkill.__init__: Rust skill constructor takes (params) where Python takes (agent, params) — Rust attaches the agent in a separate setup phase rather than via constructor

## POM (signalwire.pom.pom) — Rust idiom

The Rust POM port follows the same shape as the Java/C++ ports: a small
core type with field-mutating builders. Where Python collapses many
optional kwargs into a single signature, Rust splits them into the
canonical "minimum-required" entry-point + a `_full`/`_with` overload
or per-field setters returning `&mut Self`. Per-method renderers do
not expose the internal `level` / `section_number` recursion params at
the public surface; those remain on a private `render_*_at` helper.

signalwire.pom.pom.PromptObjectModel.__init__: rust-default-ctor — Rust ``PromptObjectModel::new()`` takes no args; Python takes ``debug=False``. Rust uses the standard `log`/`tracing` crates for diagnostics rather than a per-instance debug toggle.
signalwire.pom.pom.PromptObjectModel.add_pom_as_subsection: rust-typed-overload — Rust takes ``target_title: &str`` where Python's `target` is ``Union[str, Section]``. Rust avoids union dispatch; callers pass the title (matching the documented happy path).
signalwire.pom.pom.PromptObjectModel.add_section: rust-builder-mut-ref — Rust ``add_section(title)`` returns ``&mut Section`` for further field configuration; the additional Python kwargs (body, bullets, numbered, numberedBullets) are set via the returned mutable reference (or via the convenience `add_section_with` overload for body).
signalwire.pom.pom.PromptObjectModel.from_json: rust-typed-overload — Rust ``from_json(&str)`` takes a string only; the dict-input branch is covered by ``from_value(&Value)``. Rust does not collapse union inputs into one signature.
signalwire.pom.pom.PromptObjectModel.from_yaml: rust-typed-overload — Rust ``from_yaml(&str)`` takes a string only; the dict-input branch is covered by ``from_value(&Value)``. Rust does not collapse union inputs into one signature.
signalwire.pom.pom.Section.__init__: rust-builder-mut-ref — Rust ``Section::new(title)`` constructs with title only; remaining Python kwargs (body, bullets, numbered, numberedBullets) are set via the per-field builder methods (`add_body`, `add_bullets`) or struct-literal construction.
signalwire.pom.pom.Section.add_subsection: rust-builder-mut-ref — Rust ``add_subsection(title)`` returns ``&mut Section`` for chained configuration; the full Python signature is exposed via the ``add_subsection_full`` companion method (title, body, bullets, numbered, numbered_bullets).
signalwire.pom.pom.Section.render_markdown: rust-public-default — Rust public ``render_markdown()`` always renders at the conventional top-level (level=2, no section_number); the recursion-internal variant is the crate-private ``render_markdown_at(level, section_number)`` invoked by `PromptObjectModel`.
signalwire.pom.pom.Section.render_xml: rust-public-default — Rust public ``render_xml()`` always renders at indent=0 with no section_number; the recursion-internal variant is the crate-private ``render_xml_at(indent, section_number)`` invoked by `PromptObjectModel`.
signalwire.core.security.webhook_validator.validate_request: rust-typed-enum-instead-of-union — Python takes ``Union[str, Mapping[str, Any], List[Tuple[str, Any]], None]`` for the 4th argument; Rust uses an explicit ``ParamsOrBody`` enum (``Body(String)`` / ``Params(Vec<(String, Vec<String>)>)``) for the same dispatch. Rust does not collapse union inputs into a single positional argument.
