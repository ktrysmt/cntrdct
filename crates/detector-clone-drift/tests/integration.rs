//! Integration tests for the clone-drift detector v0 spec.
//!
//! Each test maps to a row in `cntrdct/docs/spec/clone-drift-v0.md` test plan.

use std::path::PathBuf;

use cntrdct_core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus, ParsedFile,
};
use cntrdct_detector_clone_drift::CloneDrift;

fn parsed(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: Language::Rust,
        source: src.to_string(),
    }
}

fn parsed_py(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: Language::Python,
        source: src.to_string(),
    }
}

fn run(files: Vec<ParsedFile>) -> Vec<Finding> {
    let detector = CloneDrift::new();
    register_detector(&detector).expect("clone-drift must satisfy P1");
    let stats = CorpusStats::default();
    let config = DetectorConfig::default();
    let ctx = DetectContext {
        files: &files,
        stats: &stats,
        config: &config,
    };
    detector.detect(&ctx).expect("detect must not error")
}

const FN_BASE: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 {
            result.push(item * 2);
        }
    }
    result
}
"#;

const FN_DRIFTED: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 && item < 100 {
            result.push(item * 2);
        }
    }
    result
}
"#;

#[test]
fn t1_drift_detected_4_identical_plus_1_modified() {
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 drift finding, got {}: {:#?}",
        findings.len(),
        findings
    );
    assert_eq!(findings[0].primary.file, PathBuf::from("e.rs"));
    assert_eq!(findings[0].related.len(), 4);
    assert_eq!(findings[0].detector_id, "clone-drift");
}

const FN_RENAME_A: &str = "fn alpha(a: i32) -> i32 { let r = a + 1; r }";
const FN_RENAME_B: &str = "fn beta(b: i32) -> i32 { let s = b + 1; s }";
const FN_RENAME_C: &str = "fn gamma(c: i32) -> i32 { let t = c + 1; t }";
const FN_RENAME_D: &str = "fn delta(d: i32) -> i32 { let u = d + 1; u }";
const FN_RENAME_E: &str = "fn epsilon(e: i32) -> i32 { let v = e + 1; v }";

#[test]
fn t2_no_drift_when_all_normalized_identical() {
    let files = vec![
        parsed("a.rs", FN_RENAME_A),
        parsed("b.rs", FN_RENAME_B),
        parsed("c.rs", FN_RENAME_C),
        parsed("d.rs", FN_RENAME_D),
        parsed("e.rs", FN_RENAME_E),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "expected no drift on identical-after-normalization, got {:#?}",
        findings
    );
}

const FN_VARIANT_A: &str = r#"
fn parse_value(input: &str) -> Result<i32, String> {
    let trimmed = input.trim();
    let value = trimmed.parse::<i32>().map_err(|e| e.to_string())?;
    Ok(value)
}
"#;

const FN_VARIANT_B: &str = r#"
fn parse_value(input: &str) -> Result<i32, String> {
    let trimmed = input.trim().to_lowercase();
    let value = trimmed.parse::<i32>().map_err(|e| e.to_string())?;
    Ok(value)
}
"#;

#[test]
fn t3_no_drift_on_two_two_split() {
    let files = vec![
        parsed("a1.rs", FN_VARIANT_A),
        parsed("a2.rs", FN_VARIANT_A),
        parsed("b1.rs", FN_VARIANT_B),
        parsed("b2.rs", FN_VARIANT_B),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "expected no drift on 2-vs-2 split, got {:#?}",
        findings
    );
}

#[test]
fn t4_no_drift_below_minimum_group_size() {
    let files = vec![parsed("a.rs", FN_BASE), parsed("b.rs", FN_BASE)];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "group of size 2 must not trigger; got {:#?}",
        findings
    );
}

#[test]
fn t5_drift_on_one_two_split() {
    let files = vec![
        parsed("solo.rs", FN_VARIANT_A),
        parsed("pair1.rs", FN_VARIANT_B),
        parsed("pair2.rs", FN_VARIANT_B),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 drift finding, got {:#?}",
        findings
    );
    assert_eq!(findings[0].primary.file, PathBuf::from("solo.rs"));
    assert_eq!(findings[0].related.len(), 2);
}

const FN_UNRELATED_X: &str = r#"
fn fibonacci(n: u32) -> u64 {
    if n <= 1 { return n as u64; }
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}
"#;

const FN_UNRELATED_Y: &str = r#"
fn parse_url_scheme(url: &str) -> String {
    let parts: Vec<&str> = url.split("://").collect();
    let scheme = parts[0].to_string();
    let host = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
    format!("scheme={} host={}", scheme, host)
}
"#;

#[test]
fn t6_no_clones_when_unrelated() {
    let files = vec![
        parsed("a.rs", FN_UNRELATED_X),
        parsed("b.rs", FN_UNRELATED_Y),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "unrelated fns must not cluster, got {:#?}",
        findings
    );
}

#[test]
fn t7_every_finding_has_known_citation() {
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    let known: &[&str] = &[
        "cordy-roy-icpc-2008",
        "bettenburg-msr-2009",
        "krinke-icsm-2007",
    ];
    assert!(
        !findings.is_empty(),
        "T7 prerequisite: must produce findings"
    );
    for f in &findings {
        assert!(
            !f.evidence.citation_keys.is_empty(),
            "P1: empty citation_keys"
        );
        assert!(
            f.evidence.citation_keys.iter().any(|k| known.contains(k)),
            "P1: citations {:?} contain no recognized clone-drift key",
            f.evidence.citation_keys
        );
    }
}

#[test]
fn t8_deterministic_repeatable() {
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFTED),
    ];
    let first = run(files.clone());
    let second = run(files);
    let json1 = serde_json::to_string(&first).unwrap();
    let json2 = serde_json::to_string(&second).unwrap();
    assert_eq!(json1, json2, "detect() must be deterministic");
}

