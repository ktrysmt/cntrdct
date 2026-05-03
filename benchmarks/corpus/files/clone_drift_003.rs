// Source: shape adapted from
// https://github.com/dtolnay/anyhow/blob/master/src/fmt.rs (formatter family)
// License: MIT OR Apache-2.0
// Note: four near-identical formatters plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn render_a_003(items: &[u32]) -> String {
    let mut s = String::new();
    for it in items {
        s.push_str(&it.to_string());
    }
    s
}

fn render_b_003(items: &[u32]) -> String {
    let mut s = String::new();
    for it in items {
        s.push_str(&it.to_string());
    }
    s
}

fn render_c_003(items: &[u32]) -> String {
    let mut s = String::new();
    for it in items {
        s.push_str(&it.to_string());
    }
    s
}

fn render_d_003(items: &[u32]) -> String {
    let mut s = String::new();
    for it in items {
        s.push_str(&it.to_string());
    }
    s
}

fn render_drifted_003(items: &[u32]) -> String {
    let mut s = String::new();
    for it in items {
        s.push_str(&it.to_string());
        s.push(',');
    }
    s
}
