// Source: signature pattern adapted from
// https://github.com/serde-rs/serde/blob/master/serde/src/de/mod.rs
// License: MIT OR Apache-2.0
// Note: Pattern A — the doc comment claims "Returns Err" but the return type
// is `i32`. Drift documented in Tan et al., iComment SOSP 2007.

/// Returns Err when the buffer is exhausted; the caller is expected to retry.
fn read_byte_003(buf: &[u8]) -> i32 {
    buf.first().copied().unwrap_or(0) as i32
}
