<!-- ══════════════════════════════════════════════════════════════════════════
BEFORE YOU ADD AN ENTRY TO THIS FILE — READ THIS.

Every entry here is a place the parity checker STOPS comparing. That is a real cost:
a divergence you list is a divergence no gate will ever catch again. So entries must
be RARE, and each one must earn its place. Default to skepticism: assume the entry is
NOT needed and make the case that it is.

The order of preference, always:
  1. FIX THE PORT so it matches the reference (add the missing member; make the
     signature match).
  2. FIX THE EMISSION so idiom folds onto the reference shape — the enumerator/emitter
     canonicalizes your language's spelling onto the oracle's (builder → __init__,
     getters → attributes, Result<T,E> → the plain return, CamelCase → the reference
     name, options-object/kwargs → the expanded param list, RAII/dispose → close).
     MOST divergences are idiom and belong here, not in this file.
  3. FIX THE REFERENCE if the oracle itself is wrong or stale (a Python-only symbol
     that leaked into the contract, a param the reference added and the oracle never
     re-enumerated). Fix Python / the oracle, then re-drift — do not paper over a
     broken reference with a per-port entry.
  4. Only when 1–3 genuinely cannot apply does an entry here become justified.

An entry is JUSTIFIED ONLY IF it is irreducible after correct emission — i.e. the
divergence survives because the two languages genuinely cannot express the same thing,
not because the emitter hasn't folded the idiom yet. If emission COULD fold it, the
entry is a bug in this file; go fix the emitter.

Each entry MUST state WHY, concretely, in one of these forms:
  • ADDITION — this symbol exists in the port but not the reference. Answer: is it
    genuine port-only surface with NO reference twin (say what it is and why the
    reference has no equivalent), or is it IDIOM the emitter should have folded (then
    it does not belong here — fold it)? A convenience/alias/back-compat wrapper is NOT
    a justification.
  • OMISSION — this reference symbol has no port member. Answer: WHY can it not exist
    here — what specific language feature is absent (e.g. no async-context-manager
    protocol, no __init__ method protocol)? "impossible:" means the construct cannot
    be expressed at all; if it merely LOOKS different, that's idiom → fold it, don't
    omit it. Cite a precedent when one exists (e.g. RelayClient omits the same dunder).
  • SIGNATURE — the symbol matches by name but its parameters differ. Answer: is the
    difference a foldable idiom collapse (options-object, leading context/self,
    builder) — then EXPAND it in the signature emitter so names+count match, don't list
    it — or a genuine reference-only parameter with no cross-language analogue?

