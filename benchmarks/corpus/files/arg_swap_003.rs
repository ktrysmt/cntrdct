// Source: signature pattern adapted from
// https://github.com/dtolnay/anyhow/blob/master/src/context.rs (with_context style)
// License: MIT OR Apache-2.0
// Note: the call at line 13 swaps (key, value) to exhibit the arg-swap pattern
// documented in Rice et al. (ICSE 2017).

fn insert_pair_003(key: String, value: String) -> (String, String) {
    (key, value)
}

fn entry_003() {
    let key = 1;
    let value = 2;
    let _ = insert_pair_003(value, key);
}
