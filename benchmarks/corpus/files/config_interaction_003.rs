// Source: pattern adapted from
// https://github.com/tokio-rs/tokio/blob/master/tokio/src/lib.rs (unix / windows gating)
// License: MIT
// Note: a deliberately contradictory cfg pair on an enum exhibits the
// dead-block anomaly catalogued in Tartler et al. (EuroSys 2011).

#[cfg(unix)]
#[cfg(not(unix))]
enum DeadEnum003 {
    A,
    B,
}
