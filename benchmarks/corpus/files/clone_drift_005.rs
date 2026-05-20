// Source: https://github.com/tokio-rs/tokio/blob/master/tokio/src/task/local.rs
// Note: shape adapted from upstream poll-wrapper family.
// License: MIT
// Note: four near-identical poll wrappers plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn poll_a_005(state: u32) -> u32 {
    let mut s = state;
    while s > 0 {
        s -= 1;
    }
    s
}

fn poll_b_005(state: u32) -> u32 {
    let mut s = state;
    while s > 0 {
        s -= 1;
    }
    s
}

fn poll_c_005(state: u32) -> u32 {
    let mut s = state;
    while s > 0 {
        s -= 1;
    }
    s
}

fn poll_d_005(state: u32) -> u32 {
    let mut s = state;
    while s > 0 {
        s -= 1;
    }
    s
}

fn poll_drifted_005(state: u32) -> u32 {
    let mut s = state;
    while s > 0 {
        s -= 1;
        if s == 0 {
            break;
        }
    }
    s
}
