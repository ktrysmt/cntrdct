//! Integration tests for the arg-swap detector v0 spec.

use std::path::PathBuf;

use cntrdct_core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, ParsedFile,
};
use cntrdct_detector_arg_swap::ArgSwap;

fn parsed(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: Language::Rust,
        source: src.to_string(),
    }
}

fn run(files: Vec<ParsedFile>) -> Vec<Finding> {
    let detector = ArgSwap::new();
    register_detector(&detector).expect("arg-swap must satisfy P1");
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
fn t1_swap_detected() {
    let src = r#"
fn copy(dst: &mut [u8], src: &[u8]) {
    let _ = (dst, src);
}

fn caller() {
    let dst = vec![0u8];
    let src = vec![0u8];
    copy(src, dst);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected exactly 1 swap finding, got {:#?}",
        findings
    );
    assert_eq!(findings[0].detector_id, "arg-swap");
}

#[test]
fn t2_no_finding_when_arg_names_dont_match_params() {
    let src = r#"
fn copy(dst: &mut [u8], src: &[u8]) {
    let _ = (dst, src);
}

fn caller() {
    let d = vec![0u8];
    let s = vec![0u8];
    copy(d, s);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "expected no finding when arg names don't match, got {:#?}",
        findings
    );
}

#[test]
fn t3_no_finding_on_correct_order() {
    let src = r#"
fn copy(dst: &mut [u8], src: &[u8]) {
    let _ = (dst, src);
}

fn caller() {
    let dst = vec![0u8];
    let src = vec![0u8];
    copy(dst, src);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "correct order must not trigger, got {:#?}",
        findings
    );
}

#[test]
fn t4_single_arg_function_skipped() {
    let src = r#"
fn one(x: i32) {
    let _ = x;
}

fn caller() {
    let x = 1;
    one(x);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(findings.is_empty());
}

#[test]
fn t5_three_arg_function_skipped() {
    let src = r#"
fn three(a: i32, b: i32, c: i32) {
    let _ = (a, b, c);
}

fn caller() {
    let a = 1;
    let b = 2;
    let c = 3;
    three(c, b, a);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "n-ary out of v0 scope, got {:#?}",
        findings
    );
}

#[test]
fn t6_non_identifier_args_skipped() {
    let src = r#"
fn copy(dst: i32, src: i32) {
    let _ = (dst, src);
}

fn caller() {
    let src = 1;
    copy(src, 42);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "literal arg must not trigger, got {:#?}",
        findings
    );
}

#[test]
fn t7_unknown_callee_skipped() {
    let src = r#"
fn caller() {
    let dst = 1;
    let src = 2;
    unknown_function(src, dst);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(findings.is_empty());
}

#[test]
fn t8_findings_carry_known_citation() {
    let src = r#"
fn copy(dst: i32, src: i32) {
    let _ = (dst, src);
}

fn caller() {
    let dst = 1;
    let src = 2;
    copy(src, dst);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    let known: &[&str] = &["li-zhou-fse-2005", "rice-icse-2017"];
    assert!(
        !findings.is_empty(),
        "T8 prerequisite: must produce findings"
    );
    for f in &findings {
        assert!(
            f.evidence.citation_keys.iter().any(|k| known.contains(k)),
            "P1: citations {:?} contain no recognized arg-swap key",
            f.evidence.citation_keys
        );
    }
}

#[test]
fn t9_deterministic_repeatable() {
    let src = r#"
fn copy(dst: i32, src: i32) {
    let _ = (dst, src);
}

fn caller() {
    let dst = 1;
    let src = 2;
    copy(src, dst);
}
"#;
    let f1 = run(vec![parsed("a.rs", src)]);
    let f2 = run(vec![parsed("a.rs", src)]);
    let j1 = serde_json::to_string(&f1).unwrap();
    let j2 = serde_json::to_string(&f2).unwrap();
    assert_eq!(j1, j2);
}

#[test]
fn t10_empty_input_is_safe() {
    let findings = run(vec![]);
    assert!(findings.is_empty());
}

#[test]
fn t_anomaly_class_is_interface_for_every_finding() {
    let src = r#"
fn copy(dst: i32, src: i32) {
    let _ = (dst, src);
}

fn caller() {
    let dst = 1;
    let src = 2;
    copy(src, dst);
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        !findings.is_empty(),
        "prerequisite: must produce findings to assert their anomaly_class"
    );
    for f in &findings {
        assert_eq!(
            f.anomaly_class,
            AnomalyClass::Interface,
            "arg-swap findings must classify as Interface per IEEE 1044-2009; got {:?}",
            f.anomaly_class
        );
    }
}

#[test]
fn t11_cross_file_resolution_within_scan() {
    let def_src = r#"
fn copy(dst: i32, src: i32) {
    let _ = (dst, src);
}
"#;
    let call_src = r#"
fn caller() {
    let dst = 1;
    let src = 2;
    copy(src, dst);
}
"#;
    let findings = run(vec![parsed("def.rs", def_src), parsed("call.rs", call_src)]);
    assert_eq!(
        findings.len(),
        1,
        "cross-file resolution within scan should work, got {:#?}",
        findings
    );
}
