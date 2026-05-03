// Source: signature pattern adapted from
// https://github.com/dtolnay/thiserror/blob/master/src/lib.rs
// License: MIT OR Apache-2.0
// Note: an extra `let _ = msg;` is appended after `panic!()` to exhibit the
// unreachable-after-terminator pattern documented in Hovemeyer & Pugh
// (OOPSLA 2004).

fn require_010(cond: bool, msg: &str) {
    if !cond {
        panic!("{}", msg);
        let _ = msg;
    }
}
