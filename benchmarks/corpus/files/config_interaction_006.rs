// Source: pattern adapted from
// https://github.com/hyperium/hyper/blob/master/src/lib.rs (windows-only blocks)
// License: MIT
// Note: a deliberately contradictory cfg pair on a constant exhibits the
// dead-block anomaly catalogued in Tartler et al. (EuroSys 2011).

#[cfg(windows)]
#[cfg(not(windows))]
const DEAD_CONST_006: u32 = 0;
