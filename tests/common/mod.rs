// Shared mocktest helpers for the rest_mock_* integration tests.
//
// Cargo treats every file under `tests/` as its own binary unless we route
// shared code through a `common/mod.rs`. Each integration test pulls this
// module in via `#[path = "common/mod.rs"] mod common;` so they all share
// the singleton mock-server harness from `mocktest`.

pub mod mocktest;
pub mod relay_mocktest;
pub mod tls_support;
