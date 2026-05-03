// Source: signature pattern adapted from
// https://github.com/BurntSushi/regex/blob/master/regex-syntax/src/parser.rs
// License: MIT OR Apache-2.0
// Note: Pattern A — the doc claims "fallible" but the return type is `bool`.
// Drift documented in Tan et al., iComment SOSP 2007.

/// Fallible: returns false on the first non-ASCII byte.
fn is_ascii_004(s: &str) -> bool {
    s.is_ascii()
}