If you cannot write a crisp, specific WHY that survives the "could emission fold this?"
test, the entry is not ready. Prove it's needed before you add it.
═══════════════════════════════════════════════════════════════════════════════ -->

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
signalwire.core.agent.tools.type_inference.infer_schema: idiom: Python ``infer_schema(func)`` REFLECTS a handler's runtime signature/type-hints/docstring (inspect.signature / get_type_hints) to derive the schema tuple. Rust erases parameter types at compile time and a closure carries no signature/doc metadata, so the framework cannot read them off the handler — the developer DECLARES them once via the typed ``ParamsBuilder``. Rust therefore takes ``(params: ParamsBuilder, description, has_raw_data)`` — the same three facts Python reads from the callable, passed explicitly — where Python takes the single ``func``. Reflection-vs-typed-builder idiom; the returned ``(parameters, required, description, is_typed, has_raw_data)`` tuple is identical.
signalwire.core.agent_base.AgentBase.enable_sip_routing: Rust ``enable_sip_routing()``/``register_sip_username()`` shapes differ — Rust does not take a `path` route override (auto_map is fixed) and register_sip_username uses (username, route) instead of (sip_username)
signalwire.core.agent_base.AgentBase.on_summary: Rust ``on_summary(callback)`` registers a single callback function where Python takes (summary, raw_data) override-style hook
signalwire.core.agent_base.AgentBase.register_sip_username: Rust ``enable_sip_routing()``/``register_sip_username()`` shapes differ — Rust does not take a `path` route override (auto_map is fixed) and register_sip_username uses (username, route) instead of (sip_username)
signalwire.core.contexts.Context.add_step: Rust ``Context.add_step`` takes only (name) where Python takes the full (name, task, bullets, criteria, functions, valid_steps) — Rust returns a builder for the rest
signalwire.core.contexts.GatherInfo.add_question: Rust ``GatherInfo.add_question`` takes (key, question, question_type, confirm, prompt, functions) explicitly where Python takes (key, question, **kwargs)
signalwire.core.data_map.DataMap.expression: Rust takes a string pattern where Python takes ``union<class:Pattern,string>`` — Rust users compile patterns at site rather than passing pre-compiled Pattern objects
signalwire.core.function_result.FunctionResult.add_action: Rust ``add_action(action: Value)`` takes a single value param where Python takes (name, data) — Rust users build the action JSON externally
signalwire.core.function_result.FunctionResult.pay: Rust ``pay`` matches Python's full 20-arg surface; the only divergence is ``postal_code``, which Rust takes as a pre-rendered ``&str`` (the wire value is always a string) where Python accepts ``Union[bool, str]`` — Rust has no untagged bool|str union, so callers pass ``"true"``/``"false"`` or the literal postcode
signalwire.core.function_result.FunctionResult.remove_global_data: Rust takes ``KeysArg`` (an ``impl Into<KeysArg>`` accepting either a single ``&str`` or a ``Vec<&str>``) where Python takes ``union<string,list<string>>`` — the enumerator renders the concrete enum as a class type, but it models Python's union faithfully and emits the matching wire shape per arm (bare string for one key, array for many), so the EMISSION is byte-identical (verified by diff_port_emission.py ``unset_global_data.str``/``.list``)
signalwire.core.function_result.FunctionResult.remove_metadata: Rust takes ``KeysArg`` (an ``impl Into<KeysArg>`` accepting either a single ``&str`` or a ``Vec<&str>``) where Python takes ``union<string,list<string>>`` — same as remove_global_data: models the union and emits the bare-string vs array wire shape per arm (verified by diff_port_emission.py ``unset_metadata.str``/``.list``)
signalwire.core.function_result.FunctionResult.replace_in_history: Rust takes ``list<string>`` / ``optional<string>`` / ``dict<string,bool>`` where Python accepts a union of types — Rust prefers a single concrete type per method (no union dispatch)
signalwire.core.function_result.FunctionResult.switch_context: Rust ``switch_context`` adds an extra `isolated` arg vs Python — Rust supports an extra isolation flag (forces the object form and emits the ``isolated`` wire key)
signalwire.core.mixins.auth_mixin.AuthMixin.get_basic_auth_credentials: Rust ``get_basic_auth_credentials()`` takes no `include_source` arg — always returns the 2-tuple form
signalwire.core.mixins.tool_mixin.ToolMixin.define_tool: Rust ToolMixin.define_tool takes (name, description, parameters, handler, secure) — same reduction as ToolRegistry.define_tool
signalwire.core.mixins.tool_mixin.ToolMixin.define_tools: Rust ToolMixin.define_tools takes a tool_defs argument while Python takes none — Rust accepts batch tool registration
signalwire.core.mixins.web_mixin.WebMixin.on_swml_request: Rust ``on_swml_request`` takes (request_data, callback_path) where Python additionally takes a `request` parameter (FastAPI Request object passed for context)
signalwire.core.skill_manager.SkillManager.load_skill: impossible: Rust ``load_skill(skill_name, params, agent)`` vs Python ``load_skill(skill_name, skill_class=None, params=None)``. Two language limits, not naming. (1) Python's ``skill_class`` is a runtime CLASS OBJECT the caller passes to bypass registry lookup; Rust has no class objects and cannot construct from a type value, so the registry factory keyed by ``skill_name`` is the only instantiation path — the slot has no Rust type to hold. (2) Python's SkillManager stores ``self.agent`` and reads it during load; Rust cannot hold a ``&mut AgentBase`` in the struct across calls (borrow lifetimes), so the agent is threaded as an explicit trailing argument. ``params`` IS ported and optional; it sits at index 1 instead of 2 purely because the un-portable ``skill_class`` slot is absent, which is what the residual positional ``required``-flip on index 2 reports.
signalwire.core.swml_service.SWMLService.add_verb: Rust ``add_verb(verb, section, config)`` takes (verb, section, config) where Python takes (verb_name, config) — Rust requires explicit section placement; Python infers section. Return ``void`` vs ``bool``.
signalwire.core.swml_service.SWMLService.get_basic_auth_credentials: Rust ``get_basic_auth_credentials`` returns ``tuple<string,string>`` where Python returns a 2-or-3-tuple union — Rust always returns the 2-tuple form (no source-provenance flag)
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
signalwire.relay.call.AIAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.CollectAction.pause: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.CollectAction.resume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.CollectAction.start_input_timers: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.CollectAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.CollectAction.volume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.DetectAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.FaxAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.PayAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.PlayAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.RecordAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.StandaloneCollectAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.StreamAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.TapAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.TranscribeAction.stop: Rust action ``stop`` returns void where Python returns ``dict<string,any>`` — Rust mutates the action and records/sends the sub-command, returning the response separately rather than passing it back synchronously
signalwire.relay.call.PlayAction.pause: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.PlayAction.resume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.PlayAction.volume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.RecordAction.pause: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.call.RecordAction.resume: Rust ``pause/resume/volume/start_input_timers`` return void where Python returns ``dict<string,any>`` — Rust mutates the action and returns the response separately rather than passing the response back synchronously
signalwire.relay.client.RelayClient.on_call: Rust ``on_call/on_message`` return a CallHandler/MessageHandler registration object where Python returns void
signalwire.relay.client.RelayClient.on_message: Rust ``on_call/on_message`` return a CallHandler/MessageHandler registration object where Python returns void
signalwire.rest._base.CrudResource.create: Rust CrudResource.create/update take a positional ``params: Value`` argument where Python's CrudResource expects ``**kwargs`` — base classes are excluded from the var_keyword projection because their concrete subclasses go through it instead
signalwire.rest._base.CrudResource.update: Rust CrudResource.create/update take a positional ``params: Value`` argument where Python's CrudResource expects ``**kwargs`` — base classes are excluded from the var_keyword projection because their concrete subclasses go through it instead
signalwire.rest._base.CrudResource.path: Rust ``CrudResource.path(parts)`` is the base's collection+item URL composer, exposed when the port consolidated to a SINGLE CrudResource (the former generated-layer duplicate carried this helper). Python composes item paths inline in each method and records no public ``path`` on ``_base.CrudResource`` — Rust base-helper idiom, same shape as the BaseResource/ReadResource path helpers.
signalwire.rest.client.RestClient.addresses: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.chat: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.imported_numbers: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.lookup: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.messages: Rust returns the concrete generated ``class:Messages`` accessor for the flat ``/api/messaging/messages`` send+redact resource; the Python signature oracle records no flat accessor on ``RestClient`` (accessors are enumerated only on the Rust hand client) — same port-only flat-accessor idiom as chat/pubsub/lookup/etc. Distinct from the message *logs* under ``logs().messages()``.
signalwire.rest.client.RestClient.projects: Rust returns the concrete generated ``class:Projects`` accessor for the flat ``/api/projects`` CRUD resource; the Python signature oracle records no flat accessor on ``RestClient`` (accessors are enumerated only on the Rust hand client) — same port-only flat-accessor idiom as chat/pubsub/lookup/etc.
signalwire.rest.client.RestClient.pubsub: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.recordings: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.client.RestClient.short_codes: Rust returns ``class:CrudResource`` (generic) where Python returns the specific subclass (PhoneNumbersResource, etc.) — Rust uses concrete CrudResource alias type rather than per-resource subclasses
signalwire.rest.namespaces.calling.CallingNamespace.__init__: Rust REST namespace constructor takes (client, project_id|base_path) where Python takes (http) — Rust factors HTTP client + scoping path explicitly while Python wraps both in a single http object
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.create: Rust ``CxmlApplicationsResource.create(kwargs)`` takes a single Value param where Python takes **kwargs — same rationale as CrudResource base
signalwire.rest.namespaces.fabric.FabricNamespace.cxml_webhooks: Rust returns ``class:FabricResource`` where Python returns the CxmlWebhooksResource subclass — Rust folds the webhook subclass onto the plain FabricResource (PATCH CRUD + list_addresses); Python's subclass only overrides ``create`` to emit a DeprecationWarning (these are auto-materialized via phone_numbers.set_cxml_webhook), which is not part of the cross-port wire contract
signalwire.rest.namespaces.fabric.FabricNamespace.swml_webhooks: Rust returns ``class:FabricResource`` where Python returns the SwmlWebhooksResource subclass — Rust folds the webhook subclass onto the plain FabricResource (PATCH CRUD + list_addresses); Python's subclass only overrides ``create`` to emit a DeprecationWarning (these are auto-materialized via phone_numbers.set_swml_webhook), which is not part of the cross-port wire contract
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.__init__: Rust skill constructor takes (params) where Python takes (agent, params) — Rust attaches the agent in a separate setup phase rather than via constructor
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.__init__: Rust skill constructor takes (params) where Python takes (agent, params) — Rust attaches the agent in a separate setup phase rather than via constructor
signalwire.skills.registry.SkillRegistry.add_skill_directory: Rust ``add_skill_directory(path)`` is a free function (no self) — Rust attaches the registry as a static singleton
signalwire.skills.registry.SkillRegistry.list_skills: Rust ``SkillRegistry::list_skills()`` is a free function (no self) returning richer info; Rust attaches the registry as a static singleton
signalwire.skills.registry.SkillRegistry.register_skill: Rust ``register_skill(skill_class)`` takes a SkillFactory (function pointer) where Python takes a SkillBase class — Rust does not have inheritance
signalwire.skills.weather_api.skill.WeatherApiSkill.__init__: Rust skill constructor takes (params) where Python takes (agent, params) — Rust attaches the agent in a separate setup phase rather than via constructor

