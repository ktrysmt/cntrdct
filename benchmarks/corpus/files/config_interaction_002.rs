// Source: pattern adapted from
// https://github.com/serde-rs/serde/blob/master/serde/src/lib.rs (target_os gating)
// License: MIT OR Apache-2.0
// Note: a deliberately contradictory cfg pair on a struct exhibits the
// dead-block anomaly catalogued in Tartler et al. (EuroSys 2011).

#[cfg(target_os = "linux")]
#[cfg(not(target_os = "linux"))]
struct DeadStruct002;
