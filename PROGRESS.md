# SignalWire AI Agents SDK — Porting Checklist Template

Copy this file for your language port and check items off as you go.

The purpose of tests, examples, and docs is to **prove** complete implementation. If a feature has no test and no example, it's not proven to work. Every feature must have at least one test exercising it.

**Target Language:** _______________
**Start Date:** _______________
**Python SDK Reference:** /path/to/signalwire-agents (the source of truth)

---

## Phase 1: Foundation
- [ ] Module/package initialized with git repo (main branch)
- [ ] Directory structure (see PORTING_GUIDE.md Module Layout)
- [ ] .gitignore
- [ ] README.md with quickstart example
- [ ] CLAUDE.md with development guidance
- [ ] Dependency file (cpanfile, Gemfile, go.mod, build.gradle, package.json, etc.)
- [ ] Logging system (levels: debug/info/warn/error, env: SIGNALWIRE_LOG_LEVEL, suppression: SIGNALWIRE_LOG_MODE=off)
- [ ] Tests: logger creation, level filtering, suppression, env var config
- [ ] Commit to git

## Phase 2: SWML Core
- [ ] SWML Document model (version, sections, verbs, JSON rendering)
- [ ] Schema loaded from schema.json (embedded in package)
- [ ] 38 verb methods auto-vivified from schema (see PORTING_GUIDE.md for mapping)
- [ ] Sleep verb: takes integer, not map
- [ ] AI verb: present but overridden by AgentBase
- [ ] SWMLService HTTP server
- [ ] Basic auth (auto-generated or SWML_BASIC_AUTH_USER/PASSWORD)
- [ ] Security headers (X-Content-Type-Options, X-Frame-Options, Cache-Control)
- [ ] /health, /ready endpoints (no auth)
- [ ] Routing callbacks
- [ ] SIP username extraction from request body
- [ ] SWML_PROXY_URL_BASE support
- [ ] Tests: document CRUD, schema loads 38 verbs, all verb methods callable, service auth, HTTP endpoints, security headers
- [ ] Commit to git

## Phase 3: Agent Core

### SwaigFunctionResult
- [ ] Constructor(response, post_process)
- [ ] SetResponse, SetPostProcess, AddAction, AddActions, ToMap/ToDict
- [ ] Serialization rules: response always, action only if non-empty, post_process only if true
- [ ] All 40+ action methods (see SWAIG_FUNCTION_RESULT_REFERENCE.md for exact JSON)
- [ ] Payment helpers: CreatePaymentPrompt, CreatePaymentAction, CreatePaymentParameter
- [ ] Method chaining on all methods
- [ ] Tests: construction, serialization, each action category (connect, hangup, say, update_global_data, record_call, toggle_functions, execute_rpc, send_sms, payment helpers), method chaining

### SessionManager
- [ ] HMAC-SHA256 token creation (functionName:callID:expiry, signed, base64)
- [ ] Token validation (timing-safe comparison, expiry check)
- [ ] Random 32-byte secret per manager — fail hard if entropy unavailable
- [ ] Default expiry: 3600 seconds
- [ ] Tests: token round-trip, wrong function/callID rejected, expired rejected, tampered rejected

### DataMap
- [ ] Fluent builder: New, Purpose/Description, Parameter (with enum), Expression
- [ ] Webhook, WebhookExpressions, Body, Params, Foreach
- [ ] Output, FallbackOutput, ErrorKeys, GlobalErrorKeys
- [ ] ToSwaigFunction serialization
- [ ] CreateSimpleApiTool helper
- [ ] CreateExpressionTool helper
- [ ] Tests: fluent chain, parameters, webhook config, expressions, serialization, helpers

### Contexts & Steps
- [ ] ContextBuilder: AddContext, GetContext, Validate, ToMap
- [ ] Context: AddStep, GetStep, RemoveStep, MoveStep, all setters, ToMap
- [ ] Step: all setters (text, sections, criteria, functions, navigation, gather, reset), ToMap
- [ ] GatherInfo and GatherQuestion
- [ ] CreateSimpleContext helper
- [ ] Validation: single context must be "default"
- [ ] Tests: step config, context with steps, gather info, serialization, validation rules, fillers, MoveStep

