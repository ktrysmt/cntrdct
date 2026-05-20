// Source: https://github.com/serde-rs/serde/blob/master/serde/src/de/value.rs
// Note: shape adapted from upstream visit_str family.
// License: MIT OR Apache-2.0
// Note: four near-identical visitors plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn visit_str_a_002(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        if c.is_ascii() {
            n += 1;
        }
    }
    n
}

fn visit_str_b_002(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        if c.is_ascii() {
            n += 1;
        }
    }
    n
}

fn visit_str_c_002(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        if c.is_ascii() {
            n += 1;
        }
    }
    n
}

fn visit_str_d_002(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        if c.is_ascii() {
            n += 1;
        }
    }
    n
}

fn visit_str_drifted_002(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        if c.is_ascii() && !c.is_whitespace() {
            n += 1;
        }
    }
    n
}
