use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Bridge MCP servers with SWAIG functions (handler-based).
pub struct McpGateway {
    sp: SkillParams,
}

impl McpGateway {
    pub fn new(params: Map<String, Value>) -> Self {
        McpGateway {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for McpGateway {
    fn name(&self) -> &str {
        "mcp_gateway"
    }

    fn description(&self) -> &str {
        "Bridge MCP servers with SWAIG functions"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("gateway_url").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let gateway_url = self.sp.get_str_or("gateway_url", "");
        let services = self.sp.get_array("services");
        let tool_prefix = self.sp.get_str_or("tool_prefix", "mcp_");

        if services.is_empty() {
            // Register a generic gateway tool
            let gw_url = gateway_url.clone();
            agent.define_tool(
                &format!("{}call", tool_prefix),
                "Call an MCP service through the gateway",
                json!({
                    "service": {
                        "type": "string",
                        "description": "The MCP service name",
                        "required": true,
                    },
                    "tool": {
                        "type": "string",
                        "description": "The tool name to call on the service",
                        "required": true,
                    },
                    "arguments": {
                        "type": "object",
                        "description": "Arguments to pass to the MCP tool",
                    },
                }),
                Box::new(move |args, _raw| {
                    let mut result = FunctionResult::new();
                    let service = args
                        .get("service")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let tool = args
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    result.set_response(&format!(
                        "MCP gateway call to service \"{}\", tool \"{}\" via gateway at \"{}\". \
                         In production, this would forward the request to the MCP gateway service.",
                        service, tool, gw_url
                    ));
                    result
                }),
                false,
            );
            return;
        }

        // Register one tool per service/tool pair
        for service in &services {
            let service_name = service
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let service_tools = service
                .get("tools")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if service_name.is_empty() || service_tools.is_empty() {
                continue;
            }

            for tool in &service_tools {
                let tool_name = tool
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tool_description = tool
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tool_params = tool
                    .get("parameters")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                if tool_name.is_empty() {
                    continue;
                }

                let full_tool_name = format!(
                    "{}{}_{}",
                    tool_prefix, service_name, tool_name
                );
                let full_description =
                    format!("[{}] {}", service_name, tool_description);

                let mut properties = Map::new();
                for param in &tool_params {
                    let param_name = param
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if param_name.is_empty() {
                        continue;
                    }
                    let param_type = param
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("string");
                    let param_desc = param
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or(param_name);
                    let is_required = param
                        .get("required")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let mut prop = json!({
                        "type": param_type,
                        "description": param_desc,
                    });
                    if is_required {
                        prop["required"] = json!(true);
                    }
                    properties.insert(param_name.to_string(), prop);
                }

                let gw = gateway_url.clone();
                let svc = service_name.to_string();
                let tn = tool_name.to_string();

                agent.define_tool(
                    &full_tool_name,
                    &full_description,
                    Value::Object(properties),
                    Box::new(move |args, _raw| {
                        let mut result = FunctionResult::new();
                        result.set_response(&format!(
                            "MCP gateway call to service \"{}\", tool \"{}\" via gateway at \"{}\". \
                             Arguments: {:?}. \
                             In production, this would forward the request to the MCP gateway service.",
                            svc, tn, gw, args
                        ));
                        result
                    }),
                    false,
                );
            }
        }
    }

    fn get_hints(&self) -> Vec<String> {
        let mut hints = vec!["MCP".to_string(), "gateway".to_string()];

        let services = self.sp.get_array("services");
        for service in &services {
            if let Some(name) = service.get("name").and_then(|v| v.as_str()) {
                let name_str = name.to_string();
                if !hints.contains(&name_str) {
                    hints.push(name_str);
                }
            }
        }

        hints
    }

    fn get_global_data(&self) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert(
            "mcp_gateway_url".to_string(),
            Value::String(self.sp.get_str_or("gateway_url", "")),
        );
        data.insert("mcp_session_id".to_string(), Value::Null);

        let services = self.sp.get_array("services");
        let service_names: Vec<Value> = services
            .iter()
            .filter_map(|s| s.get("name").and_then(|v| v.as_str()).map(|n| json!(n)))
            .collect();
        data.insert("mcp_services".to_string(), Value::Array(service_names));
        data
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        let services = self.sp.get_array("services");
        let mut bullets = Vec::new();

        for service in &services {
            let name = service
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let description = service
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !name.is_empty() {
                let bullet = if !description.is_empty() {
                    format!("Service: {} - {}", name, description)
                } else {
                    format!("Service: {}", name)
                };
                bullets.push(bullet);
            }
        }

        if bullets.is_empty() {
            bullets.push("MCP gateway is configured but no services are defined.".to_string());
        }

        let bullet_values: Vec<Value> = bullets.into_iter().map(|b| json!(b)).collect();

        vec![json!({
            "title": "MCP Gateway Integration",
            "body": "You have access to external services through the MCP gateway.",
            "bullets": bullet_values,
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_gateway_metadata() {
        let skill = McpGateway::new(Map::new());
        assert_eq!(skill.name(), "mcp_gateway");
    }

    #[test]
    fn test_mcp_gateway_setup_needs_url() {
        let mut skill = McpGateway::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("gateway_url".to_string(), json!("https://mcp.example.com"));
        let mut skill2 = McpGateway::new(params);
        assert!(skill2.setup());
    }
}
