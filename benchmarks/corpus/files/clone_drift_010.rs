// Source: shape adapted from
// https://github.com/rust-lang/cargo/blob/master/src/cargo/core/registry.rs (lookup family)
// License: MIT OR Apache-2.0
// Note: four near-identical lookups plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn lookup_a_010(table: &[(u32, u32)], key: u32) -> Option<u32> {
    for (k, v) in table {
        if *k == key {
            return Some(*v);
        }
    }
    None
}

fn lookup_b_010(table: &[(u32, u32)], key: u32) -> Option<u32> {
    for (k, v) in table {
        if *k == key {
            return Some(*v);
        }
    }
    None
}

fn lookup_c_010(table: &[(u32, u32)], key: u32) -> Option<u32> {
    for (k, v) in table {
        if *k == key {
            return Some(*v);
        }
    }
    None
}

fn lookup_d_010(table: &[(u32, u32)], key: u32) -> Option<u32> {
    for (k, v) in table {
        if *k == key {
            return Some(*v);
        }
    }
    None
}

fn lookup_drifted_010(table: &[(u32, u32)], key: u32) -> Option<u32> {
    for (k, v) in table {
        if *k == key && *v != 0 {
            return Some(*v);
        }
    }
    None
}