## POM (signalwire.pom.pom) — Rust idiom

The Rust POM port follows the same shape as the Java/C++ ports: a small
core type with field-mutating builders. Where Python collapses many
optional kwargs into a single signature, Rust splits them into the
canonical "minimum-required" entry-point + a `_full`/`_with` overload
or per-field setters returning `&mut Self`. Per-method renderers do
not expose the internal `level` / `section_number` recursion params at
the public surface; those remain on a private `render_*_at` helper.

signalwire.pom.pom.PromptObjectModel.add_pom_as_subsection: rust-typed-overload — Rust takes ``target_title: &str`` where Python's `target` is ``Union[str, Section]``. Rust avoids union dispatch; callers pass the title (matching the documented happy path).
signalwire.pom.pom.PromptObjectModel.add_section: rust-builder-mut-ref — Rust ``add_section(title)`` returns ``&mut Section`` for further field configuration; the additional Python kwargs (body, bullets, numbered, numberedBullets) are set via the returned mutable reference (or via the convenience `add_section_with` overload for body).
signalwire.pom.pom.PromptObjectModel.from_json: rust-typed-overload — Rust ``from_json(&str)`` takes a string only; the dict-input branch is covered by ``from_value(&Value)``. Rust does not collapse union inputs into one signature.
signalwire.pom.pom.PromptObjectModel.from_yaml: rust-typed-overload — Rust ``from_yaml(&str)`` takes a string only; the dict-input branch is covered by ``from_value(&Value)``. Rust does not collapse union inputs into one signature.
signalwire.pom.pom.Section.add_subsection: rust-builder-mut-ref — Rust ``add_subsection(title)`` returns ``&mut Section`` for chained configuration; the full Python signature is exposed via the ``add_subsection_full`` companion method (title, body, bullets, numbered, numbered_bullets).
signalwire.pom.pom.Section.render_markdown: rust-public-default — Rust public ``render_markdown()`` always renders at the conventional top-level (level=2, no section_number); the recursion-internal variant is the crate-private ``render_markdown_at(level, section_number)`` invoked by `PromptObjectModel`.
signalwire.pom.pom.Section.render_xml: rust-public-default — Rust public ``render_xml()`` always renders at indent=0 with no section_number; the recursion-internal variant is the crate-private ``render_xml_at(indent, section_number)`` invoked by `PromptObjectModel`.


