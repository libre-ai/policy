//! Pure eligibility engine: types and verdict logic, zero I/O.
//!
//! Compiled both natively (CLI, API) and to WASM (web UI local mode), so this
//! crate must stay free of platform-specific dependencies.
#![forbid(unsafe_code)]
