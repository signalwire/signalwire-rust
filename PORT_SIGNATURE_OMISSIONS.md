# PORT_SIGNATURE_OMISSIONS.md

Documented signature divergences between this Rust port and the Python
reference. Names-only divergences live in PORT_OMISSIONS.md /
PORT_ADDITIONS.md and are inherited automatically.

Excused divergences fall into:

1. **Idiom-level** (deliberate, not fixable without breaking Rust API style):
   - Rust constructors are ``::new`` static methods; param shapes follow
     Rust conventions, not Python kwargs.
   - Rust builder methods return ``Self`` for fluent chaining.
   - Rust has no defaults; every parameter is required.
   - Lifetime / ``&T`` / ``&mut T`` borrowing collapses to T in canonical
     form.

2. **Port maintenance backlog** (tracked here; will be reduced as the Rust
   port catches up to Python signature parity).


## Idiom: Rust ::new constructors

signalwire.agent_server.AgentServer.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.agents.bedrock.BedrockAgent.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.agent_base.AgentBase.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.contexts.Context.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.contexts.ContextBuilder.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.contexts.GatherInfo.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.contexts.GatherQuestion.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.contexts.Step.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.data_map.DataMap.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.function_result.FunctionResult.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.security.session_manager.SessionManager.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.skill_manager.SkillManager.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.core.swml_service.SWMLService.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.prefabs.concierge.ConciergeAgent.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.prefabs.faq_bot.FAQBotAgent.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.prefabs.info_gatherer.InfoGathererAgent.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.prefabs.receptionist.ReceptionistAgent.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.prefabs.survey.SurveyAgent.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.AIAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.Action.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.Call.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.CollectAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.DetectAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.FaxAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.PayAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.PlayAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.RecordAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.StreamAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.TapAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.call.TranscribeAction.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.client.RelayClient.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.relay.message.Message.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.rest.client.RestClient.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.search.DocumentProcessor.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.search.IndexBuilder.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.search.SearchEngine.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.search.SearchService.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.search.search_service.SearchRequest.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.search.search_service.SearchResponse.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.search.search_service.SearchResult.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.skills.spider.skill.SpiderSkill.__init__: Rust constructor (::new) signature follows Rust conventions
signalwire.skills.weather_api.skill.WeatherApiSkill.__init__: Rust constructor (::new) signature follows Rust conventions

## Idiom: Rust builder fluent pattern

signalwire.agent_server.AgentServer.get_agents: Rust fluent / builder pattern returns Self for chaining
signalwire.agent_server.AgentServer.register_sip_username: Rust fluent / builder pattern returns Self for chaining
signalwire.agent_server.AgentServer.unregister: Rust fluent / builder pattern returns Self for chaining
signalwire.core.agent_base.AgentBase.add_swaig_query_params: Rust fluent / builder pattern returns Self for chaining
signalwire.core.agent_base.AgentBase.clear_post_ai_verbs: Rust fluent / builder pattern returns Self for chaining
signalwire.core.agent_base.AgentBase.clear_post_answer_verbs: Rust fluent / builder pattern returns Self for chaining
signalwire.core.agent_base.AgentBase.clear_pre_answer_verbs: Rust fluent / builder pattern returns Self for chaining
signalwire.core.agent_base.AgentBase.clear_swaig_query_params: Rust fluent / builder pattern returns Self for chaining
signalwire.core.agent_base.AgentBase.set_post_prompt_url: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Context.move_step: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Context.remove_step: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Context.set_initial_step: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Context.set_system_prompt: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.ContextBuilder.reset: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Step.add_section: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Step.set_end: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Step.set_skip_user_turn: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Step.set_step_criteria: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Step.set_text: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Step.set_valid_contexts: Rust fluent / builder pattern returns Self for chaining
signalwire.core.contexts.Step.set_valid_steps: Rust fluent / builder pattern returns Self for chaining
signalwire.core.data_map.DataMap.error_keys: Rust fluent / builder pattern returns Self for chaining
signalwire.core.data_map.DataMap.global_error_keys: Rust fluent / builder pattern returns Self for chaining
signalwire.core.data_map.DataMap.to_swaig_function: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.clear_dynamic_hints: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.hangup: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.join_room: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.rpc_ai_unhold: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.say: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.simulate_user_input: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.sip_refer: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.stop: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.stop_background_file: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.swml_change_context: Rust fluent / builder pattern returns Self for chaining
signalwire.core.function_result.FunctionResult.swml_change_step: Rust fluent / builder pattern returns Self for chaining
signalwire.relay.call.Call.denoise: Rust fluent / builder pattern returns Self for chaining
signalwire.relay.call.Call.denoise_stop: Rust fluent / builder pattern returns Self for chaining
signalwire.relay.call.Call.disconnect: Rust fluent / builder pattern returns Self for chaining
signalwire.relay.call.Call.hold: Rust fluent / builder pattern returns Self for chaining
signalwire.relay.call.Call.unhold: Rust fluent / builder pattern returns Self for chaining

