//! Integration tests for the unreachable-after-terminator detector v0 spec.
//!
//! Each test maps to a row in
//! `cntrdct/docs/spec/unreachable-after-terminator-v0.md` test plan.

use std::path::PathBuf;

use cntrdct_core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus, ParsedFile,
};
use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;

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

// T11 (was: non-rust file skipped) is no longer expressible after F4-4b.
// `ParsedFile.language: Language` is `non_exhaustive` and closed; an
// "unsupported language" cannot be constructed at compile time. The
// language filter still runs, but its branches are now exhaustively
// covered by the supported_languages() set. Robustness of mis-labelled
// files is exercised indirectly when tree-sitter parsing fails.

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

// ---------- M-2: Python pilot ----------
//
// Mirrors T1-T15 for Python where the construct exists. v0 has no
// inline Python suppression mechanism, so the T6/T14 attribute-allow
// scenarios are not portable; cntrdct.toml-based suppression is
// covered by `crates/cli/tests/suppression.rs`.

#[test]
fn t16_python_return_followed_by_call() {
    let src = "def f():\n    return\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "expected 1 finding, got {:#?}", findings);
    let f = &findings[0];
    assert_eq!(f.detector_id, "unreachable-after-terminator");
    assert_eq!(
        f.evidence.raw["terminator_kind"], "return",
        "got: {}",
        f.evidence.raw
    );
}

#[test]
fn t17_python_raise_followed_by_call() {
    let src = "def f():\n    raise ValueError(\"x\")\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "raise",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t18_python_sys_exit_followed_by_call() {
    let src = "import sys\ndef f():\n    sys.exit(1)\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "sys.exit",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t19_python_os_underscore_exit_followed_by_call() {
    let src = "import os\ndef f():\n    os._exit(0)\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "os._exit",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t20_python_assert_false_followed_by_call() {
    let src = "def f():\n    assert False\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "assert",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t21_python_continue_inside_for_loop() {
    let src = "def f(xs):\n    for x in xs:\n        if x == 0:\n            continue\n            foo()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {:#?}", findings);
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "continue",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t22_python_only_first_follower_flagged_with_count() {
    let src = "def f():\n    raise RuntimeError\n    foo()\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "only the first follower is flagged");
    assert_eq!(
        findings[0].evidence.raw["following_count"], 2,
        "two statements follow: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t23_python_terminator_alone_no_finding() {
    let src = "def f():\n    return\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(findings.is_empty(), "got {:#?}", findings);
}

#[test]
fn t24_python_inner_block_terminator_does_not_pollute_outer() {
    let src = "def f(cond):\n    if cond:\n        return\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "terminator inside inner if-block must not flag outer follower, got {:#?}",
        findings
    );
}

#[test]
fn t25_python_assert_truthy_is_not_a_terminator() {
    let src = "def f():\n    assert True\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "assert with non-False condition must not be a terminator, got {:#?}",
        findings
    );
}

#[test]
fn t26_python_normal_call_is_not_a_terminator() {
    let src = "def f():\n    foo()\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(findings.is_empty(), "got {:#?}", findings);
}

#[test]
fn t27_python_findings_carry_unconfirmed_status() {
    // Per `docs/spec/citations-policy.md`, the v0 Rust citations on
    // this detector are grandfathered as Rust-grounded (FindBugs UR is
    // a Java pattern, Engler bugs-as-deviant-behavior is C). The
    // Python language extension survey did not yield a qualifying
    // citation; per policy the language ships with
    // `LanguageCitationStatus::Unconfirmed` on each emitted finding.
    let src = "def f():\n    return\n    bar()\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(!findings.is_empty(), "prerequisite: must produce findings");
    for f in &findings {
        assert!(
            matches!(
                f.evidence.language_citation_status,
                LanguageCitationStatus::Unconfirmed
            ),
            "Python finding must carry Unconfirmed status until a qualifying citation lands; got {:?}",
            f.evidence.language_citation_status
        );
    }
}

#[test]
fn t28_rust_findings_remain_confirmed() {
    let src = "fn f() { return; bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(!findings.is_empty(), "prerequisite: must produce findings");
    for f in &findings {
        assert!(
            matches!(
                f.evidence.language_citation_status,
                LanguageCitationStatus::Confirmed
            ),
            "Rust finding must carry Confirmed status (grandfathered v0); got {:?}",
            f.evidence.language_citation_status
        );
    }
}