## Item I — newly-implemented subsystem signature idioms (rust-hi-2)

signalwire.core.agent_base.AgentBase.mcp_servers: idiom: Rust read accessor `mcp_servers(&self) -> &[Value]`; Python exposes the list as an attribute. &self accessor idiom.
signalwire.core.logging_config.strip_control_chars: idiom: Rust `strip_control_chars(&str) -> String` sanitizes a single log value; Python `(logger, method_name, event_dict)` is a structlog processor hook — the Rust port has no structlog event_dict, so the callable operates on the value.
signalwire.core.pom_builder.PomBuilder.from_sections: no-self + typed-param: Rust `from_sections(&Value)` associated constructor (Python classmethod `cls, sections`); the sidecar unfold does not apply to a hand builder.
signalwire.core.security.session_manager.SessionManager.set_debug_mode: idiom: Rust `&mut self` setter for the debug-mode gate; Python sets `_debug_mode` at construction (no setter). Builder/setter idiom.
signalwire.core.swaig_function.SWAIGFunction.execute: idiom: Rust `execute(&self, args, raw_data: Option<&Map>) -> Value`; Python `(self, args, raw_data)` returns a dict. &self + Option idiom; returns serde_json::Value == dict.
signalwire.core.swaig_function.SWAIGFunction.to_swaig: idiom: Rust `to_swaig(base_url, token: Option, call_id: Option)`; Python adds an `include_auth` kwarg. Option-typed params idiom; the auth toggle is folded into the token/call_id presence.
signalwire.core.swml_builder.SWMLBuilder.ai: idiom: Rust `ai(&Map<String,Value>)` takes an args map (== Python `**kwargs`) rather than exploded keyword params. options-map ≡ kwargs idiom.
signalwire.core.swml_handler.VerbHandlerRegistry.register_handler: idiom: Rust `register_handler(Box<dyn SwmlVerbHandler>)` takes a trait object; Python takes a handler instance. Trait-object idiom for the interface type.
signalwire.core.swml_renderer.SwmlRenderer.render_function_response_swml: idiom: Rust `render_function_response_swml(response_text, service: &mut Service, actions: Option<&[Value]>)`; Python `(response_text, service, actions, format)` — the `format` kwarg (json/yaml) is folded to always-JSON in the Rust render path.
signalwire.core.swml_renderer.SwmlRenderer.render_swml: idiom: Rust `render_swml(prompt: &Value, service: &mut Service, opts: &RenderSwmlOptions)` bundles the ~12 render kwargs into an options struct; Python explodes them as keyword args. options-struct ≡ kwargs idiom.
signalwire.core.swml_service.SWMLService.merge_swaig_fields: idiom: Rust skill-support helper `merge_swaig_fields(&mut self, name, fields: &Map)`; no Python counterpart on SWMLService (SkillBase.define_tool merges before registering). Rust &mut self idiom.
signalwire.core.swml_service.SWMLService.register_routing_callback: idiom: Rust `register_routing_callback<F: Fn(&Value, &HashMap<String,String>)->Option<String>>(callback, path)` takes a typed `(body, headers)` closure — the same decomposed `callback_fn(body, headers)` shape Python now uses — surfaced as `any` because rustdoc cannot express a closure's `callable<...>` type; Python `(self, callback, path=..., methods=...)`. Closure-param (type-erased) + no-methods idiom.
signalwire.core.swml_service.SWMLService.register_verb_handler: idiom: Rust `register_verb_handler(Box<dyn SwmlVerbHandler>)` takes a trait object; Python takes a handler instance. Trait-object idiom.
signalwire.core.swml_service.SWMLService.routing_callback: idiom: Rust read accessor `routing_callback(&self, path) -> Option<&Arc<..>>`; no Python counterpart (the callback is invoked internally). &self accessor idiom.
signalwire.core.swml_service.SWMLService.serve: idiom: Rust `serve(&self, host: Option<&str>, port: Option<u16>)` takes Option overrides; Python `(self, host="0.0.0.0", port=None)` uses defaulted kwargs. Option-typed override idiom.
signalwire.prefabs.info_gatherer.InfoGathererAgent.on_swml_request: idiom: Rust `on_swml_request` takes explicit query_params/headers maps (no framework request object); Python takes the FastAPI request. No-framework-request idiom.
signalwire.prefabs.info_gatherer.InfoGathererAgent.set_question_callback: idiom: Rust `set_question_callback` takes a typed closure; Python takes a Callable. Closure-param idiom.
signalwire.relay.call.Action.wait: idiom: Rust `wait(&self, timeout)` blocks the calling thread and returns Option (matched/timeout); Python is async and returns the action. Sync-blocking + Option idiom.
signalwire.relay.call.Call.echo: idiom: Rust Call is a synchronous command surface; `echo` matches the wire RPC with Rust-typed params. &self/typed-param idiom.
signalwire.relay.call.Call.refer: idiom: Rust Call is a synchronous command surface; `refer` matches the wire RPC with Rust-typed params. &self/typed-param idiom.
signalwire.relay.call.Call.wait_for: idiom: Rust `wait_for(events)` blocks and returns Option; Python is async. Sync-blocking + Option idiom.
signalwire.relay.call.Call.wait_for_answered: idiom: Rust blocking wait returning Option; Python async awaitable. Sync-blocking idiom.
signalwire.relay.call.Call.wait_for_ended: idiom: Rust blocking wait returning Option; Python async awaitable. Sync-blocking idiom.
signalwire.relay.call.Call.wait_for_ending: idiom: Rust blocking wait returning Option; Python async awaitable. Sync-blocking idiom.
signalwire.relay.call.Call.wait_for_ringing: idiom: Rust blocking wait returning Option; Python async awaitable. Sync-blocking idiom.
signalwire.relay.call.StandaloneCollectAction.start_input_timers: idiom: Rust `start_input_timers(&self)` fires the wire RPC; Python async method. &self/sync idiom.
signalwire.relay.client.RelayClient.dial: idiom: Rust `dial` is a synchronous wrapper delegating to the blocking client; Python is async. Sync-blocking idiom.
signalwire.relay.client.RelayClient.send_message: idiom: Rust `send_message` is a synchronous wrapper delegating to the blocking client; Python is async. Sync-blocking idiom.
signalwire.relay.message.Message.wait: idiom: Rust `wait(&self, timeout)` blocks and returns Option; Python is async. Sync-blocking + Option idiom.
signalwire.rest._base.HttpClient.post: idiom: Rust `post(&self, path, data: &Value) -> Result<Value, SignalWireRestError>`; Python `(self, path, body, params)`. Result + &Value-body idiom (query params threaded via the path/builder).
signalwire.skills.registry.SkillRegistry.discover_skills: no-self: Rust static method over the compiled-in factory registry (no instance state); Python enumerates the bound `self`.
signalwire.skills.registry.SkillRegistry.get_all_skills_schema: no-self: Rust static method over the compiled-in factory registry; Python enumerates the bound `self`.
signalwire.skills.registry.SkillRegistry.get_skill_class: no-self + param-name idiom: Rust `get_skill_class(name)` associated fn returns a factory over the compiled-in registry; Python `(self, skill_name)` returns a class. Rust has no runtime class objects — the factory closure is the analog.
signalwire.skills.registry.SkillRegistry.list_all_skill_sources: no-self: Rust static method over the compiled-in factory registry; Python enumerates the bound `self`.
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.search_wiki: idiom: Rust `search_wiki(query, num_results) -> String` is a &self method; the reference records a positional-first shape. Rust `&self`/typed-param idiom.

