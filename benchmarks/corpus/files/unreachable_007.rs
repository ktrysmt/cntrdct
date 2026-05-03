// Source: signature pattern adapted from
// https://github.com/rust-lang/rust/blob/master/library/core/src/slice/iter.rs
// License: MIT OR Apache-2.0
// Note: an extra `let _ = ();` is appended after `break` inside the loop to
// exhibit the unreachable-after-terminator pattern documented in Hovemeyer &
// Pugh (OOPSLA 2004).

fn first_match_007(xs: &[i32], target: i32) -> Option<usize> {
    let mut found = None;
    for i in 0..xs.len() {
        if xs[i] == target {
            found = Some(i);
            break;
            let _ = ();
        }
    }
    found
}
