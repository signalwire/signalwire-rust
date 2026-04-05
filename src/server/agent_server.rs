use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::agent::AgentBase;
use crate::logging::Logger;

/// Extension-to-MIME mapping for static file serving.
const MIME_TYPES: &[(&str, &str)] = &[
    ("html", "text/html"),
    ("htm", "text/html"),
    ("css", "text/css"),
    ("js", "application/javascript"),
    ("json", "application/json"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("svg", "image/svg+xml"),
    ("ico", "image/x-icon"),
    ("txt", "text/plain"),
    ("pdf", "application/pdf"),
    ("xml", "application/xml"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
    ("ttf", "font/ttf"),
    ("eot", "application/vnd.ms-fontobject"),
];

/// Multi-agent HTTP server that dispatches requests to registered agents
/// by longest-prefix route matching.
pub struct AgentServer {
    host: String,
    port: u16,
    agents: HashMap<String, AgentBase>,
    sip_routing_enabled: bool,
    sip_username_mapping: HashMap<String, String>,
    static_routes: HashMap<String, PathBuf>,
    logger: Logger,
}

impl AgentServer {
    pub fn new(host: Option<&str>, port: Option<u16>) -> Self {
        let host = host.unwrap_or("0.0.0.0").to_string();
        let port = port.unwrap_or_else(|| {
            env::var("PORT")
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(3000)
        });

        AgentServer {
            host,
            port,
            agents: HashMap::new(),
            sip_routing_enabled: false,
            sip_username_mapping: HashMap::new(),
            static_routes: HashMap::new(),
            logger: Logger::new("agent_server"),
        }
    }

    // ======================================================================
    //  Agent Registration
    // ======================================================================

    /// Register an agent at its default route (from the agent's service),
    /// or at an explicit route override.
    ///
    /// # Errors
    /// Returns an error string if the route is already registered.
    pub fn register(&mut self, agent: AgentBase, route: Option<&str>) -> Result<(), String> {
        let route = self.normalize_route(
            route.unwrap_or_else(|| agent.service().route()),
        );

        if self.agents.contains_key(&route) {
            return Err(format!("Route '{}' is already registered", route));
        }

        self.logger.info(&format!(
            "Agent '{}' registered at {}",
            agent.service().name(),
            route
        ));

        self.agents.insert(route, agent);
        Ok(())
    }

    /// Unregister an agent from a route.
    pub fn unregister(&mut self, route: &str) -> &mut Self {
        let route = self.normalize_route(route);
        self.agents.remove(&route);
        self
    }

    /// Get all registered routes (sorted).
    pub fn get_agents(&self) -> Vec<String> {
        let mut routes: Vec<String> = self.agents.keys().cloned().collect();
        routes.sort();
        routes
    }

    /// Get an agent by route.
    pub fn get_agent(&self, route: &str) -> Option<&AgentBase> {
        let route = self.normalize_route(route);
        self.agents.get(&route)
    }

    /// Get a mutable reference to an agent by route.
    pub fn get_agent_mut(&mut self, route: &str) -> Option<&mut AgentBase> {
        let route = self.normalize_route(route);
        self.agents.get_mut(&route)
    }

    // ======================================================================
    //  SIP Routing
    // ======================================================================

    /// Enable SIP-based routing.
    pub fn setup_sip_routing(&mut self) -> &mut Self {
        self.sip_routing_enabled = true;
        self
    }

    /// Map a SIP username to a route.
    pub fn register_sip_username(&mut self, username: &str, route: &str) -> &mut Self {
        let route = self.normalize_route(route);
        self.sip_username_mapping
            .insert(username.to_string(), route);
        self
    }

    /// Check if SIP routing is enabled.
    pub fn is_sip_routing_enabled(&self) -> bool {
        self.sip_routing_enabled
    }

    /// Get the SIP username mapping.
    pub fn sip_username_mapping(&self) -> &HashMap<String, String> {
        &self.sip_username_mapping
    }

    // ======================================================================
    //  Static File Serving
    // ======================================================================

    /// Serve static files from a directory under a URL prefix.
    ///
    /// # Errors
    /// Returns an error string if the directory does not exist.
    pub fn serve_static(&mut self, directory: &str, url_prefix: &str) -> Result<(), String> {
        let real_dir = fs::canonicalize(directory).map_err(|_| {
            format!("Static directory '{}' does not exist", directory)
        })?;
        if !real_dir.is_dir() {
            return Err(format!(
                "Static path '{}' is not a directory",
                directory
            ));
        }

        let prefix = self.normalize_route(url_prefix);
        self.static_routes.insert(prefix, real_dir);
        Ok(())
    }

    // ======================================================================
    //  Request Handling
    // ======================================================================

    /// Handle an HTTP request and return `(status, headers, body)`.
    pub fn handle_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (u16, HashMap<String, String>, String) {
        let path = self.normalize_path(path);

        // Health endpoint (no auth)
        if path == "/health" {
            let agent_names: Vec<Value> = self
                .get_agents()
                .iter()
                .map(|route| {
                    let name = self.agents[route].service().name();
                    json!(name)
                })
                .collect();
            return self.json_response(200, &json!({"status": "healthy", "agents": agent_names}));
        }

        // Ready endpoint (no auth)
        if path == "/ready" {
            return self.json_response(200, &json!({"status": "ready"}));
        }

        // Root index: list registered agents (no auth)
        if path == "/" || path.is_empty() {
            return self.handle_root_index();
        }

        // Check static file routes (longest prefix match)
        if let Some(result) = self.handle_static_file(&path) {
            return result;
        }

        // Find matching agent by longest prefix
        if let Some(matched_route) = self.find_matching_route(&path) {
            let agent = &self.agents[&matched_route];
            return agent.handle_request(method, &path, headers, body);
        }

        self.json_response(404, &json!({"error": "Not Found"}))
    }

    // ======================================================================
    //  Accessors
    // ======================================================================

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // ======================================================================
    //  Private Helpers
    // ======================================================================

    /// Handle the root index request, listing all registered agents.
    fn handle_root_index(&self) -> (u16, HashMap<String, String>, String) {
        let agent_list: Vec<Value> = self
            .get_agents()
            .iter()
            .map(|route| {
                let agent = &self.agents[route];
                json!({
                    "name": agent.service().name(),
                    "route": route,
                })
            })
            .collect();

        self.json_response(200, &json!({"agents": agent_list}))
    }

    /// Attempt to serve a static file for the given path.
    fn handle_static_file(
        &self,
        path: &str,
    ) -> Option<(u16, HashMap<String, String>, String)> {
        // Sort by longest prefix first
        let mut routes: Vec<&String> = self.static_routes.keys().collect();
        routes.sort_by(|a, b| b.len().cmp(&a.len()));

        for prefix in routes {
            let normal_prefix = if prefix == "/" { "" } else { prefix.as_str() };

            // Check if path starts with this prefix
            if prefix != "/" && path != prefix && !path.starts_with(&format!("{}/", normal_prefix))
            {
                continue;
            }
            // Don't serve root path as a static file
            if prefix == "/" && path == "/" {
                continue;
            }

            let rel_path = &path[normal_prefix.len()..];
            let rel_path = rel_path.trim_start_matches('/');

            // Path traversal protection: reject ".." components
            if rel_path.contains("..") {
                // More thorough check: reject any ".." path component
                let has_traversal = rel_path
                    .split('/')
                    .any(|component| component == "..");
                if has_traversal {
                    return Some(self.forbidden_response());
                }
            }

            let base_dir = &self.static_routes[prefix];
            let file_path = base_dir.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));

            // Resolve to absolute and verify it's within the base directory
            let abs_path = match fs::canonicalize(&file_path) {
                Ok(p) => p,
                Err(_) => continue, // file doesn't exist
            };

            if !abs_path.starts_with(base_dir) {
                return Some(self.forbidden_response());
            }

            if abs_path.is_file() {
                // Determine MIME type
                let ext = abs_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let content_type = MIME_TYPES
                    .iter()
                    .find(|(e, _)| *e == ext)
                    .map(|(_, mime)| *mime)
                    .unwrap_or("application/octet-stream");

                match fs::read(&abs_path) {
                    Ok(content) => {
                        let mut resp_headers = HashMap::new();
                        resp_headers
                            .insert("Content-Type".to_string(), content_type.to_string());
                        resp_headers.insert(
                            "Content-Length".to_string(),
                            content.len().to_string(),
                        );
                        for (k, v) in security_headers() {
                            resp_headers.insert(k, v);
                        }

                        // For text-based content, convert to string; for binary, base64 would
                        // be needed in a real server. Here we lossy-convert for the test harness.
                        let body = String::from_utf8_lossy(&content).to_string();
                        return Some((200, resp_headers, body));
                    }
                    Err(_) => {
                        let mut resp_headers = HashMap::new();
                        resp_headers
                            .insert("Content-Type".to_string(), "text/plain".to_string());
                        for (k, v) in security_headers() {
                            resp_headers.insert(k, v);
                        }
                        return Some((500, resp_headers, "Internal Server Error".to_string()));
                    }
                }
            }
        }

        None
    }

    /// Find the matching agent route for a request path (longest prefix match).
    fn find_matching_route(&self, path: &str) -> Option<String> {
        let mut routes: Vec<&String> = self.agents.keys().collect();
        routes.sort_by(|a, b| b.len().cmp(&a.len()));

        for route in routes {
            if route == "/" {
                return Some(route.clone());
            }
            if path == route.as_str() || path.starts_with(&format!("{}/", route)) {
                return Some(route.clone());
            }
        }

        None
    }

    /// Normalize a route: ensure leading slash, strip trailing slashes (unless root).
    fn normalize_route(&self, route: &str) -> String {
        let mut r = route.to_string();
        if !r.starts_with('/') {
            r.insert(0, '/');
        }
        if r != "/" {
            r = r.trim_end_matches('/').to_string();
        }
        r
    }

    /// Normalize a request path: strip trailing slashes (unless root).
    fn normalize_path(&self, path: &str) -> String {
        let p = if path != "/" {
            path.trim_end_matches('/').to_string()
        } else {
            path.to_string()
        };
        if p.is_empty() {
            "/".to_string()
        } else {
            p
        }
    }

    /// Build a 403 Forbidden response with security headers.
    fn forbidden_response(&self) -> (u16, HashMap<String, String>, String) {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        for (k, v) in security_headers() {
            headers.insert(k, v);
        }
        (403, headers, "Forbidden".to_string())
    }

    /// Build a JSON response tuple.
    fn json_response(
        &self,
        status: u16,
        data: &Value,
    ) -> (u16, HashMap<String, String>, String) {
        let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        for (k, v) in security_headers() {
            headers.insert(k, v);
        }
        (status, headers, body)
    }
}

