// Source: pattern adapted from
// https://github.com/BurntSushi/regex/blob/master/regex/src/lib.rs (perf-feature gating)
// License: MIT OR Apache-2.0
// Note: a deliberately contradictory cfg pair on a module item exhibits the
// dead-block anomaly catalogued in Tartler et al. (EuroSys 2011).

#[cfg(feature = "perf")]
#[cfg(not(feature = "perf"))]
mod dead_mod_004 {
    pub fn placeholder() {}
}