## Item I — enumerator surface-parity residuals (rust-sig-finish)

The signature enumerator now discovers the same item-I subsystems / mixin
projections the surface enumerator does (path-derived class discovery, trait-body
methods, METHOD_RENAMES, SURFACE_PROJECTIONS/FORCE_CLASS_METHODS mirrored). The
entries below cover the residual SIGNATURE divergences on those now-SEEN symbols —
each is a genuine Rust idiom, not undone work (the symbol IS implemented; only its
param list / return type / receiver differs).

### Typed RELAY event wrappers (event.rs)

Each `*Event` is a thin typed wrapper over `RelayEvent` (a single `base: RelayEvent`
field) built by an associated `from_payload(payload: &Value) -> Self`; the wire fields
are exposed as `&self` accessor methods delegating to `base`. Python models each as a
dataclass (`__init__` over every wire field) plus a `classmethod from_payload(cls,
payload)`. The port constructs every event (via `from_payload`) — the divergence is
the constructor SHAPE (no field-wise `__init__`; associated fn has no `cls`/`self`
receiver), a formulaic wrapper idiom.

signalwire.relay.event.CallReceiveEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.CallStateEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.CallingErrorEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.CollectEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.ConferenceEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.ConnectEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.DenoiseEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.DetectEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.DialEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.EchoEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.FaxEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.HoldEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.MessageReceiveEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.MessageStateEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.PayEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.PlayEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.QueueEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.RecordEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.ReferEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.RelayEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.SendDigitsEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.StreamEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.TapEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.
signalwire.relay.event.TranscribeEvent.from_payload: no-self: Rust associated fn `from_payload(payload: &Value) -> Self` == Python classmethod `(cls, payload)` — same payload->event construction, no `cls` receiver on the Rust associated fn.

### Mixin-projected methods — idiom shapes hung off AgentBase

These methods are projected onto their reference mixin module from the Rust
`AgentBase` implementation (SURFACE_PROJECTIONS mirror). The Rust `AgentBase`
method carries a Rust idiom shape (fewer positional args + `serde_json::Value`
maps for the optional kwargs, `&Value`/closure params, fluent `&mut Self`
returns) where the Python mixin explodes each option to a typed keyword arg. The
same idiom is documented on the corresponding AgentBase surface (PORT_ADDITIONS);
these entries cover the signature comparison the mixin projection exposes.

signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_function_include: idiom: Rust `add_function_include(include: &Value)` takes the whole include object as one JSON value; Python explodes `(url, functions, meta_data)`. options-value ≡ kwargs idiom.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_internal_filler: idiom: Rust `add_internal_filler(filler: &str)` appends one filler string; Python `(function_name, language_code, fillers)` keys by function+language. Flattened-append idiom.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_language: idiom: Rust `add_language(name, code, voice)` takes the core three args + fluent builders / `set_language_params` for the optional `speech_fillers/function_fillers/engine/model/params`; Python takes them all as one call. Builder/options idiom.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_pattern_hint: idiom: Rust `add_pattern_hint(pattern: &str)` builds the pattern-hint from the pattern + fluent setters; Python `(hint, pattern, replace, ignore_case)` takes the whole rule inline. Builder idiom.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_pronunciation: idiom: Rust `add_pronunciation(replace, with, ignore_case: bool)` matches Python `(replace, with_text, ignore_case=False)` on the wire — same SWML keys `replace` / `with` / `ignore_case` (bool, emitted only when true, per signalwire-agents schema.json `Pronounce`). The residual signature difference is pure idiom: the param is named `with` (the natural Rust spelling of the `with` wire key) rather than Python's `with_text`, and `ignore_case: bool` is a required positional (Rust has no default-argument syntax; the false default is the caller passing `false`). Wire-equal; no semantics divergence.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.enable_debug_events: idiom: Rust `enable_debug_events(level: &str)` takes a string level label; Python `(level: int = 1)` an int. String-label vs int-level idiom (both select the same debug tier).
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_internal_fillers: idiom: Rust `set_internal_fillers(fillers: Vec<&str>)` sets a flat filler list (the nested-map form is `set_internal_fillers_map`); Python takes the full `dict<function,dict<language,list>>`. Flat-list idiom (the map variant carries the nested shape).
signalwire.core.mixins.prompt_mixin.PromptMixin.define_contexts: idiom: Rust `define_contexts(&mut self) -> &mut ContextBuilder` returns the builder to configure in place; Python `(self, contexts)` accepts a pre-built contexts arg and may return AgentBase|ContextBuilder. Builder-return idiom (no contexts arg; concrete ContextBuilder return).
signalwire.core.mixins.prompt_mixin.PromptMixin.prompt_add_section: idiom: Rust `prompt_add_section(title, body, bullets)` takes the core three args + fluent section builders for `numbered/numbered_bullets/subsections`; Python takes them all inline. Builder/options idiom.
signalwire.core.mixins.prompt_mixin.PromptMixin.prompt_add_subsection: idiom: Rust `prompt_add_subsection(parent_title, title, body)` + fluent bullets; Python folds `bullets` into the call. Builder idiom.
signalwire.core.mixins.prompt_mixin.PromptMixin.prompt_add_to_section: idiom: Rust `prompt_add_to_section(title, body, bullets: Vec)` merges the single-`bullet`/list-`bullets` Python overload into one `bullets` arg. Merged-arg idiom.
signalwire.core.mixins.serverless_mixin.ServerlessMixin.handle_serverless_request: idiom: Rust `handle_serverless_request(headers)` derives the serverless request from the HTTP headers map (no framework event/context objects); Python `(event, context, mode)` takes the cloud-provider event/context. No-framework-object idiom.
signalwire.core.mixins.skill_mixin.SkillMixin.add_skill: rust-idiom-loose-param: Rust `add_skill(name, params: Value)` carries the params object as one `serde_json::Value` (the **kwargs analogue); Python types it `optional<dict<string,any>>`. Open-value ≡ kwargs idiom.
signalwire.core.mixins.tool_mixin.ToolMixin.tool: idiom: Rust `tool(name, description, parameters, handler, secure)` is an inherent registration method taking the explicit tool spec; Python's `tool(cls, name, **kwargs)` is a classmethod decorator over kwargs. Decorator-vs-explicit-args idiom (same registration).
signalwire.core.mixins.web_mixin.WebMixin.get_app: idiom: Rust `get_app() -> String` returns the app mount identifier; Python returns a `class:FastAPI` instance. No-framework-type idiom.
signalwire.core.mixins.web_mixin.WebMixin.register_routing_callback: rust-idiom-typed-closure: Rust `register_routing_callback<F: Fn(&Value, &HashMap<String,String>)->Option<String>>` takes a typed `(body, headers)` closure — matching Python's decomposed `callback_fn(body, headers)` — surfaced as `any` because rustdoc cannot spell a closure's `callable<...>` type; Python types the callback `callable<[dict, dict], optional<str>>`. Closure-param (type-erased) idiom.
signalwire.core.mixins.web_mixin.WebMixin.run: idiom: Rust `run(&self)` blocks serving with host/port/mode read from config/env (returns void); Python `run(event, context, force_mode, host, port)` returns an optional serverless response. Blocking-serve idiom (no serverless event/response in the Rust run path).
signalwire.core.mixins.web_mixin.WebMixin.set_dynamic_config_callback: rust-idiom-typed-closure: Rust `set_dynamic_config_callback` takes a Rust closure surfaced as `any`; Python types it `callable<[dict,dict,dict,AgentBase], void>`. Closure-param idiom.

### Item-I subsystem constructors / methods — idiom shapes

signalwire.agent_server.AgentServer.agents: idiom: Python `@property agents -> dict<str, AgentBase>` read accessor; Rust exposes the registered agents via `get_agents()` and stores the map as a struct field (no zero-arg `agents` method). Property-vs-getter idiom.
signalwire.core.auth_handler.AuthHandler.flask_decorator: idiom: Python `flask_decorator(f)` wraps a Flask view fn; Rust has no Flask — the port's auth is applied in `Service::handle_request`, so the method exists as glue without the Python callable arg. No-Flask idiom.
signalwire.core.auth_handler.AuthHandler.verify_basic_auth: idiom: Rust `verify_basic_auth(username, password)` verifies the decoded credential pair; Python `verify_basic_auth(credentials: HTTPBasicCredentials)` takes the FastAPI credentials object. Decoded-pair vs framework-object idiom.
signalwire.core.auth_handler.AuthHandler.verify_bearer_token: idiom: Rust `verify_bearer_token(token: &str)` verifies the raw bearer string; Python `(credentials: HTTPAuthorizationCredentials)` takes the FastAPI auth object. Raw-token vs framework-object idiom.
signalwire.core.config_loader.ConfigLoader.substitute_vars: idiom: Rust `substitute_vars(&self, value)` performs env-var substitution with a fixed internal recursion guard; Python exposes `max_depth` as a caller arg. Internal-recursion-bound idiom.
signalwire.web.web_service.WebService.start: idiom: Rust `start(host, port)` reads SSL cert/key from config/env (the Rust static-file server is integrated into AgentServer); Python `start(host, port, ssl_cert, ssl_key)` takes SSL paths as args. Config-sourced-SSL idiom.

