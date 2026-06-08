//! Language-specific detectors. Each module here implements a detector
//! that is grounded in a single source language by design (e.g. Rust
//! `cfg(...)` attribute reasoning) rather than a cross-cutting pattern
//! reusable across languages.

pub mod go_build_tag_interaction;
pub mod python_unreachable_except;
pub mod rust_config_interaction;
