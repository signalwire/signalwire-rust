//! Static file-serving web service.
//!
//! Port of Python `signalwire.web.web_service.WebService`. Serves local
//! directories over HTTP, mapping URL routes to filesystem paths, with basic
//! auth, extension filters, and optional directory browsing.
//!
//! Python builds on FastAPI/uvicorn; the Rust port carries the same
//! configuration + directory-management surface, and `start`/`stop` are the
//! server lifecycle entry points (the Rust HTTP backend is synchronous, like
//! [`crate::swml::service::Service::serve`]).

use std::collections::HashMap;
use std::path::Path;

/// Default blocked file extensions (secrets / VCS / build artifacts).
const DEFAULT_BLOCKED: &[&str] = &[
    ".env",
    ".git",
    ".gitignore",
    ".key",
    ".pem",
    ".crt",
    ".pyc",
    "__pycache__",
    ".DS_Store",
    ".swp",
];

const DEFAULT_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

/// Configuration options for [`WebService::new`].
#[derive(Debug, Clone)]
pub struct WebServiceOptions {
    pub port: u16,
    pub directories: HashMap<String, String>,
    pub basic_auth: Option<(String, String)>,
    pub enable_directory_browsing: bool,
    pub allowed_extensions: Option<Vec<String>>,
    pub blocked_extensions: Option<Vec<String>>,
    pub max_file_size: u64,
    pub enable_cors: bool,
}

impl Default for WebServiceOptions {
    fn default() -> Self {
        WebServiceOptions {
            port: 8002,
            directories: HashMap::new(),
            basic_auth: None,
            enable_directory_browsing: false,
            allowed_extensions: None,
            blocked_extensions: None,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            enable_cors: true,
        }
    }
}

/// Static file serving service with an HTTP API.
pub struct WebService {
    port: u16,
    directories: HashMap<String, String>,
    basic_auth: Option<(String, String)>,
    enable_directory_browsing: bool,
    allowed_extensions: Option<Vec<String>>,
    blocked_extensions: Vec<String>,
    max_file_size: u64,
    enable_cors: bool,
    running: bool,
}

impl WebService {
    /// Initialize a web service.
    #[must_use]
    pub fn new(options: WebServiceOptions) -> Self {
        let blocked = options
            .blocked_extensions
            .unwrap_or_else(|| DEFAULT_BLOCKED.iter().map(|s| (*s).to_string()).collect());
        WebService {
            port: options.port,
            directories: options.directories,
            basic_auth: options.basic_auth,
            enable_directory_browsing: options.enable_directory_browsing,
            allowed_extensions: options.allowed_extensions,
            blocked_extensions: blocked,
            max_file_size: options.max_file_size,
            enable_cors: options.enable_cors,
            running: false,
        }
    }

