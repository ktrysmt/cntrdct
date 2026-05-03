// Source: signature pattern adapted from
// https://github.com/hyperium/hyper/blob/master/src/proto/h1/io.rs
// License: MIT
// Note: Pattern B — the doc says it panics but the body uses checked
// arithmetic and does not panic. Drift documented in Tan et al., iComment
// SOSP 2007.

/// Panics on overflow.
fn add_capped_007(a: u32, b: u32) -> u32 {
    a.saturating_add(b)
}
