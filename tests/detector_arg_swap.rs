//! Integration tests for the arg-swap detector v0 spec.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus,
};
use cntrdct::detectors::arg_swap::ArgSwap;
use cntrdct::ir::IrFile;

fn parsed(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Rust, src.to_string())
        .expect("ir_from_source")
}

fn parsed_python(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::Python, src.to_string())
        .expect("ir_from_source")
}

fn run(files: Vec<IrFile>) -> Vec<Finding> {
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
fn t21_python_instance_variable_method_call_skipped() {
    // F3b accepts `self.foo` / `cls.foo` receivers only. An arbitrary
    // instance-variable call site (`c.copy(...)`) lacks the static
    // hint that the receiver is the same class as the method's
    // defining class — flow analysis to recover the type of `c` is
    // out of v0 scope, so the call is conservatively skipped.
    let src = "class C:\n    def copy(self, dst, src):\n        return dst + src\n\ndef driver():\n    c = C()\n    dst = 1\n    src = 2\n    _ = c.copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "non-self/cls method receivers must stay out of v0 scope, got {:#?}",
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

// ---------- F3b / F4b / F5b lifts (added 2026-05-21) ----------

#[test]
fn t25_f4b_class_method_with_self_receiver_resolves() {
    // F4b: methods declared inside a `class_definition` are now
    // registered as defs with the leading `self` dropped, so a
    // 2-positional method matches the 2-arg call shape. The call
    // uses the `self.` receiver (F3b), and the args/params are an
    // exact swap permutation — F5a strict path.
    let src = "class C:\n    def copy(self, dst, src):\n        return dst + src\n    def driver(self):\n        dst = 1\n        src = 2\n        return self.copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4b: class method with self.X call must fire, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["match_kind"], "strict",
        "strict swap must report match_kind=strict, got {:?}",
        findings[0].evidence.raw
    );
}

#[test]
fn t26_f5b_prefix_match_swap_fires() {
    // F5b: `self._set_attrs(dst, inf)` against `_set_attrs(self,
    // info, dstfn)`. The arg names are not equal to the param
    // names, but `dst` is a strict prefix of `dstfn` and `inf` of
    // `info`, and the prefix-matching is a swap permutation. This
    // mirrors the audit-corpus rarfile_set_attrs.py:14 shape that
    // the PyPIBugs paper labels as ArgSwap.
    let src = "class RarFile:\n    def _set_attrs(self, info, dstfn):\n        return None\n    def extract(self, dirs):\n        for dst, inf in dirs:\n            self._set_attrs(dst, inf)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F5b: prefix-match swap must fire on rarfile shape, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["match_kind"], "prefix",
        "prefix swap must report match_kind=prefix, got {:?}",
        findings[0].evidence.raw
    );
}

#[test]
fn t27_f5b_prefix_floor_rejects_short_names() {
    // F5b requires the shorter name to be at least
    // PREFIX_MATCH_MIN_CHARS (3) characters. `s` prefixing `src`
    // is below the floor — no match — so the heuristic does not
    // fire on toy one-letter abbreviations.
    let src = "def copy(dst, src):\n    return dst + src\n\ndef driver():\n    d = 1\n    s = 2\n    _ = copy(s, d)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "F5b: short-name prefix must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t28_f5b_prefix_match_no_false_positive_on_identity() {
    // Both args prefix-match the corresponding params in order
    // (`tar` ⊂ `target_buf`, `src` ⊂ `source_buf`) — identity, not
    // a swap. Must not emit.
    let src = "def copy(target_buf, source_buf):\n    return target_buf + source_buf\n\ndef driver():\n    tar = 1\n    src = 2\n    _ = copy(tar, src)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "F5b: identity-by-prefix must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t29_f4b_cls_receiver_resolves() {
    // F4b also drops the conventional `cls` receiver for
    // classmethod-style defs.
    let src = "class C:\n    @classmethod\n    def copy(cls, dst, src):\n        return dst + src\n    @classmethod\n    def driver(cls):\n        dst = 1\n        src = 2\n        return cls.copy(src, dst)\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4b: cls receiver swap must fire, got {:#?}",
        findings
    );
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

// ---------------------------------------------------------------------
// Regression guard (R-1.c'' follow-up, 2026-06-03): call sites nested in
// expression shapes the converter leaves as `IrExpr::Other` (binary
// operators, closures, Python comprehensions / generators / conditional
// expressions, f-strings) must still be enumerated. The R-1.c'' IR-walk
// migration silently dropped these (calls reachable only through an
// `Other` expression were unvisited), regressing arg-swap recall on real
// code; call enumeration was reverted to the v0.5.x raw-tree walk. The
// T1 audit/wild pinning did not catch the regression because the only
// such call in those corpora (`totalsegmentator_statistics.py:10`) has no
// argument/parameter name correlation and so fires in neither version.
// These tests put a name-correlating swap inside each shape so a future
// re-narrowing of call enumeration fails the gate.
// ---------------------------------------------------------------------

#[test]
fn t30_rust_swap_inside_other_expression_shapes_detected() {
    // `make(img_file, seg_file)` is a name-swap against the definition
    // `make(seg_file, img_file)`. Each call sits inside an expression
    // the converter materialises as `IrExpr::Other` (binary operand,
    // closure body), so an IR-only walk would miss them.
    let src = r#"
fn make(seg_file: u32, img_file: u32) -> u32 { seg_file + img_file }

fn d_binary(seg_file: u32, img_file: u32) -> u32 {
    1 + make(img_file, seg_file)
}

fn d_closure(seg_file: u32, img_file: u32) -> u32 {
    (0..3).map(|_| make(img_file, seg_file)).sum()
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        2,
        "swaps nested in binary / closure expressions must be detected, got {:#?}",
        findings
    );
    assert!(findings.iter().all(|f| f.detector_id == "arg-swap"));
}

#[test]
fn t31_python_swap_inside_other_expression_shapes_detected() {
    // Each `make(img_file, seg_file)` is a name-swap against the
    // definition `make(seg_file, img_file)`, nested in (in order) a list
    // comprehension, generator expression, dict comprehension,
    // conditional expression, binary expression, and f-string — every
    // shape the converter leaves as `IrExpr::Other`.
    let src = r#"
def make(seg_file, img_file="x"):
    return seg_file, img_file

def d_listcomp(seg_file, img_file, xs):
    return [make(img_file, seg_file) for x in xs]

def d_gen(seg_file, img_file, xs):
    return (make(img_file, seg_file) for x in xs)

def d_dictcomp(seg_file, img_file, xs):
    return {x: make(img_file, seg_file) for x in xs}

def d_ternary(seg_file, img_file, c):
    return make(img_file, seg_file) if c else None

def d_binary(seg_file, img_file):
    return 1 + len(make(img_file, seg_file))

def d_fstring(seg_file, img_file):
    return f"{make(img_file, seg_file)}"
"#;
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        6,
        "swaps nested in comprehension / generator / dict-comprehension / ternary / binary / f-string must be detected, got {:#?}",
        findings
    );
    assert!(findings.iter().all(|f| f.detector_id == "arg-swap"));
}

// ---------- R-2.d: TypeScript ----------

fn parsed_typescript(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::TypeScript, src.to_string())
        .expect("ir_from_source")
}

#[test]
fn t_typescript_swap_detected() {
    let src = r#"
function connect(host: string, port: string): void {
    open(host, port);
}
function caller(port: string, host: string): void {
    connect(port, host);
}
"#;
    let findings = run(vec![parsed_typescript("a.ts", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 TS swap finding, got {findings:#?}"
    );
    assert_eq!(findings[0].detector_id, "arg-swap");
    assert_eq!(
        findings[0].evidence.language_citation_status,
        LanguageCitationStatus::Unconfirmed,
        "TypeScript arg-swap findings are Unconfirmed until R-2.f"
    );
}

#[test]
fn t_typescript_no_swap_when_order_matches() {
    let src = r#"
function connect(host: string, port: string): void {}
function caller(host: string, port: string): void {
    connect(host, port);
}
"#;
    let findings = run(vec![parsed_typescript("a.ts", src)]);
    assert!(findings.is_empty(), "no swap expected, got {findings:#?}");
}

#[test]
fn t_typescript_this_method_swap() {
    let src = r#"
class Net {
    connect(host: string, port: string): void {}
    run(port: string, host: string): void {
        this.connect(port, host);
    }
}
"#;
    let findings = run(vec![parsed_typescript("a.ts", src)]);
    assert_eq!(
        findings.len(),
        1,
        "expected 1 this.method swap finding, got {findings:#?}"
    );
}