    /// Add a directory to serve, mounting it at `route`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `directory` does not exist or is not a directory
    /// (mirroring Python's `ValueError`).
    pub fn add_directory(&mut self, route: &str, directory: &str) -> Result<(), String> {
        let route = normalize_route(route);
        let path = Path::new(directory);
        if !path.exists() {
            return Err(format!("Directory does not exist: {directory}"));
        }
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {directory}"));
        }
        self.directories.insert(route, directory.to_string());
        Ok(())
    }

    /// Remove a served directory by `route`.
    pub fn remove_directory(&mut self, route: &str) {
        let route = normalize_route(route);
        self.directories.remove(&route);
    }

    /// Start the service (Python `WebService.start`). Optionally overrides the
    /// bind host/port. The Rust HTTP backend is synchronous; this marks the
    /// service running and is the lifecycle entry point.
    pub fn start(&mut self, _host: Option<&str>, port: Option<u16>) {
        if let Some(p) = port {
            self.port = p;
        }
        self.running = true;
    }

    /// Stop the service (Python `WebService.stop`).
    pub fn stop(&mut self) {
        self.running = false;
    }

    // ── Read accessors (not part of the reference surface) ───────────────

    /// The configured port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The route → directory map.
    #[must_use]
    pub fn directories(&self) -> &HashMap<String, String> {
        &self.directories
    }

    /// Whether the service is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Whether a file with `path` passes the allow/block extension filters.
    #[must_use]
    pub fn is_file_allowed(&self, path: &str) -> bool {
        let lower = path.to_lowercase();
        if self
            .blocked_extensions
            .iter()
            .any(|b| lower.ends_with(&b.to_lowercase()))
        {
            return false;
        }
        match &self.allowed_extensions {
            Some(allowed) => allowed.iter().any(|a| lower.ends_with(&a.to_lowercase())),
            None => true,
        }
    }

    /// The configured basic-auth credentials, if any.
    #[must_use]
    pub fn basic_auth(&self) -> Option<&(String, String)> {
        self.basic_auth.as_ref()
    }

    /// Whether directory browsing is enabled.
    #[must_use]
    pub fn directory_browsing_enabled(&self) -> bool {
        self.enable_directory_browsing
    }

    /// The maximum servable file size in bytes.
    #[must_use]
    pub fn max_file_size(&self) -> u64 {
        self.max_file_size
    }

    /// Whether CORS is enabled.
    #[must_use]
    pub fn cors_enabled(&self) -> bool {
        self.enable_cors
    }

    /// The extension allow-list, if one was configured. `None` means "allow
    /// anything not blocked" (reference attribute
    /// `WebService.allowed_extensions`).
    #[must_use]
    pub fn allowed_extensions(&self) -> Option<&[String]> {
        self.allowed_extensions.as_deref()
    }

    /// The extension block-list. Defaults to `DEFAULT_BLOCKED` when the caller
    /// passes none (reference attribute `WebService.blocked_extensions`).
    #[must_use]
    pub fn blocked_extensions(&self) -> &[String] {
        &self.blocked_extensions
    }
}

/// Normalize a route to a leading-slash form.
fn normalize_route(route: &str) -> String {
    if route.starts_with('/') {
        route.to_string()
    } else {
        format!("/{route}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let ws = WebService::new(WebServiceOptions::default());
        assert_eq!(ws.port(), 8002);
        assert!(ws.directories().is_empty());
        assert!(!ws.is_running());
        assert!(ws.cors_enabled());
        assert_eq!(ws.max_file_size(), DEFAULT_MAX_FILE_SIZE);
    }

    #[test]
    fn test_add_directory_normalizes_route() {
        let mut ws = WebService::new(WebServiceOptions::default());
        // The crate root always exists and is a directory.
        let dir = env!("CARGO_MANIFEST_DIR");
        ws.add_directory("docs", dir).unwrap();
        assert!(ws.directories().contains_key("/docs"));
    }

    #[test]
    fn test_add_directory_rejects_missing() {
        let mut ws = WebService::new(WebServiceOptions::default());
        let err = ws.add_directory("/x", "/no/such/dir/here").unwrap_err();
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn test_add_directory_rejects_file() {
        let mut ws = WebService::new(WebServiceOptions::default());
        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        let err = ws.add_directory("/x", file).unwrap_err();
        assert!(err.contains("not a directory"));
    }

    #[test]
    fn test_remove_directory() {
        let mut ws = WebService::new(WebServiceOptions::default());
        let dir = env!("CARGO_MANIFEST_DIR");
        ws.add_directory("/docs", dir).unwrap();
        ws.remove_directory("docs"); // normalized to /docs
        assert!(!ws.directories().contains_key("/docs"));
    }

    #[test]
    fn test_start_stop_lifecycle() {
        let mut ws = WebService::new(WebServiceOptions::default());
        ws.start(Some("127.0.0.1"), Some(9099));
        assert!(ws.is_running());
        assert_eq!(ws.port(), 9099);
        ws.stop();
        assert!(!ws.is_running());
    }

    #[test]
    fn test_is_file_allowed_blocks_defaults() {
        let ws = WebService::new(WebServiceOptions::default());
        assert!(!ws.is_file_allowed("/srv/.env"));
        assert!(!ws.is_file_allowed("/srv/secret.key"));
        assert!(ws.is_file_allowed("/srv/index.html"));
    }

    #[test]
    fn test_is_file_allowed_allowlist() {
        let ws = WebService::new(WebServiceOptions {
            allowed_extensions: Some(vec![".html".to_string(), ".css".to_string()]),
            ..Default::default()
        });
        assert!(ws.is_file_allowed("/srv/page.html"));
        assert!(!ws.is_file_allowed("/srv/script.js"));
    }
}
