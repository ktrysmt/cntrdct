// Source: pattern adapted from
// https://github.com/dtolnay/anyhow/blob/master/src/lib.rs (cfg gating idiom)
// License: MIT OR Apache-2.0
// Note: a deliberately contradictory cfg pair on a single item exhibits the
// dead-block anomaly catalogued in Tartler et al., EuroSys 2011, and
// empirically confirmed by Nadi et al., ICSE 2014.

#[cfg(feature = "x")]
#[cfg(not(feature = "x"))]
fn dead_001() {
    let _payload = vec![0u8, 1, 2];
}
