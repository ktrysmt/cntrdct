//! unreachable-after-terminator detector — flag statements that follow a
//! divergent statement inside the same block.
//!
//! Spec: `cntrdct/docs/spec/unreachable-after-terminator-v0.md`.
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A).
//!
//! IR migration note (R-1.c'' Path b): the detector consumes
//! [`crate::ir`] nodes semantically — block statements
//! ([`IrBlock::statements`]), per-statement / per-expression terminator
//! classification, pre-computed branch-merge terminators
//! ([`IrIfStmt::terminator`] / [`IrMatchStmt::terminator`]), loop
//! break-targeting ([`IrLoopStmt::has_break_to_self`]), and the
//! source spans now carried on every [`IrExpr`] (R-1.c'' step 3) for the
//! F4d-ii / F4d-iii / F4d-iv and F4e finding endpoints. No `raw_tree()`
//! reparse. The walk visits the same positions the v0.5.x raw-tree walk
//! did — every block, call site, return / break carrier, and `if`
//! condition the converter materialises — so the T1 pinning stays
//! byte-identical.

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::{
    BranchMergeKind, DivergentKind, IrBlock, IrCallSite, IrExpr, IrExprKind, IrFile, IrIfStmt,
    IrLiteral, IrStmt, IrStmtKind, IrTerminator, IrWhileStmt,
};
use rayon::prelude::*;

pub const TERMINATOR_MACROS: &[&str] = &[
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "abort",
    "exit",
];

pub const SUPPRESSION_TOKEN: &str = "unreachable_code";

/// Python call expressions whose invocation diverges control flow.
///
/// `sys.exit` and `sys.abort` raise `SystemExit`; `os._exit` terminates
/// the process without unwinding; bare `exit` and `quit` are the
/// interactive-shell builtins that also raise `SystemExit`. Any
/// statement following a call to these in the same block is unreachable
/// under straight-line execution.
pub const PYTHON_EXIT_FUNCTIONS: &[&str] = &["sys.exit", "sys.abort", "os._exit", "exit", "quit"];

static CITATIONS: &[Citation] = &[
    Citation {
        key: "hovemeyer-pugh-oopsla-2004",
        authors: "D. Hovemeyer, W. Pugh",
        title: "Finding Bugs is Easy",
        venue: "OOPSLA 2004",
        year: 2004,
        doi: Some("10.1145/1052883.1052895"),
        url: None,
        languages: &[Language::Rust],
    },
    Citation {
        key: "engler-sosp-2001",
        authors: "D. Engler, D.Y. Chen, S. Hallem, A. Chou, B. Chelf",
        title: "Bugs as Deviant Behavior: A General Approach to Inferring Errors in Systems Code",
        venue: "SOSP 2001",
        year: 2001,
        doi: Some("10.1145/502034.502041"),
        url: None,
        languages: &[Language::Rust],
    },
];

#[derive(Debug, Default)]
pub struct UnreachableAfterTerminator;

impl UnreachableAfterTerminator {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for UnreachableAfterTerminator {
    fn id(&self) -> &'static str {
        "unreachable-after-terminator"
    }

    fn name(&self) -> &'static str {
        "Unreachable After Terminator"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [Language] {
        &[Language::Rust, Language::Python]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = ctx
            .files
            .par_iter()
            .filter(|f| matches!(f.language, Language::Rust | Language::Python))
            .flat_map_iter(|file| {
                let mut local = Vec::new();
                if !file.parse_recovered {
                    match file.language {
                        Language::Rust => scan_rust(file, &mut local),
                        Language::Python => scan_python(file, &mut local),
                    }
                }
                local
            })
            .collect();
        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
                .then(a.primary.start_col.cmp(&b.primary.start_col))
        });
        Ok(findings)
    }
}

// ---------- Shared terminator-kind strings ----------

/// Canonical message string for a divergent macro / exit call. Matches
/// the v0.5.x `rust_macro_terminator_name` / `python_exit_call_kind`
/// strings byte-for-byte.
fn divergent_kind_str(kind: DivergentKind) -> &'static str {
    match kind {
        DivergentKind::Panic => "panic",
        DivergentKind::Unreachable => "unreachable",
        DivergentKind::Todo => "todo",
        DivergentKind::Unimplemented => "unimplemented",
        DivergentKind::Abort => "abort",
        DivergentKind::Exit => "exit",
        DivergentKind::SysExit => "sys.exit",
        DivergentKind::SysAbort => "sys.abort",
        DivergentKind::OsExit => "os._exit",
        DivergentKind::ExitBuiltin => "exit",
        DivergentKind::QuitBuiltin => "quit",
    }
}

