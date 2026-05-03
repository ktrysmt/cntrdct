// Source: signature pattern adapted from
// https://github.com/hyperium/hyper/blob/master/src/headers.rs (name/value pair style)
// License: MIT
// Note: the call at line 13 swaps (name, value) to exhibit the arg-swap pattern
// documented in Rice et al. (ICSE 2017).

fn header_009(name: String, value: String) -> String {
    format!("{}: {}", name, value)
}

fn entry_009() {
    let name = 1;
    let value = 2;
    let _ = header_009(value, name);
}
