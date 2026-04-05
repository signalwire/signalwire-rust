# Examples

Working examples demonstrating the SignalWire Rust SDK.

## Getting Started

| Example | Description |
|---------|-------------|
| [simple_agent.rs](simple_agent.rs) | POM prompts, SWAIG tools, multilingual support, LLM tuning |
| [simple_static_agent.rs](simple_static_agent.rs) | Static configuration at startup |
| [simple_dynamic_agent.rs](simple_dynamic_agent.rs) | Dynamic per-request configuration |
| [simple_dynamic_enhanced.rs](simple_dynamic_enhanced.rs) | VIP/department/language routing |
| [declarative_agent.rs](declarative_agent.rs) | Declarative prompt template |

## Contexts and Workflows

| Example | Description |
|---------|-------------|
| [contexts_demo.rs](contexts_demo.rs) | Multi-persona workflow with context switching |
| [gather_info_demo.rs](gather_info_demo.rs) | Low-level gather_info mode for data collection |

## Tools and DataMap

| Example | Description |
|---------|-------------|
| [data_map_demo.rs](data_map_demo.rs) | DataMap server-side API tools |
| [advanced_datamap_demo.rs](advanced_datamap_demo.rs) | Expressions, webhooks, fallback chains |
| [swaig_features_agent.rs](swaig_features_agent.rs) | SWAIG features showcase |

## Skills

| Example | Description |
|---------|-------------|
| [skills_demo.rs](skills_demo.rs) | Built-in skills (datetime, math) |
| [joke_agent.rs](joke_agent.rs) | Raw data_map joke integration |
| [joke_skill_demo.rs](joke_skill_demo.rs) | Joke skill via modular system |

## Call Flow and Actions

| Example | Description |
|---------|-------------|
| [call_flow_and_actions_demo.rs](call_flow_and_actions_demo.rs) | Call-flow verbs, debug events, FunctionResult actions |
| [session_and_state_demo.rs](session_and_state_demo.rs) | on_summary, global data, post-prompt |
| [record_call_example.rs](record_call_example.rs) | Record/stop_record virtual helpers |
| [room_and_sip_example.rs](room_and_sip_example.rs) | Room join and SIP REFER helpers |
| [tap_example.rs](tap_example.rs) | Media tap/stream |

## Prefab Agents

| Example | Description |
|---------|-------------|
| [info_gatherer_example.rs](info_gatherer_example.rs) | InfoGathererAgent prefab |
| [dynamic_info_gatherer_example.rs](dynamic_info_gatherer_example.rs) | Dynamic question sets |
| [survey_agent_example.rs](survey_agent_example.rs) | SurveyAgent prefab |
| [faq_bot_agent.rs](faq_bot_agent.rs) | FAQBotAgent with knowledge base |
| [receptionist_agent_example.rs](receptionist_agent_example.rs) | ReceptionistAgent with departments |
| [concierge_agent_example.rs](concierge_agent_example.rs) | ConciergeAgent for venues |

## Dynamic Configuration

| Example | Description |
|---------|-------------|
| [comprehensive_dynamic_agent.rs](comprehensive_dynamic_agent.rs) | Multi-tenant routing, A/B testing |
| [custom_path_agent.rs](custom_path_agent.rs) | Custom route paths |
| [llm_params_demo.rs](llm_params_demo.rs) | LLM parameter tuning |

## Multi-Agent and Hosting

| Example | Description |
|---------|-------------|
| [multi_agent_server.rs](multi_agent_server.rs) | Multiple agents on one server |
| [multi_endpoint_agent.rs](multi_endpoint_agent.rs) | SWML + web UI + API endpoints |

## SWML Service

| Example | Description |
|---------|-------------|
| [basic_swml_service.rs](basic_swml_service.rs) | Non-AI SWML flows (IVR, voicemail) |
| [auto_vivified_example.rs](auto_vivified_example.rs) | Auto-vivified verb methods |
| [dynamic_swml_service.rs](dynamic_swml_service.rs) | Per-request dynamic SWML |
| [swml_service_example.rs](swml_service_example.rs) | Full SWML service patterns |
| [swml_service_routing_example.rs](swml_service_routing_example.rs) | SWML routing by caller |

## Deployment

| Example | Description |
|---------|-------------|
| [lambda_agent.rs](lambda_agent.rs) | AWS Lambda deployment |
| [kubernetes_ready_agent.rs](kubernetes_ready_agent.rs) | Kubernetes with health checks |

## MCP Integration

| Example | Description |
|---------|-------------|
| [mcp_agent.rs](mcp_agent.rs) | MCP client and server |
| [mcp_gateway_demo.rs](mcp_gateway_demo.rs) | MCP gateway skill |

## Search and Datasphere

| Example | Description |
|---------|-------------|
| [local_search_agent.rs](local_search_agent.rs) | Local document search |
| [web_search_agent.rs](web_search_agent.rs) | Web search integration |
| [web_search_multi_instance_demo.rs](web_search_multi_instance_demo.rs) | Multiple search instances |
| [wikipedia_demo.rs](wikipedia_demo.rs) | Wikipedia search integration |
| [datasphere_multi_instance_demo.rs](datasphere_multi_instance_demo.rs) | Multiple Datasphere instances |
| [datasphere_serverless_demo.rs](datasphere_serverless_demo.rs) | Serverless Datasphere |
| [datasphere_serverless_env_demo.rs](datasphere_serverless_env_demo.rs) | Datasphere via environment |
| [datasphere_webhook_env_demo.rs](datasphere_webhook_env_demo.rs) | Datasphere via webhook |

## RELAY

| Example | Description |
|---------|-------------|
| [relay_answer_and_welcome.rs](relay_answer_and_welcome.rs) | Answer and play TTS |

## Running Examples

```bash
# Run any example
cargo run --example simple_agent

# With environment variables
SIGNALWIRE_LOG_LEVEL=debug cargo run --example simple_agent

# Test without running a server
cargo run --bin swaig-test -- --dump-swml examples/simple_agent.rs
```
