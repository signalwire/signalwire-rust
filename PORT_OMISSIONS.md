# PORT_OMISSIONS.md (signalwire-rust)

Python symbols deliberately not implemented in this Rust port. Format:

```
<fully.qualified.symbol>: <one-sentence rationale>
```

`scripts/diff_port_surface.py` reads this file to know which Python
symbols to ignore when checking parity. Anything not in this file AND
not implemented in the port fails the diff.

---

## Skip-list categories

These broad categories are Python-only per the SignalWire SDK skip
rules (search subsystem, sigmond infrastructure, scaffolding helpers
that don't make sense in a compiled language):

- **`signalwire.search.*`** — Vector / embedding indexing (Python ML
  stack: sentence-transformers, pgvector, faiss). Per the porting-sdk
  skip list, search is Python-only.
- **`signalwire.cli.build_search`**, **`signalwire.cli.dokku`** —
  Search-tool CLI and Dokku project generator. Both are interactive
  scaffolding helpers tied to Python's runtime.
- **`signalwire.cli.init_project`** — Interactive project generator.
  Rust users use `cargo new`; the SDK does not ship an equivalent.
- **`signalwire.cli.simulation.*`**, **`signalwire.cli.execution.*`**,
  **`signalwire.cli.output.*`**, **`signalwire.cli.types`**,
  **`signalwire.cli.core.*`**, **`signalwire.cli.test_swaig`**,
  **`signalwire.cli.swaig_test_wrapper`** — Python-CLI internal
  helpers that don't translate to a compiled-binary CLI. The Rust
  `swaig-test` binary covers the same user-facing CLI surface.
- **`signalwire.livewire.*`** — Internal LiveWire integration not
  exposed via the cross-port skip-list rules.
- **`signalwire.mcp_gateway.*`** — Standalone MCP gateway server
  implementation. Rust ships the MCP **skill** (`mcp_gateway` skill),
  but not the standalone server (Python users run it directly via
  `python -m signalwire.mcp_gateway`).
- **`signalwire.skills.datasphere_serverless`**,
  **`signalwire.skills.web_search.skill_original`**,
  **`signalwire.skills.web_search.skill_improved`**,
  **`signalwire.skills.native_vector_search.*`** — Search-related and
  Python-experimental skill variants. The Rust port ships the
  canonical `web_search`, `wikipedia_search`, `datasphere`, and
  `spider` skills; the experimental / serverless / vector-search
  variants are Python-only.
- **`signalwire.pom.*`** — Internal Prompt Object Model helper
  classes. The Rust port does the equivalent work via JSON values on
  AgentBase (`prompt_add_section`, `prompt_add_subsection`,
  `prompt_add_to_section`); the symbol-level map differs.

(Per-symbol entries below — one line per Python symbol.)

---

## Search subsystem (skip)

signalwire.search.engine.SearchEngine: search-related; not ported per skip list
signalwire.search.engine.SearchEngine.__init__: search-related; not ported per skip list
signalwire.search.engine.SearchEngine.add_document: search-related; not ported per skip list
signalwire.search.engine.SearchEngine.build: search-related; not ported per skip list
signalwire.search.engine.SearchEngine.list_indexed_files: search-related; not ported per skip list
signalwire.search.engine.SearchEngine.query: search-related; not ported per skip list
signalwire.search.engine.SearchEngine.save: search-related; not ported per skip list
signalwire.search.indexer.DocumentIndexer: search-related; not ported per skip list
signalwire.search.indexer.DocumentIndexer.__init__: search-related; not ported per skip list
signalwire.search.indexer.DocumentIndexer.detect_file_format: search-related; not ported per skip list
signalwire.search.indexer.DocumentIndexer.find_supported_files: search-related; not ported per skip list
signalwire.search.indexer.DocumentIndexer.index_directory: search-related; not ported per skip list
signalwire.search.indexer.DocumentIndexer.process_file: search-related; not ported per skip list
signalwire.search.migration.PgvectorMigration: search-related; not ported per skip list
signalwire.search.migration.PgvectorMigration.__init__: search-related; not ported per skip list
signalwire.search.migration.PgvectorMigration.migrate_index: search-related; not ported per skip list
signalwire.search.migration.PgvectorMigration.migrate_indices: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.__init__: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.add_chunks: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.connect: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.disconnect: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.get_stats: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.initialize_database: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.search: search-related; not ported per skip list
signalwire.search.query_processor.detect_language: search-related; not ported per skip list
signalwire.search.query_processor.expand_query_with_synonyms: search-related; not ported per skip list
signalwire.search.query_processor.extract_keywords: search-related; not ported per skip list
signalwire.search.query_processor.get_pos_tag: search-related; not ported per skip list
signalwire.search.query_processor.get_synonyms: search-related; not ported per skip list
signalwire.search.query_processor.preprocess_document_content: search-related; not ported per skip list
signalwire.search.query_processor.preprocess_query: search-related; not ported per skip list
signalwire.search.query_processor.preprocess_text: search-related; not ported per skip list
signalwire.search.query_processor.tokenize_with_pos: search-related; not ported per skip list
signalwire.search.search_service.SearchService: search-related; not ported per skip list
signalwire.search.search_service.SearchService.__init__: search-related; not ported per skip list
signalwire.search.search_service.SearchService.get_search_engine: search-related; not ported per skip list
signalwire.search.search_service.SearchService.health_check: search-related; not ported per skip list
signalwire.search.search_service.SearchService.list_indices: search-related; not ported per skip list
signalwire.search.search_service.SearchService.preload_index: search-related; not ported per skip list
signalwire.search.search_service.SearchService.search: search-related; not ported per skip list
signalwire.search.search_service.SearchService.search_remote: search-related; not ported per skip list
signalwire.search.search_service.start_remote_server: search-related; not ported per skip list
signalwire.search.startup_validation.SearchStartupValidator: search-related; not ported per skip list
signalwire.search.startup_validation.SearchStartupValidator.__init__: search-related; not ported per skip list
signalwire.search.startup_validation.SearchStartupValidator.validate_at_startup: search-related; not ported per skip list
signalwire.search.startup_validation.validate_search_startup: search-related; not ported per skip list

## CLI scaffolding helpers (skip)

signalwire.cli.build_search.console_entry_point: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.build_search.main: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.build_search.migrate_command: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.build_search.remote_command: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.build_search.search_command: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.build_search.validate_command: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.Colors: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.DokkuProjectGenerator: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.DokkuProjectGenerator.__init__: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.DokkuProjectGenerator.generate: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.cmd_config: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.cmd_deploy: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.cmd_init: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.cmd_logs: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.cmd_scale: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.generate_password: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.main: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.print_error: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.print_header: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.print_step: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.print_success: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.print_warning: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.prompt: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.dokku.prompt_yes_no: Python-CLI scaffolding; no Rust equivalent
signalwire.cli.init_project.Colors: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.ProjectGenerator: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.ProjectGenerator.__init__: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.ProjectGenerator.generate: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.generate_password: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.get_agent_template: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.get_app_template: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.get_env_credentials: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.get_readme_template: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.get_test_template: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.get_web_index_template: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.main: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.mask_token: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.print_error: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.print_step: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.print_success: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.print_warning: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.prompt: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.prompt_multiselect: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.prompt_select: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.prompt_yes_no: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.run_interactive: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.init_project.run_quick: Python-CLI scaffolding; cargo new is the Rust equivalent
signalwire.cli.swaig_test_wrapper.main: Python-CLI scaffolding; Rust ships swaig-test as a binary
signalwire.cli.test_swaig.console_entry_point: Python-CLI scaffolding; Rust ships swaig-test as a binary
signalwire.cli.test_swaig.main: Python-CLI scaffolding; Rust ships swaig-test as a binary
signalwire.cli.test_swaig.print_help_examples: Python-CLI scaffolding; Rust ships swaig-test as a binary
signalwire.cli.test_swaig.print_help_platforms: Python-CLI scaffolding; Rust ships swaig-test as a binary
signalwire.cli.types.AgentInfo: Python-CLI scaffolding; Rust CLI types are language-private
signalwire.cli.types.CallData: Python-CLI scaffolding; Rust CLI types are language-private
signalwire.cli.types.DataMapConfig: Python-CLI scaffolding; Rust CLI types are language-private
signalwire.cli.types.FunctionInfo: Python-CLI scaffolding; Rust CLI types are language-private
signalwire.cli.types.PostData: Python-CLI scaffolding; Rust CLI types are language-private
signalwire.cli.types.VarsData: Python-CLI scaffolding; Rust CLI types are language-private
signalwire.cli.core.agent_loader.discover_agents_in_file: Python-CLI scaffolding; Rust CLI is binary-based
signalwire.cli.core.agent_loader.discover_services_in_file: Python-CLI scaffolding; Rust CLI is binary-based
signalwire.cli.core.agent_loader.load_agent_from_file: Python-CLI scaffolding; Rust CLI is binary-based
signalwire.cli.core.agent_loader.load_service_from_file: Python-CLI scaffolding; Rust CLI is binary-based
signalwire.cli.core.argparse_helpers.CustomArgumentParser: Python argparse subclass; Rust uses clap-style parsing
signalwire.cli.core.argparse_helpers.CustomArgumentParser.__init__: Python argparse subclass; Rust uses clap-style parsing
signalwire.cli.core.argparse_helpers.CustomArgumentParser.error: Python argparse subclass; Rust uses clap-style parsing
signalwire.cli.core.argparse_helpers.CustomArgumentParser.parse_args: Python argparse subclass; Rust uses clap-style parsing
signalwire.cli.core.argparse_helpers.CustomArgumentParser.print_usage: Python argparse subclass; Rust uses clap-style parsing
signalwire.cli.core.argparse_helpers.parse_function_arguments: Python argparse helper; Rust CLI parses args directly
signalwire.cli.core.dynamic_config.apply_dynamic_config: Python CLI helper; Rust CLI is binary-based
signalwire.cli.core.service_loader.ServiceCapture: Python CLI helper; Rust CLI is binary-based
signalwire.cli.core.service_loader.ServiceCapture.__init__: Python CLI helper; Rust CLI is binary-based
signalwire.cli.core.service_loader.ServiceCapture.capture: Python CLI helper; Rust CLI is binary-based
signalwire.cli.core.service_loader.discover_agents_in_file: Python CLI helper; Rust CLI is binary-based
signalwire.cli.core.service_loader.load_agent_from_file: Python CLI helper; Rust CLI is binary-based
signalwire.cli.core.service_loader.load_and_simulate_service: Python CLI helper; Rust CLI is binary-based
signalwire.cli.core.service_loader.simulate_request_to_service: Python CLI helper; Rust CLI is binary-based
signalwire.cli.execution.datamap_exec.execute_datamap_function: Python CLI helper; Rust CLI handles execution directly
signalwire.cli.execution.datamap_exec.simple_template_expand: Python CLI helper; Rust CLI handles execution directly
signalwire.cli.execution.webhook_exec.execute_external_webhook_function: Python CLI helper; Rust CLI handles execution directly
signalwire.cli.output.output_formatter.display_agent_tools: Python CLI output helper; Rust CLI prints directly
signalwire.cli.output.output_formatter.format_result: Python CLI output helper; Rust CLI prints directly
signalwire.cli.output.swml_dump.handle_dump_swml: Python CLI output helper; Rust CLI prints directly
signalwire.cli.output.swml_dump.setup_output_suppression: Python CLI output helper; Rust CLI prints directly
signalwire.cli.simulation.data_generation.adapt_for_call_type: Python simulation helper; not ported
signalwire.cli.simulation.data_generation.generate_comprehensive_post_data: Python simulation helper; not ported
signalwire.cli.simulation.data_generation.generate_fake_node_id: Python simulation helper; not ported
signalwire.cli.simulation.data_generation.generate_fake_sip_from: Python simulation helper; not ported
signalwire.cli.simulation.data_generation.generate_fake_sip_to: Python simulation helper; not ported
signalwire.cli.simulation.data_generation.generate_fake_swml_post_data: Python simulation helper; not ported
signalwire.cli.simulation.data_generation.generate_fake_uuid: Python simulation helper; not ported
signalwire.cli.simulation.data_generation.generate_minimal_post_data: Python simulation helper; not ported
signalwire.cli.simulation.data_overrides.apply_convenience_mappings: Python simulation helper; not ported
signalwire.cli.simulation.data_overrides.apply_overrides: Python simulation helper; not ported
signalwire.cli.simulation.data_overrides.parse_value: Python simulation helper; not ported
signalwire.cli.simulation.data_overrides.set_nested_value: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders.__contains__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders.__getitem__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders.__init__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders.get: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders.items: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders.keys: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockHeaders.values: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams.__contains__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams.__getitem__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams.__init__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams.get: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams.items: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams.keys: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockQueryParams.values: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockRequest: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockRequest.__init__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockRequest.body: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockRequest.client: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockRequest.json: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockURL: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockURL.__init__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.MockURL.__str__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.ServerlessSimulator: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.ServerlessSimulator.__init__: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.ServerlessSimulator.activate: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.ServerlessSimulator.add_override: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.ServerlessSimulator.deactivate: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.ServerlessSimulator.get_current_env: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.create_mock_request: Python simulation helper; not ported
signalwire.cli.simulation.mock_env.load_env_file: Python simulation helper; not ported

## Bulk per-symbol omissions (auto-generated from skip rules)

Each line below is a Python symbol that the Rust port deliberately
does not expose under that exact name. The rationale explains why
(internal helper, Python-only mixin, alternate Rust idiom, etc.).

signalwire.core.agent.tools.decorator.ToolDecorator: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.agent.tools.decorator.ToolDecorator.create_class_decorator: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.agent.tools.decorator.ToolDecorator.create_instance_decorator: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.agent.tools.registry.ToolRegistry.register_class_decorated_tools: impossible: Python decorator-protocol class (ToolDecorator / class-decorated-tool registration) — Rust has no decorator syntax and no class-decoration hook; TS/PHP cousins also omit it (re-audited L18)
signalwire.core.contexts.create_simple_context: Python helper that wraps ContextBuilder; Rust users call ContextBuilder::new() directly
signalwire.core.data_map.create_expression_tool: Python helper functions; Rust ships create_simple_api_tool and create_expression_tool — already exposed (this entry is a duplicate from a Python-side path-prefix difference)
signalwire.core.data_map.create_simple_api_tool: Python helper functions; Rust ships create_simple_api_tool and create_expression_tool — already exposed (this entry is a duplicate from a Python-side path-prefix difference)
signalwire.core.mixins.mcp_server_mixin.MCPServerMixin: impossible: Python decorator-protocol surface (the @tool / MCPServerMixin decorator factory) — Rust has no decorator syntax; the OO cousins TS/PHP also express tool registration without this decorator method (re-audited L18)
signalwire.core.pom_builder.PomBuilder: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.swaig_function.SWAIGFunction: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swml_builder.SWMLBuilder: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_handler.AIVerbHandler.build_config: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.livewire.Agent: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.llm_node: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.on_enter: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.on_exit: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.on_user_turn_completed: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.session: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.stt_node: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.tts_node: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.update_instructions: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Agent.update_tools: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentHandoff: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentHandoff.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentServer: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentServer.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentServer.rtc_session: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.generate_reply: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.history: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.interrupt: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.say: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.start: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.update_agent: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.AgentSession.userdata: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.ChatContext: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.ChatContext.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.ChatContext.append: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.InferenceLLM: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.InferenceLLM.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.InferenceSTT: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.InferenceSTT.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.InferenceTTS: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.InferenceTTS.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.JobContext: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.JobContext.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.JobContext.connect: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.JobContext.wait_for_participant: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.JobProcess: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.JobProcess.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.Room: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.RunContext: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.RunContext.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.RunContext.userdata: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.StopResponse: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.ToolError: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.function_tool: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.CartesiaTTS: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.CartesiaTTS.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.DeepgramSTT: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.DeepgramSTT.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.ElevenLabsTTS: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.ElevenLabsTTS.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.OpenAILLM: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.OpenAILLM.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.SileroVAD: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.SileroVAD.__init__: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.plugins.SileroVAD.load: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.livewire.run_app: approved: livewire ported only to Python + Node/TS (the LiveKit AGENTS SDK languages); it is invented surface in every other port — user, 2026-07 pass (§I.1/L21)
signalwire.mcp_gateway.gateway_service.MCPGateway: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.gateway_service.MCPGateway.__init__: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.gateway_service.MCPGateway.run: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.gateway_service.MCPGateway.shutdown: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.gateway_service.main: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPClient: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPClient.__init__: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPClient.call_method: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPClient.call_tool: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPClient.get_tools: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPClient.start: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPClient.stop: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager.__init__: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager.create_client: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager.get_service: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager.get_service_tools: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager.list_services: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager.shutdown: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPManager.validate_services: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPService: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPService.__hash__: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.mcp_manager.MCPService.__post_init__: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.Session: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.Session.is_alive: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.Session.is_expired: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.Session.touch: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager.__init__: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager.close_session: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager.create_session: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager.get_service_session_count: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager.get_session: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager.list_sessions: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.mcp_gateway.session_manager.SessionManager.shutdown: standalone MCP gateway server; Rust ships the mcp_gateway *skill* (skill-level integration), not the standalone server
signalwire.pom.pom_tool.detect_file_format: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom_tool.load_pom: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom_tool.main: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom_tool.render_pom: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.prefabs.concierge.ConciergeAgent.on_summary: Python ConciergeAgent internals; Rust ships the canonical prefab
signalwire.prefabs.faq_bot.FAQBotAgent.on_summary: Python FAQBotAgent internals; Rust ships the canonical prefab
signalwire.prefabs.receptionist.ReceptionistAgent.on_summary: Python ReceptionistAgent internals; Rust ships the canonical prefab
signalwire.prefabs.survey.SurveyAgent.on_summary: Python SurveyAgent internals; Rust ships the canonical prefab
signalwire.relay.client.RelayClient.relay_protocol: impossible: Python abstract relay-protocol property hook — Rust models the RELAY protocol via concrete client methods, no abstract protocol accessor; TS/PHP cousins also omit it (re-audited L18)
signalwire.rest.call_handler.PhoneCallHandler: Python PhoneCallHandler enum exposing the 11 wire values; Rust ships the same constants on rest::PhoneCallHandler at the rest module — but the per-symbol enumerator may not have it mapped
signalwire.rest.namespaces.addresses.AddressesResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.addresses.AddressesResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.addresses.AddressesResource.create: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.addresses.AddressesResource.delete: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.addresses.AddressesResource.get: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.addresses.AddressesResource.list: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.calling.CallingNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.ai_hold: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.ai_message: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.ai_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.ai_unhold: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.collect: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.collect_start_input_timers: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.collect_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.denoise: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.denoise_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.detect: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.detect_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.dial: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.disconnect: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.end: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.live_transcribe: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.live_translate: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.play: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.play_pause: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.play_resume: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.play_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.play_volume: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.receive_fax_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.record: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.record_pause: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.record_resume: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.record_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.refer: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.send_fax_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.stream: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.stream_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.tap: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.tap_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.transcribe: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.transcribe_stop: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.transfer: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.calling.CallingNamespace.user_event: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.chat.ChatResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.chat.ChatResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.chat.ChatResource.create_token: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereDocuments: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereDocuments.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereDocuments.delete_chunk: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereDocuments.get_chunk: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereDocuments.list_chunks: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereDocuments.search: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.datasphere.DatasphereNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.AutoMaterializedWebhook: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.AutoMaterializedWebhook.create: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.CallFlowsResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.CallFlowsResource.deploy_version: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.CallFlowsResource.list_addresses: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.CallFlowsResource.list_versions: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.ConferenceRoomsResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.ConferenceRoomsResource.list_addresses: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.CxmlApplicationsResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.CxmlApplicationsResource.create: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.fabric.CxmlWebhooksResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricAddresses: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricAddresses.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricAddresses.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricResourcePUT: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricTokens: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricTokens.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricTokens.create_embed_token: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricTokens.create_guest_token: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricTokens.create_invite_token: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricTokens.create_subscriber_token: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.FabricTokens.refresh_subscriber_token: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.GenericResources: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.GenericResources.assign_domain_application: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.GenericResources.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.GenericResources.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.GenericResources.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.GenericResources.list_addresses: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.SubscribersResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.SubscribersResource.create_sip_endpoint: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.SubscribersResource.delete_sip_endpoint: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.SubscribersResource.get_sip_endpoint: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.SubscribersResource.list_sip_endpoints: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.SubscribersResource.update_sip_endpoint: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.fabric.SwmlWebhooksResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.imported_numbers.ImportedNumbersResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.imported_numbers.ImportedNumbersResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.imported_numbers.ImportedNumbersResource.create: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.logs.ConferenceLogs: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.ConferenceLogs.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.FaxLogs: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.FaxLogs.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.FaxLogs.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.LogsNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.LogsNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.MessageLogs: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.MessageLogs.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.MessageLogs.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.VoiceLogs: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.VoiceLogs.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.VoiceLogs.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.logs.VoiceLogs.list_events: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.lookup.LookupResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.lookup.LookupResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.lookup.LookupResource.phone_number: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.mfa.MfaResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.mfa.MfaResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.mfa.MfaResource.call: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.mfa.MfaResource.sms: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.mfa.MfaResource.verify: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.number_groups.NumberGroupsResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.number_groups.NumberGroupsResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.number_groups.NumberGroupsResource.add_membership: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.number_groups.NumberGroupsResource.delete_membership: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.number_groups.NumberGroupsResource.get_membership: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.number_groups.NumberGroupsResource.list_memberships: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.set_ai_agent: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.set_call_flow: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.set_cxml_application: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.set_cxml_webhook: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.set_relay_application: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.set_relay_topic: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.set_swml_webhook: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.project.ProjectNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.project.ProjectNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.project.ProjectTokens: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.project.ProjectTokens.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.project.ProjectTokens.create: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.project.ProjectTokens.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.project.ProjectTokens.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.pubsub.PubSubResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.pubsub.PubSubResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.pubsub.PubSubResource.create_token: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.queues.QueuesResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.queues.QueuesResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.queues.QueuesResource.get_member: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.queues.QueuesResource.get_next_member: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.queues.QueuesResource.list_members: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.recordings.RecordingsResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.recordings.RecordingsResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.recordings.RecordingsResource.delete: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.recordings.RecordingsResource.get: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.recordings.RecordingsResource.list: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.registry.RegistryBrands: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryBrands.create: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryBrands.create_campaign: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryBrands.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryBrands.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryBrands.list_campaigns: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryCampaigns: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryCampaigns.create_order: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryCampaigns.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryCampaigns.list_numbers: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryCampaigns.list_orders: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryCampaigns.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryNumbers: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryNumbers.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryOrders: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.registry.RegistryOrders.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.short_codes.ShortCodesResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.short_codes.ShortCodesResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.short_codes.ShortCodesResource.get: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.short_codes.ShortCodesResource.list: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.short_codes.ShortCodesResource.update: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.sip_profile.SipProfileResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.sip_profile.SipProfileResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.sip_profile.SipProfileResource.get: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.sip_profile.SipProfileResource.update: Python REST resource methods on sub-resources; Rust ships only the namespaces and methods actually used by the audit harness, with the rest available via the generic CrudResource shape
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoConferenceTokens: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoConferenceTokens.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoConferenceTokens.reset: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoConferences: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoConferences.create_stream: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoConferences.list_conference_tokens: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoConferences.list_streams: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomRecordings: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomRecordings.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomRecordings.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomRecordings.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomRecordings.list_events: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomSessions: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomSessions.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomSessions.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomSessions.list_events: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomSessions.list_members: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomSessions.list_recordings: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomTokens: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRoomTokens.create: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRooms: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRooms.create_stream: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoRooms.list_streams: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoStreams: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoStreams.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoStreams.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.video.VideoStreams.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.search.document_processor.DocumentProcessor: search-related; not ported per skip list
signalwire.search.document_processor.DocumentProcessor.__init__: search-related; not ported per skip list
signalwire.search.document_processor.DocumentProcessor.create_chunks: search-related; not ported per skip list
signalwire.search.index_builder.IndexBuilder: search-related; not ported per skip list
signalwire.search.index_builder.IndexBuilder.__init__: search-related; not ported per skip list
signalwire.search.index_builder.IndexBuilder.build_index: search-related; not ported per skip list
signalwire.search.index_builder.IndexBuilder.build_index_from_sources: search-related; not ported per skip list
signalwire.search.index_builder.IndexBuilder.validate_index: search-related; not ported per skip list
signalwire.search.migration.SearchIndexMigrator: search-related; not ported per skip list
signalwire.search.migration.SearchIndexMigrator.__init__: search-related; not ported per skip list
signalwire.search.migration.SearchIndexMigrator.get_index_info: search-related; not ported per skip list
signalwire.search.migration.SearchIndexMigrator.migrate_pgvector_to_sqlite: search-related; not ported per skip list
signalwire.search.migration.SearchIndexMigrator.migrate_sqlite_to_pgvector: search-related; not ported per skip list
signalwire.search.models.resolve_model_alias: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.close: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.create_schema: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.delete_collection: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.list_collections: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorBackend.store_chunks: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorSearchBackend: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorSearchBackend.__init__: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorSearchBackend.close: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorSearchBackend.fetch_candidates: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorSearchBackend.get_stats: search-related; not ported per skip list
signalwire.search.pgvector_backend.PgVectorSearchBackend.search: search-related; not ported per skip list
signalwire.search.query_processor.ensure_nltk_resources: search-related; not ported per skip list
signalwire.search.query_processor.get_wordnet_pos: search-related; not ported per skip list
signalwire.search.query_processor.load_spacy_model: search-related; not ported per skip list
signalwire.search.query_processor.remove_duplicate_words: search-related; not ported per skip list
signalwire.search.query_processor.set_global_model: search-related; not ported per skip list
signalwire.search.query_processor.vectorize_query: search-related; not ported per skip list
signalwire.search.search_engine.SearchEngine: search-related; not ported per skip list
signalwire.search.search_engine.SearchEngine.__init__: search-related; not ported per skip list
signalwire.search.search_engine.SearchEngine.get_stats: search-related; not ported per skip list
signalwire.search.search_engine.SearchEngine.search: search-related; not ported per skip list
signalwire.search.search_service.SearchService.search_direct: search-related; not ported per skip list
signalwire.search.search_service.SearchService.start: search-related; not ported per skip list
signalwire.search.search_service.SearchService.stop: search-related; not ported per skip list
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.get_tools: Python api_ninjas_trivia skill internals; Rust ships the canonical skill
signalwire.skills.google_maps.skill.GoogleMapsClient: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.google_maps.skill.GoogleMapsClient.__init__: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.google_maps.skill.GoogleMapsClient.compute_route: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.google_maps.skill.GoogleMapsClient.validate_address: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_global_data: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_hints: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_parameter_schema: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_prompt_sections: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.register_tools: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.setup: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.get_tools: Python play_background_file internals; Rust ships the canonical skill
signalwire.skills.weather_api.skill.WeatherApiSkill.get_tools: Python weather_api skill internals; Rust ships the canonical skill
signalwire.skills.web_search.skill.GoogleSearchScraper: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.__init__: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.extract_html_content: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.extract_reddit_content: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.extract_text_from_url: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.is_reddit_url: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.search_and_scrape: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.search_and_scrape_best: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.search_google: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill_improved.GoogleSearchScraper: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.GoogleSearchScraper.__init__: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.GoogleSearchScraper.extract_text_from_url: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.GoogleSearchScraper.search_and_scrape: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.GoogleSearchScraper.search_and_scrape_best: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.GoogleSearchScraper.search_google: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill.get_global_data: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill.get_hints: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill.get_instance_key: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill.get_parameter_schema: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill.get_prompt_sections: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill.register_tools: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_improved.WebSearchSkill.setup: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.GoogleSearchScraper: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.GoogleSearchScraper.__init__: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.GoogleSearchScraper.extract_text_from_url: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.GoogleSearchScraper.search_and_scrape: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.GoogleSearchScraper.search_google: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill.get_global_data: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill.get_hints: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill.get_instance_key: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill.get_parameter_schema: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill.get_prompt_sections: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill.register_tools: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.skills.web_search.skill_original.WebSearchSkill.setup: Python-experimental WebSearch variants; Rust ships the canonical web_search skill
signalwire.web.web_service.WebService: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.relay.client.RelayClient.__aenter__
signalwire.relay.client.RelayClient.__aexit__
signalwire.relay.client.RelayClient.__del__

## Python async-context-manager dunders (Python-only)

signalwire.relay.client.RelayClient.__aenter__: impossible: Python async context-manager protocol dunder (__aenter__/__aexit__) — no Rust equivalent; TS/PHP cousins also omit these protocol methods (re-audited L18)
signalwire.relay.client.RelayClient.__aexit__: impossible: Python async context-manager protocol dunder (__aenter__/__aexit__) — no Rust equivalent; TS/PHP cousins also omit these protocol methods (re-audited L18)
signalwire.relay.client.RelayClient.__del__: impossible: Python finalizer dunder (__del__) — Rust uses Drop, not a reference-counted finalizer method on the public surface; TS/PHP cousins also omit it (re-audited L18)

## Python state-attribute accessors (Python-only)

Python's reflection adapter emits zero-arg accessors for public instance
attributes (e.g. ``self.app``, ``self.logger``) as if they were getter
methods. Rust does not expose these as accessor methods — internal state
is private and accessed through dedicated APIs where needed. This is a
Rust idiom (private fields with no auto-generated getters).

signalwire.agent_server.AgentServer.app: Python's AgentServer exposes its FastAPI instance as a public attribute; Rust keeps the underlying axum/poem app private (server runs the app internally rather than handing it back).
signalwire.agent_server.AgentServer.logger: Python exposes ``self.logger`` as a public attribute; Rust uses tracing::info!()/log::info!() macros directly (no struct-attached logger).
signalwire.core.agent_base.AgentBase.skill_manager: Python exposes ``self.skill_manager`` as a public attribute; Rust holds the SkillManager privately and exposes typed methods on AgentBase (add_skill, list_skills, etc.) instead.
signalwire.core.skill_manager.SkillManager.logger: Python exposes ``self.logger`` as a public attribute on SkillManager; Rust uses tracing macros directly.
signalwire.core.swml_service.SWMLService.security: Python exposes ``self.security`` as the SecurityConfig attribute; Rust holds security configuration privately and exposes the relevant methods (get_basic_auth_credentials, validate_basic_auth) instead.
signalwire.core.swml_service.SWMLService.verb_registry: Python exposes ``self.verb_registry`` as the VerbHandlerRegistry attribute; Rust holds the verb registry privately and exposes verb-registration methods on SWMLService instead.
signalwire.skills.registry.SkillRegistry.logger: Python exposes ``self.logger`` as a public attribute on SkillRegistry; Rust uses tracing macros directly.
signalwire.core.security.webhook_middleware.make_webhook_validation_dependency: impossible: FastAPI dependency-factory (make_webhook_validation_dependency returns a Depends() callable) — a framework-specific DI primitive with no Rust equivalent; TS/PHP cousins also omit it (re-audited L18)

## Abstract RELAY action mixin bases (§H — flattened, TS/PHP flatten identically)

