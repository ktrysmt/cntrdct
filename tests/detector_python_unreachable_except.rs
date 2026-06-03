//! Integration tests for the python-unreachable-except detector (R-5, F4f).
//!
//! Each test maps to a row in the F4f section of
//! `cntrdct/docs/spec/unreachable-after-terminator-v0.md`.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus, Severity,
};
use cntrdct::detectors::lang::python_unreachable_except::PythonUnreachableExcept;
use cntrdct::ir::IrFile;

fn parsed_python(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Python, src.to_string())
        .expect("ir_from_source")
}

fn parsed_rust(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Rust, src.to_string())
        .expect("ir_from_source")
}

fn run(files: Vec<IrFile>) -> Vec<Finding> {
    let detector = PythonUnreachableExcept::new();
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
fn t1_superclass_before_subclass() {
    let src = r#"
try:
    risky()
except Exception:
    handle()
except ValueError:
    never_runs()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {findings:#?}");
    let f = &findings[0];
    assert_eq!(f.detector_id, "python-unreachable-except");
    assert!(f.message.contains("unreachable"), "{}", f.message);
    assert!(f.message.contains("ValueError"), "{}", f.message);
    assert!(f.message.contains("Exception"), "{}", f.message);
    // primary points at the ValueError type expression (line 6).
    assert_eq!(f.primary.start_line, 6, "{f:#?}");
}

#[test]
fn t2_subclass_before_superclass_is_reachable() {
    let src = r#"
try:
    risky()
except ValueError:
    a()
except Exception:
    b()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(findings.is_empty(), "got {findings:#?}");
}

#[test]
fn t3_tuple_fully_covered() {
    let src = r#"
try:
    risky()
except Exception:
    handle()
except (KeyError, IndexError):
    nope()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {findings:#?}");
}

#[test]
fn t4_tuple_partially_covered_is_reachable() {
    // KeyError is covered by the earlier handler, but ValueError is not, so
    // the tuple handler is still reachable for ValueError.
    let src = r#"
try:
    risky()
except KeyError:
    a()
except (KeyError, ValueError):
    b()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(findings.is_empty(), "got {findings:#?}");
}

#[test]
fn t5_same_file_user_class_under_exception() {
    let src = r#"
class MyErr(ValueError):
    pass

try:
    risky()
except Exception:
    handle()
except MyErr:
    never()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {findings:#?}");
    assert!(
        findings[0].message.contains("MyErr"),
        "{}",
        findings[0].message
    );
}

#[test]
fn t6_unknown_imported_exception_is_indeterminate() {
    // SomethingImported is neither a builtin nor a same-file class, so the
    // relationship to Exception is indeterminate and must NOT be flagged.
    let src = r#"
try:
    risky()
except Exception:
    handle()
except SomethingImported:
    maybe_runs()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(findings.is_empty(), "got {findings:#?}");
}

#[test]
fn t7_bare_except_after_exception_is_reachable() {
    // `except Exception:` does NOT catch non-Exception BaseException
    // subclasses, so a trailing bare `except:` (≡ BaseException) is reachable.
    let src = r#"
try:
    risky()
except Exception:
    handle()
except:
    cleanup()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(findings.is_empty(), "got {findings:#?}");
}

#[test]
fn t8_except_star_group_is_skipped() {
    let src = r#"
try:
    risky()
except* Exception:
    handle()
except* ValueError:
    nope()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "except* groups are out of scope: {findings:#?}"
    );
}

#[test]
fn t9_citation_keys_and_unconfirmed_status() {
    let src = r#"
try:
    risky()
except Exception:
    handle()
except ValueError:
    never_runs()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert!(f
        .evidence
        .citation_keys
        .contains(&"hovemeyer-pugh-oopsla-2004"));
    assert!(f
        .evidence
        .citation_keys
        .contains(&"de-padua-shang-icpc-2017"));
    assert_eq!(
        f.evidence.language_citation_status,
        LanguageCitationStatus::Unconfirmed
    );
}

#[test]
fn t10_determinism() {
    let src = r#"
try:
    risky()
except Exception:
    handle()
except ValueError:
    never_runs()
except KeyError:
    also_never()
"#;
    let a = run(vec![parsed_python("a.py", src)]);
    let b = run(vec![parsed_python("a.py", src)]);
    assert_eq!(a.len(), 2);
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
}

#[test]
fn t11_non_python_file_skipped() {
    let src = "fn main() { let x = 1; }\n";
    let findings = run(vec![parsed_rust("a.rs", src)]);
    assert!(findings.is_empty());
}

#[test]
fn t12_anomaly_class_logic_and_severity_warning() {
    let src = r#"
try:
    risky()
except Exception:
    handle()
except ValueError:
    never_runs()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].anomaly_class, AnomalyClass::Logic);
    assert!(matches!(findings[0].raw_severity, Severity::Warning));
}

#[test]
fn t13_duplicate_handler() {
    let src = r#"
try:
    risky()
except ValueError:
    a()
except ValueError:
    b()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "duplicate handler is unreachable: {findings:#?}"
    );
}

#[test]
fn t14_builtin_subclass_chain() {
    // FileNotFoundError -> OSError; OSError before it makes it unreachable.
    let src = r#"
try:
    risky()
except OSError:
    a()
except FileNotFoundError:
    b()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(findings.len(), 1, "got {findings:#?}");
}

#[test]
fn t15_single_try_no_duplicate_when_specific_first() {
    // LookupError catches KeyError, but here KeyError is first so the second
    // (LookupError) is broader and reachable.
    let src = r#"
try:
    risky()
except KeyError:
    a()
except LookupError:
    b()
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(findings.is_empty(), "got {findings:#?}");
}
