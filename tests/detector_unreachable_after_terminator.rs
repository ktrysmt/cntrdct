//! Integration tests for the unreachable-after-terminator detector v0 spec.
//!
//! Each test maps to a row in
//! `cntrdct/docs/spec/unreachable-after-terminator-v0.md` test plan.

use std::path::PathBuf;

use cntrdct::core::{
    register_detector, AnomalyClass, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    Language, LanguageCitationStatus,
};
use cntrdct::detectors::unreachable_after_terminator::UnreachableAfterTerminator;
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

// ---------- F4b: cfg-gated terminator suppression ----------
//
// The wild Rust β corpus exposed a 10/10-FP pattern where the
// detector misread `#[cfg(...)] return ...;` as an unconditional
// terminator. F4b in `docs/spec/unreachable-after-terminator-v0.md`
// muts cfg-gated terminators; the cases below pin the contract.

#[test]
fn t29_cfg_attribute_on_terminator_suppresses() {
    let src = r#"
fn f() -> i32 {
    #[cfg(unix)]
    return 1;
    other_impl()
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "cfg-gated terminator must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t30_cfg_not_attribute_on_terminator_suppresses() {
    let src = r#"
fn f() {
    #[cfg(not(test))]
    panic!("prod-only abort");
    let _x = 1;
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "cfg(not(...)) is still a cfg gate, got {:#?}",
        findings
    );
}

#[test]
fn t31_complementary_cfg_pair_no_finding() {
    // The canonical wild-corpus β idiom: complementary cfg-gated
    // returns. Both branches are NEVER simultaneously present in
    // any compiled binary; the v0 detector misread them as
    // sequential statements (10/10 FP on benchmarks/wild-corpus).
    let src = r#"
fn f() -> i32 {
    #[cfg(feature = "preserve_order")]
    return self_swap_remove();
    #[cfg(not(feature = "preserve_order"))]
    return self_map_remove();
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "complementary cfg pair must produce no findings, got {:#?}",
        findings
    );
}

#[test]
fn t32_cfg_attr_does_not_suppress() {
    // cfg_attr conditionally applies an inner attribute; the
    // statement itself runs unconditionally. So a cfg_attr-tagged
    // terminator is still a terminator and the follower is still
    // unreachable.
    let src = r#"
fn f() {
    #[cfg_attr(test, cold)]
    return;
    bar();
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "cfg_attr is not cfg; the terminator is unconditional, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "return",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t33_cfg_with_panic_macro_terminator_suppresses() {
    let src = r#"
fn f() {
    #[cfg(target_os = "windows")]
    panic!("windows path not supported");
    let _x = 1;
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "F4b applies to macro terminators too, got {:#?}",
        findings
    );
}

#[test]
fn t34_cfg_on_follower_does_not_suppress_unconditional_terminator() {
    // The terminator is unconditional; the follower is gated. In
    // any build where the follower's cfg evaluates true, the
    // follower IS unreachable. We retain the finding (limitation
    // documented in spec F4b).
    let src = r#"
fn f() {
    return;
    #[cfg(test)]
    debug();
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "unconditional terminator + cfg-gated follower still fires, got {:#?}",
        findings
    );
}

#[test]
fn t35_hoisted_fn_item_after_return_is_not_unreachable() {
    // F4c: items declared inside a block are hoisted, so a nested
    // `fn` after a terminator is not executable code-after-terminator.
    // This is the exact shape that surfaced in semver__identifier.rs:377.
    let src = r#"
fn outer() {
    return helper();

    #[cold]
    fn helper() {}
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "F4c: nested fn after return is hoisted, got {:#?}",
        findings
    );
}

#[test]
fn t36_other_item_kinds_after_return_are_not_unreachable() {
    // Mirror of t35 for the remaining hoisted item kinds we exclude.
    let src = r#"
fn outer() {
    return;

    const C: u32 = 1;
    static S: u32 = 2;
    use std::fmt;
    struct Inner;
    enum E { A }
    type T = u32;
    mod m {}
    impl Inner {}
    trait Tr {}
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "F4c: hoisted items after return must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t37_executable_stmt_after_hoisted_item_still_fires() {
    // Items between a terminator and a real executable statement must
    // not mask the executable-stmt detection. The terminator at line 1
    // of the body is still followed by a `bar()` call once items are
    // skipped; we want exactly 1 finding pointing at `bar()`.
    let src = r#"
fn outer() {
    return;
    fn helper() {}
    bar();
}
"#;
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "executable stmt after hoisted item must still flag, got {:#?}",
        findings
    );
}

// ---------- M-2: Python pilot ----------
//
// Mirrors T1-T15 for Python where the construct exists. v0 has no
// inline Python suppression mechanism, so the T6/T14 attribute-allow
// scenarios are not portable; cntrdct.toml-based suppression is
// covered by `tests/suppression.rs`.

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

// ---------- F4d compound divergence (added 2026-05-21) ----------

#[test]
fn t40_f4d_i_branch_merge_if_else_both_return() {
    // F4d-i: an if/else where every branch ends with a divergent
    // expression is itself a terminator. The statement that follows
    // in the enclosing block is unreachable. Mirrors the rustc
    // ui-test `expr_if.rs#L27` shape (audit-corpus line 29 expected).
    let src = "fn f() { if true { return; } else { return; } bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4d-i: bar() after if-else where both branches return must fire, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "if-branches-diverge",
        "expected if-branches-diverge terminator kind, got {:?}",
        findings[0].evidence.raw
    );
}

#[test]
fn t41_f4d_i_branch_merge_if_else_no_alternative_does_not_fire() {
    // F4d-i requires an explicit else branch — `if cond { return; }`
    // alone is conditional and does NOT diverge.
    let src = "fn f() { if true { return; } bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "F4d-i: if without else must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t42_f4d_i_branch_merge_match_all_arms_return() {
    // F4d-i extension: a match where every arm's body diverges is a
    // terminator. The statement that follows is unreachable.
    let src = "fn f(x: u32) { match x { 0 => return, 1 => return, _ => return } bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4d-i: bar() after exhaustive divergent match must fire, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "match-arms-diverge",
        "expected match-arms-diverge, got {:?}",
        findings[0].evidence.raw
    );
}

#[test]
fn t43_f4d_i_branch_merge_match_one_arm_falls_through_does_not_fire() {
    // One non-divergent arm is enough for the match to NOT be a
    // terminator under F4d-i — the value flows out of that arm.
    let src = "fn f(x: u32) -> u32 { match x { 0 => return 0, _ => 1 }; bar(); 0 }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "F4d-i: match with a non-divergent arm must not fire, got {:#?}",
        findings
    );
}

#[test]
fn t44_f4d_ii_call_with_divergent_arg_then_arg() {
    // F4d-ii: arguments evaluate left-to-right. A `return` in an
    // earlier argument renders subsequent arguments unreachable.
    // Mirrors rustc ui-test `expr_call.rs#L13` (audit-corpus line 15).
    let src = "fn foo(_x: !, _y: usize) {}\nfn a() { foo(return, 22); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings
            .iter()
            .any(|f| f.evidence.raw["terminator_kind"] == "return"),
        "F4d-ii: foo(return, 22) must emit on the trailing argument, got {:#?}",
        findings
    );
}

