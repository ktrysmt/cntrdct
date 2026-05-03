// Source: shape adapted from
// https://github.com/hyperium/hyper/blob/master/src/proto/h1/parse.rs (header walkers)
// License: MIT
// Note: four near-identical walkers plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn walk_a_007(headers: &[(String, String)]) -> usize {
    let mut count = 0;
    for (k, _) in headers {
        if k.starts_with("X-") {
            count += 1;
        }
    }
    count
}

fn walk_b_007(headers: &[(String, String)]) -> usize {
    let mut count = 0;
    for (k, _) in headers {
        if k.starts_with("X-") {
            count += 1;
        }
    }
    count
}

fn walk_c_007(headers: &[(String, String)]) -> usize {
    let mut count = 0;
    for (k, _) in headers {
        if k.starts_with("X-") {
            count += 1;
        }
    }
    count
}

fn walk_d_007(headers: &[(String, String)]) -> usize {
    let mut count = 0;
    for (k, _) in headers {
        if k.starts_with("X-") {
            count += 1;
        }
    }
    count
}

fn walk_drifted_007(headers: &[(String, String)]) -> usize {
    let mut count = 0;
    for (k, v) in headers {
        if k.starts_with("X-") && !v.is_empty() {
            count += 1;
        }
    }
    count
}
