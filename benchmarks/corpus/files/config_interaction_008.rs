// Source: pattern adapted from
// https://github.com/rust-lang/rust/blob/master/library/std/src/lib.rs (test-only items)
// License: MIT OR Apache-2.0
// Note: a deliberately contradictory cfg pair on a function exhibits the
// dead-block anomaly catalogued in Tartler et al. (EuroSys 2011).

#[cfg(test)]
#[cfg(not(test))]
fn unreachable_test_helper_008() {
    let _flag: bool = false;
}
