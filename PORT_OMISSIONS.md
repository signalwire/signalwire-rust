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

signalwire.core.agent.prompt.manager.PromptManager: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.__init__: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.define_contexts: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.get_contexts: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.get_post_prompt: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.get_prompt: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.get_raw_prompt: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.prompt_add_section: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.prompt_add_subsection: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.prompt_add_to_section: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.prompt_has_section: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.set_post_prompt: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.set_prompt_pom: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.prompt.manager.PromptManager.set_prompt_text: Python internal prompt-manager class; Rust merges this functionality into AgentBase directly
signalwire.core.agent.tools.decorator.ToolDecorator: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.decorator.ToolDecorator.create_class_decorator: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.decorator.ToolDecorator.create_instance_decorator: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.__init__: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.define_tool: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.get_all_functions: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.get_function: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.has_function: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.register_class_decorated_tools: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.register_swaig_function: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.registry.ToolRegistry.remove_function: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.type_inference.create_typed_handler_wrapper: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent.tools.type_inference.infer_schema: Python internal tool registry / decorator helpers; Rust merges this into Service's tool registry and AgentBase::define_tool
signalwire.core.agent_base.AgentBase.add_answer_verb: Python-only convenience helpers; Rust users compose them via Service::route() / Service::host() / Service::port() directly
signalwire.core.agent_base.AgentBase.auto_map_sip_usernames: Python-only convenience helpers; Rust users compose them via Service::route() / Service::host() / Service::port() directly
signalwire.core.agent_base.AgentBase.get_full_url: Python-only convenience helpers; Rust users compose them via Service::route() / Service::host() / Service::port() directly
signalwire.core.agent_base.AgentBase.get_name: Python-only convenience helpers; Rust users compose them via Service::route() / Service::host() / Service::port() directly
signalwire.core.agent_base.AgentBase.set_web_hook_url: Python-only convenience helpers; Rust users compose them via Service::route() / Service::host() / Service::port() directly
signalwire.core.auth_handler.AuthHandler: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.auth_handler.AuthHandler.__init__: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.auth_handler.AuthHandler.flask_decorator: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.auth_handler.AuthHandler.get_auth_info: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.auth_handler.AuthHandler.get_fastapi_dependency: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.auth_handler.AuthHandler.verify_api_key: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.auth_handler.AuthHandler.verify_basic_auth: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.auth_handler.AuthHandler.verify_bearer_token: Python FastAPI auth-handler glue; Rust handles auth in Service::handle_request directly
signalwire.core.config_loader.ConfigLoader: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.__init__: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.find_config_file: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.get: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.get_config: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.get_config_file: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.get_section: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.has_config: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.merge_with_env: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.config_loader.ConfigLoader.substitute_vars: Python config-file loader; Rust users typically use std::env or env-loader crates of their choice
signalwire.core.contexts.Context.add_bullets: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.add_enter_filler: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.add_exit_filler: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.add_section: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.add_system_bullets: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.add_system_section: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_consolidate: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_full_reset: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_isolated: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_post_prompt: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_prompt: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_user_prompt: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_valid_contexts: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.set_valid_steps: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.Context.to_dict: Python-internal Context helpers; Rust ContextBuilder/Context uses direct field access for these (the Context shape is built into ContextBuilder methods rather than exposing per-field setters)
signalwire.core.contexts.ContextBuilder.to_dict: Python helper returning a dict; Rust's ContextBuilder serializes via serde to_value() directly
signalwire.core.contexts.GatherInfo.to_dict: Python helper returning a dict; Rust's GatherInfo serializes via serde to_value() directly
signalwire.core.contexts.GatherQuestion.to_dict: Python helper returning a dict; Rust's GatherQuestion serializes via serde to_value() directly
signalwire.core.contexts.Step.add_bullets: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.Step.clear_sections: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.Step.set_reset_consolidate: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.Step.set_reset_full_reset: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.Step.set_reset_system_prompt: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.Step.set_reset_user_prompt: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.Step.set_skip_to_next_step: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.Step.to_dict: Python-internal Step helpers; Rust Step uses direct field access
signalwire.core.contexts.create_simple_context: Python helper that wraps ContextBuilder; Rust users call ContextBuilder::new() directly
signalwire.core.data_map.DataMap.foreach: Python helper; Rust DataMap supports foreach via DataMap::foreach (already exposed) — entry is duplicate from Python's chained API
signalwire.core.data_map.create_expression_tool: Python helper functions; Rust ships create_simple_api_tool and create_expression_tool — already exposed (this entry is a duplicate from a Python-side path-prefix difference)
signalwire.core.data_map.create_simple_api_tool: Python helper functions; Rust ships create_simple_api_tool and create_expression_tool — already exposed (this entry is a duplicate from a Python-side path-prefix difference)
signalwire.core.function_result.FunctionResult.to_dict: Python FunctionResult.to_dict; Rust FunctionResult uses serde_json::to_value() / serialize() directly
signalwire.core.logging_config.configure_logging: Python logging-config helper; Rust uses logging::Logger directly
signalwire.core.logging_config.get_execution_mode: Python logging-config helper; Rust uses logging::Logger directly
signalwire.core.logging_config.get_logger: Python logging-config helper; Rust uses logging::Logger directly
signalwire.core.logging_config.reset_logging_configuration: Python logging-config helper; Rust uses logging::Logger directly
signalwire.core.logging_config.strip_control_chars: Python logging-config helper; Rust uses logging::Logger directly
signalwire.core.mixins.ai_config_mixin.AIConfigMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_function_include: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_hint: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_hints: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_internal_filler: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_language: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_mcp_server: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_pattern_hint: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.add_pronunciation: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.enable_debug_events: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.enable_mcp_server: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_function_includes: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_global_data: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_internal_fillers: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_languages: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_native_functions: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_param: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_params: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_post_prompt_llm_params: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_prompt_llm_params: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.set_pronunciations: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.ai_config_mixin.AIConfigMixin.update_global_data: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.auth_mixin.AuthMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.auth_mixin.AuthMixin.get_basic_auth_credentials: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.auth_mixin.AuthMixin.validate_basic_auth: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.mcp_server_mixin.MCPServerMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.contexts: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.define_contexts: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.get_post_prompt: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.get_prompt: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.prompt_add_section: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.prompt_add_subsection: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.prompt_add_to_section: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.prompt_has_section: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.reset_contexts: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.set_post_prompt: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.set_prompt_pom: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.prompt_mixin.PromptMixin.set_prompt_text: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.serverless_mixin.ServerlessMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.serverless_mixin.ServerlessMixin.handle_serverless_request: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.skill_mixin.SkillMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.skill_mixin.SkillMixin.add_skill: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.skill_mixin.SkillMixin.has_skill: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.skill_mixin.SkillMixin.list_skills: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.skill_mixin.SkillMixin.remove_skill: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.state_mixin.StateMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.state_mixin.StateMixin.validate_tool_token: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.tool_mixin.ToolMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.tool_mixin.ToolMixin.define_tool: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.tool_mixin.ToolMixin.define_tools: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.tool_mixin.ToolMixin.on_function_call: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.tool_mixin.ToolMixin.register_swaig_function: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.tool_mixin.ToolMixin.tool: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.as_router: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.enable_debug_routes: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.get_app: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.manual_set_proxy_url: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.on_request: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.on_swml_request: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.register_routing_callback: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.run: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.serve: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.set_dynamic_config_callback: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.mixins.web_mixin.WebMixin.setup_graceful_shutdown: Python uses mixins for AgentBase composition; Rust uses Deref<Target=Service> + direct methods on AgentBase. The functionality is present but does not surface as a separate mixin class.
signalwire.core.pom_builder.PomBuilder: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.__init__: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.add_section: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.add_subsection: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.add_to_section: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.from_sections: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.get_section: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.has_section: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.render_markdown: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.render_xml: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.to_dict: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.pom_builder.PomBuilder.to_json: Python POM builder helper class; Rust uses serde_json::Value directly
signalwire.core.security.session_manager.SessionManager.activate_session: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security.session_manager.SessionManager.create_tool_token: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security.session_manager.SessionManager.debug_token: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security.session_manager.SessionManager.end_session: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security.session_manager.SessionManager.generate_token: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security.session_manager.SessionManager.get_session_metadata: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security.session_manager.SessionManager.set_session_metadata: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security.session_manager.SessionManager.validate_tool_token: Python SessionManager session-tracking helpers; Rust ships the HMAC token primitives but not the per-call session-bookkeeping API — the equivalent is implemented inline by Service / AgentBase callers
signalwire.core.security_config.SecurityConfig: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.__init__: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.get_basic_auth: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.get_cors_config: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.get_security_headers: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.get_ssl_context_kwargs: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.get_url_scheme: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.load_from_env: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.log_config: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.should_allow_host: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.security_config.SecurityConfig.validate_ssl_config: Python security-config helper; Rust uses Service::basic_auth_credentials directly
signalwire.core.skill_base.SkillBase: Python SkillBase abstract class; Rust ships SkillBase as a trait — the trait surfaces under skill_base::SkillBase in Rust but the per-method enumerator may not pick up trait-method names
signalwire.core.skill_base.SkillBase.__init__: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.cleanup: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.define_tool: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.get_global_data: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.get_hints: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.get_instance_key: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.get_parameter_schema: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.get_prompt_sections: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.get_skill_data: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.register_tools: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.setup: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.update_skill_data: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.validate_env_vars: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_base.SkillBase.validate_packages: Python SkillBase abstract methods; Rust SkillBase trait surface is narrower (init / setup / handlers) — Python-only helpers are language-private
signalwire.core.skill_manager.SkillManager.list_loaded_skills: Python SkillManager helper; Rust skill_manager exposes list() for this
signalwire.core.swaig_function.SWAIGFunction: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swaig_function.SWAIGFunction.__call__: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swaig_function.SWAIGFunction.__init__: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swaig_function.SWAIGFunction.execute: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swaig_function.SWAIGFunction.to_swaig: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swaig_function.SWAIGFunction.validate_args: Python SWAIGFunction wrapper class; Rust merges into ToolDef on Service
signalwire.core.swml_builder.SWMLBuilder: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.__getattr__: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.__init__: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.add_section: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.ai: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.answer: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.build: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.hangup: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.play: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.render: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.reset: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_builder.SWMLBuilder.say: Python SWMLBuilder helper class; Rust uses serde_json::Value directly
signalwire.core.swml_handler.AIVerbHandler: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.AIVerbHandler.build_config: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.AIVerbHandler.get_verb_name: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.AIVerbHandler.validate_config: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.SWMLVerbHandler: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.SWMLVerbHandler.build_config: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.SWMLVerbHandler.get_verb_name: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.SWMLVerbHandler.validate_config: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.VerbHandlerRegistry: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.VerbHandlerRegistry.__init__: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.VerbHandlerRegistry.get_handler: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.VerbHandlerRegistry.has_handler: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_handler.VerbHandlerRegistry.register_handler: Python SWML request-handler helper; Rust merges into Service::handle_request and AgentBase::handle_request
signalwire.core.swml_renderer.SwmlRenderer: Python SWML renderer helper; Rust merges into Service::render_document and AgentBase::render_swml
signalwire.core.swml_renderer.SwmlRenderer.render_function_response_swml: Python SWML renderer helper; Rust merges into Service::render_document and AgentBase::render_swml
signalwire.core.swml_renderer.SwmlRenderer.render_swml: Python SWML renderer helper; Rust merges into Service::render_document and AgentBase::render_swml
signalwire.core.swml_service.SWMLService.__getattr__: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.add_section: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.add_verb_to_section: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.as_router: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.full_validation_enabled: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.get_basic_auth_credentials: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.get_document: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.manual_set_proxy_url: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.on_request: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.register_routing_callback: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.register_verb_handler: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.render_document: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.reset_document: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.serve: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.core.swml_service.SWMLService.stop: Python-only SWMLService surface (Python uses dynamic attribute lookup, FastAPI router, registered routing-callback dict, and a separate render/reset_document API); Rust users compose the equivalent via Service public methods
signalwire.livewire.Agent: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.llm_node: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.on_enter: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.on_exit: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.on_user_turn_completed: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.session: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.stt_node: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.tts_node: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.update_instructions: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Agent.update_tools: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentHandoff: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentHandoff.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentServer: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentServer.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentServer.rtc_session: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.generate_reply: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.history: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.interrupt: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.say: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.start: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.update_agent: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.AgentSession.userdata: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.ChatContext: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.ChatContext.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.ChatContext.append: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.InferenceLLM: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.InferenceLLM.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.InferenceSTT: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.InferenceSTT.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.InferenceTTS: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.InferenceTTS.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.JobContext: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.JobContext.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.JobContext.connect: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.JobContext.wait_for_participant: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.JobProcess: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.JobProcess.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.Room: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.RunContext: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.RunContext.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.RunContext.userdata: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.StopResponse: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.ToolError: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.function_tool: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.CartesiaTTS: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.CartesiaTTS.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.DeepgramSTT: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.DeepgramSTT.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.ElevenLabsTTS: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.ElevenLabsTTS.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.OpenAILLM: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.OpenAILLM.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.SileroVAD: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.SileroVAD.__init__: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.plugins.SileroVAD.load: livewire integration; Python-internal, not surfaced via cross-port skip list
signalwire.livewire.run_app: livewire integration; Python-internal, not surfaced via cross-port skip list
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
signalwire.pom.pom.PromptObjectModel: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.__init__: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.add_pom_as_subsection: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.add_section: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.find_section: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.from_json: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.from_yaml: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.render_markdown: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.render_xml: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.to_dict: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.to_json: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.PromptObjectModel.to_yaml: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section.__init__: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section.add_body: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section.add_bullets: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section.add_subsection: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section.render_markdown: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section.render_xml: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom.Section.to_dict: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom_tool.detect_file_format: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom_tool.load_pom: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom_tool.main: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.pom.pom_tool.render_pom: Prompt Object Model internal helper classes; Rust accomplishes the same via JSON values on AgentBase (prompt_add_section, prompt_add_subsection, prompt_add_to_section)
signalwire.prefabs.concierge.ConciergeAgent.check_availability: Python ConciergeAgent internals; Rust ships the canonical prefab
signalwire.prefabs.concierge.ConciergeAgent.get_directions: Python ConciergeAgent internals; Rust ships the canonical prefab
signalwire.prefabs.concierge.ConciergeAgent.on_summary: Python ConciergeAgent internals; Rust ships the canonical prefab
signalwire.prefabs.faq_bot.FAQBotAgent.on_summary: Python FAQBotAgent internals; Rust ships the canonical prefab
signalwire.prefabs.faq_bot.FAQBotAgent.search_faqs: Python FAQBotAgent internals; Rust ships the canonical prefab
signalwire.prefabs.info_gatherer.InfoGathererAgent.on_swml_request: Python InfoGathererAgent internals; Rust ships the canonical prefab with overlapping public surface
signalwire.prefabs.info_gatherer.InfoGathererAgent.set_question_callback: Python InfoGathererAgent internals; Rust ships the canonical prefab with overlapping public surface
signalwire.prefabs.info_gatherer.InfoGathererAgent.start_questions: Python InfoGathererAgent internals; Rust ships the canonical prefab with overlapping public surface
signalwire.prefabs.info_gatherer.InfoGathererAgent.submit_answer: Python InfoGathererAgent internals; Rust ships the canonical prefab with overlapping public surface
signalwire.prefabs.receptionist.ReceptionistAgent.on_summary: Python ReceptionistAgent internals; Rust ships the canonical prefab
signalwire.prefabs.survey.SurveyAgent.log_response: Python SurveyAgent internals; Rust ships the canonical prefab
signalwire.prefabs.survey.SurveyAgent.on_summary: Python SurveyAgent internals; Rust ships the canonical prefab
signalwire.prefabs.survey.SurveyAgent.validate_response: Python SurveyAgent internals; Rust ships the canonical prefab
signalwire.relay.call.AIAction: Python AI action class (Bedrock/AI verb dispatch); Rust merges AI action handling into the prefab BedrockAgent and the SWML AI verb
signalwire.relay.call.AIAction.__init__: Python AI action class (Bedrock/AI verb dispatch); Rust merges AI action handling into the prefab BedrockAgent and the SWML AI verb
signalwire.relay.call.AIAction.stop: Python AI action class (Bedrock/AI verb dispatch); Rust merges AI action handling into the prefab BedrockAgent and the SWML AI verb
signalwire.relay.call.Action.wait: Python Action.wait method; Rust action surface uses its own wait() — but the per-symbol enumerator may map differently
signalwire.relay.call.Call.echo: Python Call.echo helper; Rust Call exposes equivalent functionality through the dial / echo APIs in the broader Action surface
signalwire.relay.call.Call.pass_: Python Call.pass_ helper (reserved-word-wrapped 'pass'); Rust uses Call::pass_call to avoid the keyword collision
signalwire.relay.call.Call.refer: Python Call.refer (SIP REFER); Rust delegates SIP refer via the underlying ReferAction surface
signalwire.relay.call.Call.wait_for: Python Call event-bus helpers; Rust Call exposes register_event_callback for the same effect
signalwire.relay.call.Call.wait_for_ended: Python Call event-bus helpers; Rust Call exposes register_event_callback for the same effect
signalwire.relay.call.CollectAction.stop: Python CollectAction.stop; Rust delegates via the unified Action surface
signalwire.relay.call.CollectAction.volume: Python CollectAction.volume; Rust delegates volume control via the unified Action surface
signalwire.relay.call.DetectAction.__init__: Python DetectAction constructor; Rust constructs DetectAction internally during call.detect()
signalwire.relay.call.DetectAction.stop: Python DetectAction.stop; Rust delegates stop via the unified Action surface
signalwire.relay.call.FaxAction.stop: Python FaxAction.stop; Rust delegates stop via the unified Action surface
signalwire.relay.call.PayAction: Python PayAction class; Rust ships PayAction via the unified Action enum
signalwire.relay.call.PayAction.__init__: Python PayAction constructor; Rust constructs PayAction internally during call.pay()
signalwire.relay.call.PayAction.stop: Python PayAction.stop; Rust delegates stop via the unified Action surface
signalwire.relay.call.PlayAction.__init__: Python PlayAction constructor; Rust constructs PlayAction internally during call.play()
signalwire.relay.call.PlayAction.stop: Python PlayAction.stop; Rust delegates stop via the unified Action surface
signalwire.relay.call.RecordAction.__init__: Python RecordAction constructor; Rust constructs RecordAction internally during call.record()
signalwire.relay.call.RecordAction.stop: Python RecordAction.stop; Rust delegates stop via the unified Action surface
signalwire.relay.call.StandaloneCollectAction: Python standalone collect action variant; Rust merges into the unified CollectAction (the standalone variant is dispatched via the same Action type with a different parameter set)
signalwire.relay.call.StandaloneCollectAction.__init__: Python standalone collect action variant; Rust merges into the unified CollectAction (the standalone variant is dispatched via the same Action type with a different parameter set)
signalwire.relay.call.StandaloneCollectAction.start_input_timers: Python standalone collect action variant; Rust merges into the unified CollectAction (the standalone variant is dispatched via the same Action type with a different parameter set)
signalwire.relay.call.StandaloneCollectAction.stop: Python standalone collect action variant; Rust merges into the unified CollectAction (the standalone variant is dispatched via the same Action type with a different parameter set)
signalwire.relay.call.StreamAction: Python stream action class; Rust merges stream control into the unified Action surface (start/stop emitted via Call methods)
signalwire.relay.call.StreamAction.__init__: Python stream action class; Rust merges stream control into the unified Action surface (start/stop emitted via Call methods)
signalwire.relay.call.StreamAction.stop: Python stream action class; Rust merges stream control into the unified Action surface (start/stop emitted via Call methods)
signalwire.relay.call.TapAction: Python TapAction class; Rust merges TapAction into the Action enum
signalwire.relay.call.TapAction.__init__: Python TapAction constructor; Rust constructs TapAction internally during call.tap()
signalwire.relay.call.TapAction.stop: Python TapAction.stop; Rust calls stop via the underlying Action surface
signalwire.relay.call.TranscribeAction: Python TranscribeAction class; Rust merges into the Action surface — start/stop emitted via Call::transcribe / Call::stop_transcribe
signalwire.relay.call.TranscribeAction.__init__: Python TranscribeAction class; Rust merges into the Action surface — start/stop emitted via Call::transcribe / Call::stop_transcribe
signalwire.relay.call.TranscribeAction.stop: Python TranscribeAction class; Rust merges into the Action surface — start/stop emitted via Call::transcribe / Call::stop_transcribe
signalwire.relay.client.RelayClient.dial: Python-only RelayClient surface helpers — dial/execute are convenience wrappers; Rust users invoke equivalents via Call methods or direct connect() calls
signalwire.relay.client.RelayClient.execute: Python-only RelayClient surface helpers — dial/execute are convenience wrappers; Rust users invoke equivalents via Call methods or direct connect() calls
signalwire.relay.client.RelayClient.relay_protocol: Python-only RelayClient surface helpers — dial/execute are convenience wrappers; Rust users invoke equivalents via Call methods or direct connect() calls
signalwire.relay.client.RelayClient.run: Python-only RelayClient surface helpers — dial/execute are convenience wrappers; Rust users invoke equivalents via Call methods or direct connect() calls
signalwire.relay.client.RelayClient.send_message: Python-only RelayClient surface helpers — dial/execute are convenience wrappers; Rust users invoke equivalents via Call methods or direct connect() calls
signalwire.relay.client.RelayError: Python RelayError exception class; Rust uses Result<_, String>
signalwire.relay.client.RelayError.__init__: Python RelayError exception class; Rust uses Result<_, String>
signalwire.relay.event.CallReceiveEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.CallReceiveEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.CallStateEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.CallStateEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.CallingErrorEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.CallingErrorEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.CollectEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.CollectEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.ConferenceEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.ConferenceEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.ConnectEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.ConnectEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.DenoiseEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.DenoiseEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.DetectEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.DetectEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.DialEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.DialEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.EchoEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.EchoEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.FaxEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.FaxEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.HoldEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.HoldEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.MessageReceiveEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.MessageReceiveEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.MessageStateEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.MessageStateEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.PayEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.PayEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.PlayEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.PlayEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.QueueEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.QueueEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.RecordEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.RecordEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.ReferEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.ReferEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.RelayEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.RelayEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.SendDigitsEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.SendDigitsEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.StreamEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.StreamEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.TapEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.TapEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.TranscribeEvent: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.TranscribeEvent.from_payload: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.event.parse_event: Python event-name constants are emitted as Rust associated consts on the Event enum; the per-symbol enumerator does not pick them up
signalwire.relay.message.Message.wait: Python Message.wait; Rust users register an on_completed callback or use the futures-style API
signalwire.rest._base.BaseResource: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.BaseResource.__init__: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.CrudWithAddresses: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.CrudWithAddresses.list_addresses: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.HttpClient: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.HttpClient.__init__: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.HttpClient.delete: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.HttpClient.get: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.HttpClient.patch: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.HttpClient.post: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.HttpClient.put: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.SignalWireRestError: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._base.SignalWireRestError.__init__: Python REST base helper internals; Rust ships rest::CrudResource with a narrower public surface
signalwire.rest._pagination.PaginatedIterator: Python iterator class; Rust uses CrudResource::iter / list pagination via per-namespace methods
signalwire.rest._pagination.PaginatedIterator.__init__: Python iterator class; Rust uses CrudResource::iter / list pagination via per-namespace methods
signalwire.rest._pagination.PaginatedIterator.__iter__: Python iterator class; Rust uses CrudResource::iter / list pagination via per-namespace methods
signalwire.rest._pagination.PaginatedIterator.__next__: Python iterator class; Rust uses CrudResource::iter / list pagination via per-namespace methods
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
signalwire.rest.namespaces.compat.CompatAccounts: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatAccounts.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatAccounts.create: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatAccounts.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatAccounts.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatAccounts.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatApplications: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatApplications.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatCalls: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatCalls.start_recording: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatCalls.start_stream: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatCalls.stop_stream: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatCalls.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatCalls.update_recording: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.delete_recording: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.get_participant: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.get_recording: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.list_participants: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.list_recordings: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.remove_participant: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.start_stream: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.stop_stream: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.update_participant: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatConferences.update_recording: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatFaxes: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatFaxes.delete_media: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatFaxes.get_media: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatFaxes.list_media: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatFaxes.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatLamlBins: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatLamlBins.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatMessages: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatMessages.delete_media: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatMessages.get_media: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatMessages.list_media: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatMessages.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatNamespace: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatNamespace.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.__init__: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.import_number: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.list_available_countries: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.purchase: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.search_local: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.search_toll_free: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatPhoneNumbers.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatQueues: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatQueues.dequeue_member: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatQueues.get_member: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatQueues.list_members: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatQueues.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatRecordings: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatRecordings.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatRecordings.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatRecordings.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTokens: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTokens.create: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTokens.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTokens.update: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTranscriptions: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTranscriptions.delete: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTranscriptions.get: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.compat.CompatTranscriptions.list: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
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
signalwire.rest.namespaces.fabric.GenericResources.assign_phone_route: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
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
signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource.search: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
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
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.redial_verification: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
signalwire.rest.namespaces.verified_callers.VerifiedCallersResource.submit_verification: Python REST sub-namespace helper class methods; Rust ships the namespace-level access points and primary CRUD methods, leaving Python-internal helper methods as language-private
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
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.get_instance_key: Python api_ninjas_trivia skill internals; Rust ships the canonical skill
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.get_parameter_schema: Python api_ninjas_trivia skill internals; Rust ships the canonical skill
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.get_tools: Python api_ninjas_trivia skill internals; Rust ships the canonical skill
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.api_ninjas_trivia.skill.ApiNinjasTriviaSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.claude_skills.skill.ClaudeSkillsSkill.get_hints: claude_skills is a Python loader for SKILL.md packs; the Rust port exposes the registration hook but not the Python-specific loader plumbing
signalwire.skills.claude_skills.skill.ClaudeSkillsSkill.get_instance_key: claude_skills is a Python loader for SKILL.md packs; the Rust port exposes the registration hook but not the Python-specific loader plumbing
signalwire.skills.claude_skills.skill.ClaudeSkillsSkill.get_parameter_schema: claude_skills is a Python loader for SKILL.md packs; the Rust port exposes the registration hook but not the Python-specific loader plumbing
signalwire.skills.claude_skills.skill.ClaudeSkillsSkill.register_tools: claude_skills is a Python loader for SKILL.md packs; the Rust port exposes the registration hook but not the Python-specific loader plumbing
signalwire.skills.claude_skills.skill.ClaudeSkillsSkill.setup: claude_skills is a Python loader for SKILL.md packs; the Rust port exposes the registration hook but not the Python-specific loader plumbing
signalwire.skills.datasphere.skill.DataSphereSkill.cleanup: Python skill-base internal hooks; Rust skills override SkillBase methods directly
signalwire.skills.datasphere.skill.DataSphereSkill.get_global_data: Python datasphere skill internals; Rust ships the canonical skill
signalwire.skills.datasphere.skill.DataSphereSkill.get_hints: Python datasphere skill internals; Rust ships the canonical skill
signalwire.skills.datasphere.skill.DataSphereSkill.get_instance_key: Python datasphere skill internals; Rust ships the canonical skill
signalwire.skills.datasphere.skill.DataSphereSkill.get_parameter_schema: Python datasphere skill internals; Rust ships the canonical skill
signalwire.skills.datasphere.skill.DataSphereSkill.get_prompt_sections: Python datasphere skill internals; Rust ships the canonical skill
signalwire.skills.datasphere.skill.DataSphereSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.datasphere.skill.DataSphereSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.get_global_data: datasphere_serverless is a Python-experimental DataMap-based variant; Rust ships the canonical datasphere skill
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.get_hints: datasphere_serverless is a Python-experimental DataMap-based variant; Rust ships the canonical datasphere skill
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.get_instance_key: datasphere_serverless is a Python-experimental DataMap-based variant; Rust ships the canonical datasphere skill
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.get_parameter_schema: datasphere_serverless is a Python-experimental DataMap-based variant; Rust ships the canonical datasphere skill
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.get_prompt_sections: datasphere_serverless is a Python-experimental DataMap-based variant; Rust ships the canonical datasphere skill
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.register_tools: datasphere_serverless is a Python-experimental DataMap-based variant; Rust ships the canonical datasphere skill
signalwire.skills.datasphere_serverless.skill.DataSphereServerlessSkill.setup: datasphere_serverless is a Python-experimental DataMap-based variant; Rust ships the canonical datasphere skill
signalwire.skills.datetime.skill.DateTimeSkill.get_hints: Python datetime skill internals; Rust ships the canonical skill
signalwire.skills.datetime.skill.DateTimeSkill.get_parameter_schema: Python datetime skill internals; Rust ships the canonical skill
signalwire.skills.datetime.skill.DateTimeSkill.get_prompt_sections: Python datetime skill internals; Rust ships the canonical skill
signalwire.skills.datetime.skill.DateTimeSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.datetime.skill.DateTimeSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.google_maps.skill.GoogleMapsClient: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.google_maps.skill.GoogleMapsClient.__init__: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.google_maps.skill.GoogleMapsClient.compute_route: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.google_maps.skill.GoogleMapsClient.validate_address: Python-internal Google Maps client helper; Rust google_maps skill issues HTTP directly
signalwire.skills.google_maps.skill.GoogleMapsSkill.get_hints: Python google_maps skill internals; Rust ships the canonical skill
signalwire.skills.google_maps.skill.GoogleMapsSkill.get_parameter_schema: Python google_maps skill internals; Rust ships the canonical skill
signalwire.skills.google_maps.skill.GoogleMapsSkill.get_prompt_sections: Python google_maps skill internals; Rust ships the canonical skill
signalwire.skills.google_maps.skill.GoogleMapsSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.google_maps.skill.GoogleMapsSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.info_gatherer.skill.InfoGathererSkill.get_global_data: Python info_gatherer skill internals; Rust ships the canonical skill
signalwire.skills.info_gatherer.skill.InfoGathererSkill.get_instance_key: Python info_gatherer skill internals; Rust ships the canonical skill
signalwire.skills.info_gatherer.skill.InfoGathererSkill.get_parameter_schema: Python info_gatherer skill internals; Rust ships the canonical skill
signalwire.skills.info_gatherer.skill.InfoGathererSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.info_gatherer.skill.InfoGathererSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.joke.skill.JokeSkill.get_global_data: Python joke skill internals; Rust ships the canonical skill
signalwire.skills.joke.skill.JokeSkill.get_hints: Python joke skill internals; Rust ships the canonical skill
signalwire.skills.joke.skill.JokeSkill.get_parameter_schema: Python joke skill internals; Rust ships the canonical skill
signalwire.skills.joke.skill.JokeSkill.get_prompt_sections: Python joke skill internals; Rust ships the canonical skill
signalwire.skills.joke.skill.JokeSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.joke.skill.JokeSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.math.skill.MathSkill.get_hints: Python math skill internals; Rust ships the canonical skill
signalwire.skills.math.skill.MathSkill.get_parameter_schema: Python math skill internals; Rust ships the canonical skill
signalwire.skills.math.skill.MathSkill.get_prompt_sections: Python math skill internals; Rust ships the canonical skill
signalwire.skills.math.skill.MathSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.math.skill.MathSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_global_data: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_hints: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_parameter_schema: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.get_prompt_sections: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.register_tools: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.mcp_gateway.skill.MCPGatewaySkill.setup: skill-level MCP wiring is exposed via the mcp_gateway skill in Rust; the Python sub-module helpers (skill.MCPGatewaySkill internals) are language-private
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.cleanup: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.get_global_data: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.get_hints: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.get_instance_key: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.get_parameter_schema: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.get_prompt_sections: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.register_tools: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.native_vector_search.skill.NativeVectorSearchSkill.setup: vector-search skill requires Python ML stack; not ported per skip list
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.get_instance_key: Python play_background_file internals; Rust ships the canonical skill
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.get_parameter_schema: Python play_background_file internals; Rust ships the canonical skill
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.get_tools: Python play_background_file internals; Rust ships the canonical skill
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.play_background_file.skill.PlayBackgroundFileSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.registry.SkillRegistry.__init__: Python skill registry uses runtime .py loading; Rust registry is statically populated at compile time
signalwire.skills.registry.SkillRegistry.discover_skills: Python skill registry uses runtime .py loading; Rust registry is statically populated at compile time
signalwire.skills.registry.SkillRegistry.get_all_skills_schema: Python skill-registry internals; Rust ships SkillRegistry::register_skill, get_factory, list_skills with a narrower public surface
signalwire.skills.registry.SkillRegistry.get_skill_class: Python skill-registry internals; Rust ships SkillRegistry::register_skill, get_factory, list_skills with a narrower public surface
signalwire.skills.registry.SkillRegistry.list_all_skill_sources: Python skill-registry internals; Rust ships SkillRegistry::register_skill, get_factory, list_skills with a narrower public surface
signalwire.skills.spider.skill.SpiderSkill.cleanup: Python skill-base internal hooks; Rust skills override SkillBase methods directly
signalwire.skills.spider.skill.SpiderSkill.get_hints: Python spider skill internals; Rust ships the canonical spider skill with the same SkillBase surface
signalwire.skills.spider.skill.SpiderSkill.get_instance_key: Python spider skill internals; Rust ships the canonical spider skill with the same SkillBase surface
signalwire.skills.spider.skill.SpiderSkill.get_parameter_schema: Python spider skill internals; Rust ships the canonical spider skill with the same SkillBase surface
signalwire.skills.spider.skill.SpiderSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.spider.skill.SpiderSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.swml_transfer.skill.SWMLTransferSkill.get_hints: Python swml_transfer skill internals; Rust ships the canonical swml_transfer skill
signalwire.skills.swml_transfer.skill.SWMLTransferSkill.get_instance_key: Python swml_transfer skill internals; Rust ships the canonical swml_transfer skill
signalwire.skills.swml_transfer.skill.SWMLTransferSkill.get_parameter_schema: Python swml_transfer skill internals; Rust ships the canonical swml_transfer skill
signalwire.skills.swml_transfer.skill.SWMLTransferSkill.get_prompt_sections: Python swml_transfer skill internals; Rust ships the canonical swml_transfer skill
signalwire.skills.swml_transfer.skill.SWMLTransferSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.swml_transfer.skill.SWMLTransferSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.weather_api.skill.WeatherApiSkill.get_parameter_schema: Python weather_api skill internals; Rust ships the canonical skill
signalwire.skills.weather_api.skill.WeatherApiSkill.get_tools: Python weather_api skill internals; Rust ships the canonical skill
signalwire.skills.weather_api.skill.WeatherApiSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.weather_api.skill.WeatherApiSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.web_search.skill.GoogleSearchScraper: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.__init__: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.extract_html_content: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.extract_reddit_content: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.extract_text_from_url: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.is_reddit_url: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.search_and_scrape: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.search_and_scrape_best: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.GoogleSearchScraper.search_google: Python-internal scraper helper; Rust web_search skill issues HTTP directly via its skill handler
signalwire.skills.web_search.skill.WebSearchSkill.get_global_data: Python web_search skill internals; Rust ships the canonical skill
signalwire.skills.web_search.skill.WebSearchSkill.get_hints: Python web_search skill internals; Rust ships the canonical skill
signalwire.skills.web_search.skill.WebSearchSkill.get_instance_key: Python web_search skill internals; Rust ships the canonical skill
signalwire.skills.web_search.skill.WebSearchSkill.get_parameter_schema: Python web_search skill internals; Rust ships the canonical skill
signalwire.skills.web_search.skill.WebSearchSkill.get_prompt_sections: Python web_search skill internals; Rust ships the canonical skill
signalwire.skills.web_search.skill.WebSearchSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.web_search.skill.WebSearchSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
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
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.get_hints: Python wikipedia_search skill internals; Rust ships the canonical skill
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.get_parameter_schema: Python wikipedia_search skill internals; Rust ships the canonical skill
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.get_prompt_sections: Python wikipedia_search skill internals; Rust ships the canonical skill
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.register_tools: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.search_wiki: Python wikipedia_search skill internals; Rust ships the canonical skill
signalwire.skills.wikipedia_search.skill.WikipediaSearchSkill.setup: Python skill-internal hooks (mostly _-prefixed or setup helpers); Rust skill implementations override SkillBase methods directly
signalwire.utils.is_serverless_mode: Python utils package internals; Rust users compose equivalent helpers on the standard library
signalwire.utils.schema_utils.SchemaUtils: Python utils package internals; Rust users compose equivalent helpers on the standard library
signalwire.utils.schema_utils.SchemaUtils.__init__: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.full_validation_available: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.generate_method_body: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.generate_method_signature: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.get_all_verb_names: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.get_verb_parameters: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.get_verb_properties: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.get_verb_required_properties: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.load_schema: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.validate_document: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaUtils.validate_verb: Python schema utils; Rust ships swml::schema with a narrower public surface
signalwire.utils.schema_utils.SchemaValidationError: Python utils package internals; Rust users compose equivalent helpers on the standard library
signalwire.utils.schema_utils.SchemaValidationError.__init__: Python utils package internals; Rust users compose equivalent helpers on the standard library
signalwire.utils.url_validator.validate_url: Python utils package internals; Rust users compose equivalent helpers on the standard library
signalwire.web.web_service.WebService: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.web.web_service.WebService.__init__: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.web.web_service.WebService.add_directory: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.web.web_service.WebService.remove_directory: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.web.web_service.WebService.start: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.web.web_service.WebService.stop: Python WebService internals; Rust integrates static file serving into AgentServer
signalwire.relay.client.RelayClient.__aenter__
signalwire.relay.client.RelayClient.__aexit__
signalwire.relay.client.RelayClient.__del__

## Python async-context-manager dunders (Python-only)

signalwire.relay.client.RelayClient.__aenter__: Python async-context-manager protocol; Rust uses RAII Drop semantics
signalwire.relay.client.RelayClient.__aexit__: Python async-context-manager protocol; Rust uses RAII Drop semantics
signalwire.relay.client.RelayClient.__del__: Python finalizer; Rust uses Drop

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
signalwire.core.security.webhook_middleware.make_webhook_validation_dependency: Python ships a FastAPI dependency-factory free function; Rust ships the equivalent as a tower::Layer (``signalwire::security::webhook_layer::WebhookLayer``) per the axum/tower idiom. Functional parity exists — the surface shape differs because Rust has no FastAPI-style dependency-injection runtime.
