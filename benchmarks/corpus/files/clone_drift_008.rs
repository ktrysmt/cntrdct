// Source: shape adapted from
// https://github.com/dtolnay/thiserror/blob/master/src/lib.rs (variant constructors)
// License: MIT OR Apache-2.0
// Note: four near-identical constructors plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn build_a_008(parts: &[u32]) -> u32 {
    let mut total: u32 = 0;
    for p in parts {
        total = total.wrapping_add(*p);
    }
    total
}

fn build_b_008(parts: &[u32]) -> u32 {
    let mut total: u32 = 0;
    for p in parts {
        total = total.wrapping_add(*p);
    }
    total
}

fn build_c_008(parts: &[u32]) -> u32 {
    let mut total: u32 = 0;
    for p in parts {
        total = total.wrapping_add(*p);
    }
    total
}

fn build_d_008(parts: &[u32]) -> u32 {
    let mut total: u32 = 0;
    for p in parts {
        total = total.wrapping_add(*p);
    }
    total
}

fn build_drifted_008(parts: &[u32]) -> u32 {
    let mut total: u32 = 0;
    for p in parts {
        if *p > 0 {
            total = total.wrapping_add(*p);
        }
    }
    total
}
