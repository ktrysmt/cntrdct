// Source: signature pattern adapted from
// https://github.com/BurntSushi/regex/blob/master/regex-syntax/src/parser.rs (start/end position style)
// License: MIT OR Apache-2.0
// Note: the call at line 13 swaps (start, end) to exhibit the arg-swap pattern
// documented in Rice et al. (ICSE 2017).

fn span_006(start: usize, end: usize) -> (usize, usize) {
    debug_assert!(start <= end);
    (start, end)
}

fn entry_006() {
    let start = 1;
    let end = 2;
    let _ = span_006(end, start);
}
