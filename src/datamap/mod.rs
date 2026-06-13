// The `datamap` implementation module mirrors the Python file layout for 1:1
// traceability. It is private and `DataMap` is re-exported below, so consumers
// write `datamap::DataMap`, never `datamap::datamap::DataMap` — the public
// double-path module_inception guards against does not exist. (The lint still
// fires on the name match even for a private re-exported module, so allow it.)
#[allow(clippy::module_inception)]
mod datamap;

pub use datamap::DataMap;
