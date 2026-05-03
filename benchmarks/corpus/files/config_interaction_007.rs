// Source: pattern adapted from
// https://github.com/dtolnay/thiserror/blob/master/src/lib.rs (debug_assertions gating)
// License: MIT OR Apache-2.0
// Note: a deliberately contradictory cfg pair on a static exhibits the
// dead-block anomaly catalogued in Tartler et al. (EuroSys 2011).

#[cfg(debug_assertions)]
#[cfg(not(debug_assertions))]
static DEAD_STATIC_007: u8 = 0;