### AgentBase
- [ ] Constructor with functional options / builder pattern
- [ ] Prompt: SetPromptText, SetPostPrompt, POM (AddSection, AddSubsection, AddToSection, HasSection)
- [ ] Tools: DefineTool (with handler), RegisterSwaigFunction (DataMap), DefineTools, OnFunctionCall
- [ ] AI Config: hints, pattern hints, languages, pronunciations, params, global data, native functions, fillers, debug events, function includes, LLM params
- [ ] Verbs: AddPreAnswerVerb, AddAnswerVerb, AddPostAnswerVerb, AddPostAiVerb, Clear methods
- [ ] Contexts: DefineContexts returns ContextBuilder
- [ ] Skills: AddSkill one-liner, RemoveSkill, ListSkills, HasSkill
- [ ] Web: DynamicConfigCallback, proxy URL, webhook URL, post-prompt URL, query params
- [ ] SIP: EnableSipRouting, RegisterSipUsername, extractSipUsername utility
- [ ] Lifecycle: OnSummary, OnDebugEvent
- [ ] SWML Rendering: 5-phase pipeline (pre-answer, answer, post-answer, AI, post-AI)
- [ ] HTTP endpoints: / (SWML), /swaig (tool dispatch), /post_prompt (summary), /health, /ready
- [ ] Dynamic config: clone agent, apply callback, render from clone, original not mutated
- [ ] Webhook URL construction with auth and query params
- [ ] Run/Serve/AsRouter (or framework-specific mount)
- [ ] Request body size limit (1MB) on all POST handlers
- [ ] Tests: construction, prompt modes, tool registration+dispatch, AI config, verbs, contexts, skills integration, render_swml structure, dynamic config isolation, HTTP endpoints (auth, SWML, swaig dispatch, post_prompt, health), method chaining
- [ ] Commit to git

## Phase 4: Skills System
- [ ] SkillBase interface (see SKILLS_MANIFEST.md for full contract)
- [ ] BaseSkill with default implementations
- [ ] SkillManager: LoadSkill, UnloadSkill, ListLoadedSkills, HasSkill, GetSkill
- [ ] SkillRegistry: RegisterSkill, GetSkillFactory, ListSkills
- [ ] All 18 built-in skills (see SKILLS_MANIFEST.md for exact specifications):
  - [ ] datetime (get_current_time, get_current_date)
  - [ ] math (calculate — safe evaluator, no eval)
  - [ ] joke (tell_joke)
  - [ ] weather_api (get_weather — HTTP to WeatherAPI.com)
  - [ ] web_search (web_search — HTTP to Google CSE)
  - [ ] wikipedia_search (search_wiki — HTTP to Wikipedia API)
  - [ ] google_maps (lookup_address, compute_route)
  - [ ] spider (scrape_url — HTTP fetch + HTML strip)
  - [ ] datasphere (search_datasphere — HTTP to SignalWire DataSphere)
  - [ ] datasphere_serverless (DataMap-based DataSphere)
  - [ ] swml_transfer (transfer_call — pattern matching)
  - [ ] play_background_file (play/stop background audio)
  - [ ] api_ninjas_trivia (get_trivia)
  - [ ] native_vector_search (search_knowledge — network mode only)
  - [ ] info_gatherer (start_questions + submit_answer — stateful)
  - [ ] claude_skills (SKILL.md file loading)
  - [ ] mcp_gateway (MCP server bridge)
  - [ ] custom_skills (user-defined tools from config)
- [ ] Tests: registry lists 18, each instantiable, skills without env vars setup OK, datetime+math handlers execute, SkillManager load/unload
- [ ] Commit to git

## Phase 5: Prefab Agents
- [ ] InfoGathererAgent (questions → start_questions + submit_answer tools)
- [ ] SurveyAgent (typed questions → validate_response + log_response tools)
- [ ] ReceptionistAgent (departments → collect_caller_info + transfer_call tools)
- [ ] FAQBotAgent (FAQs → search_faqs tool with keyword scoring)
- [ ] ConciergeAgent (venue → check_availability + get_directions tools)
- [ ] Tests: each constructible, each has expected tools, tool handlers execute
- [ ] Commit to git

## Phase 6: AgentServer
- [ ] Register/Unregister agents by route
- [ ] GetAgents/GetAgent
- [ ] SIP routing (SetupSipRouting, RegisterSipUsername)
- [ ] Static file serving (with path traversal protection, security headers, MIME types)
- [ ] Health/ready endpoints
- [ ] Root index listing agents
- [ ] Run with HTTP server
- [ ] Tests: register/unregister, get agents, health endpoint, route dispatch, SIP routing, static file serving
- [ ] Commit to git

