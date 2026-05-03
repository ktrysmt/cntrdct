// Source: signature pattern adapted from
// https://github.com/clap-rs/clap/blob/master/clap_builder/src/parser/parser.rs
// License: MIT OR Apache-2.0
// Note: an extra `let _ = name;` is appended after `todo!()` to exhibit the
// unreachable-after-terminator pattern documented in Hovemeyer & Pugh
// (OOPSLA 2004).

fn handle_subcommand_008(name: &str) -> u8 {
    if name == "init" {
        todo!("not yet implemented");
        let _ = name;
    }
    0
}
