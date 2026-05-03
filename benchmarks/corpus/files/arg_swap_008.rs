// Source: signature pattern adapted from
// https://github.com/rust-lang/rust/blob/master/library/std/src/path.rs (lhs/rhs join style)
// License: MIT OR Apache-2.0
// Note: the call at line 13 swaps (parent, child) to exhibit the arg-swap pattern
// documented in Rice et al. (ICSE 2017).

fn join_path_008(parent: &str, child: &str) -> String {
    format!("{}/{}", parent, child)
}

fn entry_008() {
    let parent = 1;
    let child = 2;
    let _ = join_path_008(child, parent);
}
