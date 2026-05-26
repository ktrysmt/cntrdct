//! Integration tests for the pr-miner detector v0.1 (Rust + Python).
//!
//! Spec: `docs/spec/pr-miner-v0.md`. Coverage matrix:
//! - T1, T4-T7, T9-T12 (Rust scenarios) — below.
//! - T2, T3, T8, T13 (Python and mixed-language scenarios) — below.
//! - T14, T15 (suppression) — `tests/suppression.rs`, because
//!   suppression is wired at the CLI layer.
//!
//! Filler design: spec `pr-miner-v0.md` test-plan preamble requires every
//! fixture to pad its scenario with filler functions until the total
//! transaction count is at least `MIN_DATABASE_SIZE = 20`. Each filler
//! here calls the SAME two distinct identifiers (`filler_a()` and
//! `filler_b()`), which mines a separate `filler_a <-> filler_b` rule
//! whose support is fully satisfied by every filler — no spurious
//! violations.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus, Severity,
};
use cntrdct::detectors::pr_miner::{PrMinerDetector, MAX_RELATED};
use cntrdct::ir::IrFile;

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

fn parsed_rust(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Rust, src.to_string())
        .expect("ir_from_source")
}

fn run(files: Vec<IrFile>) -> Vec<Finding> {
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

fn t1_corpus() -> Vec<IrFile> {
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
    // F4b dilution: add enough fillers to keep `a`'s cardinality below
    // MAX_ITEM_CARDINALITY = 0.5. With 33 pair_both + 1 violator + 35
    // fillers (69 total), `a` sits at 34/69 = 0.493 < 0.5. The filler
    // rule's cardinality (35/69 = 0.507) exceeds the threshold and the
    // filler_a -> filler_b rule is dropped, but no filler is a violator
    // of itself, so no spurious finding leaks into the assertion.
    src.push_str(&fillers(35));
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

// ---------- Python helpers (v0.1) ----------

fn parsed_python(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Python, src.to_string())
        .expect("ir_from_source")
}

/// Build N Python filler functions, each calling the same `filler_a();
/// filler_b()` pair. Mirrors the Rust `fillers()` helper above.
fn python_fillers(n: usize) -> String {
    let mut out = String::new();
    for i in 0..n {
        out.push_str(&format!(
            "def py_filler_{i}():\n    filler_a()\n    filler_b()\n"
        ));
    }
    out
}

// ---------- T2: Python pair violation ----------

#[test]
fn t2_python_open_close_pair_with_one_violator() {
    let mut src = String::new();
    for i in 0..9 {
        src.push_str(&format!(
            "def py_good_{i}():\n    open_handle()\n    close_handle()\n"
        ));
    }
    src.push_str("def py_lone_violator():\n    open_handle()\n    py_helper()\n");
    src.push_str(&python_fillers(10));
    let findings = run(vec![parsed_python("t2.py", &src)]);
    let open_close: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("open_handle") && rhs == Some("close_handle")
        })
        .collect();
    assert_eq!(
        open_close.len(),
        1,
        "expected exactly one open_handle->close_handle violation, got: {:#?}",
        findings
    );
    assert_eq!(open_close[0].detector_id, "pr-miner");
}

// ---------- T3: cross-language rule mined from mixed corpus ----------
//
// Spec: "5 Rust fns + 5 Python fns calling lock(); unlock(); 1 Rust fn
// calling lock(); helper() only; expect 1 Finding (rule mined cross-
// language)". Both languages share the literal identifier `lock` /
// `unlock`, so the Apriori miner sees them as the same items —
// confirming the spec F3 single-shared-database design.

