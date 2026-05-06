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

use cntrdct::core::Severity;
use tempfile::TempDir;

fn scan_with_config_in(dir: &Path) -> Vec<cntrdct::core::Finding> {
    let (raw, files) = cntrdct::scan_full(dir).expect("scan_full");
    cntrdct::apply_suppression(None, dir, &files, raw).expect("apply_suppression")
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
fn pr_miner_t15_python_language_suppression_drops_findings() {
    // Spec: docs/spec/pr-miner-v0.md test plan T15. With
    // `[languages.python] suppress = ["pr-miner"]` and a Python-only
    // pr-miner violation, no pr-miner finding survives. Rust violations
    // (if any) are unaffected — but to keep the assertion narrow this
    // test seeds a Python-only corpus.
    let dir = TempDir::new().unwrap();
    let mut src = String::new();
    for i in 0..7 {
        src.push_str(&format!(
            "def py_ok_{i}(x):\n    open_handle()\n    _ = x + {i}\n    close_handle()\n"
        ));
    }
    src.push_str("def py_violator():\n    open_handle()\n    suppression_t15_helper()\n");
    for i in 0..12 {
        src.push_str(&format!(
            "def py_filler_{i}():\n    filler_a()\n    filler_b()\n"
        ));
    }
    fs::write(dir.path().join("a.py"), src).unwrap();
    fs::write(
        dir.path().join("cntrdct.toml"),
        "[languages.python]\nsuppress = [\"pr-miner\"]\n",
    )
    .unwrap();

    let findings = scan_with_config_in(dir.path());
    let pr_miner_count = findings
        .iter()
        .filter(|f| f.detector_id == "pr-miner")
        .count();
    assert_eq!(
        pr_miner_count, 0,
        "[languages.python] suppress must drop all pr-miner findings on Python files; got: {:?}",
        findings
    );
}

#[test]
fn pr_miner_t14_attribute_allow_suppresses_violation() {
    // Spec: docs/spec/pr-miner-v0.md test plan T14.
    // The fixture seeds 7 lock/unlock satisfiers + 1 violator, plus 12
    // filler functions sharing a separate pair, so the mining database
    // hits MIN_DATABASE_SIZE = 20 and the lock -> unlock rule mines with
    // confidence 7/8 = 0.875 (above MIN_CONFIDENCE = 0.85). Without the
    // attribute the violator would surface; with the attribute the
    // CLI's apply_suppression must drop it.
    let dir = TempDir::new().unwrap();
    let mut src = String::new();
    for i in 0..7 {
        src.push_str(&format!(
            "fn ok_{i}(x: i32) -> i32 {{\n    lock();\n    let r = x + {i};\n    unlock();\n    r\n}}\n"
        ));
    }
    src.push_str(
        "#[cntrdct::allow(pr-miner)]\n\
         fn missing_unlock_violator() {\n    lock();\n    suppression_t14_helper();\n}\n",
    );
    for i in 0..12 {
        src.push_str(&format!(
            "fn filler_{i}() {{\n    filler_a();\n    filler_b();\n}}\n"
        ));
    }
    fs::write(dir.path().join("a.rs"), src).unwrap();

    let findings = scan_with_config_in(dir.path());
    let pr_miner_count = findings
        .iter()
        .filter(|f| f.detector_id == "pr-miner")
        .count();
    assert_eq!(
        pr_miner_count, 0,
        "attribute suppression must drop the pr-miner finding; got: {:?}",
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
