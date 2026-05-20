// Source: https://github.com/rust-lang/rust/blob/master/library/core/src/option.rs
// Note: shape adapted from upstream mapping family.
// License: MIT OR Apache-2.0
// Note: four near-identical mappers plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn map_a_009(xs: &[i64]) -> Vec<i64> {
    let mut out = Vec::new();
    for x in xs {
        out.push(x.abs());
    }
    out
}

fn map_b_009(xs: &[i64]) -> Vec<i64> {
    let mut out = Vec::new();
    for x in xs {
        out.push(x.abs());
    }
    out
}

fn map_c_009(xs: &[i64]) -> Vec<i64> {
    let mut out = Vec::new();
    for x in xs {
        out.push(x.abs());
    }
    out
}

fn map_d_009(xs: &[i64]) -> Vec<i64> {
    let mut out = Vec::new();
    for x in xs {
        out.push(x.abs());
    }
    out
}

fn map_drifted_009(xs: &[i64]) -> Vec<i64> {
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push(x.abs());
    }
    out
}
