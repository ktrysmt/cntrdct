// Source: signature pattern adapted from
// https://github.com/dtolnay/anyhow/blob/master/src/error.rs
// License: MIT OR Apache-2.0
// Note: an extra `let _ = drop(());` is appended after `panic!()` at line 9 to
// exhibit the unreachable-after-terminator pattern documented in Hovemeyer &
// Pugh, "Finding Bugs is Easy", OOPSLA 2004.

fn assert_initialized_003(state: u8) {
    if state == 0 {
        panic!("uninitialized state");
        let _ = drop(());
    }
}
