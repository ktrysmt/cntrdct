// Source: signature pattern adapted from
// https://github.com/serde-rs/serde/blob/master/serde/src/de/mod.rs
// License: MIT OR Apache-2.0
// Note: an extra `let _ = ();` is appended after `unreachable!()` to exhibit
// the unreachable-after-terminator pattern documented in Hovemeyer & Pugh
// (OOPSLA 2004).

fn dispatch_004(tag: u8) -> &'static str {
    match tag {
        0 => "zero",
        _ => {
            unreachable!("invalid tag");
            let _ = ();
        }
    }
}
