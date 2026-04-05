/// REST module -- SignalWire REST API client, HTTP transport, and CRUD
/// resource helpers.

pub mod error;
pub mod http_client;
pub mod crud_resource;
pub mod client;
pub mod namespaces;

pub use error::SignalWireRestError;
pub use http_client::HttpClient;
pub use crud_resource::CrudResource;
pub use client::RestClient;
