# SignalWire AI Agents SDK — Porting Checklist Template

Copy this file for your language port and check items off as you go.

The purpose of tests, examples, and docs is to **prove** complete implementation. If a feature has no test and no example, it's not proven to work. Every feature must have at least one test exercising it.

**Target Language:** _______________
**Start Date:** _______________
**Python SDK Reference:** /path/to/signalwire-agents (the source of truth)

---

## Phase 1: Foundation
- [x] Module/package initialized with git repo (main branch)
- [x] Directory structure (see PORTING_GUIDE.md Module Layout)
- [x] .gitignore
- [x] README.md with quickstart example
- [x] CLAUDE.md with development guidance
- [x] Dependency file (cpanfile, Gemfile, go.mod, build.gradle, package.json, etc.)
- [x] Logging system (levels: debug/info/warn/error, env: SIGNALWIRE_LOG_LEVEL, suppression: SIGNALWIRE_LOG_MODE=off)
- [x] Tests: logger creation, level filtering, suppression, env var config
- [x] Commit to git

## Phase 2: SWML Core
- [x] SWML Document model (version, sections, verbs, JSON rendering)
- [x] Schema loaded from schema.json (embedded in package)
- [x] 38 verb methods auto-vivified from schema (see PORTING_GUIDE.md for mapping)
- [x] Sleep verb: takes integer, not map
- [x] AI verb: present but overridden by AgentBase
- [x] SWMLService HTTP server
- [x] Basic auth (auto-generated or SWML_BASIC_AUTH_USER/PASSWORD)
- [x] Security headers (X-Content-Type-Options, X-Frame-Options, Cache-Control)
- [x] /health, /ready endpoints (no auth)
- [x] Routing callbacks
- [x] SIP username extraction from request body
- [x] SWML_PROXY_URL_BASE support
- [x] Tests: document CRUD, schema loads 38 verbs, all verb methods callable, service auth, HTTP endpoints, security headers
- [x] Commit to git

## Phase 3: Agent Core

### SwaigFunctionResult
- [x] Constructor(response, post_process)
- [x] SetResponse, SetPostProcess, AddAction, AddActions, ToMap/ToDict
- [x] Serialization rules: response always, action only if non-empty, post_process only if true
- [x] All 40+ action methods (see SWAIG_FUNCTION_RESULT_REFERENCE.md for exact JSON)
- [x] Payment helpers: CreatePaymentPrompt, CreatePaymentAction, CreatePaymentParameter
- [x] Method chaining on all methods
- [x] Tests: construction, serialization, each action category (connect, hangup, say, update_global_data, record_call, toggle_functions, execute_rpc, send_sms, payment helpers), method chaining

### SessionManager
- [x] HMAC-SHA256 token creation (functionName:callID:expiry, signed, base64)
- [x] Token validation (timing-safe comparison, expiry check)
- [x] Random 32-byte secret per manager — fail hard if entropy unavailable
- [x] Default expiry: 3600 seconds
- [x] Tests: token round-trip, wrong function/callID rejected, expired rejected, tampered rejected

### DataMap
- [x] Fluent builder: New, Purpose/Description, Parameter (with enum), Expression
- [x] Webhook, WebhookExpressions, Body, Params, Foreach
- [x] Output, FallbackOutput, ErrorKeys, GlobalErrorKeys
- [x] ToSwaigFunction serialization
- [x] CreateSimpleApiTool helper
- [x] CreateExpressionTool helper
- [x] Tests: fluent chain, parameters, webhook config, expressions, serialization, helpers

### Contexts & Steps
- [x] ContextBuilder: AddContext, GetContext, Validate, ToMap
- [x] Context: AddStep, GetStep, RemoveStep, MoveStep, all setters, ToMap
- [x] Step: all setters (text, sections, criteria, functions, navigation, gather, reset), ToMap
- [x] GatherInfo and GatherQuestion
- [x] CreateSimpleContext helper
- [x] Validation: single context must be "default"
- [x] Tests: step config, context with steps, gather info, serialization, validation rules, fillers, MoveStep

