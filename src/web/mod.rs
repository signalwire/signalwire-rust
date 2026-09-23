//! Static file-serving web service.
//!
//! Matches `signalwire.web` package. Currently houses
//! [`WebService`], a static file-serving service with an HTTP API.

pub mod web_service;

pub use web_service::{WebService, WebServiceOptions};
