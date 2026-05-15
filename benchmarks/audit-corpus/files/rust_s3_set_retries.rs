// Source: https://github.com/durch/rust-s3/blob/771be1650c23bf9aa482907c2d88dcf55d2e3ebc/s3/src/lib.rs
// License: MIT
// Note: minimal extract of one top-level `pub fn set_retries` item from durch/rust-s3@771be1650c23bf9aa482907c2d88dcf55d2e3ebc s3/src/lib.rs (upstream line 54, corpus line 23 after the 3-line provenance header + 1 blank-line offset + the kept `use` declaration / `static RETRIES` declaration / blank-line gap that anchors the function in the same lexical context the upstream file uses). After `docs/spec/comment-code-v0.md` F2 rendering (each `///` line's prefix stripped, lines joined with `\n`, result case-folded) the doc text contains the substring `may fail`, which is one of the six Pattern A trigger phrases (`returns err`, `returns result`, `may fail`, `fallible`, `returns option`, `may return none`) enumerated in spec F3. The function signature `pub fn set_retries(retries: u8)` has unit return so the return type does not contain `Result` / `Option`, satisfying F3's return-type negation — the doc claim of fallibility is not propagated to the caller through the type system. The body is `RETRIES.store(retries, std::sync::atomic::Ordering::SeqCst);`, which is unconditionally infallible (`AtomicU8::store` cannot fail), so spec F4 Pattern B does NOT fire (the doc contains no `panic` substring anyway) and spec F5 Pattern C does NOT fire (the doc contains no `deprecated` substring) — only Pattern A. This is the THIRD Pattern A upstream in the audit corpus, deepening the two-upstream coverage batch 17 wasmtime introduced — boundless `default_registry` is the silent-absorb-and-log sub-shape (doc says "may fail", body absorbs constructor failures via `tracing::warn!` and returns a partial `Registry`), wasmtime `roundtrip` is the documented-panic-on-failure sub-shape (doc says "may fail", body panics via `unwrap` / `assert_eq!` to surface failure to the `arbitrary` fuzzer harness), and this rust-s3 `set_retries` case is the configuration-doc-references-external-fallibility sub-shape (the doc's `may fail` substring describes downstream S3 request operations that the configured retries protect against, not the function body itself, while the function body — an atomic store — is unconditionally infallible). All three are syntactic Pattern A hits — the doc claim that something "may fail" is not propagated to the caller through the type system, regardless of whether the body absorbs the failure silently (boundless), surfaces it via panic (wasmtime), or describes a fallibility scope unrelated to the function body (rust-s3) — which is exactly the behaviour the textbook Tan SOSP 2007 §3.1 Pattern A ("Description Comments that describe what the function returns") catches: a syntactic mismatch between the doc claim and the type signature, regardless of the semantic intent the doc author had in mind. The `use` declaration and `static RETRIES` declaration are preserved from the upstream lexical context so the audit-corpus file parses cleanly under tree-sitter without dropping the identifier resolution context the function depends on; the `pub fn get_retries() -> u8` companion fn at upstream line 72 is NOT carried because (a) its doc has no Pattern A / B / C trigger substring (`Retrieves the current number of retries`) so it would not contribute a labelled finding and (b) it would inflate the file without changing the audit signal. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

use std::sync::atomic::AtomicU8;

static RETRIES: AtomicU8 = AtomicU8::new(1);

/// Sets the number of retries for operations that may fail and need to be retried.
///
/// This function stores the specified number of retries in an atomic variable,
/// which can be safely shared across threads. This is used by the retry! macro to automatically retry all requests.
///
/// # Arguments
///
/// * `retries` - The number of retries to set.
///
/// # Example
///
/// ```rust
/// s3::set_retries(3);
/// ```
pub fn set_retries(retries: u8) {
    RETRIES.store(retries, std::sync::atomic::Ordering::SeqCst);
}
