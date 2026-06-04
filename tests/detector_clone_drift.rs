//! Integration tests for the clone-drift detector v0 spec.
//!
//! Each test maps to a row in `cntrdct/docs/spec/clone-drift-v0.md` test plan.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus,
};
use cntrdct::detectors::clone_drift::CloneDrift;
use cntrdct::ir::IrFile;

fn parsed(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Rust, src.to_string())
        .expect("ir_from_source")
}

fn parsed_py(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Python, src.to_string())
        .expect("ir_from_source")
}

fn run(files: Vec<IrFile>) -> Vec<Finding> {
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

// ---------- F5b: scope-bounded clustering ----------
//
// The wild Rust β corpus exposed cross-crate clustering as the
// dominant FP source (112/124). F5b restricts cluster + partition
// to a single scope at a time. The cases below pin the four scope
// inference rules: (1) provenance header, (2) Cargo `/src/` layout,
// (3) filename `__` separator, (4) parent-directory fallback.

fn pf(path: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(path), Language::Rust, src.to_string())
        .expect("ir_from_source")
}

fn pf_with_source(path: &str, header: &str, body: &str) -> IrFile {
    let mut s = String::new();
    s.push_str(header);
    if !header.ends_with('\n') {
        s.push('\n');
    }
    s.push_str(body);
    pf(path, &s)
}

