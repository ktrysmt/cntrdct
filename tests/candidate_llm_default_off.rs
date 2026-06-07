//! R-4 (P3 amendment, review M2): network-independent guard that the
//! DEFAULT scan path never originates a Layer 0 LLM candidate.
//!
//! The `network-isolation` netns job is defence-in-depth only: because
//! Layer 0 reaches the network via a subprocess (not a cntrdct socket),
//! the netns gate cannot distinguish "Layer 0 stayed dormant" from
//! "Layer 0 ran but its `claude --print` subprocess failed to reach the
//! network" — both yield zero candidates. This test is the AUTHORITATIVE
//! guard: it asserts that the library scan pipeline (`cntrdct::scan`,
//! which is exactly what the default CLI `scan` runs before any opt-in
//! flag) constructs no Layer 0 provider and emits no `Origin::Layer0Llm`
//! finding — with no network involved at all.
//!
//! Spec: `docs/spec/p3-amendment-v0.md` §3.2 / §9.

use cntrdct::core::Origin;

/// The flagship Bound B case: a 2-arg call whose identifiers carry no
/// lexical correlation with the parameter names, nested in a list
/// comprehension. Layer 1 (arg-swap F5) emits nothing here; if the
/// default path ever wired in Layer 0, a `Layer0Llm` finding would
/// appear. It must not.
const FLAGSHIP_SRC: &str = "\
def get_radiomics_features(seg_file, img_file):\n\
    return 0\n\
\n\
def run(ct_file, mask, masks):\n\
    return [get_radiomics_features(ct_file, mask) for _ in masks]\n";

#[test]
fn default_scan_emits_no_layer0_candidate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("totalsegmentator_statistics.py");
    std::fs::write(&file, FLAGSHIP_SRC).expect("write fixture");

    let findings = cntrdct::scan(&file).expect("scan succeeds");

    assert!(
        findings
            .iter()
            .all(|f| f.origin == Origin::Layer1Deterministic),
        "the default scan path must originate no Layer 0 candidate, but a \
         Finding carried Origin::Layer0Llm — Layer 0 must be opt-in only (P3)"
    );
}

#[test]
fn default_scan_origin_field_absent_from_json() {
    // B2 corollary: a default-path finding must serialise byte-identically
    // to the pre-R-4 shape — the `origin` field is skipped when default,
    // so the discriminator string never leaks into default output.
    let dir = tempfile::tempdir().expect("tempdir");
    // A file that DOES produce a Layer 1 finding, so we have a finding to
    // serialise. A trivially-swapped arg-swap call fires deterministically.
    let file = dir.path().join("swap.py");
    std::fs::write(
        &file,
        "def f(src, dst):\n    return 0\n\ndef g(src, dst):\n    return f(dst, src)\n",
    )
    .expect("write fixture");

    let findings = cntrdct::scan(&file).expect("scan succeeds");
    assert!(
        !findings.is_empty(),
        "expected at least one Layer 1 finding"
    );
    let json = serde_json::to_string(&findings).expect("serialise findings");
    assert!(
        !json.contains("origin") && !json.contains("Layer1Deterministic"),
        "default-origin findings must omit the `origin` field from JSON \
         (T1 byte-identity, B2); got: {json}"
    );
}
