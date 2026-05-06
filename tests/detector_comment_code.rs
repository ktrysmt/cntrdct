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

// ---------- F5b: Python `:raises:` factory suppression ----------
//
// Wild Python β corpus exposed 14/14 FPs in attrs.validators where the
// factory shape `def f(...): ... return _Validator(...)` carried a
// `:raises X:` doc that describes the RETURNED validator's behavior.
// The v0 detector misread these as function-level claims.

#[test]
fn t21_python_raises_factory_with_value_return_suppressed() {
    // The canonical attrs.validators shape: returns a constructed
    // helper object; doc `:raises:` describes that helper's behavior.
    let src = r#"
class _InstanceOfValidator:
    pass

def instance_of(type):
    """
    A validator that raises a TypeError when called with the wrong type.

    :raises TypeError: With a human readable error message.
    """
    return _InstanceOfValidator()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "factory + :raises: + value return must be suppressed, got {:#?}",
        findings
    );
}

#[test]
fn t22_python_raises_with_no_return_still_fires() {
    // Control: doc claims raises but body has neither raise nor a
    // value-returning statement. v0 behavior preserved.
    let src = r#"
def f(x):
    """:raises ValueError: when x is bad."""
    print(x)
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "no body raise + no value return = still a finding, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["pattern"], "py-raises",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t23_python_raises_with_return_none_still_fires() {
    // Bare `return` and explicit `return None` are NOT factory shape.
    // The doc's `:raises:` claim still mismatches the body.
    let src_bare = r#"
def f(x):
    """:raises ValueError: when x is bad."""
    print(x)
    return
"#;
    let src_none = r#"
def f(x):
    """:raises ValueError: when x is bad."""
    print(x)
    return None
"#;
    for (label, src) in [("bare", src_bare), ("None", src_none)] {
        let findings = run(vec![parsed_python("a.py", src)]);
        assert_eq!(
            findings.len(),
            1,
            "{label}: return-without-value is not factory shape, got {:#?}",
            findings
        );
    }
}

#[test]
fn t24_python_raises_factory_nested_def_return_does_not_count() {
    // A return inside a NESTED `def` belongs to that inner scope.
    // The outer factory still has no value-returning statement; the
    // suppression should NOT apply (and the trigger fires).
    let src = r#"
def f(x):
    """:raises ValueError: when x is bad."""
    def inner():
        return 1
    print(x)
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "nested-def return must not trigger factory suppression, got {:#?}",
        findings
    );
}

// ---------- F5c: Python `.. deprecated::` directive subject ----------

#[test]
fn t25_python_deprecated_directive_emphasized_param_suppressed() {
    // Wild β corpus shape: attrs's attrib() docstring contains
    // `.. deprecated:: VERSION *paramname*` directives that
    // deprecate a parameter, not the function. The function itself
    // remains supported and undecorated.
    let src = r#"
def attrib(default=None, cmp=None):
    """
    Create a new attribute on a class.

    .. deprecated:: 17.4.0 *convert*
    .. deprecated:: 19.2.0 *cmp* Removal on or after 2021-06-01.
    .. versionchanged:: 21.1.0 *cmp* undeprecated
    """
    return None
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    let dep: Vec<_> = findings
        .iter()
        .filter(|f| f.evidence.raw["pattern"] == "py-deprecated")
        .collect();
    assert!(
        dep.is_empty(),
        "all `.. deprecated::` directives are parameter-level (emphasized body); must be suppressed, got {:#?}",
        dep
    );
}

#[test]
fn t26_python_deprecated_directive_with_prose_body_still_fires() {
    // Function-level signal: directive body is prose, not emphasis.
    let src = r#"
def f():
    """
    Do a thing.

    .. deprecated:: 1.0
       This function will be removed in v2.0.
    """
    return None
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    let dep: Vec<_> = findings
        .iter()
        .filter(|f| f.evidence.raw["pattern"] == "py-deprecated")
        .collect();
    assert_eq!(
        dep.len(),
        1,
        "bare `.. deprecated::` directive (no emphasis on body line) must fire, got {:#?}",
        dep
    );
}

#[test]
fn t27_python_mixed_deprecated_directives_function_level_wins() {
    // If ANY function-level `.. deprecated::` directive coexists
    // with parameter-level ones, fire (function IS deprecated).
    let src = r#"
def f(old_param=None):
    """
    Do a thing.

    .. deprecated:: 1.0
    .. deprecated:: 2.0 *old_param*
    """
    return None
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    let dep: Vec<_> = findings
        .iter()
        .filter(|f| f.evidence.raw["pattern"] == "py-deprecated")
        .collect();
    assert_eq!(
        dep.len(),
        1,
        "function-level signal must override parameter-level, got {:#?}",
        dep
    );
}

#[test]
fn t28b_python_deprecated_directive_with_indented_literal_continuation_suppressed() {
    // Wild β corpus: attrs's `attrs()` docstring has a
    // `.. deprecated:: 18.2.0` whose body lives on the next indented
    // line, opening with reST literal markup (``__lt__``). This is
    // an item-level (method-behavior) deprecation, not function-level.
    let src = r#"
def attrs():
    """
    Build a class.

    .. deprecated:: 18.2.0
       ``__lt__``, ``__le__``, ``__gt__``, and ``__ge__`` now raise
       a `DeprecationWarning` if compared subclass-to-subclass.
    """
    return None
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    let dep: Vec<_> = findings
        .iter()
        .filter(|f| f.evidence.raw["pattern"] == "py-deprecated")
        .collect();
    assert!(
        dep.is_empty(),
        "directive with literal-markup continuation body must be parameter/item-level, got {:#?}",
        dep
    );
}

#[test]
fn t28_python_deprecated_prose_only_still_fires() {
    // No `.. deprecated::` directive at all; the word appears in
    // free-form prose. v0 behavior preserved.
    let src = r#"
def f():
    """This function is deprecated. Use g() instead."""
    return None
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    let dep: Vec<_> = findings
        .iter()
        .filter(|f| f.evidence.raw["pattern"] == "py-deprecated")
        .collect();
    assert_eq!(
        dep.len(),
        1,
        "prose-only deprecation still fires per v0, got {:#?}",
        dep
    );
}