## Phase 7: RELAY Client
- [ ] WebSocket connection to wss://{space}
- [ ] JSON-RPC 2.0 framing
- [ ] Authentication (project/token and JWT)
- [ ] Authorization state for fast reconnect
- [ ] Auto-reconnect with exponential backoff (1s → 30s)
- [ ] 4 correlation mechanisms (JSON-RPC id, call_id, control_id, tag)
- [ ] Event ACK (immediate response to signalwire.event)
- [ ] Ping handling (respond to signalwire.ping)
- [ ] Server disconnect handling (restart flag)
- [ ] Context subscription/unsubscription
- [ ] Call object with 30+ methods (see RELAY_IMPLEMENTATION_GUIDE.md)
- [ ] 11 action types with Wait/Stop/IsDone/OnCompleted
- [ ] PlayAction: Pause, Resume, Volume
- [ ] play_and_collect gotcha handled (filter by event_type)
- [ ] detect gotcha handled (resolve on first meaningful result)
- [ ] dial tag correlation (call_id nested in params.call.call_id)
- [ ] call-gone (404/410) handled gracefully
- [ ] 22+ typed event types
- [ ] SMS/MMS messaging (SendMessage, OnMessage, delivery tracking)
- [ ] Tests: constants, event parsing (all types), action wait/resolve/callback, call creation, message state, client construction, correlation mechanism verification
- [ ] Commit to git

## Phase 8: REST Client
- [ ] HTTP client with Basic Auth
- [ ] CrudResource (List, Create, Get, Update, Delete)
- [ ] Pagination support
- [ ] SignalWireRestError
- [ ] All namespaces (see rest-apis/ OpenAPI specs):
  - [ ] Fabric (16+ sub-resources)
  - [ ] Calling (37 commands)
  - [ ] PhoneNumbers
  - [ ] Datasphere
  - [ ] Video
  - [ ] Compat (Twilio LAML)
  - [ ] Addresses, Queues, Recordings
  - [ ] NumberGroups, VerifiedCallers, SipProfile
  - [ ] Lookup, ShortCodes, ImportedNumbers
  - [ ] MFA, Registry, Logs
  - [ ] Project, PubSub, Chat
- [ ] Tests: client creation, all namespaces initialized (non-nil), CRUD path construction, error formatting, sub-resource verification
- [ ] Commit to git

## Phase 9: Serverless (Optional)
- [ ] AWS Lambda adapter
- [ ] Google Cloud Functions adapter
- [ ] Azure Functions adapter
- [ ] CGI adapter
- [ ] Auto-detection