#[test]
fn t9_empty_input_returns_empty() {
    let findings = run(vec![]);
    assert!(findings.is_empty());
}

const FN_SYNTAX_ERROR: &str = "fn broken( { let x = ; }";

#[test]
fn t_anomaly_class_is_logic_for_every_finding() {
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert!(
        !findings.is_empty(),
        "prerequisite: must produce findings to assert their anomaly_class"
    );
    for f in &findings {
        assert_eq!(
            f.anomaly_class,
            AnomalyClass::Logic,
            "clone-drift findings must classify as Logic per IEEE 1044-2009; got {:?}",
            f.anomaly_class
        );
    }
}

// ---------- Python pilot (M-3) ----------

const FN_PY_BASE: &str = r#"
def process(items):
    out = []
    for it in items:
        if it > 0:
            out.append(it * 2)
    return out
"#;

const FN_PY_DRIFTED: &str = r#"
def process(items):
    out = []
    for it in items:
        if it > 0 and it < 100:
            out.append(it * 2)
    return out
"#;

#[test]
fn t11_python_drift_detected_4_identical_plus_1_modified() {
    let files = vec![
        parsed_py("a.py", FN_PY_BASE),
        parsed_py("b.py", FN_PY_BASE),
        parsed_py("c.py", FN_PY_BASE),
        parsed_py("d.py", FN_PY_BASE),
        parsed_py("e.py", FN_PY_DRIFTED),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 Python drift finding, got {}: {:#?}",
        findings.len(),
        findings
    );
    assert_eq!(findings[0].primary.file, PathBuf::from("e.py"));
    assert_eq!(findings[0].related.len(), 4);
    assert_eq!(findings[0].detector_id, "clone-drift");
}

#[test]
fn t12_python_findings_emit_confirmed_status() {
    let files = vec![
        parsed_py("a.py", FN_PY_BASE),
        parsed_py("b.py", FN_PY_BASE),
        parsed_py("c.py", FN_PY_BASE),
        parsed_py("d.py", FN_PY_BASE),
        parsed_py("e.py", FN_PY_DRIFTED),
    ];
    let findings = run(files);
    assert!(
        !findings.is_empty(),
        "prerequisite: must produce findings to assert language_citation_status"
    );
    for f in &findings {
        assert_eq!(
            f.evidence.language_citation_status,
            LanguageCitationStatus::Confirmed,
            "Python clone-drift findings must emit Confirmed (assi-tosem-2025)",
        );
        assert!(
            f.evidence.citation_keys.contains(&"assi-tosem-2025"),
            "Python findings must carry assi-tosem-2025 in citation_keys; got {:?}",
            f.evidence.citation_keys,
        );
    }
}

#[test]
fn t13_rust_findings_still_emit_confirmed_status() {
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert!(
        !findings.is_empty(),
        "T13 prerequisite: must produce findings"
    );
    for f in &findings {
        assert_eq!(
            f.evidence.language_citation_status,
            LanguageCitationStatus::Confirmed,
            "Rust clone-drift findings must remain Confirmed (grandfathered v0)",
        );
        assert!(
            !f.evidence.citation_keys.contains(&"assi-tosem-2025"),
            "Rust findings must NOT carry the Python-only assi-tosem-2025 key; got {:?}",
            f.evidence.citation_keys,
        );
    }
}

#[test]
fn t14_mixed_scan_does_not_cross_match_languages() {
    // Rust and Python pipelines must run in isolation. Same-shape fns
    // in different languages must not group together.
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed_py("c.py", FN_PY_BASE),
        parsed_py("d.py", FN_PY_BASE),
        parsed_py("e.py", FN_PY_DRIFTED),
    ];
    let findings = run(files);
    // No Rust drift: only 2 Rust fns, < MIN_GROUP_SIZE.
    // No Python cross-pollination: only 3 Python fns visible to the Python
    // pipeline; partition is 2 + 1 → drift signal triggers on e.py.
    assert_eq!(
        findings.len(),
        1,
        "expected exactly one Python finding, no Rust cross-match: {:#?}",
        findings,
    );
    assert_eq!(findings[0].primary.file, PathBuf::from("e.py"));
    assert!(
        findings[0]
            .related
            .iter()
            .all(|loc| loc.file.extension().and_then(|s| s.to_str()) == Some("py")),
        "Python finding's related set must contain only .py files",
    );
}

#[test]
fn t10_skip_parse_errors_safely() {
    let files = vec![
        parsed("ok1.rs", FN_BASE),
        parsed("ok2.rs", FN_BASE),
        parsed("broken.rs", FN_SYNTAX_ERROR),
        parsed("ok3.rs", FN_BASE),
        parsed("drift.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "valid drift fixture should still surface despite broken file: {:#?}",
        findings
    );
    assert_eq!(findings[0].primary.file, PathBuf::from("drift.rs"));
}
