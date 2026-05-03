// Source: signature pattern adapted from
// https://github.com/clap-rs/clap/blob/master/clap_builder/src/builder/arg.rs
// License: MIT OR Apache-2.0
// Note: Pattern C — the doc says "Deprecated" but no `#[deprecated]` attribute
// is attached. Drift documented in Tan et al., iComment SOSP 2007.

/// Deprecated wrapper preserved for compatibility; prefer `validate_strict`.
fn validate_legacy_009(input: &str) -> bool {
    !input.is_empty()
}
