use serde_json::{json, Value};

use crate::agent::{AgentBase, AgentOptions};
use crate::swaig::FunctionResult;

/// A pre-built receptionist agent that routes callers to departments.
pub struct ReceptionistAgent {
    agent: AgentBase,
    departments: Vec<Value>,
    greeting: String,
}

impl ReceptionistAgent {
    /// Create a new ReceptionistAgent.
    ///
    /// # Arguments
    /// - `name` — agent name (defaults to `"receptionist"` if empty).
    /// - `departments` — list of `{name, description, number?, transfer_type?, swml_url?}` objects.
    /// - `greeting` — optional greeting message.
    /// - `route` — optional route (defaults to `"/receptionist"`).
    pub fn new(
        name: &str,
        departments: Vec<Value>,
        greeting: Option<&str>,
        route: Option<&str>,
    ) -> Self {
        let agent_name = if name.is_empty() {
            "receptionist"
        } else {
            name
        };
        let greeting_text = greeting
            .unwrap_or("Thank you for calling. How can I help you today?")
            .to_string();

        let mut opts = AgentOptions::new(agent_name);
        opts.route = Some(route.unwrap_or("/receptionist").to_string());
        opts.use_pom = true;

        let mut agent = AgentBase::new(opts);

        // Global data
        agent.set_global_data(json!({
            "departments": departments,
            "caller_info": {},
        }));

        // Build department list for prompt
        let mut dept_bullets: Vec<String> = vec![
            "Greet the caller warmly".to_string(),
            "Determine which department they need".to_string(),
            "Transfer them to the correct department".to_string(),
        ];
        for dept in &departments {
            let dept_name = dept
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let dept_desc = dept
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            dept_bullets.push(format!("{}: {}", dept_name, dept_desc));
        }

        let bullet_refs: Vec<&str> = dept_bullets.iter().map(|s| s.as_str()).collect();
        agent.prompt_add_section("Receptionist Role", &greeting_text, bullet_refs);

        // Tool: collect_caller_info
        agent.define_tool(
            "collect_caller_info",
            "Collect and store caller identification information",
            json!({
                "caller_name": {"type": "string", "description": "Name of the caller"},
                "caller_phone": {"type": "string", "description": "Phone number of the caller"},
                "reason": {"type": "string", "description": "Reason for calling"},
            }),
            Box::new(|args, _raw| {
                let caller_name = args
                    .get("caller_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let reason = args
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Not specified");
                FunctionResult::with_response(&format!(
                    "Caller info recorded: {}, reason: {}",
                    caller_name, reason
                ))
            }),
            false,
        );

        // Tool: transfer_call
        let depts_clone = departments.clone();
        agent.define_tool(
            "transfer_call",
            "Transfer the caller to the specified department",
            json!({
                "department": {"type": "string", "description": "Department name to transfer to"},
            }),
            Box::new(move |args, _raw| {
                let dept_name = args
                    .get("department")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                for dept in &depts_clone {
                    let name = dept
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if name.to_lowercase() == dept_name.to_lowercase() {
                        let transfer_type = dept
                            .get("transfer_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("phone");

                        let mut result =
                            FunctionResult::with_response(&format!("Transferring to {}", name));

                        if transfer_type == "swml" {
                            if let Some(swml_url) = dept.get("swml_url").and_then(|v| v.as_str()) {
                                result.swml_transfer(
                                    swml_url,
                                    &format!("Transferring you to {} now.", name),
                                );
                            }
                        } else if let Some(number) = dept.get("number").and_then(|v| v.as_str()) {
                            result.connect(number, false, "");
                        }

                        return result;
                    }
                }

                FunctionResult::with_response(&format!("Department '{}' not found", dept_name))
            }),
            false,
        );

        ReceptionistAgent {
            agent,
            departments,
            greeting: greeting_text,
        }
    }

    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    pub fn departments(&self) -> &[Value] {
        &self.departments
    }

    pub fn greeting(&self) -> &str {
        &self.greeting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_departments() -> Vec<Value> {
        vec![
            json!({"name": "Sales", "description": "Sales department", "number": "+15551234567"}),
            json!({"name": "Support", "description": "Technical support", "swml_url": "https://example.com/support", "transfer_type": "swml"}),
        ]
    }

    #[test]
    fn test_receptionist_construction() {
        let agent = ReceptionistAgent::new("test", sample_departments(), None, None);
        assert_eq!(agent.agent().service().name(), "test");
        assert_eq!(agent.agent().service().route(), "/receptionist");
        assert_eq!(agent.departments().len(), 2);
    }

    #[test]
    fn test_receptionist_has_tools() {
        let agent = ReceptionistAgent::new("test", sample_departments(), None, None);
        let raw = serde_json::Map::new();

        let mut args = serde_json::Map::new();
        args.insert("caller_name".to_string(), json!("Alice"));
        args.insert("reason".to_string(), json!("Billing inquiry"));
        let result = agent.agent().on_function_call("collect_caller_info", &args, &raw);
        assert!(result.is_some());

        let mut args2 = serde_json::Map::new();
        args2.insert("department".to_string(), json!("Sales"));
        let result2 = agent.agent().on_function_call("transfer_call", &args2, &raw);
        assert!(result2.is_some());
    }

    #[test]
    fn test_receptionist_default_name() {
        let agent = ReceptionistAgent::new("", sample_departments(), None, None);
        assert_eq!(agent.agent().service().name(), "receptionist");
    }
}
