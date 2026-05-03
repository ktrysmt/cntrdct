// Source: shape adapted from
// https://github.com/clap-rs/clap/blob/master/clap_builder/src/builder/arg_predicate.rs (predicate family)
// License: MIT OR Apache-2.0
// Note: four near-identical predicates plus one drifted member exhibit the
// Type-3 with Type-2 partition drift documented in Bettenburg et al. (MSR 2009).

fn matches_a_006(name: &str, target: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name == target
}

fn matches_b_006(name: &str, target: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name == target
}

fn matches_c_006(name: &str, target: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name == target
}

fn matches_d_006(name: &str, target: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name == target
}

fn matches_drifted_006(name: &str, target: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.eq_ignore_ascii_case(target)
}
