use serde_json::{Value, json};

use crate::agent::{AgentBase, AgentOptions};
use crate::prefabs::PrefabSummaryCallback;
use crate::swaig::FunctionResult;

/// Options for constructing a [`ReceptionistAgent`].
///
/// Mirrors the Python reference's `__init__` (`prefabs/receptionist.py:38-45`)
/// param-for-param. `departments` is the reference's one REQUIRED positional,
/// so it is the sole argument to [`ReceptionistOptions::new`]; every other
/// field carries the reference's default.
#[must_use]
pub struct ReceptionistOptions {
    /// Departments to transfer to, each `{name, description, number?}`.
    pub departments: Vec<Value>,
    /// Agent name (reference default `"receptionist"`).
    pub name: String,
    /// HTTP route (reference default `"/receptionist"`).
    pub route: String,
    /// Initial greeting message.
    pub greeting: String,
    /// Voice id used for the configured language (reference default
    /// `"rime.spore"`).
    pub voice: String,
}

impl Default for ReceptionistOptions {
    fn default() -> Self {
        ReceptionistOptions {
            departments: Vec::new(),
            name: "receptionist".to_string(),
            route: "/receptionist".to_string(),
            greeting: "Thank you for calling. How can I help you today?".to_string(),
            voice: "rime.spore".to_string(),
        }
    }
}

impl ReceptionistOptions {
    /// Options for `departments`, with every other field at its reference
    /// default — the port of the reference's `ReceptionistAgent(departments)`.
    pub fn new(departments: Vec<Value>) -> Self {
        ReceptionistOptions {
            departments,
            ..Default::default()
        }
    }

    /// Replace the departments callers can be transferred to.
    pub fn departments(mut self, departments: Vec<Value>) -> Self {
        self.departments = departments;
        self
    }

    /// Set the agent name (default `"receptionist"`).
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the HTTP route (default `"/receptionist"`).
    pub fn route(mut self, route: &str) -> Self {
        self.route = route.to_string();
        self
    }

    /// Set the initial greeting message.
    pub fn greeting(mut self, greeting: &str) -> Self {
        self.greeting = greeting.to_string();
        self
    }

    /// Set the voice id (default `"rime.spore"`).
    pub fn voice(mut self, voice: &str) -> Self {
        self.voice = voice.to_string();
        self
    }
}

/// A pre-built receptionist agent that routes callers to departments.
pub struct ReceptionistAgent {
    agent: AgentBase,
    departments: Vec<Value>,
    greeting: String,
}

impl ReceptionistAgent {
    /// Create a new `ReceptionistAgent` from [`ReceptionistOptions`].
    ///
    /// `ReceptionistAgent::new(ReceptionistOptions::new(departments))` ports
    /// the reference's minimal `ReceptionistAgent(departments)`.
    pub fn new(options: ReceptionistOptions) -> Self {
        let ReceptionistOptions {
            departments,
            name,
            route,
            greeting,
            voice,
        } = options;

        let agent_name = if name.is_empty() {
            "receptionist"
        } else {
            &name
        };
        let greeting_text = greeting;

        let mut opts = AgentOptions::new(agent_name);
        opts.route = Some(if route.is_empty() {
            "/receptionist".to_string()
        } else {
            route
        });
        opts.use_pom = true;

        let mut agent = AgentBase::new(opts);

        // Language + voice (reference `_configure_agent_settings(voice)`).
        agent.add_language("English", "en-US", &voice);

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
            dept_bullets.push(format!("{dept_name}: {dept_desc}"));
        }

        let bullet_refs: Vec<&str> = dept_bullets
            .iter()
            .map(std::string::String::as_str)
            .collect();
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
                    "Caller info recorded: {caller_name}, reason: {reason}"
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
                    let name = dept.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    if name.to_lowercase() == dept_name.to_lowercase() {
                        let transfer_type = dept
                            .get("transfer_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("phone");

                        let mut result =
                            FunctionResult::with_response(&format!("Transferring to {name}"));

                        if transfer_type == "swml" {
                            if let Some(swml_url) = dept.get("swml_url").and_then(|v| v.as_str()) {
                                result.swml_transfer(
                                    swml_url,
                                    &format!("Transferring you to {name} now."),
                                    // `None` = the reference default final=true:
                                    // permanent transfer — the receptionist
                                    // hands the call off entirely.
                                    None,
                                );
                            }
                        } else if let Some(number) = dept.get("number").and_then(|v| v.as_str()) {
                            // final=false is DELIBERATE (not the default): the
                            // caller returns to the receptionist afterwards. No
                            // caller-ID override, so `from` is omitted.
                            result.connect(number, Some(false), None);
                        }

                        return result;
                    }
                }