#[test]
fn t45_f4d_ii_call_with_only_divergent_arg() {
    // F4d-ii: when the divergent argument is the last/only one, the
    // call itself never invokes — flag the call expression.
    // Mirrors rustc ui-test `expr_call.rs#L18` (audit-corpus line 20).
    let src = "fn bar(_x: !) {}\nfn b() { bar(return); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        !findings.is_empty(),
        "F4d-ii: bar(return) must emit on the call, got nothing"
    );
}

#[test]
fn t46_f4d_iii_return_with_divergent_block_value() {
    // F4d-iii: `return EXPR` where EXPR evaluation diverges. The
    // outer return is itself unreachable because EXPR never produces
    // a value. Mirrors rustc ui-test `expr_return.rs#L10` (audit-
    // corpus line 12: nested return-block).
    let src = "fn a() { let _x: () = { return { return; } }; }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        !findings.is_empty(),
        "F4d-iii: return with divergent value must fire, got nothing"
    );
}

#[test]
fn t47_f4d_iv_if_condition_diverges() {
    // F4d-iv: `if {return} { ... }` — the if-condition is a block
    // that diverges. The consequence block is unreachable because the
    // condition never produces a value. Mirrors rustc ui-test
    // `expr_if.rs#L7` (audit-corpus line 9).
    let src = "fn foo() { if { return } { bar(); } }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        !findings.is_empty(),
        "F4d-iv: divergent if-condition must flag the consequence, got nothing"
    );
}

