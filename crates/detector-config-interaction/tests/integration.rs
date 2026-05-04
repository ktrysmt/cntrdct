//! Integration tests for the config-interaction detector v0 spec.
//!
//! Each test maps to a row in `cntrdct/docs/spec/config-interaction-v0.md`
//! test plan.

use std::path::PathBuf;

use cntrdct_core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, ParsedFile,
};
use cntrdct_detector_config_interaction::ConfigInteraction;

fn parsed(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: Language::Rust,
        source: src.to_string(),
    }
}

fn run(files: Vec<ParsedFile>) -> Vec<Finding> {
    let detector = ConfigInteraction::new();
    register_detector(&detector).expect("detector must satisfy P1");
    let stats = CorpusStats::default();
    let config = DetectorConfig::default();
    let ctx = DetectContext {
        files: &files,
        stats: &stats,
        config: &config,
    };
    detector.detect(&ctx).expect("detect must not error")
}

#[test]
fn t1_feature_pair_contradiction_on_fn() {
    let src = r#"#[cfg(feature = "x")]
#[cfg(not(feature = "x"))]
fn f() {}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    let f = &findings[0];
    assert_eq!(f.detector_id, "config-interaction");
    assert_eq!(
        f.related.len(),
        2,
        "should link to both attribute locations"
    );
    assert_eq!(
        f.evidence.raw["inner_predicate"].as_str().unwrap_or(""),
        "feature = \"x\"",
        "got raw: {}",
        f.evidence.raw
    );
}

#[test]
fn t2_unix_pair_contradiction_on_struct() {
    let src = "#[cfg(unix)]\n#[cfg(not(unix))]\nstruct S;\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
}

#[test]
fn t3_single_cfg_no_finding() {
    let src = "#[cfg(unix)]\nfn f() {}\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(findings.is_empty(), "got {:#?}", findings);
}

#[test]
fn t4_two_cfgs_no_negation() {
    let src = "#[cfg(feature = \"a\")]\n#[cfg(feature = \"b\")]\nfn f() {}\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(findings.is_empty(), "got {:#?}", findings);
}

#[test]
fn t5_order_reversed_still_fires() {
    let src = "#[cfg(not(unix))]\n#[cfg(unix)]\nfn f() {}\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
}

#[test]
fn t6_all_predicate_pair() {
    let src = "#[cfg(all(unix, x))]\n#[cfg(not(all(unix, x)))]\nfn f() {}\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
}

#[test]
fn t7_different_inner_predicates() {
    let src = "#[cfg(unix)]\n#[cfg(not(windows))]\nfn f() {}\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(findings.is_empty(), "got {:#?}", findings);
}

#[test]
fn t8_citation_keys_include_required() {
    let src = r#"#[cfg(feature = "x")]
#[cfg(not(feature = "x"))]
fn f() {}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1);
    let keys: Vec<&str> = findings[0].evidence.citation_keys.to_vec();
    assert!(
        keys.contains(&"tartler-eurosys-2011"),
        "missing tartler-eurosys-2011 in {:?}",
        keys
    );
    assert!(
        keys.contains(&"nadi-icse-2014"),
        "missing nadi-icse-2014 in {:?}",
        keys
    );
}

#[test]
fn t9_determinism_two_runs_identical() {
    let src = r#"#[cfg(feature = "x")]
#[cfg(not(feature = "x"))]
fn f() {}
"#;
    let a = run(vec![parsed("a.rs", src)]);
    let b = run(vec![parsed("a.rs", src)]);
    let to_json = |fs: &Vec<Finding>| serde_json::to_value(fs).unwrap();
    assert_eq!(to_json(&a), to_json(&b));
}

#[test]
fn t10_empty_input() {
    let findings = run(Vec::new());
    assert!(findings.is_empty());
}

#[test]
fn t11_non_rust_file_skipped() {
    let other = ParsedFile {
        path: PathBuf::from("a.py"),
        language: Language::Python,
        source: "# nothing\n".to_string(),
    };
    let findings = run(vec![other]);
    assert!(
        findings.is_empty(),
        "config-interaction supports only Rust; Python file must be skipped silently"
    );
}

#[test]
fn t12_cfg_attr_out_of_scope() {
    let src = r#"#[cfg_attr(unix, cfg(not(unix)))]
fn f() {}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(findings.is_empty(), "got {:#?}", findings);
}

#[test]
fn t13_three_attrs_one_finding_with_additional_pairs() {
    let src = "#[cfg(unix)]\n#[cfg(not(unix))]\n#[cfg(not(unix))]\nfn f() {}\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    let extra = findings[0].evidence.raw["additional_pairs"]
        .as_u64()
        .unwrap_or(99);
    assert_eq!(extra, 1, "got raw: {}", findings[0].evidence.raw);
}

#[test]
fn t14_anomaly_class_logic() {
    let src = "#[cfg(unix)]\n#[cfg(not(unix))]\nfn f() {}\n";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1);
    assert!(
        matches!(findings[0].anomaly_class, AnomalyClass::Logic),
        "got: {:?}",
        findings[0].anomaly_class
    );
}
