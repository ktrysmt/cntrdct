// Source: signature pattern adapted from
// https://github.com/rust-lang/rust/blob/master/library/core/src/cmp.rs (a/b ordering)
// License: MIT OR Apache-2.0
// Note: the call at line 13 swaps (numerator, denominator) to exhibit the
// arg-swap pattern documented in Rice et al. (ICSE 2017).

fn divide_010(numerator: f64, denominator: f64) -> f64 {
    numerator / denominator
}

fn entry_010() {
    let numerator = 1;
    let denominator = 2;
    let _ = divide_010(denominator, numerator);
}