// ---------- Rust scan ----------

fn scan_rust(file: &IrFile, findings: &mut Vec<Finding>) {
    for f in &file.fns {
        // F4 suppression: `#[allow(unreachable_code)]` on the function
        // suppresses block-level findings throughout its body
        // (mirrors v0.5.x `is_rust_suppressed` ancestor walk; the F4d
        // rules are not gated by suppression).
        let fn_suppressed = f
            .decorators
            .iter()
            .any(|d| d.raw.contains(SUPPRESSION_TOKEN));
        walk_rust_block(file, &f.body, fn_suppressed, findings);
    }
}

fn walk_rust_block(
    file: &IrFile,
    block: &IrBlock,
    inherited_suppressed: bool,
    findings: &mut Vec<Finding>,
) {
    // A block is suppressed when an enclosing function / block carried
    // `#[allow(unreachable_code)]`, or when an attribute on any of its
    // own statements (the IR home for the block's direct
    // `attribute_item` / inner `#![...]` children) contains the token.
    let suppressed = inherited_suppressed || block_has_unreachable_allow(block);
    if !suppressed {
        analyze_rust_block(block, findings);
    }
    for stmt in &block.statements {
        walk_rust_stmt(file, stmt, suppressed, findings);
    }
}

fn block_has_unreachable_allow(block: &IrBlock) -> bool {
    block.statements.iter().any(|s| {
        s.attributes
            .iter()
            .any(|a| a.raw.contains(SUPPRESSION_TOKEN))
    })
}

/// F4a block-level rule: the first divergent statement in the block (that
/// is not `#[cfg(...)]`-gated) renders the following statement
/// unreachable.
fn analyze_rust_block(block: &IrBlock, findings: &mut Vec<Finding>) {
    // F4c: item declarations are hoisted by the compiler — they do not
    // execute in source order, so exclude them from the statement
    // stream (mirrors v0.5.x `is_rust_block_statement`).
    let stmts: Vec<&IrStmt> = block
        .statements
        .iter()
        .filter(|s| !matches!(s.kind, IrStmtKind::HoistedItem { .. }))
        .collect();

    for (i, stmt) in stmts.iter().enumerate() {
        if let Some(kind) = rust_stmt_terminator_kind(stmt) {
            // F4b: a `#[cfg(...)]`-gated statement is conditional and
            // does NOT qualify as a terminator; skip and keep scanning
            // so the complementary-cfg-pair idiom produces no finding.
            if is_cfg_gated(stmt) {
                continue;
            }
            let following = stmts.len() - i - 1;
            if following == 0 {
                return;
            }
            let follower = stmts[i + 1];
            findings.push(build_finding(
                ir_loc_to_core(&follower.location),
                ir_loc_to_core(&stmt.location),
                kind,
                following,
                LanguageCitationStatus::Confirmed,
            ));
            return;
        }
    }
}

/// True when any attribute attached to `stmt` is `#[cfg(...)]` (NOT
/// `#[cfg_attr(...)]`, whose first path segment is `cfg_attr`).
fn is_cfg_gated(stmt: &IrStmt) -> bool {
    stmt.attributes
        .iter()
        .any(|a| a.name_path.first().map(|s| s == "cfg").unwrap_or(false))
}

/// Statement-level terminator classification for Rust. Mirrors v0.5.x
/// `rust_terminator_kind`: `assert!` is intentionally NOT a terminator
/// (only the [`TERMINATOR_MACROS`] set diverges).
fn rust_stmt_terminator_kind(stmt: &IrStmt) -> Option<&'static str> {
    match &stmt.kind {
        IrStmtKind::Return(_) => Some("return"),
        IrStmtKind::Break(_) => Some("break"),
        IrStmtKind::Continue(_) => Some("continue"),
        IrStmtKind::DivergentCall { kind, .. } => Some(divergent_kind_str(*kind)),
        IrStmtKind::If(if_stmt) => match if_stmt.terminator {
            Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::IfBranchesDiverge,
            }) => Some("if-branches-diverge"),
            _ => None,
        },
        IrStmtKind::Match(match_stmt) => match match_stmt.terminator {
            Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::MatchArmsDiverge,
            }) => Some("match-arms-diverge"),
            _ => None,
        },
        IrStmtKind::Loop(loop_stmt) => {
            if loop_stmt.has_break_to_self {
                None
            } else {
                Some("loop-no-break")
            }
        }
        _ => None,
    }
}

