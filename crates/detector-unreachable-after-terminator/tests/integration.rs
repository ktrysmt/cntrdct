//! Integration tests for the unreachable-after-terminator detector v0 spec.
//!
//! Each test maps to a row in
//! `cntrdct/docs/spec/unreachable-after-terminator-v0.md` test plan.

use std::path::PathBuf;

use cntrdct_core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    ParsedFile,
};
use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;

fn parsed(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: "rust".to_string(),
        source: src.to_string(),
    }
}

fn run(files: Vec<ParsedFile>) -> Vec<Finding> {
    let detector = UnreachableAfterTerminator::new();
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
fn t1_return_followed_by_call() {
    let src = "fn f() { return; bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "expected 1 finding, got {:#?}", findings);
    let f = &findings[0];
    assert_eq!(f.detector_id, "unreachable-after-terminator");
    assert_eq!(f.related.len(), 1, "should link to terminator location");
    assert_eq!(
        f.evidence.raw["terminator_kind"], "return",
        "got: {}",
        f.evidence.raw
    );
}

#[test]
fn t2_terminator_alone_no_finding() {
    let src = "fn f() { return; }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "no following statement → no finding, got {:#?}",
        findings
    );
}

#[test]
fn t3_panic_macro_terminator() {
    let src = r#"fn f() { panic!("x"); let x = 1; }"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "panic",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t4_only_first_follower_flagged_with_count() {
    let src = "fn f() { unreachable!(); foo(); bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "only the first follower is flagged");
    assert_eq!(
        findings[0].evidence.raw["following_count"], 2,
        "two statements follow the terminator: {}",
        findings[0].evidence.raw
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "unreachable",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t5_continue_inside_loop() {
    let src = r#"
fn f(xs: &[i32]) {
    for x in xs {
        if *x == 0 {
            continue;
            foo();
        }
    }
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "continue",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t6_outer_allow_attribute_suppresses() {
    let src = r#"
#[allow(unreachable_code)]
fn f() {
    return;
    bar();
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "outer #[allow(unreachable_code)] must suppress, got {:#?}",
        findings
    );
}

#[test]
fn t7_terminator_in_inner_block_does_not_pollute_outer() {
    let src = r#"
fn f() {
    if true { return; }
    bar();
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "terminator inside inner if-block must not flag outer follower, got {:#?}",
        findings
    );
}

#[test]
fn t8_known_citations_present() {
    let src = "fn f() { return; bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    let known: &[&str] = &["hovemeyer-pugh-oopsla-2004", "engler-sosp-2001"];
    assert!(
        !findings.is_empty(),
        "T8 prerequisite: must produce findings"
    );
    for f in &findings {
        assert!(
            f.evidence.citation_keys.iter().any(|k| known.contains(k)),
            "P1: citations {:?} contain no recognized key",
            f.evidence.citation_keys
        );
    }
}

#[test]
fn t9_deterministic_repeatable() {
    let files = vec![parsed("a.rs", "fn f() { return; bar(); }")];
    let first = run(files.clone());
    let second = run(files);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap(),
        "detect() must be deterministic"
    );
}

#[test]
fn t10_empty_input_returns_empty() {
    let findings = run(vec![]);
    assert!(findings.is_empty());
}

#[test]
fn t11_non_rust_file_skipped() {
    let file = ParsedFile {
        path: PathBuf::from("a.js"),
        language: "javascript".to_string(),
        source: "function f() { return; foo(); }".to_string(),
    };
    let findings = run(vec![file]);
    assert!(
        findings.is_empty(),
        "non-rust file must be skipped silently"
    );
}

#[test]
fn t12_todo_macro_terminator() {
    let src = "fn f() { todo!(); bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "todo",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t13_no_terminator_no_finding() {
    let src = "fn f() { let x = 1; bar(); baz(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "no terminator → no finding, got {:#?}",
        findings
    );
}

#[test]
fn t14_inner_allow_attribute_suppresses() {
    let src = r#"
fn f() {
    #![allow(unreachable_code)]
    return;
    bar();
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "inner #![allow(unreachable_code)] must suppress, got {:#?}",
        findings
    );
}

#[test]
fn t15_anomaly_class_is_logic() {
    let src = "fn f() { return; bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(!findings.is_empty(), "prerequisite: must produce findings");
    for f in &findings {
        assert_eq!(
            f.anomaly_class,
            AnomalyClass::Logic,
            "must classify as Logic per IEEE 1044-2009; got {:?}",
            f.anomaly_class
        );
    }
}
