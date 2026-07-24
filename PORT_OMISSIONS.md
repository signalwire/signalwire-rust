# PORT_OMISSIONS.md (signalwire-rust)

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

Python symbols deliberately not implemented in this Rust port. Each entry states an
`impossible:` or `approved:` reason. Both the surface diff (folded keys) and the
signature diff (unfolded per-class keys, incl. class-level coverage) read this file.

> NOTE: `signalwire.rest.*` `*Resource`/`*Namespace` entries are FROZEN pending the
> spec-driven REST base+subclass parity pass — do not delete/rename them here.

---

agentbase-family.skill_manager: impossible: Python AgentBase exposes ``self.skill_manager`` as a SkillManager attribute; Rust owns the SkillManager privately and exposes the skill operations as methods — no per-instance manager object surfaced.
signalwire.agent_server.AgentServer.agents: impossible: Python exposes ``agents`` as a public dict attribute; Rust keeps the map private and exposes it via ``get_agents()`` (private-field + accessor idiom, matching go).
signalwire.agent_server.AgentServer.logger: impossible: Python exposes ``self.logger`` as a per-instance logging.Logger attribute; Rust logging is module-level (``tracing``/``log`` macros), no per-instance logger struct field to surface (same divergence cpp/go record).
signalwire.ai_chat.client.AIChatClient.__aenter__: impossible: Rust has no async-context-manager (`async with`) protocol; the pooled `reqwest::Client` is used directly. No `__aenter__` analogue exists (TS cousin omits it too).
signalwire.ai_chat.client.AIChatClient.__aexit__: impossible: Rust has no async-context-manager protocol (see `__aenter__`); the client is released on drop. No `__aexit__` analogue exists (TS cousin omits it too).
signalwire.core.agent.tools.decorator.ToolDecorator: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.agent.tools.decorator.ToolDecorator.create_class_decorator: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.agent.tools.decorator.ToolDecorator.create_instance_decorator: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.agent.tools.registry.ToolRegistry.register_class_decorated_tools: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.contexts.create_simple_context: Python helper that wraps ContextBuilder; Rust users call ContextBuilder::new() directly
signalwire.core.data_map.create_expression_tool: Python helper functions; Rust ships create_simple_api_tool and create_expression_tool — already exposed (this entry is a duplicate from a Python-side path-prefix difference)
signalwire.core.data_map.create_simple_api_tool: Python helper functions; Rust ships create_simple_api_tool and create_expression_tool — already exposed (this entry is a duplicate from a Python-side path-prefix difference)
signalwire.core.mixins.mcp_server_mixin.MCPServerMixin: impossible: Python decorator-protocol surface (the @tool / MCPServerMixin decorator factory) — Rust has no decorator syntax; the OO cousins TS/PHP also express tool registration without this decorator method (re-audited L18)
signalwire.core.pom_builder.PomBuilder: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.security.webhook_middleware.make_webhook_validation_dependency: impossible: FastAPI dependency-factory (make_webhook_validation_dependency returns a Depends() callable) — a framework-specific DI primitive with no Rust equivalent; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.skill_base.SkillBase.logger: impossible: see AgentServer.logger — Rust uses module-level ``tracing`` macros, no per-instance logger attribute on the SkillBase trait.
signalwire.core.skill_manager.SkillManager.loaded_skills: impossible: Python exposes ``loaded_skills`` as a public dict attribute; Rust keeps the HashMap private and exposes it via ``list_loaded_skills()``.
signalwire.core.skill_manager.SkillManager.logger: impossible: see AgentServer.logger — Rust uses module-level ``tracing`` macros, no per-instance logger attribute on SkillManager.
signalwire.core.swaig_function.SWAIGFunction: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swml_builder.SWMLBuilder: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_handler.AIVerbHandler.build_config: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_service.SWMLService.security: impossible: Python's ``self.security`` is a SecurityConfig object attribute; Rust holds auth state in private fields and exposes only the operations (get_basic_auth_credentials, validate_basic_auth) — no security object surfaced (same fold cpp/go apply).
signalwire.core.swml_service.SWMLService.verb_registry: impossible: Python's ``self.verb_registry`` exposes the VerbHandlerRegistry map; Rust keeps it private and exposes only the verb-registration operations (add_verb, register_verb_handler) — no registry object surfaced (same fold cpp/go apply).
signalwire.prefabs.concierge.ConciergeAgent.on_summary: Python ConciergeAgent internals; Rust ships the canonical prefab
signalwire.prefabs.faq_bot.FAQBotAgent.on_summary: Python FAQBotAgent internals; Rust ships the canonical prefab
signalwire.prefabs.receptionist.ReceptionistAgent.on_summary: Python ReceptionistAgent internals; Rust ships the canonical prefab
signalwire.prefabs.survey.SurveyAgent.on_summary: Python SurveyAgent internals; Rust ships the canonical prefab
signalwire.relay.client.RelayClient.__aenter__: impossible: Python async context-manager protocol dunder (__aenter__/__aexit__) — no Rust equivalent; TS/PHP cousins also omit these protocol methods (re-audited L18)
signalwire.relay.client.RelayClient.__aexit__: impossible: Python async context-manager protocol dunder (__aenter__/__aexit__) — no Rust equivalent; TS/PHP cousins also omit these protocol methods (re-audited L18)
signalwire.relay.client.RelayClient.__del__: impossible: Python finalizer dunder (__del__) — Rust uses Drop, not a reference-counted finalizer method on the public surface; TS/PHP cousins also omit it (re-audited L18)
signalwire.relay.client.RelayClient.relay_protocol: impossible: Python abstract relay-protocol property hook — Rust models the RELAY protocol via concrete client methods, no abstract protocol accessor; TS/PHP cousins also omit it (re-audited L18)
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.get_tools: Python api_ninjas_trivia skill internals; Rust ships the canonical skill
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.get_tools: Python play_background_file internals; Rust ships the canonical skill
signalwire.skills.registry.SkillRegistry.logger: impossible: see AgentServer.logger — Rust uses module-level ``tracing`` macros, no per-instance logger attribute on SkillRegistry.
signalwire.skills.weather_api.skill.WeatherApiSkill.get_tools: Python weather_api skill internals; Rust ships the canonical skill
signalwire.web.web_service.WebService: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.web.web_service.WebService.security: impossible: Python's WebService ``security`` is a SecurityConfig attribute; Rust holds auth state privately and exposes only the operations, mirroring SWMLService.security.
