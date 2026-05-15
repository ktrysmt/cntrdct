// Source: https://github.com/boundless-xyz/boundless/blob/1a2770b2d824df7d931f3fdf3907ae1633f9bc80/crates/executor/src/backends/mod.rs
// License: Apache-2.0
// Note: minimal extract of one top-level `pub fn default_registry() -> Registry` item from boundless-xyz/boundless@1a2770b2d824df7d931f3fdf3907ae1633f9bc80 crates/executor/src/backends/mod.rs (upstream line 44, corpus line 10 after the 3-line provenance header + 1 blank-line offset). After `docs/spec/comment-code-v0.md` F2 rendering (each `///` line's prefix stripped, lines joined with `\n`, result case-folded) the doc text contains the substring `may fail`, which is one of the six Pattern A trigger phrases (`returns err`, `returns result`, `may fail`, `fallible`, `returns option`, `may return none`) enumerated in spec F3. The function signature `() -> Registry` lacks the `Result` / `Option` substring required by F3's return-type negation, so the doc claim of fallibility is not propagated to the caller through the type system — the body absorbs constructor failures via `tracing::warn!` and returns a partially-populated `Registry`, which is the textbook Tan SOSP 2007 §3.1 Pattern A ("Description Comments that describe what the function returns") bug shape: a reviewer reading just the signature would not anticipate the silent partial-failure absorption. The companion claim "on failure the backend is omitted and a warning is logged so the rest of the service can still start" documents the absorb-and-log behaviour but does not move the failure information into the return type, so callers still cannot distinguish a fully-populated `Registry` from a partially-populated one programmatically. Diversifies `comment-code`'s audit evidence to six permissive-licensed Rust upstreams (whisky-archive Cardano Plutus-data helpers 4 Pattern C + tls-parser TLS NextProtocol parsers 2 Pattern C + glium OpenGL draw-parameter check 1 Pattern C + pkg-config-rs build-tool / system-package bindings 1 Pattern C + zarrs Zarr-format data-type bindings 6 Pattern B + boundless zkVM executor registry 1 Pattern A) AND from two-pattern (Pattern B + Pattern C, batch 14) to three-pattern (Pattern A + Pattern B + Pattern C, batch 15) coverage — completing the audit-corpus coverage of all three `docs/spec/comment-code-v0.md` patterns. Surrounding module-level docs (`//!`), the feature-gated `pub mod risc0;`, and the `use` statements (`use std::sync::Arc;`, `use crate::backend::{Executor, Registry};`) are dropped because cntrdct's per-fn loop only inspects `///` doc blocks immediately preceding `function_item`; tree-sitter parses the `match risc0::Risc0Executor::new() { Ok(exec) => ..., Err(e) => tracing::warn!(...) }` body without requiring identifier resolution. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// Build a [`Registry`] containing every backend enabled at compile time.
///
/// Backend constructors that need runtime resources (e.g. locating `r0vm`)
/// may fail; on failure the backend is omitted and a warning is logged so
/// the rest of the service can still start.
pub fn default_registry() -> Registry {
    #[allow(unused_mut)]
    let mut reg = Registry::new();

    #[cfg(feature = "risc0")]
    {
        match risc0::Risc0Executor::new() {
            Ok(exec) => reg = reg.with(Arc::new(exec) as Arc<dyn Executor>),
            Err(e) => tracing::warn!(error = %e, "risc0 backend disabled"),
        }
    }

    reg
}
