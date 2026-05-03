//! T2-7 acceptance: in-source attribute suppression and `cntrdct.toml`
//! overrides flow through the CLI scan pipeline.
//!
//! Three required scenarios:
//! (a) `#[cntrdct::allow(<id>)]` on a single item drops the matching finding.
//! (b) `cntrdct.toml` with `[detectors.<id>] enabled = false` disables the
//!     detector entirely.
//! (c) `cntrdct.toml` with `[detectors.<id>] severity = "error"` remaps the
//!     emitted finding's `raw_severity`.

use std::fs;
use std::path::Path;

use cntrdct_core::Severity;
use tempfile::TempDir;

fn scan_with_config_in(dir: &Path) -> Vec<cntrdct_core::Finding> {
    let (raw, files) = cntrdct_cli::scan_full(dir).expect("scan_full");
    cntrdct_cli::apply_suppression(None, dir, &files, raw).expect("apply_suppression")
}

#[test]
fn attribute_allow_suppresses_unreachable_finding() {
    let dir = TempDir::new().unwrap();
    // Without the attribute the unreachable-after-terminator detector fires
    // on the `let _ = 2;` statement after `return 1;`. With the attribute
    // wrapping the same function, the finding must be dropped.
    fs::write(
        dir.path().join("a.rs"),
        "#[cntrdct::allow(unreachable-after-terminator)]\n\
         pub fn dead() -> i32 {\n\
         \treturn 1;\n\
         \tlet _ = 2;\n\
         \t3\n\
         }\n",
    )
    .unwrap();

    let findings = scan_with_config_in(dir.path());
    let unreachable_count = findings
        .iter()
        .filter(|f| f.detector_id == "unreachable-after-terminator")
        .count();
    assert_eq!(
        unreachable_count, 0,
        "attribute suppression must drop the unreachable-after-terminator finding; got: {:?}",
        findings
    );
}

#[test]
fn config_disables_detector_entirely() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("a.rs"),
        "pub fn dead() -> i32 {\n\
         \treturn 1;\n\
         \tlet _ = 2;\n\
         \t3\n\
         }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("cntrdct.toml"),
        "[detectors.unreachable-after-terminator]\nenabled = false\n",
    )
    .unwrap();

    let findings = scan_with_config_in(dir.path());
    assert!(
        findings
            .iter()
            .all(|f| f.detector_id != "unreachable-after-terminator"),
        "[detectors.<id>] enabled = false must drop every finding from that detector; got: {:?}",
        findings
    );
}

#[test]
fn config_remaps_severity_to_error() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("a.rs"),
        "pub fn dead() -> i32 {\n\
         \treturn 1;\n\
         \tlet _ = 2;\n\
         \t3\n\
         }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("cntrdct.toml"),
        "[detectors.unreachable-after-terminator]\nseverity = \"error\"\n",
    )
    .unwrap();

    let findings = scan_with_config_in(dir.path());
    let unreachable: Vec<_> = findings
        .iter()
        .filter(|f| f.detector_id == "unreachable-after-terminator")
        .collect();
    assert!(!unreachable.is_empty(), "expected at least one finding");
    for f in unreachable {
        assert!(
            matches!(f.raw_severity, Severity::Error),
            "severity remap must elevate unreachable findings to Error; got {:?}",
            f.raw_severity
        );
    }
}