#[test]
fn t20_cargo_src_layout_separates_crates() {
    // Each crate's files live under `<crate>/src/`. The drift sits in
    // crateB; the four base copies sit in crateA. F5b must NOT cluster
    // them together — different scopes.
    let files = vec![
        pf("crateA/src/a.rs", FN_BASE),
        pf("crateA/src/b.rs", FN_BASE),
        pf("crateA/src/c.rs", FN_BASE),
        pf("crateA/src/d.rs", FN_BASE),
        pf("crateB/src/e.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "scopes split by Cargo src layout must not cross-cluster, got {:#?}",
        findings
    );
}

#[test]
fn t20b_non_url_source_line_falls_through_to_next_rule() {
    // F5b: a `// Source: <free-text>` line that does NOT carry a URL
    // (scheme://value) must NOT establish a provenance scope —
    // accepting it would collapse every fixture sharing the same
    // descriptive note into one super-scope and silently suppress
    // real drift findings under F5c-i / F5d-i. The line falls
    // through to the next scope rule (here, Cargo `/src/` layout),
    // so the drift in crateB stays isolated and yields zero findings.
    let header = "// Source: shape adapted from upstream foo family\n";
    let files = vec![
        pf_with_source("crateA/src/a.rs", header, FN_BASE),
        pf_with_source("crateA/src/b.rs", header, FN_BASE),
        pf_with_source("crateA/src/c.rs", header, FN_BASE),
        pf_with_source("crateA/src/d.rs", header, FN_BASE),
        pf_with_source("crateB/src/e.rs", header, FN_DRIFTED),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F5b: non-URL Source line must not cross-cluster crates, got {:#?}",
        findings
    );
}

#[test]
fn t21_same_provenance_clusters_within_scope() {
    // All five files share `// Source: .../crates/foo/...` provenance
    // -> same scope. The drift surfaces normally.
    let header = "// Source: https://static.crates.io/crates/foo/foo-1.0.0.crate";
    let files = vec![
        pf_with_source("benchmarks/wild-corpus/files/foo__a.rs", header, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/foo__b.rs", header, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/foo__c.rs", header, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/foo__d.rs", header, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/foo__e.rs", header, FN_DRIFTED),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "same-provenance scope must surface drift, got {:#?}",
        findings
    );
}

#[test]
fn t22_different_provenance_no_cross_cluster() {
    // Same flat dir but two crates' provenance — must NOT cluster.
    let foo_h = "// Source: https://static.crates.io/crates/foo/foo-1.0.0.crate";
    let bar_h = "// Source: https://static.crates.io/crates/bar/bar-1.0.0.crate";
    let files = vec![
        pf_with_source("benchmarks/wild-corpus/files/foo__a.rs", foo_h, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/foo__b.rs", foo_h, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/foo__c.rs", foo_h, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/foo__d.rs", foo_h, FN_BASE),
        pf_with_source("benchmarks/wild-corpus/files/bar__e.rs", bar_h, FN_DRIFTED),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "different cratesio scopes must not cluster, got {:#?}",
        findings
    );
}

#[test]
fn t23_filename_underscore_prefix_separates_scopes() {
    // No provenance header, but the basename `__` separator carries
    // the crate name. Different prefixes -> different scopes.
    let files = vec![
        pf("files/foo__a.rs", FN_BASE),
        pf("files/foo__b.rs", FN_BASE),
        pf("files/foo__c.rs", FN_BASE),
        pf("files/foo__d.rs", FN_BASE),
        pf("files/bar__e.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "different `__`-prefix scopes must not cluster, got {:#?}",
        findings
    );
}

#[test]
fn t24_bare_names_share_parent_dir_scope_backcompat() {
    // T1 with bare names (no path / provenance / __ separator).
    // All files share the empty-parent scope; existing behaviour
    // is preserved exactly.
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
        "bare-name backcompat must preserve T1 behaviour, got {:#?}",
        findings
    );
}

// ---------- F5c: within-scope tightening (added 2026-05-07) ----------

const FN_VARIANT_C: &str = r#"
fn parse_value(input: &str) -> Result<i32, String> {
    let trimmed = input.trim().trim_start_matches('+');
    let value = trimmed.parse::<i32>().map_err(|e| e.to_string())?;
    Ok(value)
}
"#;

const FN_VARIANT_D: &str = r#"
fn parse_value(input: &str) -> Result<i32, String> {
    let trimmed = input.trim().trim_end_matches('-');
    let value = trimmed.parse::<i32>().map_err(|e| e.to_string())?;
    Ok(value)
}
"#;

#[test]
fn t25_no_drift_on_no_clear_majority() {
    // F5c-i: a cluster of size 4 split into [2, 1, 1] has the largest
    // partition at 50% — not a strict majority. The two singletons
    // could each "look drifted" against the dominant pair, but in a
    // family-of-variants pattern the dominant is not actually a
    // majority. We must NOT fire.
    let files = vec![
        parsed("a.rs", FN_VARIANT_A),
        parsed("b.rs", FN_VARIANT_A),
        parsed("c.rs", FN_VARIANT_C),
        parsed("d.rs", FN_VARIANT_D),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F5c-i: 4-fn cluster split [2, 1, 1] has no strict majority, got {:#?}",
        findings
    );
}

#[test]
fn t26_drift_fires_when_strict_majority_holds() {
    // Sanity check: 3-vs-1 split is strict majority (3*2=6 > 4),
    // and the singleton differs from the dominant by a small drift
    // that keeps Jaccard >= NEAR_DUPLICATE_THRESHOLD.
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "F5c-i: strict-majority [3, 1] still fires, got {:#?}",
        findings
    );
}

const FN_FAMILY_HEAD: &str = r#"
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

const FN_FAMILY_STRUCTURAL_VARIANT: &str = r#"
fn process(items: Vec<i32>, threshold: i32, ctx: &Context, scale: i32) -> Vec<i32> {
    let mut result = Vec::with_capacity(items.len());
    let validator = ctx.validator_for(threshold);
    for (idx, item) in items.into_iter().enumerate() {
        match validator.check(item, idx) {
            Ok(_) if item > threshold => result.push(item * scale),
            Ok(_) => {}
            Err(e) => {
                ctx.log_error(idx, &e);
                continue;
            }
        }
    }
    result
}
"#;

#[test]
fn t27_no_drift_on_low_dominant_jaccard() {
    // F5c-ii: a function pulled into a cluster transitively (Jaccard
    // >= 0.5 with one neighbour) but which differs structurally from
    // the dominant exemplar (Jaccard < NEAR_DUPLICATE_THRESHOLD)
    // is NOT a drift. This is the residual parser-combinator /
    // designed-family-variant case.
    let files = vec![
        parsed("a.rs", FN_FAMILY_HEAD),
        parsed("b.rs", FN_FAMILY_HEAD),
        parsed("c.rs", FN_FAMILY_HEAD),
        parsed("d.rs", FN_FAMILY_HEAD),
        parsed("e.rs", FN_FAMILY_STRUCTURAL_VARIANT),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F5c-ii: structural variant (low Jaccard with dominant) must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t28_evidence_carries_dominant_jaccard() {
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFTED),
    ];
    let findings = run(files);
    assert_eq!(findings.len(), 1);
    let raw = &findings[0].evidence.raw;
    assert!(
        raw.get("dominant_jaccard")
            .and_then(|v| v.as_f64())
            .is_some(),
        "F5c: evidence.raw must include dominant_jaccard, got {raw}"
    );
    assert!(
        raw.get("near_duplicate_threshold")
            .and_then(|v| v.as_f64())
            .is_some(),
        "F5c: evidence.raw must include near_duplicate_threshold, got {raw}"
    );
}

// ---------- F5d: residual sibling-family discriminator (added 2026-05-07) ----------

const FN_DRIFTED_SECOND: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 || item == -1 {
            result.push(item * 2);
        }
    }
    result
}
"#;

#[test]
fn t29_no_drift_on_multi_singleton_cluster() {
    // F5d-i: a cluster with two or more size-1 partitions is the
    // structural signature of a designed family of N variants
    // (e.g. charset_normalizer's `is_<script>` siblings each searching
    // for a different substring), not the textbook drifted-clone
    // shape. 4 base copies + 2 distinct singletons → partition
    // [4, 1, 1] → suppress.
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFTED),
        parsed("f.rs", FN_DRIFTED_SECOND),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F5d-i: multi-singleton cluster must not fire, got {:#?}",
        findings
    );
}

const FN_REPEATED_BODY: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 {
            result.push(item * 2);
            result.push(item * 2);
            result.push(item * 2);
            result.push(item * 2);
        }
    }
    result
}
"#;