#[test]
fn t3_cross_language_rule_fires_across_corpus() {
    let mut rust_src = String::new();
    for i in 0..5 {
        rust_src.push_str(&format!(
            "fn rust_good_{i}() {{\n    lock();\n    unlock();\n}}\n"
        ));
    }
    rust_src.push_str("fn rust_violator() {\n    lock();\n    helper();\n}\n");
    // 12 rust fillers (not 9) so the `lock` cardinality stays below the
    // F4b MAX_ITEM_CARDINALITY = 0.5 threshold. Total: 5+1+12+5 = 23
    // transactions, lock in 11/23 = 0.478.
    rust_src.push_str(&fillers(12));

    let mut py_src = String::new();
    for i in 0..5 {
        py_src.push_str(&format!("def py_good_{i}():\n    lock()\n    unlock()\n"));
    }

    let findings = run(vec![
        parsed_rust("t3.rs", &rust_src),
        parsed_python("t3.py", &py_src),
    ]);

    let lock_unlock: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("lock") && rhs == Some("unlock")
        })
        .collect();
    assert_eq!(
        lock_unlock.len(),
        1,
        "expected one lock->unlock violation from the mixed corpus, got: {:#?}",
        findings
    );
}

// ---------- T8: Python finding language_citation_status ----------

#[test]
fn t8_python_findings_are_unconfirmed() {
    // Reuse T2's corpus.
    let mut src = String::new();
    for i in 0..9 {
        src.push_str(&format!(
            "def py_good_{i}():\n    open_handle()\n    close_handle()\n"
        ));
    }
    src.push_str("def py_lone_violator():\n    open_handle()\n    py_helper()\n");
    src.push_str(&python_fillers(10));
    let findings = run(vec![parsed_python("t8.py", &src)]);
    let py: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("open_handle") && rhs == Some("close_handle")
        })
        .collect();
    assert_eq!(py.len(), 1);
    assert_eq!(
        py[0].evidence.language_citation_status,
        LanguageCitationStatus::Unconfirmed,
        "Python findings ship Unconfirmed per docs/surveys/pr-miner-python-2026-05.md"
    );
    // citation_keys still includes li-zhou-fse-2005 — it satisfies the
    // overall P1 gate even when the per-language grounding is
    // Unconfirmed (citations-policy.md SHOULD-not-MUST clause).
    assert!(py[0].evidence.citation_keys.contains(&"li-zhou-fse-2005"));
}

// ---------- T13: mixed-language synonym pair does not mine cross-rule ----------
//
// Rust calls lock()/unlock(); Python calls acquire()/release(). The
// identifiers are different strings, so no rule should pair "lock" with
// "acquire" or "unlock" with "release" — the miner is purely textual.

#[test]
fn t13_mixed_language_synonym_pair_yields_no_cross_rule() {
    let mut rust_src = String::new();
    for i in 0..6 {
        rust_src.push_str(&format!(
            "fn rust_locker_{i}() {{\n    lock();\n    unlock();\n}}\n"
        ));
    }

    let mut py_src = String::new();
    for i in 0..6 {
        py_src.push_str(&format!(
            "def py_acquirer_{i}():\n    acquire()\n    release()\n"
        ));
    }
    py_src.push_str(&python_fillers(8));

    let findings = run(vec![
        parsed_rust("t13.rs", &rust_src),
        parsed_python("t13.py", &py_src),
    ]);

    let cross_pairs = [
        ("lock", "acquire"),
        ("acquire", "lock"),
        ("unlock", "release"),
        ("release", "unlock"),
        ("lock", "release"),
        ("release", "lock"),
        ("unlock", "acquire"),
        ("acquire", "unlock"),
    ];
    for f in &findings {
        let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
        let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
        for (a, b) in &cross_pairs {
            assert!(
                !(lhs == Some(*a) && rhs == Some(*b)),
                "miner produced spurious cross-language rule {} -> {}: {:#?}",
                a,
                b,
                f
            );
        }
    }
}

