// Source: signature pattern adapted from
// https://github.com/clap-rs/clap/blob/master/clap_builder/src/builder/arg.rs (long/short style)
// License: MIT OR Apache-2.0
// Note: the call at line 13 swaps (haystack, needle) to exhibit the arg-swap
// pattern documented in Rice et al. (ICSE 2017).

fn find_byte_007(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|b| *b == needle)
}

fn entry_007() {
    let haystack = 1;
    let needle = 2;
    let _ = find_byte_007(needle, haystack);
}