/// Recurse through a statement, applying the F4d-ii / F4d-iii / F4d-iv
/// rules to the call sites / return carriers / `if` conditions it
/// contains and descending into every nested block / expression.
fn walk_rust_stmt(file: &IrFile, stmt: &IrStmt, suppressed: bool, findings: &mut Vec<Finding>) {
    match &stmt.kind {
        IrStmtKind::Call(call) => walk_rust_expr_call(file, call, findings),
        IrStmtKind::Return(value) => {
            if let Some(v) = value {
                analyze_rust_divergent_carrier(&stmt.location, v, findings);
                walk_rust_expr(file, v, suppressed, findings);
            }
        }
        IrStmtKind::Raise(value) => {
            if let Some(v) = value {
                walk_rust_expr(file, v, suppressed, findings);
            }
        }
        IrStmtKind::Assert(v) => walk_rust_expr(file, v, suppressed, findings),
        IrStmtKind::Let { value } | IrStmtKind::Assign { value } => {
            if let Some(v) = value {
                walk_rust_expr(file, v, suppressed, findings);
            }
        }
        IrStmtKind::DivergentCall { args, .. } => {
            for a in args {
                walk_rust_expr(file, a, suppressed, findings);
            }
        }
        IrStmtKind::If(if_stmt) => walk_rust_if(file, if_stmt, suppressed, findings),
        IrStmtKind::While(w) => {
            walk_rust_expr(file, &w.condition, suppressed, findings);
            walk_rust_block(file, &w.body, suppressed, findings);
        }
        IrStmtKind::Loop(l) => walk_rust_block(file, &l.body, suppressed, findings),
        IrStmtKind::For(f) => {
            walk_rust_expr(file, &f.iterable, suppressed, findings);
            walk_rust_block(file, &f.body, suppressed, findings);
        }
        IrStmtKind::Match(m) => {
            walk_rust_expr(file, &m.scrutinee, suppressed, findings);
            for arm in &m.arms {
                walk_rust_expr(file, &arm.body, suppressed, findings);
            }
        }
        IrStmtKind::With(wi) => {
            for cm in &wi.context_managers {
                walk_rust_expr(file, cm, suppressed, findings);
            }
            walk_rust_block(file, &wi.body, suppressed, findings);
        }
        IrStmtKind::Try(t) => {
            walk_rust_block(file, &t.body, suppressed, findings);
            for h in &t.handlers {
                walk_rust_block(file, h, suppressed, findings);
            }
            if let Some(o) = &t.orelse {
                walk_rust_block(file, o, suppressed, findings);
            }
            if let Some(fb) = &t.finalbody {
                walk_rust_block(file, fb, suppressed, findings);
            }
        }
        IrStmtKind::Break(_)
        | IrStmtKind::Continue(_)
        | IrStmtKind::HoistedItem { .. }
        | IrStmtKind::Other { .. } => {}
    }
}

fn walk_rust_if(file: &IrFile, if_stmt: &IrIfStmt, suppressed: bool, findings: &mut Vec<Finding>) {
    analyze_rust_if_condition(if_stmt, findings);
    walk_rust_expr(file, &if_stmt.condition, suppressed, findings);
    walk_rust_block(file, &if_stmt.consequence, suppressed, findings);
    if let Some(alt) = &if_stmt.alternative {
        walk_rust_block(file, alt, suppressed, findings);
    }
}

