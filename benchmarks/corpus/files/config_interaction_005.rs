// Source: pattern adapted from
// https://github.com/clap-rs/clap/blob/master/clap_builder/src/lib.rs (target_arch gating)
// License: MIT OR Apache-2.0
// Note: a deliberately contradictory cfg pair on a function exhibits the
// dead-block anomaly catalogued in Tartler et al. (EuroSys 2011).

#[cfg(target_arch = "x86_64")]
#[cfg(not(target_arch = "x86_64"))]
fn arch_specific_005() {
    let _label: &str = "x86_64";
}
