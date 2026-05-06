//! Integration tests for the pr-miner detector v0.0 (Rust only).
//!
//! Spec: `docs/spec/pr-miner-v0.md`. The test-plan IDs (T1, T4-T7, T9-T12)
//! covered here are the Rust-scoped subset of the full plan; T2, T3, T8,
//! T13, T15 land alongside the Python extension in v0.1, and T14 lives in
//! `crates/cli/tests/suppression.rs` because suppression is wired at the
//! CLI layer.
//!
//! Filler design: spec `pr-miner-v0.md` test-plan preamble requires every
//! fixture to pad its scenario with filler functions until the total
//! transaction count is at least `MIN_DATABASE_SIZE = 20`. Each filler
//! here calls the SAME two distinct identifiers (`filler_a()` and
//! `filler_b()`), which mines a separate `filler_a <-> filler_b` rule
//! whose support is fully satisfied by every filler — no spurious
//! violations.

use std::path::PathBuf;

use cntrdct_core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus, ParsedFile, Severity,
};
use cntrdct_detector_pr_miner::{PrMinerDetector, MAX_RELATED};

/// Build N filler functions that each call `filler_a(); filler_b();`. The
/// shared pair contributes a high-confidence `filler_a <-> filler_b` rule
/// satisfied by every filler, so no spurious violations leak into the
/// scenario being tested. Returns the rendered Rust source.
fn fillers(n: usize) -> String {
    let mut out = String::new();
    for i in 0..n {
        out.push_str(&format!(
            "fn filler_{i}() {{\n    filler_a();\n    filler_b();\n}}\n"
        ));
    }
    out
}

fn parsed_rust(name: &str, src: &str) -> ParsedFile {
    ParsedFile {
        path: PathBuf::from(name),
        language: Language::Rust,
        source: src.to_string(),
    }
}

fn run(files: Vec<ParsedFile>) -> Vec<Finding> {
    let detector = PrMinerDetector::new();
    register_detector(&detector).expect("pr-miner must satisfy P1");
    let stats = CorpusStats::default();
    let config = DetectorConfig::default();
    let ctx = DetectContext {
        files: &files,
        stats: &stats,
        config: &config,
    };
    detector.detect(&ctx).expect("detect must not error")
}

// ---------- T1: single-violation scenario ----------

fn t1_corpus() -> Vec<ParsedFile> {
    let mut src = String::new();
    for i in 0..9 {
        src.push_str(&format!(
            "fn good_{i}() {{\n    acquire();\n    release();\n}}\n"
        ));
    }
    src.push_str("fn lone_violator() {\n    acquire();\n    helper();\n}\n");
    src.push_str(&fillers(10));
    vec![parsed_rust("t1.rs", &src)]
}

#[test]
fn t1_acquire_release_pair_with_one_violator() {
    let findings = run(t1_corpus());
    let acquire_release: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str()) == Some("acquire")
                && f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str()) == Some("release")
        })
        .collect();
    assert_eq!(
        acquire_release.len(),
        1,
        "expected exactly one acquire->release violation, got: {:#?}",
        findings
    );
    let f = acquire_release[0];
    assert_eq!(f.detector_id, "pr-miner");
    assert!(matches!(f.raw_severity, Severity::Warning));
    assert_eq!(f.anomaly_class, AnomalyClass::Logic);
    assert_eq!(
        f.related.len(),
        9,
        "related must list the 9 satisfying functions"
    );
}

// ---------- T4: low-confidence direction is dropped ----------
//
// Spec literal text: "9 fns each calling acquire(); helper() only; 1 fn
// calling acquire(); release()". A literal 2-call lone fn would still
// violate the acquire->helper rule (mined at confidence 0.9), producing
// 1 finding rather than the expected 0. To honour the spec's stated
// intent ("confidence too low") the lone fn here calls acquire, helper,
// AND release, satisfying the acquire->helper rule (so it is not a
// violator) while keeping acquire->release confidence at 1/10 = 0.10
// — well below MIN_CONFIDENCE = 0.85, so the rule is not mined.

#[test]
fn t4_low_confidence_pair_yields_no_findings() {
    let mut src = String::new();
    for i in 0..9 {
        src.push_str(&format!(
            "fn good_{i}() {{\n    acquire();\n    helper();\n}}\n"
        ));
    }
    src.push_str("fn carries_release() {\n    acquire();\n    helper();\n    release();\n}\n");
    src.push_str(&fillers(10));
    let findings = run(vec![parsed_rust("t4.rs", &src)]);
    let acquire_release: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("acquire") && rhs == Some("release")
        })
        .collect();
    assert!(
        acquire_release.is_empty(),
        "acquire->release confidence is below threshold; rule must not be mined: {:#?}",
        findings
    );
}