### AgentBase
- [x] Constructor with functional options / builder pattern
- [x] Prompt: SetPromptText, SetPostPrompt, POM (AddSection, AddSubsection, AddToSection, HasSection)
- [x] Tools: DefineTool (with handler), RegisterSwaigFunction (DataMap), DefineTools, OnFunctionCall
- [x] AI Config: hints, pattern hints, languages, pronunciations, params, global data, native functions, fillers, debug events, function includes, LLM params
- [x] Verbs: AddPreAnswerVerb, AddAnswerVerb, AddPostAnswerVerb, AddPostAiVerb, Clear methods
- [x] Contexts: DefineContexts returns ContextBuilder
- [x] Skills: AddSkill one-liner, RemoveSkill, ListSkills, HasSkill
- [x] Web: DynamicConfigCallback, proxy URL, webhook URL, post-prompt URL, query params
- [x] SIP: EnableSipRouting, RegisterSipUsername, extractSipUsername utility
- [x] Lifecycle: OnSummary, OnDebugEvent
- [x] SWML Rendering: 5-phase pipeline (pre-answer, answer, post-answer, AI, post-AI)
- [x] HTTP endpoints: / (SWML), /swaig (tool dispatch), /post_prompt (summary), /health, /ready
- [x] Dynamic config: clone agent, apply callback, render from clone, original not mutated
- [x] Webhook URL construction with auth and query params
- [x] Run/Serve/AsRouter (or framework-specific mount)
- [x] Request body size limit (1MB) on all POST handlers
- [x] Tests: construction, prompt modes, tool registration+dispatch, AI config, verbs, contexts, skills integration, render_swml structure, dynamic config isolation, HTTP endpoints (auth, SWML, swaig dispatch, post_prompt, health), method chaining
- [x] Commit to git

## Phase 4: Skills System
- [x] SkillBase interface (see SKILLS_MANIFEST.md for full contract)
- [x] BaseSkill with default implementations
- [x] SkillManager: LoadSkill, UnloadSkill, ListLoadedSkills, HasSkill, GetSkill
- [x] SkillRegistry: RegisterSkill, GetSkillFactory, ListSkills
- [x] All 18 built-in skills (see SKILLS_MANIFEST.md for exact specifications):
  - [x] datetime (get_current_time, get_current_date)
  - [x] math (calculate — safe evaluator, no eval)
  - [x] joke (tell_joke)
  - [x] weather_api (get_weather — HTTP to WeatherAPI.com)
  - [x] web_search (web_search — HTTP to Google CSE)
  - [x] wikipedia_search (search_wiki — HTTP to Wikipedia API)
  - [x] google_maps (lookup_address, compute_route)
  - [x] spider (scrape_url — HTTP fetch + HTML strip)
  - [x] datasphere (search_datasphere — HTTP to SignalWire DataSphere)
  - [x] datasphere_serverless (DataMap-based DataSphere)
  - [x] swml_transfer (transfer_call — pattern matching)
  - [x] play_background_file (play/stop background audio)
  - [x] api_ninjas_trivia (get_trivia)
  - [x] native_vector_search (search_knowledge — network mode only)
  - [x] info_gatherer (start_questions + submit_answer — stateful)
  - [x] claude_skills (SKILL.md file loading)
  - [x] mcp_gateway (MCP server bridge)
  - [x] custom_skills (user-defined tools from config)
- [x] Tests: registry lists 18, each instantiable, skills without env vars setup OK, datetime+math handlers execute, SkillManager load/unload
- [x] Commit to git

## Phase 5: Prefab Agents
- [x] InfoGathererAgent (questions → start_questions + submit_answer tools)
- [x] SurveyAgent (typed questions → validate_response + log_response tools)
- [x] ReceptionistAgent (departments → collect_caller_info + transfer_call tools)
- [x] FAQBotAgent (FAQs → search_faqs tool with keyword scoring)
- [x] ConciergeAgent (venue → check_availability + get_directions tools)
- [x] Tests: each constructible, each has expected tools, tool handlers execute
- [x] Commit to git

## Phase 6: AgentServer
- [x] Register/Unregister agents by route
- [x] GetAgents/GetAgent
- [x] SIP routing (SetupSipRouting, RegisterSipUsername)
- [x] Static file serving (with path traversal protection, security headers, MIME types)
- [x] Health/ready endpoints
- [x] Root index listing agents
- [x] Run with HTTP server
- [x] Tests: register/unregister, get agents, health endpoint, route dispatch, SIP routing, static file serving
- [x] Commit to git