## Phase 10: CLI
- [ ] swaig-test: --url, --dump-swml, --list-tools, --exec, --param, --raw, --verbose
- [ ] URL auth extraction (http://user:pass@host:port/path)
- [ ] Tests: URL parsing, param parsing, integration with live agent
- [ ] Commit to git

## Phase 11: Documentation & Examples

Documentation and examples prove the implementation is complete and usable.

**The rule is simple: if the Python SDK has a doc or example (except search-related and bedrock-related), the port must have an equivalent in the target language.** This is not a suggestion — it's a requirement. Read the Python SDK's `docs/`, `examples/`, `relay/`, and `rest/` directories and port every file. Missing docs or examples mean missing proof that the feature works.

Do NOT copy `RELAY_IMPLEMENTATION_GUIDE.md` into language repos — the canonical copy lives in the porting-sdk repo and is only needed during development.

### Top-level docs/ (20 files — ALL from Python SDK except search/bedrock/comparison)

Port every one of these. Each must contain code examples in the target language, not Python.

- [ ] architecture.md
- [ ] agent_guide.md
- [ ] api_reference.md
- [ ] swaig_reference.md
- [ ] datamap_guide.md
- [ ] contexts_guide.md
- [ ] skills_system.md
- [ ] skills_parameter_schema.md
- [ ] third_party_skills.md
- [ ] security.md
- [ ] configuration.md
- [ ] llm_parameters.md
- [ ] sdk_features.md
- [ ] cli_guide.md
- [ ] swml_service_guide.md
- [ ] web_service.md
- [ ] cloud_functions_guide.md
- [ ] mcp_gateway_reference.md
- [ ] mcp_integration.md

Skip: search_*.md (4 files), bedrock_agent.md, livekit_comparison.md, pipecat_comparison.md

### Top-level relay/ directory (REQUIRED — 9 files)
- [ ] relay/README.md (API overview, quick start, code examples in target language)
- [ ] relay/docs/getting-started.md
- [ ] relay/docs/call-methods.md
- [ ] relay/docs/events.md
- [ ] relay/docs/messaging.md
- [ ] relay/docs/client-reference.md
- [ ] relay/examples/relay_answer_and_welcome.* (proves: answer, play TTS, hangup)
- [ ] relay/examples/relay_dial_and_play.* (proves: outbound dial, play, hangup)
- [ ] relay/examples/relay_ivr_connect.* (proves: collect DTMF, connect to department)

### Top-level rest/ directory (REQUIRED — 19 files)
- [ ] rest/README.md (API overview, namespace examples in target language)
- [ ] rest/docs/getting-started.md
- [ ] rest/docs/namespaces.md
- [ ] rest/docs/calling.md
- [ ] rest/docs/fabric.md
- [ ] rest/docs/compat.md
- [ ] rest/docs/client-reference.md
- [ ] rest/examples/rest_10dlc_registration.* (proves: registry namespace)
- [ ] rest/examples/rest_calling_ivr_and_ai.* (proves: calling namespace)
- [ ] rest/examples/rest_calling_play_and_record.* (proves: calling play/record)
- [ ] rest/examples/rest_compat_laml.* (proves: compat namespace)
- [ ] rest/examples/rest_datasphere_search.* (proves: datasphere namespace)
- [ ] rest/examples/rest_fabric_conferences_and_routing.* (proves: fabric sub-resources)
- [ ] rest/examples/rest_fabric_subscribers_and_sip.* (proves: fabric SIP)
- [ ] rest/examples/rest_fabric_swml_and_callflows.* (proves: fabric SWML)
- [ ] rest/examples/rest_manage_resources.* (proves: CRUD operations)
- [ ] rest/examples/rest_phone_number_management.* (proves: phone numbers)
- [ ] rest/examples/rest_queues_mfa_and_recordings.* (proves: queues, MFA, recordings)
- [ ] rest/examples/rest_video_rooms.* (proves: video namespace)

### Agent examples/ directory (port ALL from Python except search/bedrock)

Every Python example has a counterpart in the port. The list below is the minimum — if Python has more, port those too. Run `ls ~/src/signalwire-python/examples/*.py` to get the current list.

- [ ] examples/README.md (index with descriptions)
- [ ] simple_agent.* (proves: AgentBase, prompt, tools, hints, language, run)
- [ ] simple_dynamic_agent.* (proves: DynamicConfigCallback, per-request customization)
- [ ] simple_dynamic_enhanced.* (proves: advanced dynamic config)
- [ ] simple_static_agent.* (proves: static config, no dynamic callback)
- [ ] multi_agent_server.* (proves: AgentServer, multiple agents, route dispatch)
- [ ] multi_endpoint_agent.* (proves: single agent, multiple endpoints)
- [ ] contexts_demo.* (proves: DefineContexts, steps, criteria, navigation, fillers)
- [ ] data_map_demo.* (proves: DataMap webhook + expression tools)
- [ ] advanced_datamap_demo.* (proves: advanced DataMap patterns)
- [ ] skills_demo.* (proves: AddSkill one-liner, skill registry)
- [ ] joke_skill_demo.* (proves: joke skill with API key)
- [ ] web_search_agent.* (proves: web search skill)
- [ ] web_search_multi_instance_demo.* (proves: multiple skill instances)
- [ ] wikipedia_demo.* (proves: wikipedia search skill)
- [ ] datasphere_serverless_demo.* (proves: datasphere serverless skill)
- [ ] datasphere_serverless_env_demo.* (proves: datasphere with env vars)
- [ ] datasphere_webhook_env_demo.* (proves: datasphere webhook)
- [ ] datasphere_multi_instance_demo.* (proves: multiple datasphere instances)
- [ ] session_and_state_demo.* (proves: global data, post-prompt, OnSummary callback)
- [ ] call_flow_and_actions_demo.* (proves: pre/post answer verbs, debug events, FunctionResult actions)
- [ ] swaig_features_agent.* (proves: type inference, fillers, webhook URLs)
- [ ] comprehensive_dynamic_agent.* (proves: per-request dynamic config, multi-tenant)
- [ ] gather_info_demo.* (proves: GatherInfo/GatherQuestion)
- [ ] llm_params_demo.* (proves: LLM parameter tuning)
- [ ] record_call_example.* (proves: call recording)
- [ ] tap_example.* (proves: call tapping)
- [ ] room_and_sip_example.* (proves: SIP routing, rooms)
- [ ] custom_path_agent.* (proves: custom routes)
- [ ] auto_vivified_example.* (proves: auto-vivified SWML verbs)
- [ ] basic_swml_service.* (proves: SWMLService without AgentBase)
- [ ] dynamic_swml_service.* (proves: dynamic SWML generation)
- [ ] swml_service_example.* (proves: SWML service patterns)
- [ ] swml_service_routing_example.* (proves: SWML service routing)
- [ ] declarative_agent.* (proves: declarative config)
- [ ] lambda_agent.* (proves: AWS Lambda deployment)
- [ ] kubernetes_ready_agent.* (proves: K8s deployment patterns)
- [ ] mcp_agent.* (proves: MCP integration)
- [ ] mcp_gateway_demo.* (proves: MCP gateway skill)
- [ ] info_gatherer_example.* (proves: InfoGathererAgent prefab)
- [ ] dynamic_info_gatherer_example.* (proves: dynamic InfoGatherer)
- [ ] survey_agent_example.* (proves: SurveyAgent prefab)
- [ ] faq_bot_agent.* (proves: FAQBotAgent prefab)
- [ ] receptionist_agent_example.* (proves: ReceptionistAgent prefab)
- [ ] concierge_agent_example.* (proves: ConciergeAgent prefab)
- [ ] joke_agent.* (proves: simple agent with joke skill)
- [ ] relay_answer_and_welcome.* (proves: RELAY answer+play — also in relay/examples/)

Skip: bedrock_*.py, search_*.py, pgvector_*.py, sigmond_*.py

### Commit to git

## Phase 12: Testing Verification

Tests are proof of implementation. The port must test **everything the Python SDK tests**. Read the Python test files in `tests/unit/` and ensure equivalent coverage exists in your port for every tested behavior.

- [ ] Every public method has at least one test exercising it
- [ ] Every test the Python SDK has (except search-related) has an equivalent in the port
- [ ] All tests pass with zero failures, no tests skipped
- [ ] Test coverage matches Python SDK organization:
  - [ ] Core: agent_base, swml_service, swml_builder, swml_renderer, swml_handler
  - [ ] SWAIG: swaig_function, function_result (all 40+ actions)
  - [ ] Security: session_manager, auth_handler
  - [ ] DataMap: data_map (all builder methods, serialization)
  - [ ] Contexts: contexts (steps, navigation, validation, gather_info)
  - [ ] Mixins/Config: prompt, tool, web, auth, serverless, state, ai_config, skill
  - [ ] Skills: registry, manager, each of the 18 built-in skills individually
  - [ ] Prefabs: each of the 5 prefab agents
  - [ ] AgentServer: registration, routing, SIP, static files
  - [ ] RELAY: client, call, action types, events, messages
  - [ ] REST: client, base resource, each major namespace, pagination
  - [ ] CLI: argument parsing, tool listing, execution
  - [ ] Utilities: schema_utils, logging, pom_builder, type_inference

## Phase 13: Final Audit (REQUIRED)

### Completeness Audit
- [ ] Every AgentBase public method from Python SDK has an equivalent
- [ ] All 40+ SwaigFunctionResult action methods present (including payment helpers)
- [ ] All 38 SWML verb methods present and schema-validated
- [ ] RELAY client: 4 correlation mechanisms implemented
- [ ] REST client: all 21+ namespaces initialized with correct paths
- [ ] Skills registry: all 18 built-in skills registered
- [ ] agent.AddSkill() one-liner integration works (not just manual SkillManager)
- [ ] SIP username extraction utility exists
- [ ] Static file serving in AgentServer with path traversal protection
- [ ] No TODO/FIXME/HACK/PLACEHOLDER comments remain
- [ ] Every example compiles/runs without syntax errors
- [ ] Top-level relay/ and rest/ directories have README, docs, examples

### Security Audit
Read all source code and review the full implementation for security issues. The items below are known vulnerabilities found in prior ports — check these first, then review for anything else specific to your language/framework:
- [ ] Basic auth uses timing-safe comparison (NOT `==`)
- [ ] Passwords never appear in log output
- [ ] No weak fallback passwords — fail to start if crypto/rand fails
- [ ] All POST handlers enforce request body size limits (1MB)
- [ ] SIP username extraction validates input format
- [ ] JSON parse errors are checked, not silently ignored
- [ ] All shared state protected by mutexes (global data, tool registry, RELAY maps)
- [ ] HMAC token validation uses timing-safe comparison
- [ ] Security headers set on all authenticated endpoints
- [ ] Third-party dependencies checked for known vulnerabilities
- [ ] General review: no other injection, XSS, SSRF, or language-specific vulnerabilities