## Backlog: real signature divergences (356 symbols)

signalwire.RestClient: BACKLOG / missing-port/ in reference, not in port
signalwire.add_skill_directory: BACKLOG / missing-port/ in reference, not in port
signalwire.agent_server.AgentServer.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.agent_server.AgentServer.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.agent_server.AgentServer.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.agent_server.AgentServer.register: BACKLOG / param-mismatch/ param[2] (route)/ required False vs True; default None vs '<absent>'
signalwire.agent_server.AgentServer.register_global_routing_callback: BACKLOG / param-mismatch/ param[1] (callback_fn)/ name 'callback_fn' vs 'callback'; type 'callable<list<cl; return-mismatch/ retur
signalwire.agent_server.AgentServer.run: BACKLOG / param-count-mismatch/ reference has 5 param(s), port has 3/ reference=['self', 'event', 'context', 'ho; return-mismatch/
signalwire.agent_server.AgentServer.serve_static_files: BACKLOG / param-mismatch/ param[2] (route)/ required False vs True; default '/' vs '<absent>'
signalwire.agent_server.AgentServer.setup_sip_routing: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 1/ reference=['self', 'route', 'auto_map'] po; return-mismatch/
signalwire.agent_server.AgentServer.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.agent_server.AgentServer.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.agent_server.AgentServer.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.deref: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.deref_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.repr: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.set_inference_params: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.set_llm_model: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.set_llm_temperature: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.set_post_prompt_llm_params: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.set_prompt_llm_params: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.set_voice: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.agents.bedrock.BedrockAgent.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.add_post_ai_verb: BACKLOG / param-mismatch/ param[1] (verb_name)/ name 'verb_name' vs 'verb'; param-mismatch/ param[2] (config)/ type 'dict<string,a
signalwire.core.agent_base.AgentBase.add_post_answer_verb: BACKLOG / param-mismatch/ param[1] (verb_name)/ name 'verb_name' vs 'verb'; param-mismatch/ param[2] (config)/ type 'dict<string,a
signalwire.core.agent_base.AgentBase.add_pre_answer_verb: BACKLOG / param-mismatch/ param[1] (verb_name)/ name 'verb_name' vs 'verb'; param-mismatch/ param[2] (config)/ type 'dict<string,a
signalwire.core.agent_base.AgentBase.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.deref: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.deref_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.enable_sip_routing: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 1/ reference=['self', 'auto_map', 'path'] por; return-mismatch/
signalwire.core.agent_base.AgentBase.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.on_debug_event: BACKLOG / param-mismatch/ param[1] (handler)/ name 'handler' vs 'callback'; type 'class/Callable' vs 'any'; return-mismatch/ retur
signalwire.core.agent_base.AgentBase.on_summary: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 2/ reference=['self', 'summary', 'raw_data'] ; return-mismatch/
signalwire.core.agent_base.AgentBase.register_sip_username: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 3/ reference=['self', 'sip_username'] port=['; return-mismatch/
signalwire.core.agent_base.AgentBase.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.agent_base.AgentBase.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.add_step: BACKLOG / param-count-mismatch/ reference has 7 param(s), port has 2/ reference=['self', 'name', 'task', 'bullet
signalwire.core.contexts.Context.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.set_enter_fillers: BACKLOG / param-mismatch/ param[1] (enter_fillers)/ name 'enter_fillers' vs 'fillers'; type 'dict<string,l; return-mismatch/ retur
signalwire.core.contexts.Context.set_exit_fillers: BACKLOG / param-mismatch/ param[1] (exit_fillers)/ name 'exit_fillers' vs 'fillers'; type 'dict<string,lis; return-mismatch/ retur
signalwire.core.contexts.Context.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Context.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.ContextBuilder.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.add_question: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 7/ reference=['self', 'key', 'question', 'kwa; return-mismatch/
signalwire.core.contexts.GatherInfo.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherInfo.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.GatherQuestion.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.add_gather_question: BACKLOG / param-mismatch/ param[3] (type)/ name 'type' vs 'question_type'; required False vs True; default; param-mismatch/ param[
signalwire.core.contexts.Step.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.set_functions: BACKLOG / param-mismatch/ param[1] (functions)/ type 'union<list<string>,string>' vs 'class/signalwire.val; return-mismatch/ retur
signalwire.core.contexts.Step.set_gather_info: BACKLOG / param-mismatch/ param[1] (output_key)/ required False vs True; default None vs '<absent>'; param-mismatch/ param[2] (com
signalwire.core.contexts.Step.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.contexts.Step.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.body: BACKLOG / param-mismatch/ param[1] (data)/ type 'dict<string,any>' vs 'class/signalwire.value.Value'; return-mismatch/ returns 'cl
signalwire.core.data_map.DataMap.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.description: BACKLOG / param-mismatch/ param[1] (description)/ name 'description' vs 'desc'; return-mismatch/ returns 'class/signalwire.core.da
signalwire.core.data_map.DataMap.expression: BACKLOG / param-mismatch/ param[2] (pattern)/ type 'union<class/Pattern,string>' vs 'string'; param-mismatch/ param[3] (output)/ t
signalwire.core.data_map.DataMap.fallback_output: BACKLOG / param-mismatch/ param[1] (result)/ type 'class/signalwire.core.function_result.FunctionResult' v; return-mismatch/ retur
signalwire.core.data_map.DataMap.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.output: BACKLOG / param-mismatch/ param[1] (result)/ type 'class/signalwire.core.function_result.FunctionResult' v; return-mismatch/ retur
signalwire.core.data_map.DataMap.parameter: BACKLOG / param-mismatch/ param[4] (required)/ required False vs True; default False vs '<absent>'; param-mismatch/ param[5] (enum
signalwire.core.data_map.DataMap.params: BACKLOG / param-mismatch/ param[1] (data)/ type 'dict<string,any>' vs 'class/signalwire.value.Value'; return-mismatch/ returns 'cl
signalwire.core.data_map.DataMap.purpose: BACKLOG / param-mismatch/ param[1] (description)/ name 'description' vs 'desc'; return-mismatch/ returns 'class/signalwire.core.da
signalwire.core.data_map.DataMap.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.data_map.DataMap.webhook: BACKLOG / param-mismatch/ param[3] (headers)/ type 'optional<dict<string,string>>' vs 'class/signalwire.va; param-mismatch/ param[
signalwire.core.data_map.DataMap.webhook_expressions: BACKLOG / param-mismatch/ param[1] (expressions)/ type 'list<dict<string,any>>' vs 'list<class/signalwire.; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.add_action: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 2/ reference=['self', 'name', 'data'] port=['; return-mismatch/
signalwire.core.function_result.FunctionResult.add_actions: BACKLOG / param-mismatch/ param[1] (actions)/ type 'list<dict<string,any>>' vs 'list<class/signalwire.valu; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.add_dynamic_hints: BACKLOG / param-mismatch/ param[1] (hints)/ type 'list<union<dict<string,any>,string>>' vs 'list<class/sig; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.connect: BACKLOG / param-mismatch/ param[2] (final)/ name 'final' vs '_final'; required False vs True; default True; param-mismatch/ param[
signalwire.core.function_result.FunctionResult.create_payment_action: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 4/ reference=['action_type', 'phrase'] port=[; return-mismatch/
signalwire.core.function_result.FunctionResult.create_payment_parameter: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 3/ reference=['name', 'value'] port=['name', ; return-mismatch/
signalwire.core.function_result.FunctionResult.create_payment_prompt: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 3/ reference=['for_situation', 'actions', 'ca; return-mismatch/
signalwire.core.function_result.FunctionResult.enable_extensive_data: BACKLOG / param-mismatch/ param[1] (enabled)/ required False vs True; default True vs '<absent>'; return-mismatch/ returns 'class/
signalwire.core.function_result.FunctionResult.enable_functions_on_timeout: BACKLOG / param-mismatch/ param[1] (enabled)/ required False vs True; default True vs '<absent>'; return-mismatch/ returns 'class/
signalwire.core.function_result.FunctionResult.execute_rpc: BACKLOG / param-count-mismatch/ reference has 5 param(s), port has 3/ reference=['self', 'method', 'params', 'ca; return-mismatch/
signalwire.core.function_result.FunctionResult.execute_swml: BACKLOG / param-mismatch/ param[1] (swml_content)/ type 'any' vs 'class/signalwire.value.Value'; param-mismatch/ param[2] (transfe
signalwire.core.function_result.FunctionResult.hold: BACKLOG / param-mismatch/ param[1] (timeout)/ required False vs True; default 300 vs '<absent>'; return-mismatch/ returns 'class/s
signalwire.core.function_result.FunctionResult.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.join_conference: BACKLOG / param-count-mismatch/ reference has 19 param(s), port has 5/ reference=['self', 'name', 'muted', 'beep; return-mismatch/
signalwire.core.function_result.FunctionResult.pay: BACKLOG / param-count-mismatch/ reference has 20 param(s), port has 6/ reference=['self', 'payment_connector_url; return-mismatch/
signalwire.core.function_result.FunctionResult.play_background_file: BACKLOG / param-mismatch/ param[2] (wait)/ required False vs True; default False vs '<absent>'; return-mismatch/ returns 'class/si
signalwire.core.function_result.FunctionResult.record_call: BACKLOG / param-count-mismatch/ reference has 12 param(s), port has 5/ reference=['self', 'control_id', 'stereo'; return-mismatch/
signalwire.core.function_result.FunctionResult.remove_global_data: BACKLOG / param-mismatch/ param[1] (keys)/ type 'union<list<string>,string>' vs 'list<string>'; return-mismatch/ returns 'class/si
signalwire.core.function_result.FunctionResult.remove_metadata: BACKLOG / param-mismatch/ param[1] (keys)/ type 'union<list<string>,string>' vs 'list<string>'; return-mismatch/ returns 'class/si
signalwire.core.function_result.FunctionResult.replace_in_history: BACKLOG / param-mismatch/ param[1] (text)/ type 'union<bool,string>' vs 'optional<string>'; required False; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.rpc_ai_message: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 3/ reference=['self', 'call_id', 'message_tex; return-mismatch/
signalwire.core.function_result.FunctionResult.rpc_dial: BACKLOG / param-count-mismatch/ reference has 5 param(s), port has 6/ reference=['self', 'to_number', 'from_numb; return-mismatch/
signalwire.core.function_result.FunctionResult.send_sms: BACKLOG / param-count-mismatch/ reference has 7 param(s), port has 6/ reference=['self', 'to_number', 'from_numb; return-mismatch/
signalwire.core.function_result.FunctionResult.set_end_of_speech_timeout: BACKLOG / param-mismatch/ param[1] (milliseconds)/ name 'milliseconds' vs 'ms'; return-mismatch/ returns 'class/signalwire.core.fu
signalwire.core.function_result.FunctionResult.set_metadata: BACKLOG / param-mismatch/ param[1] (data)/ type 'dict<string,any>' vs 'class/signalwire.value.Value'; return-mismatch/ returns 'cl
signalwire.core.function_result.FunctionResult.set_post_process: BACKLOG / param-mismatch/ param[1] (post_process)/ name 'post_process' vs 'val'; return-mismatch/ returns 'class/signalwire.core.f
signalwire.core.function_result.FunctionResult.set_response: BACKLOG / param-mismatch/ param[1] (response)/ name 'response' vs 'text'; return-mismatch/ returns 'class/signalwire.core.function
signalwire.core.function_result.FunctionResult.set_speech_event_timeout: BACKLOG / param-mismatch/ param[1] (milliseconds)/ name 'milliseconds' vs 'ms'; return-mismatch/ returns 'class/signalwire.core.fu
signalwire.core.function_result.FunctionResult.stop_record_call: BACKLOG / param-mismatch/ param[1] (control_id)/ type 'optional<string>' vs 'string'; required False vs Tr; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.stop_tap: BACKLOG / param-mismatch/ param[1] (control_id)/ type 'optional<string>' vs 'string'; required False vs Tr; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.switch_context: BACKLOG / param-count-mismatch/ reference has 5 param(s), port has 6/ reference=['self', 'system_prompt', 'user_; return-mismatch/
signalwire.core.function_result.FunctionResult.swml_transfer: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 3/ reference=['self', 'dest', 'ai_response', ; return-mismatch/
signalwire.core.function_result.FunctionResult.swml_user_event: BACKLOG / param-mismatch/ param[1] (event_data)/ type 'dict<string,any>' vs 'class/signalwire.value.Value'; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.tap: BACKLOG / param-count-mismatch/ reference has 7 param(s), port has 5/ reference=['self', 'uri', 'control_id', 'd; return-mismatch/
signalwire.core.function_result.FunctionResult.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.toggle_functions: BACKLOG / param-mismatch/ param[1] (function_toggles)/ name 'function_toggles' vs 'toggles'; type 'list<di; return-mismatch/ retur
signalwire.core.function_result.FunctionResult.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.update_global_data: BACKLOG / param-mismatch/ param[1] (data)/ type 'dict<string,any>' vs 'class/signalwire.value.Value'; return-mismatch/ returns 'cl
signalwire.core.function_result.FunctionResult.update_settings: BACKLOG / param-mismatch/ param[1] (settings)/ type 'dict<string,any>' vs 'class/signalwire.value.Value'; return-mismatch/ returns
signalwire.core.function_result.FunctionResult.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.function_result.FunctionResult.wait_for_user: BACKLOG / param-mismatch/ param[1] (enabled)/ required False vs True; default None vs '<absent>'; param-mismatch/ param[2] (timeou
signalwire.core.security.session_manager.SessionManager.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.clone_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.clone_to_uninit: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.create_session: BACKLOG / param-mismatch/ param[1] (call_id)/ required False vs True; default None vs '<absent>'
signalwire.core.security.session_manager.SessionManager.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.to_owned: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.security.session_manager.SessionManager.validate_token: BACKLOG / param-mismatch/ param[1] (call_id)/ name 'call_id' vs 'function_name'; param-mismatch/ param[2] (function_name)/ name 'f
signalwire.core.security.session_manager.SessionManager.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.skill_manager.SkillManager.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.skill_manager.SkillManager.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.skill_manager.SkillManager.get_skill: BACKLOG / param-mismatch/ param[1] (skill_identifier)/ name 'skill_identifier' vs 'key'; return-mismatch/ returns 'optional<class/
signalwire.core.skill_manager.SkillManager.has_skill: BACKLOG / param-mismatch/ param[1] (skill_identifier)/ name 'skill_identifier' vs 'key'
signalwire.core.skill_manager.SkillManager.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.skill_manager.SkillManager.load_skill: BACKLOG / param-mismatch/ param[2] (skill_class)/ name 'skill_class' vs 'params'; type 'class/signalwire.c; param-mismatch/ param[
signalwire.core.skill_manager.SkillManager.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.skill_manager.SkillManager.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.skill_manager.SkillManager.unload_skill: BACKLOG / param-mismatch/ param[1] (skill_identifier)/ name 'skill_identifier' vs 'key'
signalwire.core.skill_manager.SkillManager.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.core.swml_service.SWMLService.add_verb: BACKLOG / missing-port/ in reference, not in port
signalwire.core.swml_service.SWMLService.extract_sip_username: BACKLOG / missing-port/ in reference, not in port
signalwire.list_skills: BACKLOG / missing-port/ in reference, not in port
signalwire.list_skills_with_params: BACKLOG / missing-port/ in reference, not in port
signalwire.prefabs.concierge.ConciergeAgent.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.concierge.ConciergeAgent.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.concierge.ConciergeAgent.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.concierge.ConciergeAgent.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.concierge.ConciergeAgent.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.concierge.ConciergeAgent.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.faq_bot.FAQBotAgent.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.faq_bot.FAQBotAgent.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.faq_bot.FAQBotAgent.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.faq_bot.FAQBotAgent.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.faq_bot.FAQBotAgent.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.faq_bot.FAQBotAgent.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.info_gatherer.InfoGathererAgent.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.info_gatherer.InfoGathererAgent.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.info_gatherer.InfoGathererAgent.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.info_gatherer.InfoGathererAgent.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.info_gatherer.InfoGathererAgent.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.info_gatherer.InfoGathererAgent.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.receptionist.ReceptionistAgent.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.receptionist.ReceptionistAgent.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.receptionist.ReceptionistAgent.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.receptionist.ReceptionistAgent.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.receptionist.ReceptionistAgent.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.receptionist.ReceptionistAgent.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.survey.SurveyAgent.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.survey.SurveyAgent.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.survey.SurveyAgent.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.survey.SurveyAgent.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.survey.SurveyAgent.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.prefabs.survey.SurveyAgent.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.register_skill: BACKLOG / missing-port/ in reference, not in port
signalwire.relay.call.Action.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Action.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Action.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Action.is_done: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Action.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Action.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Action.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Call.ai: BACKLOG / param-count-mismatch/ reference has 16 param(s), port has 2/ reference=['self', 'control_id', 'agent',; return-mismatch/
signalwire.relay.call.Call.ai_hold: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 1/ reference=['self', 'timeout', 'prompt', 'k; return-mismatch/
signalwire.relay.call.Call.ai_message: BACKLOG / param-count-mismatch/ reference has 6 param(s), port has 2/ reference=['self', 'message_text', 'role',; return-mismatch/
signalwire.relay.call.Call.ai_unhold: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 1/ reference=['self', 'prompt', 'kwargs'] por; return-mismatch/
signalwire.relay.call.Call.amazon_bedrock: BACKLOG / param-count-mismatch/ reference has 8 param(s), port has 2/ reference=['self', 'prompt', 'SWAIG', 'ai_; return-mismatch/
signalwire.relay.call.Call.answer: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 1/ reference=['self', 'kwargs'] port=['self']; return-mismatch/
signalwire.relay.call.Call.bind_digit: BACKLOG / param-count-mismatch/ reference has 7 param(s), port has 2/ reference=['self', 'digits', 'bind_method'; return-mismatch/
signalwire.relay.call.Call.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Call.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Call.clear_digit_bindings: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 1/ reference=['self', 'realm', 'kwargs'] port; return-mismatch/
signalwire.relay.call.Call.collect: BACKLOG / param-count-mismatch/ reference has 11 param(s), port has 2/ reference=['self', 'digits', 'speech', 'i; return-mismatch/
signalwire.relay.call.Call.connect: BACKLOG / param-count-mismatch/ reference has 8 param(s), port has 2/ reference=['self', 'devices', 'ringback', ; return-mismatch/
signalwire.relay.call.Call.detect: BACKLOG / param-count-mismatch/ reference has 6 param(s), port has 2/ reference=['self', 'detect', 'timeout', 'c; return-mismatch/
signalwire.relay.call.Call.hangup: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 1/ reference=['self', 'reason'] port=['self']; return-mismatch/
signalwire.relay.call.Call.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Call.join_conference: BACKLOG / param-count-mismatch/ reference has 22 param(s), port has 2/ reference=['self', 'name', 'muted', 'beep; return-mismatch/
signalwire.relay.call.Call.join_room: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 2/ reference=['self', 'name', 'status_url', '; return-mismatch/
signalwire.relay.call.Call.leave_conference: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 1/ reference=['self', 'conference_id', 'kwarg; return-mismatch/
signalwire.relay.call.Call.leave_room: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 1/ reference=['self', 'kwargs'] port=['self']; return-mismatch/
signalwire.relay.call.Call.live_transcribe: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 2/ reference=['self', 'action', 'kwargs'] por; return-mismatch/
signalwire.relay.call.Call.live_translate: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 2/ reference=['self', 'action', 'status_url',; return-mismatch/
signalwire.relay.call.Call.on: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 2/ reference=['self', 'event_type', 'handler'
signalwire.relay.call.Call.pay: BACKLOG / param-count-mismatch/ reference has 22 param(s), port has 2/ reference=['self', 'payment_connector_url; return-mismatch/
signalwire.relay.call.Call.play: BACKLOG / param-count-mismatch/ reference has 8 param(s), port has 2/ reference=['self', 'media', 'volume', 'dir; return-mismatch/
signalwire.relay.call.Call.play_and_collect: BACKLOG / param-count-mismatch/ reference has 7 param(s), port has 2/ reference=['self', 'media', 'collect', 'vo; return-mismatch/
signalwire.relay.call.Call.queue_enter: BACKLOG / param-count-mismatch/ reference has 5 param(s), port has 2/ reference=['self', 'queue_name', 'control_; return-mismatch/
signalwire.relay.call.Call.queue_leave: BACKLOG / param-count-mismatch/ reference has 6 param(s), port has 1/ reference=['self', 'queue_name', 'control_; return-mismatch/
signalwire.relay.call.Call.receive_fax: BACKLOG / param-count-mismatch/ reference has 4 param(s), port has 2/ reference=['self', 'control_id', 'on_compl; return-mismatch/
signalwire.relay.call.Call.record: BACKLOG / param-count-mismatch/ reference has 5 param(s), port has 2/ reference=['self', 'audio', 'control_id', ; return-mismatch/
signalwire.relay.call.Call.repr: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Call.send_digits: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 2/ reference=['self', 'digits', 'control_id']; return-mismatch/
signalwire.relay.call.Call.send_fax: BACKLOG / param-count-mismatch/ reference has 7 param(s), port has 2/ reference=['self', 'document', 'identity',; return-mismatch/
signalwire.relay.call.Call.stream: BACKLOG / param-count-mismatch/ reference has 12 param(s), port has 2/ reference=['self', 'url', 'name', 'codec'; return-mismatch/
signalwire.relay.call.Call.tap: BACKLOG / param-count-mismatch/ reference has 6 param(s), port has 2/ reference=['self', 'tap', 'device', 'contr; return-mismatch/
signalwire.relay.call.Call.transcribe: BACKLOG / param-count-mismatch/ reference has 5 param(s), port has 2/ reference=['self', 'control_id', 'status_u; return-mismatch/
signalwire.relay.call.Call.transfer: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 2/ reference=['self', 'dest', 'kwargs'] port=; return-mismatch/
signalwire.relay.call.Call.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Call.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.Call.user_event: BACKLOG / param-count-mismatch/ reference has 3 param(s), port has 2/ reference=['self', 'event', 'kwargs'] port; return-mismatch/
signalwire.relay.call.Call.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.CollectAction.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.CollectAction.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.CollectAction.deref: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.CollectAction.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.CollectAction.start_input_timers: BACKLOG / return-mismatch/ returns 'dict<any,any>' vs 'void'
signalwire.relay.call.CollectAction.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.CollectAction.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.CollectAction.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.action: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.deref: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.DetectAction.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.FaxAction.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.FaxAction.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.FaxAction.deref: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.FaxAction.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.FaxAction.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.FaxAction.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.FaxAction.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.action: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.deref: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.pause: BACKLOG / return-mismatch/ returns 'dict<any,any>' vs 'void'
signalwire.relay.call.PlayAction.resume: BACKLOG / return-mismatch/ returns 'dict<any,any>' vs 'void'
signalwire.relay.call.PlayAction.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.PlayAction.volume: BACKLOG / param-mismatch/ param[1] (volume)/ name 'volume' vs 'db'; return-mismatch/ returns 'dict<any,any>' vs 'void'
signalwire.relay.call.PlayAction.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.action: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.deref: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.pause: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 1/ reference=['self', 'behavior'] port=['self; return-mismatch/
signalwire.relay.call.RecordAction.resume: BACKLOG / return-mismatch/ returns 'dict<any,any>' vs 'void'
signalwire.relay.call.RecordAction.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.call.RecordAction.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.client.RelayClient.connect: BACKLOG / missing-port/ in reference, not in port
signalwire.relay.client.RelayClient.disconnect: BACKLOG / missing-port/ in reference, not in port
signalwire.relay.client.RelayClient.on_call: BACKLOG / missing-port/ in reference, not in port
signalwire.relay.client.RelayClient.on_message: BACKLOG / missing-port/ in reference, not in port
signalwire.relay.client.RelayClient.receive: BACKLOG / missing-port/ in reference, not in port
signalwire.relay.client.RelayClient.unreceive: BACKLOG / missing-port/ in reference, not in port
signalwire.relay.message.Message.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.is_done: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.on: BACKLOG / param-mismatch/ param[1] (handler)/ name 'handler' vs 'cb'; type 'class/Callable' vs 'any'
signalwire.relay.message.Message.repr: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.result: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.relay.message.Message.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.create: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.delete: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.get: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.list: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.update: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest._base.CrudResource.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest.client.RestClient.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest.client.RestClient.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest.client.RestClient.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest.client.RestClient.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest.client.RestClient.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.rest.client.RestClient.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.run_agent: BACKLOG / missing-port/ in reference, not in port
signalwire.search.preprocess_document_content: BACKLOG / missing-port/ in reference, not in port
signalwire.search.preprocess_query: BACKLOG / missing-port/ in reference, not in port
signalwire.skills.registry.SkillRegistry.add_skill_directory: BACKLOG / param-count-mismatch/ reference has 2 param(s), port has 1/ reference=['self', 'path'] port=['path']
signalwire.skills.registry.SkillRegistry.borrow: BACKLOG / missing-reference/ in port, not in reference
signalwire.skills.registry.SkillRegistry.borrow_mut: BACKLOG / missing-reference/ in port, not in reference
signalwire.skills.registry.SkillRegistry.into: BACKLOG / missing-reference/ in port, not in reference
signalwire.skills.registry.SkillRegistry.list_skills: BACKLOG / param-count-mismatch/ reference has 1 param(s), port has 0/ reference=['self'] port=[]; return-mismatch/ returns 'list<d
signalwire.skills.registry.SkillRegistry.register_skill: BACKLOG / param-mismatch/ param[0] (self)/ name 'self' vs 'name'; kind 'self' vs 'positional'; param-mismatch/ param[1] (skill_cla
signalwire.skills.registry.SkillRegistry.try_into: BACKLOG / missing-reference/ in port, not in reference
signalwire.skills.registry.SkillRegistry.type_id: BACKLOG / missing-reference/ in port, not in reference
signalwire.skills.registry.SkillRegistry.vzip: BACKLOG / missing-reference/ in port, not in reference
signalwire.start_agent: BACKLOG / missing-port/ in reference, not in port