fn walk_rust_expr(file: &IrFile, expr: &IrExpr, suppressed: bool, findings: &mut Vec<Finding>) {
    match &expr.kind {
        IrExprKind::Call(call) => walk_rust_expr_call(file, call, findings),
        IrExprKind::Return(value) => {
            if let Some(v) = value {
                analyze_rust_divergent_carrier(&expr.location, v, findings);
                walk_rust_expr(file, v, suppressed, findings);
            }
        }
        IrExprKind::Raise(value) => {
            if let Some(v) = value {
                walk_rust_expr(file, v, suppressed, findings);
            }
        }
        IrExprKind::Block(b) => walk_rust_block(file, b, suppressed, findings),
        IrExprKind::If(if_stmt) => walk_rust_if(file, if_stmt, suppressed, findings),
        IrExprKind::Match(m) => {
            walk_rust_expr(file, &m.scrutinee, suppressed, findings);
            for arm in &m.arms {
                walk_rust_expr(file, &arm.body, suppressed, findings);
            }
        }
        IrExprKind::Loop(l) => walk_rust_block(file, &l.body, suppressed, findings),
        IrExprKind::DivergentCall { args, .. } => {
            for a in args {
                walk_rust_expr(file, a, suppressed, findings);
            }
        }
        IrExprKind::Ident(_)
        | IrExprKind::Path(_)
        | IrExprKind::Literal(_)
        | IrExprKind::Break(_)
        | IrExprKind::Continue(_)
        | IrExprKind::Other { .. } => {}
    }
}

/// Recurse into a call site, applying F4d-ii to its arguments and
/// descending into each argument for further nested call sites.
fn walk_rust_expr_call(file: &IrFile, call: &IrCallSite, findings: &mut Vec<Finding>) {
    analyze_rust_call_args(call, findings);
    for a in &call.args {
        walk_rust_expr(file, a, false, findings);
    }
}

/// F4d-ii: arguments evaluate left-to-right, so a divergent argument
/// renders the following argument (or, if it is the last, the call
/// itself) unreachable.
fn analyze_rust_call_args(call: &IrCallSite, findings: &mut Vec<Finding>) {
    for (i, arg) in call.args.iter().enumerate() {
        if let Some(kind) = rust_expr_diverges(arg) {
            let (follower_loc, following_count) = if i + 1 < call.args.len() {
                (
                    ir_loc_to_core(&call.args[i + 1].location),
                    call.args.len() - (i + 1),
                )
            } else {
                (ir_loc_to_core(&call.location), 1)
            };
            findings.push(build_finding(
                follower_loc,
                ir_loc_to_core(&arg.location),
                kind,
                following_count,
                LanguageCitationStatus::Confirmed,
            ));
            return;
        }
    }
}

/// F4d-iii: `return EXPR` (or `break EXPR`) where EXPR evaluation
/// diverges — the surrounding control transfer is itself unreachable.
/// `carrier_loc` is the location of the `return` / `break` expression
/// (its inner value's divergence is what fires).
fn analyze_rust_divergent_carrier(
    carrier_loc: &crate::ir::Location,
    value: &IrExpr,
    findings: &mut Vec<Finding>,
) {
    if let Some(kind) = rust_expr_diverges(value) {
        findings.push(build_finding(
            ir_loc_to_core(carrier_loc),
            ir_loc_to_core(&value.location),
            kind,
            1,
            LanguageCitationStatus::Confirmed,
        ));
    }
}

/// F4d-iv: an `if` whose condition expression diverges — the condition
/// never produces a value, so the consequence block is unreachable.
fn analyze_rust_if_condition(if_stmt: &IrIfStmt, findings: &mut Vec<Finding>) {
    if let Some(kind) = rust_expr_diverges(&if_stmt.condition) {
        findings.push(build_finding(
            ir_loc_to_core(&if_stmt.consequence.location),
            ir_loc_to_core(&if_stmt.condition.location),
            kind,
            1,
            LanguageCitationStatus::Confirmed,
        ));
    }
}

/// True (with the divergence kind string) iff evaluating `expr` always
/// diverges. Mirrors v0.5.x `rust_expression_diverges`.
fn rust_expr_diverges(expr: &IrExpr) -> Option<&'static str> {
    match &expr.kind {
        IrExprKind::Return(_) => Some("return"),
        IrExprKind::Break(_) => Some("break"),
        IrExprKind::Continue(_) => Some("continue"),
        IrExprKind::DivergentCall { kind, .. } => Some(divergent_kind_str(*kind)),
        IrExprKind::Block(b) => rust_block_diverges(b),
        IrExprKind::If(if_stmt) => match if_stmt.terminator {
            Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::IfBranchesDiverge,
            }) => Some("if-branches-diverge"),
            _ => None,
        },
        IrExprKind::Match(m) => match m.terminator {
            Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::MatchArmsDiverge,
            }) => Some("match-arms-diverge"),
            _ => None,
        },
        IrExprKind::Loop(l) => {
            if l.has_break_to_self {
                None
            } else {
                Some("loop-no-break")
            }
        }
        _ => None,
    }
}