### SkillBase trait / SkillManager / registry idioms

signalwire.core.skill_base.SkillBase.define_tool: idiom: Rust `define_tool(agent, name, description, parameters, handler, secure)` registers a tool with explicit args; Python `define_tool(self, **kwargs)` takes kwargs. Explicit-args vs kwargs idiom.
signalwire.core.skill_base.SkillBase.register_tools: idiom: Rust `register_tools(&self, agent: &mut AgentBase)` receives the agent to register onto; Python `register_tools(self)` registers onto the bound `self.agent`. Explicit-agent-arg idiom (Rust skill trait is not agent-bound).
signalwire.core.skill_base.SkillBase.update_skill_data: idiom: Rust `update_skill_data(&self, ...)` mutates skill state and returns unit; Python returns a `FunctionResult`. Rust unit-return idiom.
signalwire.core.skill_base.SkillBase.validate_env_vars: idiom: Rust `validate_env_vars(&self) -> Vec<String>` returns the list of MISSING env vars (empty == ok); Python returns a `bool`. Missing-list vs bool idiom (both express the same validity check).
signalwire.core.skill_manager.SkillManager.loaded_skills: idiom: Python `@property loaded_skills -> dict<str, SkillBase>` read accessor; Rust exposes the loaded set via `list_loaded_skills()` and stores the map as a struct field (no zero-arg `loaded_skills` method). Property-vs-getter idiom.
signalwire.register_skill: idiom: Rust `register_skill(skill_class)` takes a `SkillFactory` function pointer (Rust has no runtime class objects); Python takes a `SkillSpec`/`SkillBase` class. Factory-fn vs class idiom.

### POM / Context / DataMap / FunctionResult / security idioms (pre-existing loose params)

signalwire.core.agent_base.AgentBase.on_debug_event: rust-idiom-typed-closure: Rust `on_debug_event` takes a Rust closure surfaced as `any`; Python types it `callable<[any], any>`. Closure-param idiom.
signalwire.core.contexts.Context.set_enter_fillers: rust-idiom-loose-param: Rust `set_enter_fillers(&Value)` carries the filler map as one `serde_json::Value`; Python types it `dict<string,list<string>>`. Open-value idiom.
signalwire.core.contexts.Context.set_exit_fillers: rust-idiom-loose-param: Rust `set_exit_fillers(&Value)` carries the filler map as one `serde_json::Value`; Python types it `dict<string,list<string>>`. Open-value idiom.
signalwire.core.data_map.DataMap.fallback_output: rust-idiom-loose-param: Rust `fallback_output(result: &Value)` takes the result as a JSON value (the FunctionResult serialized form); Python types it `class:FunctionResult`. Serialized-value idiom.
signalwire.core.data_map.DataMap.output: rust-idiom-loose-param: Rust `output(result: &Value)` takes the result as a JSON value; Python types it `class:FunctionResult`. Serialized-value idiom.
signalwire.core.data_map.DataMap.webhook: rust-idiom-loose-param: Rust `webhook(method, url, headers: &Value)` carries optional headers as a JSON value; Python types them `optional<dict<string,string>>`. Open-value idiom.
signalwire.core.function_result.FunctionResult.create_payment_prompt: rust-idiom-loose-param: Rust `create_payment_prompt(actions: &Value)` carries the actions list as one JSON value; Python types it `list<dict<string,string>>`. Open-value idiom.
signalwire.core.function_result.FunctionResult.execute_rpc: rust-idiom-loose-param: Rust `execute_rpc(method, params: &Value)` carries the params as one JSON value; Python types it `optional<dict<string,any>>`. Open-value idiom.
signalwire.core.security.security_utils.filter_sensitive_headers: idiom: Rust `filter_sensitive_headers<V>(headers: &HashMap<String, V>) -> HashMap<String, V>` is generic over the header value type (surfaced as a `_V` typevar); Python fixes `dict<string,string>`. Generic-value idiom (the redaction is value-type agnostic).
signalwire.pom.pom.PromptObjectModel.sections: idiom: Python `@property sections -> list<Section>` read accessor; Rust exposes the sections as a public struct field / via `find_section` (no zero-arg `sections` method). Property-vs-field idiom.
signalwire.pom.pom.Section.add_bullets: rust-idiom-loose-param: Rust `add_bullets(bullets: &Value)` carries the bullet list as one JSON value; Python types it `list<string>`. Open-value idiom.
signalwire.pom.pom.Section.subsections: idiom: Python `@property subsections -> list<Section>` read accessor; Rust exposes the subsections as a public struct field (no zero-arg `subsections` method). Property-vs-field idiom.
signalwire.relay.message.Message.on: rust-idiom-typed-closure: Rust `Message.on(event, handler)` takes a Rust closure surfaced as `any`; Python types it `callable<[RelayEvent], any>`. Closure-param idiom.
signalwire.rest._base.CrudWithAddresses.list_addresses: idiom: the reference `list_addresses(resource_id, params)` lives on the `CrudWithAddresses` mixin (base copy); the Rust port carries the real `list_addresses` on the concrete fabric resources (FabricResource) with the exploded params — the abstract-base copy has the reference shape only. Abstract-base idiom (concrete copies checked on both sides).

