//! REST module -- SignalWire REST API client, HTTP transport, and CRUD
//! resource helpers.

pub mod client;
pub mod crud_resource;
pub mod error;
pub mod generated_bases;
pub mod http_client;
pub mod namespaces;
pub mod pagination;

pub use client::RestClient;
pub use crud_resource::CrudResource;
pub use error::SignalWireRestError;
pub use http_client::HttpClient;
pub use pagination::PaginatedIterator;
