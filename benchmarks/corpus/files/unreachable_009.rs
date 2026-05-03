// Source: signature pattern adapted from
// https://github.com/hyperium/hyper/blob/master/src/proto/h1/io.rs
// License: MIT
// Note: an extra `let _ = 0;` is appended after `unimplemented!()` to exhibit
// the unreachable-after-terminator pattern documented in Hovemeyer & Pugh
// (OOPSLA 2004).

fn protocol_negotiate_009(version: u8) -> u8 {
    match version {
        1 => 1,
        _ => {
            unimplemented!("unsupported version");
            let _ = 0;
        }
    }
}