## RequestOptions envelope (plan 4.2)

The request-options envelope adds an optional `request_options` to the REST
verbs + client constructors, plus the `RequestOptions` value type. Rust expresses
Python's optional `request_options=` kwarg as a distinct `*_with_options` method
(no default/keyword args) and Python's dataclass fields as chained builder
setters, so the base verb / bare constructor signatures diverge by that one arg.

signalwire.rest._base.HttpClient.put: idiom: Rust `put(&self, path, data: &Value) -> Result<Value, SignalWireRestError>`; the reference gained a `request_options` param (`(self, path, body, request_options)`). Rust threads the per-request override through the sibling `put_with_options` (no default-arg overloading) — the client-default RequestOptions still applies to plain `put`. Result + &Value-body idiom.
signalwire.rest._base.HttpClient.patch: idiom: Rust `patch(&self, path, data: &Value) -> Result<Value, SignalWireRestError>`; the reference gained a `request_options` param (`(self, path, body, request_options)`). Rust threads the per-request override through the sibling `patch_with_options`. Result + &Value-body idiom.
signalwire.rest._request_options.RequestOptions.abort_signal: idiom: `abort_signal` is a dataclass FIELD in Python (accessor `(self)`); Rust exposes it as a chained builder setter `abort_signal(self, signal)` that stores the field. Builder-setter idiom for the same optional field.
signalwire.rest._request_options.RequestOptions.timeout: idiom: `timeout` is a dataclass FIELD in Python (accessor `(self)`); Rust exposes it as a chained builder setter `timeout(self, seconds)` that stores the same `Option<f64>` field. Builder-setter idiom for the same optional field (the field itself is the `pub timeout` struct member — same shape as `abort_signal`).
signalwire.rest._request_options.RequestOptions.retries: idiom: `retries` is a dataclass FIELD in Python (accessor `(self)`); Rust exposes it as a chained builder setter `retries(self, retries)` that stores the same `Option<u32>` field. Builder-setter idiom for the same optional field (same shape as `abort_signal`).
signalwire.rest._request_options.RequestOptions.retry_on_status: idiom: `retry_on_status` is a dataclass FIELD in Python (accessor `(self)`); Rust exposes it as a chained builder setter `retry_on_status(self, statuses)` that stores the same `Option<BTreeSet<u16>>` field. Builder-setter idiom for the same optional field (same shape as `abort_signal`).
signalwire.rest._request_options.RequestOptions.retry_backoff: idiom: `retry_backoff` is a dataclass FIELD in Python (accessor `(self)`); Rust exposes it as a chained builder setter `retry_backoff(self, seconds)` that stores the same `Option<f64>` field. Builder-setter idiom for the same optional field (same shape as `abort_signal`).
signalwire.rest._request_options.resolve: type-alias: Rust `resolve(...) -> EffectiveOptions`; the reference returns the private `_EffectiveOptions`. Rust cannot name a cross-module type with a leading-underscore-private visibility idiom (the retry loop lives in another module and must read the resolved form), so the resolved type is the public `EffectiveOptions` — the same resolved-options struct, sans the reference's private-name underscore.
signalwire.rest._request_options.status_is_retryable: type-alias: Rust `status_is_retryable(method, status, opts: &EffectiveOptions)`; the reference's `opts` is the private `_EffectiveOptions`. Same resolved-options type, Rust's public name (see `resolve`).
signalwire.agent_server.AgentServer.app: impossible: Python's AgentServer exposes its FastAPI instance as a public attribute; Rust keeps the underlying axum/poem app private (the server runs it internally rather than handing it back). Excused HERE not in PORT_OMISSIONS because the surface oracle EXCLUDES this attr (so surface-DIFF flags a PORT_OMISSIONS copy as dead cruft) but the SIGNATURE oracle still records it — dual-gate, so the missing-port drift is excused via this DRIFT-only file.
signalwire.core.agent_base.AgentBase.skill_manager: impossible: Python exposes ``self.skill_manager`` as a SkillManager attribute; Rust owns the SkillManager privately and exposes the skill operations as typed methods on AgentBase (add_skill, list_skills, has_skill, remove_skill) — no per-instance manager object surfaced. Excused HERE (not PORT_OMISSIONS) because the surface fold covers the AgentBase.* → agentbase-family key so a PORT_OMISSIONS copy reads as dead to surface-DIFF, while the SIGNATURE gate keys unfolded and needs this AgentBase-scoped excuse — dual-gate.
signalwire.core.data_map.DataMap.create_expression_tool: impossible: Rust exposes DataMap::create_expression_tool as an associated constructor; the surface enumerator folds it away (composition-delegate) so surface-DIFF reads a PORT_ADDITIONS copy as dead, but the SIGNATURE enumerator emits it as a port-only method — dual-gate, excused here so DRIFT is green.
signalwire.core.data_map.DataMap.create_simple_api_tool: impossible: Rust exposes DataMap::create_simple_api_tool as an associated constructor; surface-folded (so a PORT_ADDITIONS copy is dead to surface-DIFF) but signature-emitted as port-only — dual-gate, excused here for the DRIFT missing-reference.
