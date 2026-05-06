//! Integration tests for the arg-swap detector v0 spec.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus, ParsedFile,
};
use cntrdct::detectors::arg_swap::ArgSwap;

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

// ---------- M-3: Python pilot ----------
//
// Mirrors the Rust path. The Python algorithm only inspects top-level
// `def` (and `async def`) and rejects calls that are not bare
// identifier(identifier, identifier). Methods inside class bodies,
// keyword arguments, and *args / **kwargs are out of v0 scope and
// covered by negative tests.

#[test]
fn t12_python_swap_detected() {
    let src = "def copy(dst, src):\n    return dst + src\n\ndef driver():\n    dst = 1\n    src = 2\n    _ = copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected exactly 1 Python swap finding, got {:#?}",
        findings
    );
    assert_eq!(findings[0].detector_id, "arg-swap");
    assert_eq!(findings[0].anomaly_class, AnomalyClass::Interface);
}

#[test]
fn t13_python_no_finding_when_arg_names_dont_match_params() {
    let src = "def copy(dst, src):\n    return dst + src\n\ndef driver():\n    d = 1\n    s = 2\n    _ = copy(d, s)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "Python: no name match must not trigger, got {:#?}",
        findings
    );
}

#[test]
fn t14_python_no_finding_on_correct_order() {
    let src = "def copy(dst, src):\n    return dst + src\n\ndef driver():\n    dst = 1\n    src = 2\n    _ = copy(dst, src)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "Python: correct order must not trigger, got {:#?}",
        findings
    );
}

#[test]
fn t15_python_keyword_args_skipped() {
    // copy(src=src, dst=dst) reorders by name, not position. v0 skips
    // any call with a keyword argument.
    let src = "def copy(dst, src):\n    return dst + src\n\ndef driver():\n    dst = 1\n    src = 2\n    _ = copy(src=src, dst=dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "Python: keyword args must skip, got {:#?}",
        findings
    );
}

#[test]
fn t16_python_splat_args_skipped() {
    let src =
        "def copy(dst, src):\n    return dst + src\n\ndef driver(args):\n    _ = copy(*args)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "Python: *args must skip, got {:#?}",
        findings
    );
}

#[test]
fn t17_python_three_arg_function_skipped() {
    let src = "def three(a, b, c):\n    return a + b + c\n\ndef driver():\n    a = 1\n    b = 2\n    c = 3\n    _ = three(c, b, a)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "Python: 3-ary out of v0 scope, got {:#?}",
        findings
    );
}

#[test]
fn t18_python_typed_params_resolve_correctly() {
    let src = "def copy(dst: int, src: int) -> int:\n    return dst + src\n\ndef driver():\n    dst = 1\n    src = 2\n    _ = copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "typed parameters must still resolve, got {:#?}",
        findings
    );
}

#[test]
fn t19_python_decorated_definition_resolves() {
    let src = "from functools import lru_cache\n\n@lru_cache\ndef copy(dst, src):\n    return dst + src\n\ndef driver():\n    dst = 1\n    src = 2\n    _ = copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "decorated def must still resolve, got {:#?}",
        findings
    );
}

#[test]
fn t20_python_async_def_resolves() {
    let src = "async def copy(dst, src):\n    return dst + src\n\nasync def driver():\n    dst = 1\n    src = 2\n    _ = await copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "async def must still resolve, got {:#?}",
        findings
    );
}

#[test]
fn t21_python_class_method_not_top_level_skipped() {
    // v0 only inspects module-top-level def. Methods inside a class body
    // are not collected as definitions and so cannot resolve calls.
    let src = "class C:\n    def copy(self, dst, src):\n        return dst + src\n\ndef driver():\n    c = C()\n    dst = 1\n    src = 2\n    _ = c.copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "method calls and class bodies are out of v0 scope, got {:#?}",
        findings
    );
}

#[test]
fn t22_python_findings_carry_confirmed_status() {
    // Allamanis et al NeurIPS 2021 (PyBugLab + PyPIBugs) is the Python
    // grounding for arg-swap; see docs/surveys/arg-swap-python-2026-05.md.
    let src = "def copy(dst, src):\n    return dst + src\n\ndef driver():\n    dst = 1\n    src = 2\n    _ = copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(!findings.is_empty(), "prerequisite: must produce findings");
    for f in &findings {
        assert!(
            matches!(
                f.evidence.language_citation_status,
                LanguageCitationStatus::Confirmed
            ),
            "Python finding must carry Confirmed per citations-policy.md (allamanis-neurips-2021); got {:?}",
            f.evidence.language_citation_status
        );
        assert!(
            f.evidence.citation_keys.contains(&"allamanis-neurips-2021"),
            "Python finding must include allamanis-neurips-2021 in citation_keys; got {:?}",
            f.evidence.citation_keys
        );
    }
}

#[test]
fn t23_rust_findings_remain_confirmed_in_mixed_scan() {
    let rust_src = r#"
fn copy(dst: i32, src: i32) {
    let _ = (dst, src);
}

fn caller() {
    let dst = 1;
    let src = 2;
    copy(src, dst);
}
"#;
    let py_src = "def copy(dst, src):\n    return dst + src\n\ndef driver():\n    dst = 1\n    src = 2\n    _ = copy(src, dst)\n";
    let findings = run(vec![
        parsed("a.rs", rust_src),
        parsed_python("a.py", py_src),
    ]);
    assert_eq!(
        findings.len(),
        2,
        "expected one finding per language, got {:#?}",
        findings
    );
    for f in &findings {
        assert!(
            matches!(
                f.evidence.language_citation_status,
                LanguageCitationStatus::Confirmed
            ),
            "every finding must be Confirmed in mixed scan; got {:?} for {}",
            f.evidence.language_citation_status,
            f.primary.file.display()
        );
    }
    // The Rust finding cites only the Rust-grounded papers; the Python
    // finding additionally cites Allamanis et al.
    let rust_finding = findings
        .iter()
        .find(|f| f.primary.file.extension().and_then(|e| e.to_str()) == Some("rs"))
        .expect("Rust finding present");
    let py_finding = findings
        .iter()
        .find(|f| f.primary.file.extension().and_then(|e| e.to_str()) == Some("py"))
        .expect("Python finding present");
    assert!(!rust_finding
        .evidence
        .citation_keys
        .contains(&"allamanis-neurips-2021"));
    assert!(py_finding
        .evidence
        .citation_keys
        .contains(&"allamanis-neurips-2021"));
}

#[test]
fn t24_python_and_rust_defs_do_not_cross_match() {
    // Same callee name `copy(dst, src)` defined ONLY in Rust, called in
    // Python. The Python pipeline must not see the Rust def. Therefore
    // the Python call resolves against zero defs and produces no
    // finding even though arg names suggest a swap.
    let rust_src = r#"
fn copy(dst: i32, src: i32) {
    let _ = (dst, src);
}
"#;
    let py_src = "def driver():\n    dst = 1\n    src = 2\n    _ = copy(src, dst)\n";
    let findings = run(vec![
        parsed("a.rs", rust_src),
        parsed_python("a.py", py_src),
    ]);
    assert!(
        findings.is_empty(),
        "Python call must not resolve against a Rust definition, got {:#?}",
        findings
    );
}