                FunctionResult::with_response(&format!("Department '{dept_name}' not found"))
            }),
            false,
        );

        ReceptionistAgent {
            agent,
            departments,
            greeting: greeting_text,
        }
    }

    /// Borrow the underlying [`AgentBase`] this prefab wraps.
    ///
    /// `ReceptionistAgent` composes an agent rather than inheriting from one, so
    /// this is how you read the configured prompt, tools, and skills.
    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    /// Mutably borrow the underlying [`AgentBase`].
    ///
    /// Use this to layer extra configuration — additional tools, skills,
    /// hints, or verbs — on top of what the prefab already set up.
    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    /// The departments this receptionist can transfer to, as configured.
    /// Each entry carries the department's name and its destination.
    pub fn departments(&self) -> &[Value] {
        &self.departments
    }

    /// The greeting spoken when the call is answered — the caller's value
    /// or the reference default.
    pub fn greeting(&self) -> &str {
        &self.greeting
    }

    /// Register a callback that processes the conversation summary.
    ///
    /// Delegates to [`AgentBase::on_summary`], matching the Python
    /// `ReceptionistAgent.on_summary` override point (a no-op subclasses replace).
    pub fn on_summary(&mut self, callback: PrefabSummaryCallback) -> &mut Self {
        self.agent.on_summary(callback);
        self
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
        let agent =
            ReceptionistAgent::new(ReceptionistOptions::new(sample_departments()).name("test"));
        assert_eq!(agent.agent().service().name(), "test");
        assert_eq!(agent.agent().service().route(), "/receptionist");
        assert_eq!(agent.departments().len(), 2);
    }

    #[test]
    fn test_receptionist_has_tools() {
        let agent =
            ReceptionistAgent::new(ReceptionistOptions::new(sample_departments()).name("test"));
        let raw = serde_json::Map::new();

        let mut args = serde_json::Map::new();
        args.insert("caller_name".to_string(), json!("Alice"));
        args.insert("reason".to_string(), json!("Billing inquiry"));
        let result = agent
            .agent()
            .on_function_call("collect_caller_info", &args, Some(&raw));
        assert!(result.is_some());

        let mut args2 = serde_json::Map::new();
        args2.insert("department".to_string(), json!("Sales"));
        let result2 = agent
            .agent()
            .on_function_call("transfer_call", &args2, Some(&raw));
        assert!(result2.is_some());
    }

    #[test]
    fn test_receptionist_default_name() {
        let agent = ReceptionistAgent::new(ReceptionistOptions::new(sample_departments()).name(""));
        assert_eq!(agent.agent().service().name(), "receptionist");
    }

    #[test]
    fn test_receptionist_on_summary_fires() {
        use std::sync::{Arc, Mutex};

        let mut agent =
            ReceptionistAgent::new(ReceptionistOptions::new(sample_departments()).name("test"));
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_clone = Arc::clone(&captured);
        agent.on_summary(Box::new(move |summary, _data, _headers| {
            *captured_clone.lock().unwrap() = summary.to_string();
        }));

        let (user, pass) = agent.agent().service().basic_auth_credentials();
        let auth = {
            use base64::Engine;
            use base64::engine::general_purpose::STANDARD as BASE64;
            format!("Basic {}", BASE64.encode(format!("{user}:{pass}")))
        };
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), auth);

        let body = json!({"summary": "Routed to Sales"});
        let (status, _, _) = agent.agent_mut().handle_request(
            "POST",
            "/receptionist/post_prompt",
            &headers,
            Some(&body.to_string()),
        );
        assert_eq!(status, 200);
        assert_eq!(*captured.lock().unwrap(), "Routed to Sales");
    }
}
