// Source: signature pattern adapted from
// https://github.com/serde-rs/serde/blob/master/serde/src/de/mod.rs (visit_pair style)
// License: MIT OR Apache-2.0
// Note: the call at line 13 swaps (left, right) to exhibit the arg-swap pattern
// documented in Rice et al. (ICSE 2017).

fn min_max_004(left: i32, right: i32) -> (i32, i32) {
    if left <= right { (left, right) } else { (right, left) }
}

fn entry_004() {
    let left = 1;
    let right = 2;
    let _ = min_max_004(right, left);
}