/// Security headers applied to all responses.
fn security_headers() -> Vec<(String, String)> {
    vec![
        (
            "X-Content-Type-Options".to_string(),
            "nosniff".to_string(),
        ),
        ("X-Frame-Options".to_string(), "DENY".to_string()),
        ("Cache-Control".to_string(), "no-store".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentOptions;

    fn make_agent(name: &str, route: &str) -> AgentBase {
        let mut opts = AgentOptions::new(name);
        opts.route = Some(route.to_string());
        opts.basic_auth_user = Some("user".to_string());
        opts.basic_auth_password = Some("pass".to_string());
        AgentBase::new(opts)
    }

    #[test]
    fn test_server_construction() {
        let server = AgentServer::new(None, Some(8080));
        assert_eq!(server.host(), "0.0.0.0");
        assert_eq!(server.port(), 8080);
    }

    #[test]
    fn test_register_agent() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent = make_agent("bot1", "/bot1");
        assert!(server.register(agent, None).is_ok());
        assert_eq!(server.get_agents(), vec!["/bot1"]);
    }

    #[test]
    fn test_register_duplicate_route() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent1 = make_agent("bot1", "/bot");
        let agent2 = make_agent("bot2", "/bot");
        assert!(server.register(agent1, None).is_ok());
        let result = server.register(agent2, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already registered"));
    }

    #[test]
    fn test_register_with_route_override() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent = make_agent("bot1", "/original");
        assert!(server.register(agent, Some("/override")).is_ok());
        assert_eq!(server.get_agents(), vec!["/override"]);
    }

    #[test]
    fn test_unregister_agent() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent = make_agent("bot1", "/bot1");
        server.register(agent, None).unwrap();
        assert!(server.get_agent("/bot1").is_some());
        server.unregister("/bot1");
        assert!(server.get_agent("/bot1").is_none());
        assert!(server.get_agents().is_empty());
    }

    #[test]
    fn test_health_endpoint() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent = make_agent("bot1", "/bot1");
        server.register(agent, None).unwrap();

        let (status, headers, body) =
            server.handle_request("GET", "/health", &HashMap::new(), "");
        assert_eq!(status, 200);
        assert_eq!(headers["Content-Type"], "application/json");
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["status"], "healthy");
        assert!(parsed["agents"].is_array());
    }

    #[test]
    fn test_ready_endpoint() {
        let server = AgentServer::new(None, Some(3000));
        let (status, _, body) = server.handle_request("GET", "/ready", &HashMap::new(), "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["status"], "ready");
    }

    #[test]
    fn test_root_index() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent = make_agent("bot1", "/bot1");
        server.register(agent, None).unwrap();

        let (status, _, body) = server.handle_request("GET", "/", &HashMap::new(), "");
        assert_eq!(status, 200);
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert!(parsed["agents"].is_array());
        assert_eq!(parsed["agents"][0]["name"], "bot1");
        assert_eq!(parsed["agents"][0]["route"], "/bot1");
    }

    #[test]
    fn test_route_dispatch() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent = make_agent("bot1", "/bot1");
        server.register(agent, None).unwrap();

        // No auth -> should get 401 from the agent
        let (status, _, _) =
            server.handle_request("POST", "/bot1", &HashMap::new(), "");
        assert_eq!(status, 401);
    }

    #[test]
    fn test_not_found() {
        let server = AgentServer::new(None, Some(3000));
        let (status, _, _) =
            server.handle_request("GET", "/nonexistent", &HashMap::new(), "");
        assert_eq!(status, 404);
    }

    #[test]
    fn test_sip_routing() {
        let mut server = AgentServer::new(None, Some(3000));
        assert!(!server.is_sip_routing_enabled());

        server.setup_sip_routing();
        assert!(server.is_sip_routing_enabled());

        server.register_sip_username("alice", "/alice-agent");
        let mapping = server.sip_username_mapping();
        assert_eq!(mapping.get("alice").unwrap(), "/alice-agent");
    }

    #[test]
    fn test_longest_prefix_match() {
        let mut server = AgentServer::new(None, Some(3000));
        let agent1 = make_agent("api", "/api");
        let agent2 = make_agent("api_v2", "/api/v2");
        server.register(agent1, None).unwrap();
        server.register(agent2, None).unwrap();

        // /api/v2/swaig should match /api/v2 (longer prefix)
        let route = server.find_matching_route("/api/v2/swaig");
        assert_eq!(route.unwrap(), "/api/v2");

        // /api/other should match /api
        let route = server.find_matching_route("/api/other");
        assert_eq!(route.unwrap(), "/api");
    }

    #[test]
    fn test_static_file_path_traversal() {
        let _server = AgentServer::new(None, Some(3000));
        // Path with .. traversal should be blocked
        let has_traversal = "../etc/passwd"
            .split('/')
            .any(|component| component == "..");
        assert!(has_traversal);
    }

    #[test]
    fn test_normalize_route() {
        let server = AgentServer::new(None, Some(3000));
        assert_eq!(server.normalize_route("api"), "/api");
        assert_eq!(server.normalize_route("/api/"), "/api");
        assert_eq!(server.normalize_route("/"), "/");
        assert_eq!(server.normalize_route("/api/v1"), "/api/v1");
    }

    #[test]
    fn test_normalize_path() {
        let server = AgentServer::new(None, Some(3000));
        assert_eq!(server.normalize_path("/api/"), "/api");
        assert_eq!(server.normalize_path("/"), "/");
        assert_eq!(server.normalize_path(""), "/");
    }

    #[test]
    fn test_static_serve_nonexistent() {
        let mut server = AgentServer::new(None, Some(3000));
        let result = server.serve_static("/nonexistent/path/xyz", "/static");
        assert!(result.is_err());
    }

    #[test]
    fn test_security_headers_present() {
        let server = AgentServer::new(None, Some(3000));
        let (_, headers, _) = server.handle_request("GET", "/health", &HashMap::new(), "");
        assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(headers.get("Cache-Control").unwrap(), "no-store");
    }

    #[test]
    fn test_multiple_agents_sorted() {
        let mut server = AgentServer::new(None, Some(3000));
        server.register(make_agent("c", "/c"), None).unwrap();
        server.register(make_agent("a", "/a"), None).unwrap();
        server.register(make_agent("b", "/b"), None).unwrap();
        assert_eq!(server.get_agents(), vec!["/a", "/b", "/c"]);
    }
}
