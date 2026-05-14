// Source: https://github.com/rust-lang/pkg-config-rs/blob/f36d32a09824a6b2c18475c8a4b7df1cb2c50c95/src/lib.rs
// License: MIT OR Apache-2.0
// Note: minimal extract of one top-level `pub fn` from rust-lang/pkg-config-rs@f36d32a09824a6b2c18475c8a4b7df1cb2c50c95 src/lib.rs (upstream lines 451-455, audit-corpus lines 5-9 after the 3-line header + 1 blank-line offset). The function carries a single-line `///` doc block reading `Deprecated in favor of the probe_library function` and a `#[doc(hidden)]` attribute, but does not carry the `#[deprecated]` runtime attribute the Rust deprecation lints honour, so downstream consumers receive no compiler warning — the textbook Tan SOSP 2007 §3.2 Pattern C ("bad comment") bug shape, the same one cntrdct's batch-3 `sidan-lab/whisky-archive` con_str* family, batch-11 `rusticata/tls-parser` `parse_tls_handshake_*next_protocol` family, and batch-12 `glium/glium` `validate` flag. Unrelated imports, struct / enum / impl definitions, and surrounding pkg-config helpers are dropped because comment-code v0 walks top-level `function_item` only and the doc-walk + Pattern C check operate per-fn; tree-sitter parses unresolved type identifiers (`Library`, `Error`) syntactically without requiring resolution. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// Deprecated in favor of the probe_library function
#[doc(hidden)]
pub fn find_library(name: &str) -> Result<Library, String> {
    probe_library(name).map_err(|e| e.to_string())
}
