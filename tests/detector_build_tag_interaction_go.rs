//! Integration tests for the `build-tag-interaction-go` detector.
//!
//! Spec: `docs/spec/build-tag-interaction-go-v0.md`.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    LanguageCitationStatus,
};
use cntrdct::detectors::lang::go_build_tag_interaction::GoBuildTagInteraction;
use cntrdct::parsers::Language;

fn parsed_go(name: &str, src: &str) -> cntrdct::ir::IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Go, src.to_string())
        .expect("ir_from_source")
}

fn run(files: Vec<cntrdct::ir::IrFile>) -> Vec<Finding> {
    let detector = GoBuildTagInteraction::new();
    register_detector(&detector).expect("build-tag-interaction-go must satisfy P1");
    let stats = CorpusStats::default();
    let config = DetectorConfig::default();
    let ctx = DetectContext {
        files: &files,
        stats: &stats,
        config: &config,
    };
    detector.detect(&ctx).expect("detect must not error")
}

fn go_with_constraint(expr: &str) -> String {
    format!("//go:build {expr}\n\npackage main\n\nfunc f() int {{ return 0 }}\n")
}

#[test]
fn t1_simple_contradiction_fires() {
    let files = vec![parsed_go("a.go", &go_with_constraint("linux && !linux"))];
    let findings = run(files);
    assert_eq!(findings.len(), 1, "got {findings:#?}");
    let f = &findings[0];
    assert_eq!(f.detector_id, "build-tag-interaction-go");
    assert_eq!(f.primary.start_line, 1);
    assert_eq!(f.evidence.raw["kind"], "go-build-tag-contradiction");
    assert_eq!(f.evidence.raw["conflicting_tag"], "linux");
}

#[test]
fn t2_nested_conjunction_contradiction_fires() {
    let files = vec![parsed_go(
        "a.go",
        &go_with_constraint("(linux && amd64) && !linux"),
    )];
    assert_eq!(run(files).len(), 1);
}

#[test]
fn t3_satisfiable_constraint_is_clean() {
    for expr in ["linux && amd64", "linux && !windows", "!cgo"] {
        let files = vec![parsed_go("a.go", &go_with_constraint(expr))];
        assert!(run(files).is_empty(), "satisfiable `{expr}` must not fire");
    }
}

#[test]
fn t4_disjunction_is_indeterminate() {
    let files = vec![parsed_go("a.go", &go_with_constraint("linux || !linux"))];
    assert!(run(files).is_empty());
}

#[test]
fn t5_de_morgan_negated_paren_is_indeterminate() {
    let files = vec![parsed_go(
        "a.go",
        &go_with_constraint("linux && !(linux && amd64)"),
    )];
    assert!(run(files).is_empty());
}

#[test]
fn t6_double_negation_is_positive() {
    let files = vec![parsed_go("a.go", &go_with_constraint("!!linux && !linux"))];
    assert_eq!(run(files).len(), 1);
}

#[test]
fn t7_non_go_source_is_skipped() {
    // The same text in a Rust file must not fire (Go-only detector).
    let rust = cntrdct::ir_from_source(
        &PathBuf::from("a.rs"),
        Language::Rust,
        "fn main() {}\n".to_string(),
    )
    .expect("ir");
    assert!(run(vec![rust]).is_empty());
}

#[test]
fn t8_citation_status_unconfirmed_and_keys_present() {
    let files = vec![parsed_go("a.go", &go_with_constraint("darwin && !darwin"))];
    let findings = run(files);
    let f = &findings[0];
    assert_eq!(
        f.evidence.language_citation_status,
        LanguageCitationStatus::Unconfirmed
    );
    assert!(f.evidence.citation_keys.contains(&"tartler-eurosys-2011"));
    assert!(f.evidence.citation_keys.contains(&"nadi-icse-2014"));
}

#[test]
fn t9_deterministic() {
    let src = go_with_constraint("unix && arm64 && !unix");
    let a = run(vec![parsed_go("a.go", &src)]);
    let b = run(vec![parsed_go("a.go", &src)]);
    assert_eq!(a.len(), b.len());
    assert_eq!(a[0].message, b[0].message);
}

#[test]
fn t10_build_tag_after_package_is_ignored() {
    // A `//go:build`-looking comment after the package clause is not a real
    // constraint and must not be read.
    let src = "package main\n\n//go:build linux && !linux\nfunc f() {}\n";
    let files = vec![parsed_go("a.go", src)];
    assert!(run(files).is_empty());
}
