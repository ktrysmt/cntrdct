// Source: signature pattern adapted from
// https://github.com/BurntSushi/regex/blob/master/regex-syntax/src/parser.rs
// License: MIT OR Apache-2.0
// Note: an extra `let _ = i;` is appended after `continue` inside the loop body
// to exhibit the unreachable-after-terminator pattern documented in Hovemeyer
// & Pugh (OOPSLA 2004).

fn skip_zeros_006(xs: &[i32]) -> i32 {
    let mut acc = 0;
    for i in 0..xs.len() {
        if xs[i] == 0 {
            continue;
            let _ = i;
        }
        acc += xs[i];
    }
    acc
}