/// A block diverges iff its first (non-hoisted) divergent statement is a
/// terminator. Mirrors v0.5.x `rust_block_diverges` for the
/// materialised statement shapes (`assert!` is not a terminator, so it
/// is excluded by `rust_stmt_terminator_kind`).
fn rust_block_diverges(block: &IrBlock) -> Option<&'static str> {
    block.statements.iter().find_map(rust_stmt_terminator_kind)
}

// ---------- Python scan ----------
//
// Pattern A: the block-level walk is shared with Rust at the
// algorithmic level; the terminator set differs (return / raise / break
// / continue / `assert False` / exit-call) and Python has no
// branch-merge or F4d-ii/iii/iv rules. F4e (constant-condition `if` /
// `while`) is Python-only and reads the literal condition off
// [`IrExpr`]. Python carries no detector-internal suppression — the
// `# cntrdct: allow(...)` form is handled by `crate::config::apply`.

fn scan_python(file: &IrFile, findings: &mut Vec<Finding>) {
    for f in &file.fns {
        walk_python_block(file, &f.body, findings);
    }
}

fn walk_python_block(file: &IrFile, block: &IrBlock, findings: &mut Vec<Finding>) {
    analyze_python_block(block, findings);
    for stmt in &block.statements {
        walk_python_stmt(file, stmt, findings);
    }
}

fn analyze_python_block(block: &IrBlock, findings: &mut Vec<Finding>) {
    let stmts = &block.statements;
    for (i, stmt) in stmts.iter().enumerate() {
        if let Some(kind) = python_stmt_terminator_kind(stmt) {
            let following = stmts.len() - i - 1;
            if following == 0 {
                return;
            }
            findings.push(build_finding(
                ir_loc_to_core(&stmts[i + 1].location),
                ir_loc_to_core(&stmt.location),
                kind,
                following,
                LanguageCitationStatus::Unconfirmed,
            ));
            return;
        }
    }
}

fn python_stmt_terminator_kind(stmt: &IrStmt) -> Option<&'static str> {
    match &stmt.kind {
        IrStmtKind::Return(_) => Some("return"),
        IrStmtKind::Raise(_) => Some("raise"),
        IrStmtKind::Break(_) => Some("break"),
        IrStmtKind::Continue(_) => Some("continue"),
        IrStmtKind::DivergentCall { kind, .. } => Some(divergent_kind_str(*kind)),
        // Only the literal `assert False` form is a terminator in v0
        // (constant-folding `0` / `None` is out of scope).
        IrStmtKind::Assert(IrExpr {
            kind: IrExprKind::Literal(IrLiteral::Bool(false)),
            ..
        }) => Some("assert"),
        _ => None,
    }
}

fn walk_python_stmt(file: &IrFile, stmt: &IrStmt, findings: &mut Vec<Finding>) {
    match &stmt.kind {
        IrStmtKind::If(if_stmt) => {
            analyze_python_if_constant(if_stmt, findings);
            walk_python_block(file, &if_stmt.consequence, findings);
            if let Some(alt) = &if_stmt.alternative {
                walk_python_block(file, alt, findings);
            }
        }
        IrStmtKind::While(w) => {
            analyze_python_while_constant(w, findings);
            walk_python_block(file, &w.body, findings);
        }
        IrStmtKind::For(f) => walk_python_block(file, &f.body, findings),
        IrStmtKind::With(wi) => walk_python_block(file, &wi.body, findings),
        IrStmtKind::Try(t) => {
            walk_python_block(file, &t.body, findings);
            for h in &t.handlers {
                walk_python_block(file, h, findings);
            }
            if let Some(o) = &t.orelse {
                walk_python_block(file, o, findings);
            }
            if let Some(fb) = &t.finalbody {
                walk_python_block(file, fb, findings);
            }
        }
        _ => {}
    }
}

