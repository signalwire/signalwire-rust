use std::collections::HashMap;
use std::fmt::Write as _;

use serde_json::{Map, Value, json};

use crate::agent::{AgentBase, AgentOptions};
use crate::prefabs::PrefabSummaryCallback;
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
    /// Create a new `ConciergeAgent`.
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
                    .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let amenities: HashMap<String, Value> = venue_info
            .get("amenities")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
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
                    .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let welcome_message = venue_info
            .get("welcome_message")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string);

        let agent_name = if name.is_empty() { "concierge" } else { name };

        let mut opts = AgentOptions::new(agent_name);
        opts.route = Some(route.unwrap_or("/concierge").to_string());
        opts.use_pom = true;

        let mut agent = AgentBase::new(opts);

        let welcome = welcome_message
            .clone()
            .unwrap_or_else(|| format!("Welcome to {venue_name}. How can I assist you today?"));

        // Global data
        agent.set_global_data(json!({
            "venue_name": venue_name,
            "services": services,
            "amenities": amenities,
        }));

        // Role section
        agent.prompt_add_section(
            "Concierge Role",
            &format!("You are the virtual concierge for {venue_name}. {welcome}"),
            vec![
                "Welcome users and explain available services",
                "Answer questions about amenities, hours, and directions",
                "Help with bookings and reservations",
                "Provide personalized recommendations",
            ],
        );

        // Services section
        if !services.is_empty() {
            let svc_refs: Vec<&str> = services.iter().map(std::string::String::as_str).collect();
            agent.prompt_add_section("Available Services", "", svc_refs);
        }

        // Amenities section
        if !amenities.is_empty() {
            let mut amenity_bullets: Vec<String> = Vec::new();
            for (amenity_name, info) in &amenities {
                let mut desc = amenity_name.clone();
                if let Some(hours) = info.get("hours").and_then(|v| v.as_str()) {
                    let _ = write!(desc, " - Hours: {hours}");
                }
                if let Some(location) = info.get("location").and_then(|v| v.as_str()) {
                    let _ = write!(desc, " - Location: {location}");
                }
                amenity_bullets.push(desc);
            }
            let bullet_refs: Vec<&str> = amenity_bullets
                .iter()
                .map(std::string::String::as_str)
                .collect();
            agent.prompt_add_section("Amenities", "", bullet_refs);
        }

        // Hours of operation section
        if !hours_of_operation.is_empty() {
            let mut hour_bullets: Vec<String> = Vec::new();
            for (day, hours) in &hours_of_operation {
                hour_bullets.push(format!("{day}: {hours}"));
            }
            let bullet_refs: Vec<&str> = hour_bullets
                .iter()
                .map(std::string::String::as_str)
                .collect();
            agent.prompt_add_section("Hours of Operation", "", bullet_refs);
        }

        // Special instructions section
        if !special_instructions.is_empty() {
            let bullet_refs: Vec<&str> = special_instructions
                .iter()
                .map(std::string::String::as_str)
                .collect();
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
                let service = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
                let date = args.get("date").and_then(|v| v.as_str()).unwrap_or("");
                let mut response = format!("Checking availability for {service} at {vn}");
                if !date.is_empty() {
                    let _ = write!(response, " on {date}");
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
                            "The {amenity_name} at {vn2} is located at: {location}"
                        ));
                    }
                }

                FunctionResult::with_response(&format!(
                    "Directions to {destination} at {vn2}: please ask the front desk for assistance."
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

    /// Check availability for a service on a specific date and time.
    ///
    /// Ported from Python `ConciergeAgent.check_availability`. Simulated: if the
    /// requested service is one of the venue's offered services it reports it as
    /// available, otherwise it lists the available services. `args` reads
    /// `service`, `date`, and `time`; `raw_data` is accepted for handler-signature
    /// parity but unused.
    pub fn check_availability(
        &self,
        args: &Map<String, Value>,
        _raw_data: &Map<String, Value>,
    ) -> FunctionResult {
        let service = args
            .get("service")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let date = args.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let time = args.get("time").and_then(|v| v.as_str()).unwrap_or("");

        let known = self.services.iter().any(|s| s.to_lowercase() == service);
        if known {
            FunctionResult::with_response(&format!(
                "Yes, {service} is available on {date} at {time}. Would you like to make a reservation?"
            ))
        } else {
            FunctionResult::with_response(&format!(
                "I'm sorry, we don't offer {service} at {}. Our available services are: {}.",
                self.venue_name,
                self.services.join(", ")
            ))
        }
    }

    /// Provide directions to a specific location or amenity.
    ///
    /// Ported from Python `ConciergeAgent.get_directions`. If the requested
    /// location matches an amenity that declares a `location`, it gives directions
    /// there; otherwise it defers to front-desk staff. `args` reads `location`;
    /// `raw_data` is accepted for handler-signature parity but unused.
    pub fn get_directions(
        &self,
        args: &Map<String, Value>,
        _raw_data: &Map<String, Value>,
    ) -> FunctionResult {
        let location = args
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(info) = self.amenities.get(&location)
            && let Some(amenity_location) = info.get("location").and_then(|v| v.as_str())
        {
            return FunctionResult::with_response(&format!(
                "The {location} is located at {amenity_location}. \
                 From the main entrance, follow the signs to {amenity_location}."
            ));
        }

        FunctionResult::with_response(&format!(
            "I don't have specific directions to {location}. \
             You can ask our staff at the front desk for assistance."
        ))
    }

    /// Register a callback that processes the interaction summary.
    ///
    /// Delegates to [`AgentBase::on_summary`], matching the Python
    /// `ConciergeAgent.on_summary` override point (which logs the summary).
    pub fn on_summary(&mut self, callback: PrefabSummaryCallback) -> &mut Self {
        self.agent.on_summary(callback);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_venue_info() -> Map<String, Value> {
        let mut info = Map::new();
        info.insert("venue_name".to_string(), json!("Grand Hotel"));
        info.insert(
            "services".to_string(),
            json!(["Room Service", "Spa", "Valet"]),
        );
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

    #[test]
    fn test_check_availability_known_service() {
        let info = sample_venue_info();
        let agent = ConciergeAgent::new("test", &info, None);
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("service".to_string(), json!("Spa"));
        args.insert("date".to_string(), json!("2026-01-01"));
        args.insert("time".to_string(), json!("14:00"));
        let json_str = agent.check_availability(&args, &raw).to_json();
        assert!(json_str.contains("available on 2026-01-01 at 14:00"));
    }

    #[test]
    fn test_check_availability_unknown_service() {
        let info = sample_venue_info();
        let agent = ConciergeAgent::new("test", &info, None);
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("service".to_string(), json!("Skydiving"));
        let json_str = agent.check_availability(&args, &raw).to_json();
        assert!(json_str.contains("we don't offer skydiving"));
        assert!(json_str.contains("Room Service"));
    }

    #[test]
    fn test_get_directions_known_amenity() {
        // Ported Python semantics: the lookup key is lowercased, so it matches
        // only amenities whose map key is itself lowercase.
        let mut info = Map::new();
        info.insert("venue_name".to_string(), json!("Grand Hotel"));
        info.insert(
            "amenities".to_string(),
            json!({"pool": {"hours": "6am-10pm", "location": "Floor 3"}}),
        );
        let agent = ConciergeAgent::new("test", &info, None);
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("location".to_string(), json!("Pool"));
        let json_str = agent.get_directions(&args, &raw).to_json();
        assert!(json_str.contains("Floor 3"));
        assert!(json_str.contains("follow the signs to Floor 3"));
    }

    #[test]
    fn test_get_directions_unknown_location() {
        let info = sample_venue_info();
        let agent = ConciergeAgent::new("test", &info, None);
        let raw = Map::new();
        let mut args = Map::new();
        args.insert("location".to_string(), json!("Rooftop"));
        let json_str = agent.get_directions(&args, &raw).to_json();
        assert!(json_str.contains("don't have specific directions to rooftop"));
    }

    #[test]
    fn test_concierge_on_summary_fires() {
        use std::sync::{Arc, Mutex};

        let info = sample_venue_info();
        let mut agent = ConciergeAgent::new("test", &info, None);

        let captured = Arc::new(Mutex::new(String::new()));
        let captured_clone = Arc::clone(&captured);
        agent.on_summary(Box::new(move |summary, _data, _headers| {
            *captured_clone.lock().unwrap() = summary.to_string();
        }));

        // Drive the real /post_prompt path so the delegated callback fires.
        let (user, pass) = agent.agent().service().basic_auth_credentials();
        let auth = {
            use base64::Engine;
            use base64::engine::general_purpose::STANDARD as BASE64;
            format!("Basic {}", BASE64.encode(format!("{user}:{pass}")))
        };
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), auth);

        let body = json!({"summary": "Great concierge call"});
        let (status, _, _) = agent.agent_mut().handle_request(
            "POST",
            "/concierge/post_prompt",
            &headers,
            &body.to_string(),
        );
        assert_eq!(status, 200);
        assert_eq!(*captured.lock().unwrap(), "Great concierge call");
    }
}
