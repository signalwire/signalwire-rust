use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Validate addresses and compute driving routes using Google Maps (DataMap-based).
pub struct GoogleMaps {
    sp: SkillParams,
}

impl GoogleMaps {
    pub fn new(params: Map<String, Value>) -> Self {
        GoogleMaps {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for GoogleMaps {
    fn name(&self) -> &str {
        "google_maps"
    }

    fn description(&self) -> &str {
        "Validate addresses and compute driving routes using Google Maps"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let api_key = self.sp.get_str_or("api_key", "");
        let lookup_tool = self.sp.get_str_or("lookup_tool_name", "lookup_address");
        let route_tool = self.sp.get_str_or("route_tool_name", "compute_route");

        // lookup_address — DataMap with Google Geocoding API
        let mut lookup_def = json!({
            "function": lookup_tool,
            "purpose": "Look up and validate an address using Google Maps Geocoding",
            "argument": {
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "The address to look up",
                    },
                    "bias_lat": {
                        "type": "number",
                        "description": "Latitude to bias results toward (optional)",
                    },
                    "bias_lng": {
                        "type": "number",
                        "description": "Longitude to bias results toward (optional)",
                    },
                },
                "required": ["address"],
            },
            "data_map": {
                "webhooks": [{
                    "url": format!(
                        "https://maps.googleapis.com/maps/api/geocode/json?address=${{enc:args.address}}&key={}",
                        api_key
                    ),
                    "method": "GET",
                    "output": {
                        "response": "Address found: ${results[0].formatted_address}. \
                                     Latitude: ${results[0].geometry.location.lat}, \
                                     Longitude: ${results[0].geometry.location.lng}",
                        "action": [{"say_it": true}],
                    },
                    "error_output": {
                        "response": "Unable to look up the address. Please check the address and try again.",
                        "action": [{"say_it": true}],
                    },
                }],
            },
        });

        // compute_route — DataMap with Google Routes API
        let mut route_def = json!({
            "function": route_tool,
            "purpose": "Compute a driving route between two locations using Google Maps",
            "argument": {
                "type": "object",
                "properties": {
                    "origin_lat": { "type": "number", "description": "Latitude of the origin" },
                    "origin_lng": { "type": "number", "description": "Longitude of the origin" },
                    "dest_lat": { "type": "number", "description": "Latitude of the destination" },
                    "dest_lng": { "type": "number", "description": "Longitude of the destination" },
                },
                "required": ["origin_lat", "origin_lng", "dest_lat", "dest_lng"],
            },
            "data_map": {
                "webhooks": [{
                    "url": "https://routes.googleapis.com/directions/v2:computeRoutes",
                    "method": "POST",
                    "headers": {
                        "X-Goog-Api-Key": api_key,
                        "X-Goog-FieldMask": "routes.duration,routes.distanceMeters,routes.legs",
                        "Content-Type": "application/json",
                    },
                    "body": {
                        "origin": {
                            "location": {
                                "latLng": {
                                    "latitude": "${args.origin_lat}",
                                    "longitude": "${args.origin_lng}",
                                }
                            }
                        },
                        "destination": {
                            "location": {
                                "latLng": {
                                    "latitude": "${args.dest_lat}",
                                    "longitude": "${args.dest_lng}",
                                }
                            }
                        },
                        "travelMode": "DRIVE",
                    },
                    "output": {
                        "response": "Route computed. Distance: ${routes[0].distanceMeters} meters, \
                                     Duration: ${routes[0].duration}",
                        "action": [{"say_it": true}],
                    },
                    "error_output": {
                        "response": "Unable to compute route between the specified locations.",
                        "action": [{"say_it": true}],
                    },
                }],
            },
        });

        let swaig_fields = self.get_swaig_fields();
        if !swaig_fields.is_empty() {
            if let Value::Object(ref mut obj) = lookup_def {
                for (k, v) in &swaig_fields {
                    obj.insert(k.clone(), v.clone());
                }
            }
            if let Value::Object(ref mut obj) = route_def {
                for (k, v) in swaig_fields {
                    obj.insert(k, v);
                }
            }
        }

        agent.register_swaig_function(lookup_def);
        agent.register_swaig_function(route_def);
    }

    fn get_hints(&self) -> Vec<String> {
        vec![
            "address".to_string(),
            "location".to_string(),
            "route".to_string(),
            "directions".to_string(),
            "miles".to_string(),
            "distance".to_string(),
        ]
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Google Maps",
            "body": "You can look up addresses and compute driving routes.",
            "bullets": [
                "Use lookup_address to validate and geocode an address.",
                "Use compute_route to get driving distance and duration between two coordinates.",
                "First look up addresses to get coordinates, then compute routes between them.",
            ],
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_maps_metadata() {
        let skill = GoogleMaps::new(Map::new());
        assert_eq!(skill.name(), "google_maps");
    }

    #[test]
    fn test_google_maps_setup_needs_key() {
        let mut skill = GoogleMaps::new(Map::new());
        assert!(!skill.setup());
    }

    #[test]
    fn test_google_maps_hints() {
        let skill = GoogleMaps::new(Map::new());
        let hints = skill.get_hints();
        assert!(hints.contains(&"address".to_string()));
        assert!(hints.contains(&"directions".to_string()));
    }
}