// ---------- F4c R6: stop-listed items never reach the mining database ----------
//
// Spec `docs/spec/pr-miner-v0.md` F4c (R6 — per-language stop-list):
// stop-listed items are dropped from each transaction before mining,
// so the rules they would participate in are never even mined. This
// fixture exercises the empirical Python FM-A pathology:
// `TypeError -> isinstance` mined across click validators, which the
// stop-list eliminates by removing both items from every transaction.

#[test]
fn f4c_stoplisted_items_do_not_appear_in_mined_rules() {
    let mut src = String::new();
    for i in 0..14 {
        src.push_str(&format!(
            "def validate_{i}(x):\n    if not isinstance(x, int): raise TypeError('bad')\n    return x + {i}\n"
        ));
    }
    for i in 0..6 {
        src.push_str(&format!(
            "def unguarded_{i}(x):\n    if x is None: raise TypeError('none')\n    return x\n"
        ));
    }
    // Plus enough fillers to keep the database above MIN_DATABASE_SIZE,
    // with low-cardinality unique items.
    for i in 0..12 {
        src.push_str(&format!(
            "def py_filler_{i}():\n    py_filler_a_{i}()\n    py_filler_b_{i}()\n"
        ));
    }
    let findings = run(vec![parsed_python("type_error.py", &src)]);

    // No finding may carry TypeError or isinstance on either side.
    for f in &findings {
        let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
        let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
        for stop in &["TypeError", "isinstance"] {
            assert!(
                lhs != Some(*stop) && rhs != Some(*stop),
                "R6 stop-list must keep {stop} out of mined rules; finding: {:#?}",
                f
            );
        }
    }
}

// ---------- F4b R7: stdlib-constructor co-occurrence is filtered ----------
//
// Spec `docs/spec/pr-miner-v0.md` F4b (R7 — item-cardinality post-filter):
// the v0.1 calibration corpus shipped 21/22 FPs from rules whose LHS or
// RHS was a stdlib constructor / builtin appearing in the majority of
// functions (Rust `Err -> Ok` in 19 cases, Python `TypeError -> isinstance`
// in 2). This fixture exercises the failure mode: 20 functions calling
// `Err(...)` paired with `Ok(...)` and 4 lone `Err`-callers. Without F4b
// the miner emits a high-confidence `Err -> Ok` rule and flags the 4
// lone callers; with F4b at MAX_ITEM_CARDINALITY = 0.5 the rule is
// dropped because `Err`'s cardinality is 24/24 = 1.0.

#[test]
fn f4b_stdlib_constructor_cooccurrence_filtered_out() {
    let mut src = String::new();
    for i in 0..20 {
        src.push_str(&format!(
            "fn fallible_{i}() -> Result<i32, ()> {{\n    if {i} == 0 {{ return Err(()); }}\n    Ok({i})\n}}\n"
        ));
    }
    for i in 0..4 {
        src.push_str(&format!(
            "fn lone_err_{i}() -> Result<i32, ()> {{\n    Err(())\n}}\n"
        ));
    }
    // Plus enough fillers to keep the database well above
    // MIN_DATABASE_SIZE; their items stay low-cardinality (each filler
    // pair is unique).
    for i in 0..8 {
        src.push_str(&format!(
            "fn ufiller_{i}() {{\n    ufiller_a_{i}();\n    ufiller_b_{i}();\n}}\n"
        ));
    }
    let findings = run(vec![parsed_rust("err_ok.rs", &src)]);

    // The Err -> Ok rule must NOT mine, so no lone_err_* function is a
    // violator under it.
    let err_ok_violations: Vec<&Finding> = findings
        .iter()
        .filter(|f| {
            let lhs = f.evidence.raw.get("rule_lhs").and_then(|v| v.as_str());
            let rhs = f.evidence.raw.get("rule_rhs").and_then(|v| v.as_str());
            lhs == Some("Err") && rhs == Some("Ok")
        })
        .collect();
    assert!(
        err_ok_violations.is_empty(),
        "F4b must drop the Err -> Ok rule (Err is universally present); got: {:#?}",
        err_ok_violations
    );
}