/// F4e classifier — truthiness of a Python condition at parse time.
/// `Some(true)` truthy constant, `Some(false)` falsy constant, `None`
/// for any non-literal (or hex / non-decimal integer). Mirrors v0.5.x
/// `python_constant_condition`.
fn python_constant_condition(expr: &IrExpr) -> Option<bool> {
    match &expr.kind {
        IrExprKind::Literal(IrLiteral::Bool(b)) => Some(*b),
        IrExprKind::Literal(IrLiteral::None) => Some(false),
        IrExprKind::Literal(IrLiteral::Int(Some(v))) => Some(*v != 0),
        // Hex / binary / octal integers stay `Int(None)` → indeterminate.
        IrExprKind::Literal(IrLiteral::Int(None)) => None,
        IrExprKind::Literal(IrLiteral::String { is_empty }) => Some(!*is_empty),
        _ => None,
    }
}

fn analyze_python_while_constant(w: &IrWhileStmt, findings: &mut Vec<Finding>) {
    if let Some(false) = python_constant_condition(&w.condition) {
        if let Some(first) = w.body.statements.first() {
            findings.push(build_finding(
                ir_loc_to_core(&first.location),
                ir_loc_to_core(&w.condition.location),
                "constant-false-while",
                1,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    }
}

fn analyze_python_if_constant(if_stmt: &IrIfStmt, findings: &mut Vec<Finding>) {
    let Some(cond_value) = python_constant_condition(&if_stmt.condition) else {
        return;
    };

    if !cond_value {
        // F4e-ii: `if False:` consequence is unreachable. Carve-outs:
        // type-checking import guards and the generator-marker idiom.
        if python_if_false_body_is_carveout(&if_stmt.consequence) {
            return;
        }
        if let Some(first) = if_stmt.consequence.statements.first() {
            findings.push(build_finding(
                ir_loc_to_core(&first.location),
                ir_loc_to_core(&if_stmt.condition.location),
                "constant-false-if",
                1,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    } else if let Some(alt) = &if_stmt.alternative {
        // F4e-iii: `if True: ... else: <unreachable>`. v0 reports only
        // the immediate else body (elif widening is a v1 non-goal).
        if let Some(first) = alt.statements.first() {
            findings.push(build_finding(
                ir_loc_to_core(&first.location),
                ir_loc_to_core(&if_stmt.condition.location),
                "constant-true-if-else",
                1,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    }
}

/// F4e-ii carve-outs. True when every statement in the `if False:` body
/// is an import (type-checking guard) or a `yield` expression statement
/// (generator marker). Empty bodies are not carve-outs.
fn python_if_false_body_is_carveout(block: &IrBlock) -> bool {
    let stmts = &block.statements;
    if stmts.is_empty() {
        return false;
    }
    let all_imports = stmts.iter().all(|s| {
        matches!(
            &s.kind,
            IrStmtKind::Other {
                node_kind: "import_statement" | "import_from_statement" | "future_import_statement",
                ..
            }
        )
    });
    if all_imports {
        return true;
    }
    stmts.iter().all(|s| {
        matches!(
            &s.kind,
            IrStmtKind::Other {
                node_kind: "yield",
                ..
            }
        )
    })
}

// ---------- Shared finding construction ----------

fn build_finding(
    primary: Location,
    terminator: Location,
    kind: &'static str,
    following_count: usize,
    citation_status: LanguageCitationStatus,
) -> Finding {
    let terminator_line = terminator.start_line;
    Finding {
        detector_id: "unreachable-after-terminator".to_string(),
        primary,
        related: vec![terminator],
        message: format!(
            "statement is unreachable; preceded by {} on line {}",
            kind, terminator_line
        ),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["hovemeyer-pugh-oopsla-2004", "engler-sosp-2001"],
            raw: serde_json::json!({
                "terminator_kind": kind,
                "terminator_line": terminator_line,
                "following_count": following_count,
            }),
            language_citation_status: citation_status,
        },
    }
}

/// Project an [`crate::ir::Location`] (byte offsets included) onto the
/// 4-field [`crate::core::Location`] the [`Finding`] surface uses.
fn ir_loc_to_core(loc: &crate::ir::Location) -> Location {
    Location {
        file: loc.file.clone(),
        start_line: loc.start_line,
        start_col: loc.start_col,
        end_line: loc.end_line,
        end_col: loc.end_col,
    }
}
