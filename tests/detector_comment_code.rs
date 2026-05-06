//! Integration tests for the comment-code detector v0 spec.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus, ParsedFile, Severity,
};
use cntrdct::detectors::comment_code::CommentCode;

fn parsed(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: Language::Rust,
        source: src.to_string(),
    }
}

fn parsed_python(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: Language::Python,
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

// ---------- M-3: Python pilot ----------
//
// Mirrors the Rust patterns where Python has a faithful equivalent.
// Pattern A (Rust Result/Option signature claim) does not transfer:
// Python lacks a static return-type signal. py-raises substitutes and
// is closer to Rust's Pattern B in spirit (doc claims a divergent
// effect, body lacks the corresponding construct). py-deprecated
// mirrors Rust's Pattern C.

#[test]
fn t10_python_pattern_raises_without_raise_in_body() {
    let src = "def parse_header(buf):\n    \"\"\"Raises ValueError on truncated input.\"\"\"\n    return buf[:4]\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 py-raises finding, got {:#?}",
        findings
    );
    let f = &findings[0];
    assert_eq!(f.detector_id, "comment-code");
    assert_eq!(f.anomaly_class, AnomalyClass::Documentation);
    assert!(matches!(f.raw_severity, Severity::Note));
    assert_eq!(
        f.evidence.raw.get("pattern").and_then(|v| v.as_str()),
        Some("py-raises"),
    );
}

#[test]
fn t11_python_pattern_raises_satisfied_when_raise_present() {
    let src = "def parse_header(buf):\n    \"\"\"Raises ValueError on truncated input.\"\"\"\n    if len(buf) < 4:\n        raise ValueError(\"truncated\")\n    return buf[:4]\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "raise statement satisfies py-raises, got {:#?}",
        findings
    );
}

#[test]
fn t12_python_pattern_deprecated_without_decorator() {
    let src = "def foo():\n    \"\"\"Deprecated: use bar instead.\"\"\"\n    return 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 py-deprecated finding, got {:#?}",
        findings
    );
    let f = &findings[0];
    assert_eq!(
        f.evidence.raw.get("pattern").and_then(|v| v.as_str()),
        Some("py-deprecated"),
    );
}

#[test]
fn t13_python_pattern_deprecated_satisfied_with_warnings_decorator() {
    let src = "import warnings\n\n@warnings.deprecated(\"use bar instead\")\ndef foo():\n    \"\"\"Deprecated: use bar instead.\"\"\"\n    return 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "@warnings.deprecated must satisfy py-deprecated, got {:#?}",
        findings
    );
}

#[test]
fn t14_python_pattern_deprecated_satisfied_with_bare_decorator() {
    let src = "from typing_extensions import deprecated\n\n@deprecated(\"use bar instead\")\ndef foo():\n    \"\"\"Deprecated: use bar instead.\"\"\"\n    return 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "bare @deprecated must satisfy py-deprecated, got {:#?}",
        findings
    );
}

#[test]
fn t15_python_no_docstring_no_finding() {
    let src = "def quiet():\n    return 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "fn without docstring must not trigger, got {:#?}",
        findings
    );
}

#[test]
fn t16_python_findings_carry_unconfirmed_status() {
    let src = "def parse_header(buf):\n    \"\"\"Raises ValueError on truncated input.\"\"\"\n    return buf[:4]\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(!findings.is_empty(), "prerequisite: must produce findings");
    for f in &findings {
        assert!(
            matches!(
                f.evidence.language_citation_status,
                LanguageCitationStatus::Unconfirmed
            ),
            "Python finding must carry Unconfirmed per citations-policy.md; got {:?}",
            f.evidence.language_citation_status
        );
    }
}

#[test]
fn t17_rust_findings_remain_confirmed() {
    let src = r#"
/// Returns Err on failure.
fn parse_int(s: &str) -> i32 {
    s.len() as i32
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(!findings.is_empty(), "prerequisite: must produce findings");
    for f in &findings {
        assert!(
            matches!(
                f.evidence.language_citation_status,
                LanguageCitationStatus::Confirmed
            ),
            "Rust finding must carry Confirmed (grandfathered v0); got {:?}",
            f.evidence.language_citation_status
        );
    }
}

#[test]
fn t18_python_triple_single_quote_docstring() {
    let src = "def foo():\n    '''Raises ValueError when bad.'''\n    return 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0]
            .evidence
            .raw
            .get("pattern")
            .and_then(|v| v.as_str()),
        Some("py-raises"),
    );
}

#[test]
fn t19_python_throws_phrase_is_a_trigger() {
    let src = "def foo():\n    \"\"\"Throws TypeError on bad input.\"\"\"\n    return 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0]
            .evidence
            .raw
            .get("trigger")
            .and_then(|v| v.as_str()),
        Some("throws"),
    );
}

#[test]
fn t20_python_class_method_not_top_level_skipped() {
    // v0 only inspects module-top-level def. Methods inside a class
    // body are not analysed.
    let src = "class C:\n    def m(self):\n        \"\"\"Raises ValueError when bad.\"\"\"\n        return 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "method inside class is not top-level; must be skipped, got {:#?}",
        findings
    );
}