#[test]
fn t30_no_drift_on_length_imbalance_with_weak_dominant() {
    // F5d-ii: a singleton that shares high n-gram Jaccard with the
    // dominant exemplar (because repeated body blocks contribute the
    // same n-grams) but whose token-length differs by >
    // LENGTH_IMBALANCE_THRESHOLD AND whose dominant partition holds
    // only 2 members (the F5c-i strict-majority floor for a 3-fn
    // cluster) is the weak-evidence family-of-variants shape. Compare
    // with corpus_005 (dominant size 4, length imbalance 0.258) which
    // stays a TP — see t30b.
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_REPEATED_BODY),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F5d-ii: weak-dominant + length-imbalanced singleton must not fire, got {:#?}",
        findings
    );
}

const FN_DRIFT_ADDS_BREAK: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 {
            result.push(item * 2);
            if item == 0 { break; }
        }
    }
    result
}
"#;

#[test]
fn t30b_strong_dominant_keeps_drift_under_length_imbalance() {
    // F5d-ii is exempt when the dominant partition holds ≥ 3
    // functions: the canonical-form evidence is strong enough that a
    // structurally larger drifted singleton (the textbook "1 of N
    // copies missed an update" shape, e.g. corpus_005 at length
    // imbalance 0.258 with N = 4) still fires. Pinned here so a
    // future tightening of LENGTH_IMBALANCE_DOMINANT_FLOOR cannot
    // silently drop the TP.
    let files = vec![
        parsed("a.rs", FN_BASE),
        parsed("b.rs", FN_BASE),
        parsed("c.rs", FN_BASE),
        parsed("d.rs", FN_BASE),
        parsed("e.rs", FN_DRIFT_ADDS_BREAK),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "F5d-ii exemption: dominant size ≥ 3 must keep length-imbalanced drifts as TPs, got {:#?}",
        findings
    );
    assert_eq!(findings[0].primary.file, PathBuf::from("e.rs"));
}