// ---------- T5: support below threshold ----------
//
// Spec literal text: "4 fns calling pair, 16 calling neither". With 20
// transactions a 4-occurrence pair has support 0.20, well above
// MIN_SUPPORT = 0.05. To exercise the support floor we drop the pair
// occurrence to 1 over 30 transactions: 1/30 ≈ 0.033 < 0.05.

#[test]
fn t5_support_below_threshold_yields_no_findings() {
    let mut src = String::new();
    src.push_str("fn lone_pair() {\n    a();\n    b();\n}\n");
    src.push_str(&fillers(29));
    let findings = run(vec![parsed_rust("t5.rs", &src)]);
    assert!(
        findings.iter().all(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            !(lhs == Some("a") && rhs == Some("b") || lhs == Some("b") && rhs == Some("a"))
        }),
        "{{a, b}} support is below threshold; no rule should be mined: {:#?}",
        findings
    );
}

// ---------- T6: determinism ----------

#[test]
fn t6_identical_input_yields_identical_output() {
    let run_a = run(t1_corpus());
    let run_b = run(t1_corpus());
    let serialise =
        |findings: &[Finding]| serde_json::to_string(findings).expect("findings must serialise");
    assert_eq!(
        serialise(&run_a),
        serialise(&run_b),
        "pr-miner must be deterministic across runs"
    );
}

// ---------- T7: citation key ----------

#[test]
fn t7_findings_carry_li_zhou_citation() {
    let findings = run(t1_corpus());
    assert!(!findings.is_empty(), "expected at least one finding");
    for f in &findings {
        assert!(
            f.evidence.citation_keys.contains(&"li-zhou-fse-2005"),
            "every pr-miner finding must include li-zhou-fse-2005: {:?}",
            f.evidence.citation_keys
        );
        assert_eq!(
            f.evidence.language_citation_status,
            LanguageCitationStatus::Confirmed,
            "Rust findings ground in li-zhou-fse-2005 are Confirmed"
        );
    }
}

// ---------- T9: empty input ----------

#[test]
fn t9_empty_input_yields_no_findings_no_error() {
    let findings = run(Vec::new());
    assert!(findings.is_empty());
}

// ---------- T10: parse error tolerated ----------

#[test]
fn t10_parse_error_file_is_skipped_violation_still_detected() {
    let mut files = t1_corpus();
    files.push(parsed_rust("broken.rs", "fn f( { unbalanced }\n"));
    let findings = run(files);
    let acquire_release_violations: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("acquire") && rhs == Some("release")
        })
        .collect();
    assert_eq!(
        acquire_release_violations.len(),
        1,
        "parse-error file must be skipped without affecting the T1 violation: {:#?}",
        findings
    );
}

// ---------- T11: both directions mined ----------

#[test]
fn t11_both_rule_directions_emit_independent_violations() {
    let mut src = String::new();
    for i in 0..9 {
        src.push_str(&format!("fn pair_both_{i}() {{\n    a();\n    b();\n}}\n"));
    }
    src.push_str("fn only_a() {\n    a();\n    other_a();\n}\n");
    src.push_str("fn only_b() {\n    b();\n    other_b();\n}\n");
    src.push_str(&fillers(9));
    let findings = run(vec![parsed_rust("t11.rs", &src)]);

    let a_to_b: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("a") && rhs == Some("b")
        })
        .collect();
    let b_to_a: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("b") && rhs == Some("a")
        })
        .collect();

    assert_eq!(
        a_to_b.len(),
        1,
        "expected one a->b violation: {:#?}",
        findings
    );
    assert_eq!(
        b_to_a.len(),
        1,
        "expected one b->a violation: {:#?}",
        findings
    );
}

// ---------- T12: MAX_RELATED cap ----------

#[test]
fn t12_related_is_capped_and_flag_is_set() {
    let mut src = String::new();
    let satisfying = MAX_RELATED + 1;
    for i in 0..satisfying {
        src.push_str(&format!("fn pair_both_{i}() {{\n    a();\n    b();\n}}\n"));
    }
    src.push_str("fn lone_violator() {\n    a();\n    other();\n}\n");
    let findings = run(vec![parsed_rust("t12.rs", &src)]);
    let a_to_b: &Finding = findings
        .iter()
        .find(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("a") && rhs == Some("b")
        })
        .expect("expected one a->b violation");
    assert_eq!(a_to_b.related.len(), MAX_RELATED);
    assert_eq!(
        a_to_b
            .evidence
            .raw
            .get("related_capped")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
}
