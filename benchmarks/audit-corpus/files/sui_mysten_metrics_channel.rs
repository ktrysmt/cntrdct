// Source: https://github.com/MystenLabs/sui/blob/add9d4722104173186e78df5aac4b07b6e019b40/crates/mysten-metrics/src/metered_channel.rs
// License: Apache-2.0
// Note: minimal extract of two top-level `pub fn` items from MystenLabs/sui@add9d4722104173186e78df5aac4b07b6e019b40 crates/mysten-metrics/src/metered_channel.rs (upstream lines 318-357, audit-corpus lines 5-44 after the 3-line header + 1 blank-line offset). `pub fn channel` at upstream line 321 (corpus line 8) carries a two-line `///` doc block whose second line reads `Deprecated: use `monitored_mpsc::channel` instead.`; `pub fn channel_with_total` at upstream line 339 (corpus line 26) carries a single-line `///` doc block reading `Deprecated: use `monitored_mpsc::channel` instead.` Both functions carry the `#[track_caller]` attribute (which propagates the caller location for panic reporting but does NOT trigger the Rust deprecation lints) and neither carries the `#[deprecated]` runtime attribute the Rust deprecation lints honour, so downstream consumers receive no compiler warning — the textbook Tan SOSP 2007 §3.2 Pattern C ("bad comment": deprecation prose without `#[deprecated]` attribute) bug shape, the same one cntrdct's batch-3 `sidan-lab/whisky-archive` con_str* family, batch-11 `rusticata/tls-parser` `parse_tls_handshake_*next_protocol` family, batch-12 `glium/glium` `validate` flag, and batch-13 `rust-lang/pkg-config-rs` `find_library` flag. Diversifies `comment-code`'s Pattern C audit evidence from four upstreams (whisky-archive Cardano Plutus-data helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium OpenGL draw-parameter check 1 + pkg-config-rs build-tool / system-package bindings 1) to five upstreams (... + sui mysten-metrics async-channel metrics wrapper 2) on five unrelated domains, reducing the regression-detection risk of single-source dominance in Pattern C the way batches 16 and 19 progressively did for Pattern B and batches 17 and 18 progressively did for Pattern A. The `#[track_caller]` attribute on both functions is structurally analogous to the `#[doc(hidden)]` attribute on batch-13 pkg-config-rs `find_library` in that BOTH are non-suppressive attributes present alongside the deprecation prose without triggering the `#[deprecated]` lint — confirming again that cntrdct's Pattern C check walks the literal first identifier of the attribute path (`track_caller` / `doc` vs. `deprecated`) and does not interpret the attribute's behavioural semantics. Unrelated imports, struct / enum / impl definitions, the intervening blank line between the two functions (upstream line 336), and surrounding `mpsc` facade helpers are dropped because comment-code v0 walks top-level `function_item` only and the doc-walk + Pattern C check operate per-fn; tree-sitter parses unresolved type identifiers (`IntGauge`, `IntCounter`, `Sender`, `Receiver`, `mpsc`) syntactically without requiring resolution. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// Similar to `mpsc::channel`, `channel` creates a pair of `Sender` and `Receiver`
/// Deprecated: use `monitored_mpsc::channel` instead.
#[track_caller]
pub fn channel<T>(size: usize, gauge: &IntGauge) -> (Sender<T>, Receiver<T>) {
    gauge.set(0);
    let (sender, receiver) = mpsc::channel(size);
    (
        Sender {
            inner: sender,
            gauge: gauge.clone(),
        },
        Receiver {
            inner: receiver,
            gauge: gauge.clone(),
            total: None,
        },
    )
}

/// Deprecated: use `monitored_mpsc::channel` instead.
#[track_caller]
pub fn channel_with_total<T>(
    size: usize,
    gauge: &IntGauge,
    total_gauge: &IntCounter,
) -> (Sender<T>, Receiver<T>) {
    gauge.set(0);
    let (sender, receiver) = mpsc::channel(size);
    (
        Sender {
            inner: sender,
            gauge: gauge.clone(),
        },
        Receiver {
            inner: receiver,
            gauge: gauge.clone(),
            total: Some(total_gauge.clone()),
        },
    )
}
