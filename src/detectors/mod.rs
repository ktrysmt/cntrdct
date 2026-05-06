//! Layer 1 deterministic detectors. Each submodule corresponds to one
//! detector previously shipped as its own crate (`cntrdct-detector-*`).

pub mod arg_swap;
pub mod clone_drift;
pub mod comment_code;
pub mod config_interaction;
pub mod pr_miner;
pub mod unreachable_after_terminator;