const FN_TINY_DELEGATE_A: &str = r#"
pub fn parse_a<T: Parse>(s: &str) -> Result<T> {
    Parser::parse_a(T::parse, s)
}
"#;

const FN_TINY_DELEGATE_B: &str = r#"
pub fn parse_b<T: Parse>(s: &str) -> Result<T> {
    Parser::parse_b(T::parse, s)
}
"#;

const FN_TINY_DELEGATE_C: &str = r#"
pub fn parse_c<T: Parse>(t: TokenStream) -> Result<T> {
    Parser::parse_c(T::parse, t)
}
"#;

// ---------- F2b intra-fn if-branch clone (added 2026-05-21) ----------

const FN_IF_SAME_THEN_ELSE: &str = r#"
fn case() {
    if true {
        let _ = Foo { bar: 42 };
        let _ = 0..10;
        let _ = ..;
        let _ = 0..;
        foo();
        bar();
    } else {
        let _ = Foo { bar: 42 };
        let _ = 0..10;
        let _ = ..;
        let _ = 0..;
        foo();
        bar();
    }
}
"#;

#[test]
fn t32_f2b_intra_fn_if_branches_identical_source_fires() {
    // F2b: clippy `if_same_then_else` shape — branches are byte-for-byte
    // identical Rust source (modulo whitespace + comments). Mirrors the
    // audit-corpus `clippy_ui_if_same_then_else.rs:29` expectation.
    let files = vec![parsed("solo.rs", FN_IF_SAME_THEN_ELSE)];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "F2b: identical-source if-branches must fire, got {:#?}",
        findings
    );
    let raw = &findings[0].evidence.raw;
    assert_eq!(
        raw["kind"], "intra-fn-if-same-then-else",
        "F2b emission must carry kind=intra-fn-if-same-then-else, got {raw}"
    );
}

const FN_IF_TYPE_2_VARIANT: &str = r#"
fn case() {
    if cond {
        let _ = self.i.next();
        let _ = self.i.peek();
        let _ = self.i.collect::<Vec<_>>();
        return self.i.first();
    } else {
        let _ = self.j.next();
        let _ = self.j.peek();
        let _ = self.j.collect::<Vec<_>>();
        return self.j.first();
    }
}
"#;

#[test]
fn t33_f2b_does_not_fire_on_type2_identifier_variant() {
    // F2b uses source-text equality, NOT normalised-token equality.
    // Type-2 clones (same shape, different identifiers) are the
    // canonical fan-out-by-argument pattern in real Rust code
    // (itertools, regex_syntax, object on the wild β corpus); a
    // normalised-token comparison would have produced 20 FPs there.
    let files = vec![parsed("itertools.rs", FN_IF_TYPE_2_VARIANT)];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F2b: Type-2 identifier variant must not fire, got {:#?}",
        findings
    );
}

const FN_IF_TINY_IDENTICAL: &str = r#"
fn case() -> u32 {
    if cond { 42 } else { 42 }
}
"#;

#[test]
fn t34_f2b_below_min_tokens_does_not_fire() {
    // `if c { 42 } else { 42 }` normalises to ~7 tokens per branch
    // — well below INTRA_FN_IF_MIN_TOKENS. Stylistic placeholders
    // at this size are common (clippy itself sometimes warns at a
    // smaller token threshold; cntrdct stays conservative for v0).
    let files = vec![parsed("tiny.rs", FN_IF_TINY_IDENTICAL)];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F2b: branches below INTRA_FN_IF_MIN_TOKENS must not fire, got {:#?}",
        findings
    );
}

