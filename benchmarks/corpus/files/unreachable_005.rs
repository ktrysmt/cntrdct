// Source: signature pattern adapted from
// https://github.com/tokio-rs/tokio/blob/master/tokio/src/runtime/handle.rs
// License: MIT
// Note: an extra `let _ = 1;` is appended after `return` to exhibit the
// unreachable-after-terminator pattern documented in Hovemeyer & Pugh
// (OOPSLA 2004).

fn early_return_005(flag: bool) -> u32 {
    if flag {
        return 42;
        let _ = 1;
    }
    0
}
