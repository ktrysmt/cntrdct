// Source: signature pattern adapted from
// https://github.com/rust-lang/rust/blob/master/library/core/src/num/mod.rs
// License: MIT OR Apache-2.0
// Note: Pattern B — the doc says the function panics, but the body has no
// panicking construct. Drift documented in Tan et al., iComment SOSP 2007.

/// Panics if the divisor is zero.
fn safe_div_005(a: i32, b: i32) -> i32 {
    if b == 0 { 0 } else { a / b }
}