const FN_ELSE_IF_CHAIN: &str = r#"
fn case(x: u32) {
    if x == 0 {
        let _ = Foo { bar: 42 };
        let _ = 0..10;
        let _ = ..;
        foo();
        bar();
    } else if x == 1 {
        let _ = Foo { bar: 42 };
        let _ = 0..10;
        let _ = ..;
        foo();
        bar();
    } else {
        baz();
    }
}
"#;

#[test]
fn t35_f2b_else_if_chain_outer_pair_does_not_fire() {
    // F2b only fires when the `alternative` is a flat `else { block }`.
    // An `else if ...` chain has another if_expression as the
    // alternative; the inner if (with its own else-block) is checked
    // recursively by the walker, but the OUTER pair (consequence vs
    // else-if alternative) does not compare apples-to-apples.
    let files = vec![parsed("chain.rs", FN_ELSE_IF_CHAIN)];
    let findings = run(files);
    // The inner if (`x == 1 { ... } else { baz(); }`) has
    // dissimilar branches, so no F2b finding. The outer pair is not
    // compared because the alternative is not a block.
    assert!(
        findings.is_empty(),
        "F2b: outer pair of else-if chain must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t31_no_drift_on_small_cluster_floor() {
    // F5d-iii: a cluster at exactly MIN_GROUP_SIZE whose dominant
    // exemplar normalises to within SMALL_CLUSTER_TOKEN_BUFFER tokens
    // of MIN_FN_TOKENS is at the detector's resolution limit. The
    // wild-β syn parse-API family (`parse` / `parse2` / `parse_str`)
    // is a 3-fn cluster of 1-line delegate wrappers whose dominant
    // exemplar normalises to 22 tokens; this fixture mirrors that
    // shape with three independently typed parse-style delegates.
    let files = vec![
        parsed("a.rs", FN_TINY_DELEGATE_A),
        parsed("b.rs", FN_TINY_DELEGATE_B),
        parsed("c.rs", FN_TINY_DELEGATE_C),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "F5d-iii: small-cluster + dominant-near-MIN_FN_TOKENS must not fire, got {:#?}",
        findings
    );
}

// ---------- R-2.d: TypeScript ----------

fn parsed_typescript(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::TypeScript, src.to_string())
        .expect("ir_from_source")
}

const FN_TS_BASE: &str = r#"
function process(items) {
    const out = [];
    for (const it of items) {
        if (it > 0) {
            out.push(it * 2);
        }
    }
    return out;
}
"#;

const FN_TS_DRIFTED: &str = r#"
function process(items) {
    const out = [];
    for (const it of items) {
        if (it > 0 && it < 100) {
            out.push(it * 2);
        }
    }
    return out;
}
"#;

#[test]
fn t_typescript_drift_detected_4_identical_plus_1_modified() {
    let files = vec![
        parsed_typescript("a.ts", FN_TS_BASE),
        parsed_typescript("b.ts", FN_TS_BASE),
        parsed_typescript("c.ts", FN_TS_BASE),
        parsed_typescript("d.ts", FN_TS_BASE),
        parsed_typescript("e.ts", FN_TS_DRIFTED),
    ];
    let findings = run(files);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 TS drift finding, got {}: {:#?}",
        findings.len(),
        findings
    );
    assert_eq!(findings[0].detector_id, "clone-drift");
    assert_eq!(
        findings[0].evidence.language_citation_status,
        LanguageCitationStatus::Unconfirmed,
        "TypeScript clone-drift findings are Unconfirmed until R-2.f"
    );
}

#[test]
fn t_typescript_no_drift_when_all_identical() {
    let files = vec![
        parsed_typescript("a.ts", FN_TS_BASE),
        parsed_typescript("b.ts", FN_TS_BASE),
        parsed_typescript("c.ts", FN_TS_BASE),
        parsed_typescript("d.ts", FN_TS_BASE),
    ];
    let findings = run(files);
    assert!(
        findings.is_empty(),
        "all-identical clones are not drift; got {findings:#?}"
    );
}
