/// Serverless adapter -- auto-detect runtime environment (Lambda, Azure,
/// server) and handle requests accordingly.

pub mod adapter;

pub use adapter::Adapter;