## Phase 7: RELAY Client
- [x] WebSocket connection to wss://{space}
- [x] JSON-RPC 2.0 framing
- [x] Authentication (project/token and JWT)
- [x] Authorization state for fast reconnect
- [x] Auto-reconnect with exponential backoff (1s → 30s)
- [x] 4 correlation mechanisms (JSON-RPC id, call_id, control_id, tag)
- [x] Event ACK (immediate response to signalwire.event)
- [x] Ping handling (respond to signalwire.ping)
- [x] Server disconnect handling (restart flag)
- [x] Context subscription/unsubscription
- [x] Call object with 30+ methods (see RELAY_IMPLEMENTATION_GUIDE.md)
- [x] 11 action types with Wait/Stop/IsDone/OnCompleted
- [x] PlayAction: Pause, Resume, Volume
- [x] play_and_collect gotcha handled (filter by event_type)
- [x] detect gotcha handled (resolve on first meaningful result)
- [x] dial tag correlation (call_id nested in params.call.call_id)
- [x] call-gone (404/410) handled gracefully
- [x] 22+ typed event types
- [x] SMS/MMS messaging (SendMessage, OnMessage, delivery tracking)
- [x] Tests: constants, event parsing (all types), action wait/resolve/callback, call creation, message state, client construction, correlation mechanism verification
- [x] Commit to git

## Phase 8: REST Client
- [x] HTTP client with Basic Auth
- [x] CrudResource (List, Create, Get, Update, Delete)
- [x] Pagination support
- [x] SignalWireRestError
- [x] All namespaces (see rest-apis/ OpenAPI specs):
  - [x] Fabric (16+ sub-resources)
  - [x] Calling (37 commands)
  - [x] PhoneNumbers
  - [x] Datasphere
  - [x] Video
  - [x] Compat (Twilio LAML)
  - [x] Addresses, Queues, Recordings
  - [x] NumberGroups, VerifiedCallers, SipProfile
  - [x] Lookup, ShortCodes, ImportedNumbers
  - [x] MFA, Registry, Logs
  - [x] Project, PubSub, Chat
- [x] Tests: client creation, all namespaces initialized (non-nil), CRUD path construction, error formatting, sub-resource verification
- [x] Commit to git

## Phase 9: Serverless (Optional)
- [x] AWS Lambda adapter
- [x] Google Cloud Functions adapter
- [x] Azure Functions adapter
- [x] CGI adapter
- [x] Auto-detection

