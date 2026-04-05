use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::agent::{AgentBase, AgentOptions};
use crate::swaig::FunctionResult;

/// A pre-built concierge agent for venues — answers questions about services,
/// amenities, hours, and provides directions.
pub struct ConciergeAgent {
    agent: AgentBase,
    venue_name: String,
    services: Vec<String>,
    amenities: HashMap<String, Value>,
}

impl ConciergeAgent {
    /// Create a new ConciergeAgent.
    ///
    /// # Arguments
    /// - `name` — agent name (defaults to `"concierge"` if empty).
    /// - `venue_info` — map with `venue_name` (required), plus optional `services`,
    ///   `amenities`, `hours_of_operation`, `special_instructions`, `welcome_message`.
    /// - `route` — optional route (defaults to `"/concierge"`).
    pub fn new(name: &str, venue_info: &Map<String, Value>, route: Option<&str>) -> Self {
        let venue_name = venue_info
            .get("venue_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Venue")
            .to_string();

        let services: Vec<String> = venue_info
            .get("services")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let amenities: HashMap<String, Value> = venue_info
            .get("amenities")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let hours_of_operation: HashMap<String, String> = venue_info
            .get("hours_of_operation")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let special_instructions: Vec<String> = venue_info
            .get("special_instructions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let welcome_message = venue_info
            .get("welcome_message")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let agent_name = if name.is_empty() {
            "concierge"
        } else {
            name
        };

        let mut opts = AgentOptions::new(agent_name);
        opts.route = Some(route.unwrap_or("/concierge").to_string());
        opts.use_pom = true;

        let mut agent = AgentBase::new(opts);

        let welcome = welcome_message
            .clone()
            .unwrap_or_else(|| format!("Welcome to {}. How can I assist you today?", venue_name));

        // Global data
        agent.set_global_data(json!({
            "venue_name": venue_name,
            "services": services,
            "amenities": amenities,
        }));

        // Role section
        agent.prompt_add_section(
            "Concierge Role",
            &format!(
                "You are the virtual concierge for {}. {}",
                venue_name, welcome
            ),
            vec![
                "Welcome users and explain available services",
                "Answer questions about amenities, hours, and directions",
                "Help with bookings and reservations",
                "Provide personalized recommendations",
            ],
        );

        // Services section
        if !services.is_empty() {
            let svc_refs: Vec<&str> = services.iter().map(|s| s.as_str()).collect();
            agent.prompt_add_section("Available Services", "", svc_refs);
        }

        // Amenities section
        if !amenities.is_empty() {
            let mut amenity_bullets: Vec<String> = Vec::new();
            for (amenity_name, info) in &amenities {
                let mut desc = amenity_name.clone();
                if let Some(hours) = info.get("hours").and_then(|v| v.as_str()) {
                    desc.push_str(&format!(" - Hours: {}", hours));
                }
                if let Some(location) = info.get("location").and_then(|v| v.as_str()) {
                    desc.push_str(&format!(" - Location: {}", location));
                }
                amenity_bullets.push(desc);
            }
            let bullet_refs: Vec<&str> = amenity_bullets.iter().map(|s| s.as_str()).collect();
            agent.prompt_add_section("Amenities", "", bullet_refs);
        }

        // Hours of operation section
        if !hours_of_operation.is_empty() {
            let mut hour_bullets: Vec<String> = Vec::new();
            for (day, hours) in &hours_of_operation {
                hour_bullets.push(format!("{}: {}", day, hours));
            }
            let bullet_refs: Vec<&str> = hour_bullets.iter().map(|s| s.as_str()).collect();
            agent.prompt_add_section("Hours of Operation", "", bullet_refs);
        }

        // Special instructions section
        if !special_instructions.is_empty() {
            let bullet_refs: Vec<&str> =
                special_instructions.iter().map(|s| s.as_str()).collect();
            agent.prompt_add_section("Special Instructions", "", bullet_refs);
        }

        // Tool: check_availability
        let vn = venue_name.clone();
        agent.define_tool(
            "check_availability",
            "Check availability for a service or amenity",
            json!({
                "service": {"type": "string", "description": "Service or amenity to check"},
                "date": {"type": "string", "description": "Date to check (optional)"},
            }),
            Box::new(move |args, _raw| {
                let service = args
                    .get("service")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let date = args
                    .get("date")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mut response = format!("Checking availability for {} at {}", service, vn);
                if !date.is_empty() {
                    response.push_str(&format!(" on {}", date));
                }
                FunctionResult::with_response(&response)
            }),
            false,
        );

        // Tool: get_directions
        let vn2 = venue_name.clone();
        let amenities_clone: HashMap<String, Value> = amenities.clone();
        agent.define_tool(
            "get_directions",
            "Get directions to a service or amenity within the venue",
            json!({
                "destination": {"type": "string", "description": "The amenity or area to get directions to"},
            }),
            Box::new(move |args, _raw| {
                let destination = args
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let dest_lower = destination.to_lowercase();

                for (amenity_name, info) in &amenities_clone {
                    if amenity_name.to_lowercase() == dest_lower {
                        let location = info
                            .get("location")
                            .and_then(|v| v.as_str())
                            .unwrap_or("location not specified");
                        return FunctionResult::with_response(&format!(
                            "The {} at {} is located at: {}",
                            amenity_name, vn2, location
                        ));
                    }
                }

                FunctionResult::with_response(&format!(
                    "Directions to {} at {}: please ask the front desk for assistance.",
                    destination, vn2
                ))
            }),
            false,
        );

        ConciergeAgent {
            agent,
            venue_name,
            services,
            amenities,
        }
    }

    pub fn agent(&self) -> &AgentBase {
        &self.agent
    }

    pub fn agent_mut(&mut self) -> &mut AgentBase {
        &mut self.agent
    }

    pub fn venue_name(&self) -> &str {
        &self.venue_name
    }

    pub fn services(&self) -> &[String] {
        &self.services
    }

    pub fn amenities(&self) -> &HashMap<String, Value> {
        &self.amenities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_venue_info() -> Map<String, Value> {
        let mut info = Map::new();
        info.insert("venue_name".to_string(), json!("Grand Hotel"));
        info.insert("services".to_string(), json!(["Room Service", "Spa", "Valet"]));
        info.insert(
            "amenities".to_string(),
            json!({
                "Pool": {"hours": "6am-10pm", "location": "Floor 3"},
                "Gym": {"hours": "24 hours", "location": "Floor 2"},
            }),
        );
        info.insert(
            "hours_of_operation".to_string(),
            json!({
                "Monday-Friday": "24 hours",
                "Saturday-Sunday": "24 hours",
            }),
        );
        info
    }

    #[test]
    fn test_concierge_construction() {
        let info = sample_venue_info();
        let agent = ConciergeAgent::new("test", &info, None);
        assert_eq!(agent.agent().service().name(), "test");
        assert_eq!(agent.agent().service().route(), "/concierge");
        assert_eq!(agent.venue_name(), "Grand Hotel");
        assert_eq!(agent.services().len(), 3);
        assert_eq!(agent.amenities().len(), 2);
    }

    #[test]
    fn test_concierge_has_tools() {
        let info = sample_venue_info();
        let agent = ConciergeAgent::new("test", &info, None);
        let raw = serde_json::Map::new();

        let mut args = serde_json::Map::new();
        args.insert("service".to_string(), json!("Spa"));
        let result = agent
            .agent()
            .on_function_call("check_availability", &args, &raw);
        assert!(result.is_some());

        let mut args2 = serde_json::Map::new();
        args2.insert("destination".to_string(), json!("Pool"));
        let result2 = agent
            .agent()
            .on_function_call("get_directions", &args2, &raw);
        assert!(result2.is_some());
        let json_str = result2.unwrap().to_json();
        assert!(json_str.contains("Floor 3"));
    }

    #[test]
    fn test_concierge_default_name() {
        let info = sample_venue_info();
        let agent = ConciergeAgent::new("", &info, None);
        assert_eq!(agent.agent().service().name(), "concierge");
    }
}