// ---------- F4d-v: loop without targeting break (added 2026-05-21) ----------

#[test]
fn t50_f4d_v_loop_with_return_flags_following_stmt() {
    // F4d-v retires the former t48 non-goal. A bare `loop { return; }`
    // diverges (the loop has no break targeting it), so the following
    // statement is unreachable.
    let src = "fn a() { loop { return; } bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4d-v: loop-with-return-no-break must flag, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "loop-no-break",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t51_f4d_v_loop_with_unlabelled_break_does_not_flag() {
    // The break exits this same loop (innermost-rule), so the loop is
    // reachable past its body — no F4d-v finding.
    let src = "fn b() { loop { break; } bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "loop with innermost break must not flag, got {:#?}",
        findings
    );
}

#[test]
fn t52_f4d_v_labelled_break_targets_outer() {
    // The inner `loop` has no break to itself, but the labelled break
    // exits the outer 'outer loop. The outer loop is therefore exited
    // by the break — no F4d-v finding on the println.
    let src = "fn d() { 'outer: loop { loop { break 'outer; } } println!(\"alive\"); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        findings.is_empty(),
        "'outer loop has a labelled break targeting it, must not flag, got {:#?}",
        findings
    );
}

#[test]
fn t53_f4d_v_inner_break_to_inner_label_outer_diverges() {
    // The break targets the 'middle loop (the inner labelled one), not
    // the outermost loop. The outermost loop therefore has no break
    // targeting it and diverges → following println is unreachable.
    let src = "fn e() { loop { 'middle: loop { loop { break 'middle; } } } println!(\"dead\"); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert_eq!(
        findings.len(),
        1,
        "outer loop without a break targeting it must flag, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "loop-no-break",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t54_f4d_v_break_inside_closure_does_not_escape() {
    // A closure introduces a hard break-target boundary. The break
    // inside the closure cannot target the outer `loop`, so the outer
    // loop still has no break targeting it and diverges.
    let src = "fn f() { loop { let _c = || { loop { break; } }; } bar(); }";
    let findings = run(vec![parsed("a.rs", src)]);
    assert!(
        !findings.is_empty(),
        "closure break boundary: outer loop must still flag, got nothing"
    );
    let outer_loop_flag = findings
        .iter()
        .any(|f| f.evidence.raw["terminator_kind"] == "loop-no-break");
    assert!(
        outer_loop_flag,
        "expected a loop-no-break finding, got kinds: {:?}",
        findings
            .iter()
            .map(|f| f.evidence.raw["terminator_kind"].clone())
            .collect::<Vec<_>>()
    );
}

// ---------- F4e: Python constant-condition branch (added 2026-05-21) ----------

#[test]
fn t55_f4e_while_false_body_unreachable() {
    let src = "def f():\n    while False:\n        x = 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4e-i: while False body must flag, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "constant-false-while",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t56_f4e_while_zero_body_unreachable() {
    let src = "def f():\n    while 0:\n        x = 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4e-i: while 0 body must flag, got {:#?}",
        findings
    );
}