## Phase 10: CLI
- [x] swaig-test: --url, --dump-swml, --list-tools, --exec, --param, --raw, --verbose
- [x] URL auth extraction (http://user:pass@host:port/path)
- [x] Tests: URL parsing, param parsing, integration with live agent
- [x] Commit to git

## Phase 11: Documentation & Examples

Documentation and examples prove the implementation is complete and usable.

**The rule is simple: if the Python SDK has a doc or example (except search-related and bedrock-related), the port must have an equivalent in the target language.** This is not a suggestion — it's a requirement. Read the Python SDK's `docs/`, `examples/`, `relay/`, and `rest/` directories and port every file. Missing docs or examples mean missing proof that the feature works.

Do NOT copy `RELAY_IMPLEMENTATION_GUIDE.md` into language repos — the canonical copy lives in the porting-sdk repo and is only needed during development.

### Top-level docs/ (20 files — ALL from Python SDK except search/bedrock/comparison)

Port every one of these. Each must contain code examples in the target language, not Python.

- [x] architecture.md
- [x] agent_guide.md
- [x] api_reference.md
- [x] swaig_reference.md
- [x] datamap_guide.md
- [x] contexts_guide.md
- [x] skills_system.md
- [x] skills_parameter_schema.md
- [x] third_party_skills.md
- [x] security.md
- [x] configuration.md
- [x] llm_parameters.md
- [x] sdk_features.md
- [x] cli_guide.md
- [x] swml_service_guide.md
- [x] web_service.md
- [x] cloud_functions_guide.md
- [x] mcp_gateway_reference.md
- [x] mcp_integration.md

Skip: search_*.md (4 files), bedrock_agent.md, livekit_comparison.md, pipecat_comparison.md

### Top-level relay/ directory (REQUIRED — 9 files)
- [x] relay/README.md (API overview, quick start, code examples in target language)
- [x] relay/docs/getting-started.md
- [x] relay/docs/call-methods.md
- [x] relay/docs/events.md
- [x] relay/docs/messaging.md
- [x] relay/docs/client-reference.md
- [x] relay/examples/relay_answer_and_welcome.* (proves: answer, play TTS, hangup)
- [x] relay/examples/relay_dial_and_play.* (proves: outbound dial, play, hangup)
- [x] relay/examples/relay_ivr_connect.* (proves: collect DTMF, connect to department)

### Top-level rest/ directory (REQUIRED — 19 files)
- [x] rest/README.md (API overview, namespace examples in target language)
- [x] rest/docs/getting-started.md
- [x] rest/docs/namespaces.md
- [x] rest/docs/calling.md
- [x] rest/docs/fabric.md
- [x] rest/docs/compat.md
- [x] rest/docs/client-reference.md
- [x] rest/examples/rest_10dlc_registration.* (proves: registry namespace)
- [x] rest/examples/rest_calling_ivr_and_ai.* (proves: calling namespace)
- [x] rest/examples/rest_calling_play_and_record.* (proves: calling play/record)
- [x] rest/examples/rest_compat_laml.* (proves: compat namespace)
- [x] rest/examples/rest_datasphere_search.* (proves: datasphere namespace)
- [x] rest/examples/rest_fabric_conferences_and_routing.* (proves: fabric sub-resources)
- [x] rest/examples/rest_fabric_subscribers_and_sip.* (proves: fabric SIP)
- [x] rest/examples/rest_fabric_swml_and_callflows.* (proves: fabric SWML)
- [x] rest/examples/rest_manage_resources.* (proves: CRUD operations)
- [x] rest/examples/rest_phone_number_management.* (proves: phone numbers)
- [x] rest/examples/rest_queues_mfa_and_recordings.* (proves: queues, MFA, recordings)
- [x] rest/examples/rest_video_rooms.* (proves: video namespace)

### Agent examples/ directory (port ALL from Python except search/bedrock)

Every Python example has a counterpart in the port. The list below is the minimum — if Python has more, port those too. Run `ls ~/src/signalwire-python/examples/*.py` to get the current list.

- [x] examples/README.md (index with descriptions)
- [x] simple_agent.* (proves: AgentBase, prompt, tools, hints, language, run)
- [x] simple_dynamic_agent.* (proves: DynamicConfigCallback, per-request customization)
- [x] simple_dynamic_enhanced.* (proves: advanced dynamic config)
- [x] simple_static_agent.* (proves: static config, no dynamic callback)
- [x] multi_agent_server.* (proves: AgentServer, multiple agents, route dispatch)
- [x] multi_endpoint_agent.* (proves: single agent, multiple endpoints)
- [x] contexts_demo.* (proves: DefineContexts, steps, criteria, navigation, fillers)
- [x] data_map_demo.* (proves: DataMap webhook + expression tools)
- [x] advanced_datamap_demo.* (proves: advanced DataMap patterns)
- [x] skills_demo.* (proves: AddSkill one-liner, skill registry)
- [x] joke_skill_demo.* (proves: joke skill with API key)
- [x] web_search_agent.* (proves: web search skill)
- [x] web_search_multi_instance_demo.* (proves: multiple skill instances)
- [x] wikipedia_demo.* (proves: wikipedia search skill)
- [x] datasphere_serverless_demo.* (proves: datasphere serverless skill)
- [x] datasphere_serverless_env_demo.* (proves: datasphere with env vars)
- [x] datasphere_webhook_env_demo.* (proves: datasphere webhook)
- [x] datasphere_multi_instance_demo.* (proves: multiple datasphere instances)
- [x] session_and_state_demo.* (proves: global data, post-prompt, OnSummary callback)
- [x] call_flow_and_actions_demo.* (proves: pre/post answer verbs, debug events, FunctionResult actions)
- [x] swaig_features_agent.* (proves: type inference, fillers, webhook URLs)
- [x] comprehensive_dynamic_agent.* (proves: per-request dynamic config, multi-tenant)
- [x] gather_info_demo.* (proves: GatherInfo/GatherQuestion)
- [x] llm_params_demo.* (proves: LLM parameter tuning)
- [x] record_call_example.* (proves: call recording)
- [x] tap_example.* (proves: call tapping)
- [x] room_and_sip_example.* (proves: SIP routing, rooms)
- [x] custom_path_agent.* (proves: custom routes)
- [x] auto_vivified_example.* (proves: auto-vivified SWML verbs)
- [x] basic_swml_service.* (proves: SWMLService without AgentBase)
- [x] dynamic_swml_service.* (proves: dynamic SWML generation)
- [x] swml_service_example.* (proves: SWML service patterns)
- [x] swml_service_routing_example.* (proves: SWML service routing)
- [x] declarative_agent.* (proves: declarative config)
- [x] lambda_agent.* (proves: AWS Lambda deployment)
- [x] kubernetes_ready_agent.* (proves: K8s deployment patterns)
- [x] mcp_agent.* (proves: MCP integration)
- [x] mcp_gateway_demo.* (proves: MCP gateway skill)
- [x] info_gatherer_example.* (proves: InfoGathererAgent prefab)
- [x] dynamic_info_gatherer_example.* (proves: dynamic InfoGatherer)
- [x] survey_agent_example.* (proves: SurveyAgent prefab)
- [x] faq_bot_agent.* (proves: FAQBotAgent prefab)
- [x] receptionist_agent_example.* (proves: ReceptionistAgent prefab)
- [x] concierge_agent_example.* (proves: ConciergeAgent prefab)
- [x] joke_agent.* (proves: simple agent with joke skill)
- [x] relay_answer_and_welcome.* (proves: RELAY answer+play — also in relay/examples/)

Skip: bedrock_*.py, search_*.py, pgvector_*.py, sigmond_*.py

### Commit to git

## Phase 12: Testing Verification

Tests are proof of implementation. The port must test **everything the Python SDK tests**. Read the Python test files in `tests/unit/` and ensure equivalent coverage exists in your port for every tested behavior.

- [x] Every public method has at least one test exercising it
- [x] Every test the Python SDK has (except search-related) has an equivalent in the port
- [x] All tests pass with zero failures, no tests skipped
- [x] Test coverage matches Python SDK organization:
  - [x] Core: agent_base, swml_service, swml_builder, swml_renderer, swml_handler
  - [x] SWAIG: swaig_function, function_result (all 40+ actions)
  - [x] Security: session_manager, auth_handler
  - [x] DataMap: data_map (all builder methods, serialization)
  - [x] Contexts: contexts (steps, navigation, validation, gather_info)
  - [x] Mixins/Config: prompt, tool, web, auth, serverless, state, ai_config, skill
  - [x] Skills: registry, manager, each of the 18 built-in skills individually
  - [x] Prefabs: each of the 5 prefab agents
  - [x] AgentServer: registration, routing, SIP, static files
  - [x] RELAY: client, call, action types, events, messages
  - [x] REST: client, base resource, each major namespace, pagination
  - [x] CLI: argument parsing, tool listing, execution
  - [x] Utilities: schema_utils, logging, pom_builder, type_inference

## Phase 13: Final Audit (REQUIRED)

### Completeness Audit
- [x] Every AgentBase public method from Python SDK has an equivalent
- [x] All 40+ SwaigFunctionResult action methods present (including payment helpers)
- [x] All 38 SWML verb methods present and schema-validated
- [x] RELAY client: 4 correlation mechanisms implemented
- [x] REST client: all 21+ namespaces initialized with correct paths
- [x] Skills registry: all 18 built-in skills registered
- [x] agent.AddSkill() one-liner integration works (not just manual SkillManager)
- [x] SIP username extraction utility exists
- [x] Static file serving in AgentServer with path traversal protection
- [x] No TODO/FIXME/HACK/PLACEHOLDER comments remain
- [x] Every example compiles/runs without syntax errors
- [x] Top-level relay/ and rest/ directories have README, docs, examples

### Security Audit
Read all source code and review the full implementation for security issues. The items below are known vulnerabilities found in prior ports — check these first, then review for anything else specific to your language/framework:
- [x] Basic auth uses timing-safe comparison (NOT `==`)
- [x] Passwords never appear in log output
- [x] No weak fallback passwords — fail to start if crypto/rand fails
- [x] All POST handlers enforce request body size limits (1MB)
- [x] SIP username extraction validates input format
- [x] JSON parse errors are checked, not silently ignored
- [x] All shared state protected by mutexes (global data, tool registry, RELAY maps)
- [x] HMAC token validation uses timing-safe comparison
- [x] Security headers set on all authenticated endpoints
- [x] Third-party dependencies checked for known vulnerabilities
- [x] General review: no other injection, XSS, SSRF, or language-specific vulnerabilities
