// Source: signature pattern adapted from
// https://github.com/rust-lang/rust/blob/master/library/core/src/slice/mod.rs (copy_within et al.)
// License: MIT OR Apache-2.0
// Note: the call at line 13 reverses (dst, src) to exhibit the arg-swap pattern
// documented in Rice et al., "Detecting Argument Selection Defects" (ICSE 2017).

fn copy_buf_002(dst: &mut [u8], src: &[u8]) -> usize {
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

fn entry_002() {
    let dst = 1;
    let src = 2;
    let _ = copy_buf_002(src, dst);
}