#[test]
fn t57_f4e_if_false_body_unreachable() {
    let src = "def f():\n    if False:\n        x = 1\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4e-ii: if False body must flag, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "constant-false-if",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t58_f4e_if_true_else_body_unreachable() {
    let src = "def f():\n    if True:\n        x = 1\n    else:\n        y = 2\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert_eq!(
        findings.len(),
        1,
        "F4e-iii: else of if True must flag, got {:#?}",
        findings
    );
    assert_eq!(
        findings[0].evidence.raw["terminator_kind"], "constant-true-if-else",
        "got: {}",
        findings[0].evidence.raw
    );
}

#[test]
fn t59_f4e_if_false_typecheck_import_carveout() {
    // CodeQL UnreachableCode fixture explicitly does NOT flag this
    // shape (pre-`typing.TYPE_CHECKING` idiom). cntrdct mirrors the
    // carve-out so eval precision against the audit corpus is not
    // dragged down by a deliberate by-design body.
    let src = "if False:\n    from typing import Any\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "F4e-ii carve-out (import idiom) must not flag, got {:#?}",
        findings
    );
}

#[test]
fn t60_f4e_if_false_generator_marker_carveout() {
    // CodeQL ODASA-6783: `if False: yield ...` is the "this def is a
    // generator" marker. cntrdct mirrors the carve-out.
    let src = "def gen():\n    if False:\n        yield None\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "F4e-ii carve-out (generator-marker idiom) must not flag, got {:#?}",
        findings
    );
}

#[test]
fn t61_f4e_indeterminate_condition_no_flag() {
    // `if x:` where `x` is not a recognised literal must NOT trip F4e.
    let src = "def f(x):\n    if x:\n        a = 1\n    else:\n        b = 2\n";
    let findings = run(vec![parsed_python("a.py", src)]);
    assert!(
        findings.is_empty(),
        "F4e must remain silent for non-literal conditions, got {:#?}",
        findings
    );
}

// ---------- R-2.d: TypeScript ----------

fn parsed_typescript(name: &str, src: &str) -> IrFile {
    cntrdct::ir_from_source(&PathBuf::from(name), Language::TypeScript, src.to_string())
        .expect("ir_from_source")
}

#[test]
fn t_typescript_unreachable_after_throw() {
    let src = r#"
function f(x: number): number {
    throw new Error("e");
    return x;
}
"#;
    let findings = run(vec![parsed_typescript("a.ts", src)]);
    assert_eq!(findings.len(), 1, "expected 1 finding, got {findings:#?}");
    assert_eq!(findings[0].detector_id, "unreachable-after-terminator");
    assert_eq!(
        findings[0].evidence.language_citation_status,
        LanguageCitationStatus::Unconfirmed
    );
}

#[test]
fn t_typescript_unreachable_after_process_exit() {
    let src = r#"
function f(): void {
    process.exit(1);
    cleanup();
}
"#;
    let findings = run(vec![parsed_typescript("a.ts", src)]);
    assert_eq!(
        findings.len(),
        1,
        "process.exit must terminate; got {findings:#?}"
    );
}

#[test]
fn t_typescript_unreachable_after_if_both_branches_diverge() {
    let src = r#"
function f(x: number): number {
    if (x > 0) {
        return x;
    } else {
        throw new Error("e");
    }
    cleanup();
}
"#;
    let findings = run(vec![parsed_typescript("a.ts", src)]);
    assert_eq!(
        findings.len(),
        1,
        "if/else both diverging must make trailing stmt unreachable; got {findings:#?}"
    );
}

#[test]
fn t_typescript_reachable_when_no_terminator() {
    let src = r#"
function f(x: number): number {
    const y = x + 1;
    return y;
}
"#;
    let findings = run(vec![parsed_typescript("a.ts", src)]);
    assert!(
        findings.is_empty(),
        "no unreachable code, got {findings:#?}"
    );
}
