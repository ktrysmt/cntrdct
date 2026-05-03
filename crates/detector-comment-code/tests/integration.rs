//! Integration tests for the comment-code detector v0 spec.

use std::path::PathBuf;

use cntrdct_core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig,
    Finding, ParsedFile, Severity,
};
use cntrdct_detector_comment_code::CommentCode;

fn parsed(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: "rust".to_string(),
        source: src.to_string(),
    }
}

fn run(files: Vec<ParsedFile>) -> Vec<Finding> {
    let detector = CommentCode::new();
    register_detector(&detector).expect("comment-code must satisfy P1");
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
fn t1_pattern_a_err_claim_without_result() {
    let src = r#"
/// Returns Err on failure.
fn parse_int(s: &str) -> i32 {
    s.len() as i32
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 Pattern A finding, got {:#?}",
        findings
    );
    let f = &findings[0];
    assert_eq!(f.detector_id, "comment-code");
    assert!(matches!(f.raw_severity, Severity::Note));
    assert_eq!(f.anomaly_class, AnomalyClass::Documentation);
    assert_eq!(
        f.evidence.raw.get("pattern").and_then(|v| v.as_str()),
        Some("A"),
    );
}

#[test]
fn t2_pattern_a_correct_when_returns_result() {
    let src = r#"
/// Returns Err on failure.
fn parse_int(s: &str) -> Result<i32, String> {
    Err(s.to_string())
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "Result return type must satisfy Pattern A, got {:#?}",
        findings
    );
}

#[test]
fn t3_pattern_b_panic_claim_without_panic() {
    let src = r#"
/// Panics if x is zero.
fn divide(x: i32, y: i32) -> i32 {
    y / x
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 Pattern B finding, got {:#?}",
        findings
    );
    let f = &findings[0];
    assert_eq!(
        f.evidence.raw.get("pattern").and_then(|v| v.as_str()),
        Some("B"),
    );
}

#[test]
fn t4_pattern_b_correct_when_unwrap_present() {
    let src = r#"
/// Panics if x is zero.
fn divide(x: i32, y: i32) -> i32 {
    let z: Option<i32> = Some(y / x);
    z.unwrap()
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "unwrap() satisfies Pattern B, got {:#?}",
        findings
    );
}

#[test]
fn t5_pattern_c_deprecated_text_without_attribute() {
    let src = r#"
/// Deprecated: use bar instead.
fn foo() {
    let _ = 1;
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 Pattern C finding, got {:#?}",
        findings
    );
    let f = &findings[0];
    assert_eq!(
        f.evidence.raw.get("pattern").and_then(|v| v.as_str()),
        Some("C"),
    );
}

#[test]
fn t6_pattern_c_correct_when_deprecated_attr_present() {
    let src = r#"
/// Deprecated: use bar instead.
#[deprecated(note = "use bar instead")]
fn foo() {
    let _ = 1;
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "#[deprecated] satisfies Pattern C, got {:#?}",
        findings
    );
}

#[test]
fn t7_no_doc_comment_no_finding() {
    let src = r#"
fn quiet(x: i32) -> i32 {
    x + 1
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "fn without doc comment must not trigger, got {:#?}",
        findings
    );
}

#[test]
fn t8_findings_carry_known_citations() {
    let src = r#"
/// Returns Err on failure.
fn parse_int(s: &str) -> i32 {
    s.len() as i32
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    let known: &[&str] = &["tan-sosp-2007", "tan-pldi-2011"];
    assert!(
        !findings.is_empty(),
        "T8 prerequisite: must produce findings"
    );
    for f in &findings {
        assert!(
            !f.evidence.citation_keys.is_empty(),
            "P1: every finding must carry at least one citation"
        );
        for k in &f.evidence.citation_keys {
            assert!(
                known.contains(k),
                "citation key {} not in known set {:?}",
                k,
                known
            );
        }
    }
}

#[test]
fn t9_deterministic_repeatable() {
    let src = r#"
/// Returns Err on failure.
fn parse_int(s: &str) -> i32 {
    s.len() as i32
}

/// Panics if x is zero.
fn divide(x: i32, y: i32) -> i32 {
    y / x
}

/// Deprecated: use bar instead.
fn foo() {
    let _ = 1;
}
"#;
    let f1 = run(vec![parsed("a.rs", src)]);
    let f2 = run(vec![parsed("a.rs", src)]);
    let j1 = serde_json::to_string(&f1).expect("serialize");
    let j2 = serde_json::to_string(&f2).expect("serialize");
    assert_eq!(j1, j2, "two runs must produce identical findings");
}
