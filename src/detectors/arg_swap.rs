//! arg-swap detector — Rice-style param/arg name matching for binary calls.
//!
//! Spec: `cntrdct/docs/spec/arg-swap-v0.md` (Rust v0).
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A).
//!
//! Algorithm (shared, IR-based since R-1.c''):
//! 1. Read each [`IrFile`]'s functions (the converter parsed every file
//!    once; the detector never reparses).
//! 2. Across all files in the scan, build a map from function name to
//!    definitions (collecting parameter names). Rust registers only
//!    top-level functions (`!is_method`, matching the v0.5.x root-level
//!    `function_item` walk); Python additionally registers class methods
//!    (F4b) with the leading `self` / `cls` receiver dropped.
//! 3. Recursively walk every function body's IR statements / expressions
//!    and collect [`crate::ir::IrCallSite`]s whose callee is a bare
//!    identifier (Rust) or a bare identifier / `self.` / `cls.` method
//!    (Python) and whose arguments are all bare identifiers.
//! 4. For binary callees with a unique 2-arg definition, check whether the
//!    argument identifier multiset is the swap permutation of the parameter
//!    name multiset (case-insensitive, F5a strict / F5b prefix).
//! 5. Emit a Finding for each clean swap.
//!
//! Each language runs in isolation: Rust definitions never match Python
//! calls and vice versa, because each pipeline iterates only over its
//! own language's files. Python findings carry
//! `LanguageCitationStatus::Confirmed` grounded by Allamanis et al.
//! NeurIPS 2021 (PyBugLab + PyPIBugs); see
//! `docs/surveys/arg-swap-python-2026-05.md`.
//!
//! IR migration note (R-1.c'' Path b): the v0.5.x raw tree-sitter walk
//! visited every `call_expression` / `call` node in the file. The IR
//! walk reaches the same call sites the converter materialises —
//! statement position, `let` / assignment RHS, `for` iterable / body,
//! `try` / `with` blocks, `if` / `match` / `while` / `loop` bodies, and
//! call-argument nesting — plus the transparent `await` wrapper
//! (ir-v0.md §F2). Calls buried in still-`IrExpr::Other` shapes
//! (`binary_operator`, `subscript`, …) are not visited; this is a strict
//! subset of the v0.5.x traversal and so can never manufacture a finding
//! v0.5.x did not also produce, preserving the T1 byte-identical pinning.

use std::collections::HashMap;

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::{
    IrBlock, IrCallSite, IrExpr, IrExprKind, IrFile, IrFn, IrIfStmt, IrPath, IrStmtKind, ParamKind,
};
use rayon::prelude::*;

static CITATIONS: &[Citation] = &[
    Citation {
        key: "li-zhou-fse-2005",
        authors: "Z. Li, Y. Zhou",
        title: "PR-Miner: Automatically Extracting Implicit Programming Rules and Detecting Violations in Large Software Code",
        venue: "ESEC/FSE 2005",
        year: 2005,
        doi: None,
        url: None,
        languages: &[Language::Rust],
    },
    Citation {
        key: "rice-icse-2017",
        authors: "A. Rice, E. Aftandilian, C. Jaspan, E. Johnston, M. Pradel, Y. Arroyo-Paredes",
        title: "Detecting Argument Selection Defects",
        venue: "ICSE 2017",
        year: 2017,
        doi: None,
        url: None,
        languages: &[Language::Rust],
    },
    Citation {
        key: "allamanis-neurips-2021",
        authors: "M. Allamanis, H. Jackson-Flux, M. Brockschmidt",
        title: "Self-Supervised Bug Detection and Repair",
        venue: "NeurIPS 2021",
        year: 2021,
        doi: None,
        url: None,
        languages: &[Language::Python],
    },
];

#[derive(Debug, Default)]
pub struct ArgSwap;

impl ArgSwap {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct FnDef {
    params: Vec<String>,
    location: Location,
}

#[derive(Debug, Clone)]
struct CallSite {
    callee: String,
    args: Vec<String>,
    location: Location,
}

type ExtractDefs = fn(&IrFile) -> Option<Vec<(String, FnDef)>>;
type ExtractCalls = fn(&IrFile) -> Option<Vec<CallSite>>;

impl Detector for ArgSwap {
    fn id(&self) -> &'static str {
        "arg-swap"
    }

    fn name(&self) -> &'static str {
        "Argument Swap"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [Language] {
        &[Language::Rust, Language::Python]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = Vec::new();
        findings.extend(run_pipeline(
            ctx,
            Language::Rust,
            extract_rust_fn_defs,
            extract_rust_call_sites,
            LanguageCitationStatus::Confirmed,
            &["li-zhou-fse-2005", "rice-icse-2017"],
        ));
        findings.extend(run_pipeline(
            ctx,
            Language::Python,
            extract_python_fn_defs,
            extract_python_call_sites,
            LanguageCitationStatus::Confirmed,
            &[
                "li-zhou-fse-2005",
                "rice-icse-2017",
                "allamanis-neurips-2021",
            ],
        ));

        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
        });

        Ok(findings)
    }
}

/// Per-language pipeline: extract definitions, build the name→defs map,
/// then walk call sites and emit Findings for each clean swap. Each
/// language runs in isolation so a Rust definition never resolves a
/// Python call (and vice versa).
fn run_pipeline(
    ctx: &DetectContext,
    lang: Language,
    extract_defs: ExtractDefs,
    extract_calls: ExtractCalls,
    citation_status: LanguageCitationStatus,
    citation_keys: &'static [&'static str],
) -> Vec<Finding> {
    let per_file_defs: Vec<(String, FnDef)> = ctx
        .files
        .par_iter()
        .filter(|f| f.language == lang)
        .filter_map(extract_defs)
        .flatten()
        .collect();
    let mut defs_by_name: HashMap<String, Vec<FnDef>> = HashMap::new();
    for (name, def) in per_file_defs {
        defs_by_name.entry(name).or_default().push(def);
    }

    ctx.files
        .par_iter()
        .filter(|f| f.language == lang)
        .filter_map(extract_calls)
        .flatten()
        .filter_map(|call| check_swap(&call, &defs_by_name, citation_status, citation_keys))
        .collect()
}

/// F5b prefix-name match floor: the shorter name must be at least
/// this many characters before a prefix match counts. Single- and
/// two-letter abbreviations (`a`, `b`, `s`, `d`) are too noisy to
/// prefix-match safely — `a` would otherwise prefix-match `alpha`,
/// `apple`, `args`, etc.
pub const PREFIX_MATCH_MIN_CHARS: usize = 3;

/// True iff `a` and `b` agree case-insensitively under either strict
/// equality (F5a) or strict prefix containment (F5b). The shorter
/// name must be at least `PREFIX_MATCH_MIN_CHARS` characters long
/// before prefix matching counts. Equal-length names that differ in
/// any position never prefix-match (they would otherwise produce
/// spurious matches between sibling identifiers of the same length).
fn name_matches(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if a.len() == b.len() {
        return false;
    }
    let (short, long) = if a.len() < b.len() { (a, b) } else { (b, a) };
    short.len() >= PREFIX_MATCH_MIN_CHARS && long.starts_with(short)
}

/// Apply the swap rule (F5 from the spec): the argument identifier
/// multiset must be the reverse permutation of the parameter name
/// multiset, case-insensitively. The detector skips identity matches
/// where caller used the same names in the same order.
///
/// F5b (added 2026-05-21): the per-position name match accepts a
/// strict prefix in either direction once the shorter side is
/// `PREFIX_MATCH_MIN_CHARS` characters or longer. The Rice et al.
/// ICSE 2017 detector — already cited as `rice-icse-2017` — uses
/// abbreviation-aware matching to catch swaps like `set_attrs(dst,
/// inf)` against `set_attrs(info, dstfn)` (audit-corpus
/// `rarfile_set_attrs.py:14`). The strict path (F5a) and the prefix
/// path (F5b) are tagged on `evidence.raw.match_kind` so downstream
/// calibration can stratify priors.
fn check_swap(
    call: &CallSite,
    defs_by_name: &HashMap<String, Vec<FnDef>>,
    citation_status: LanguageCitationStatus,
    citation_keys: &'static [&'static str],
) -> Option<Finding> {
    if call.args.len() != 2 {
        return None;
    }
    let candidates = defs_by_name.get(&call.callee)?;
    let matching: Vec<&FnDef> = candidates.iter().filter(|d| d.params.len() == 2).collect();
    if matching.len() != 1 {
        return None;
    }
    let def = matching[0];

    let a0 = call.args[0].to_lowercase();
    let a1 = call.args[1].to_lowercase();
    let p0 = def.params[0].to_lowercase();
    let p1 = def.params[1].to_lowercase();

    let identity = name_matches(&a0, &p0) && name_matches(&a1, &p1);
    let swapped = name_matches(&a0, &p1) && name_matches(&a1, &p0);

    if swapped && !identity {
        let match_kind = if a0 == p1 && a1 == p0 {
            "strict"
        } else {
            "prefix"
        };
        Some(Finding {
            detector_id: "arg-swap".to_string(),
            primary: call.location.clone(),
            related: vec![def.location.clone()],
            message: format!(
                "call argument order swapped relative to definition of `{}`",
                call.callee
            ),
            raw_severity: Severity::Warning,
            anomaly_class: AnomalyClass::Interface,
            evidence: Evidence {
                citation_keys: citation_keys.to_vec(),
                raw: serde_json::json!({
                    "callee": call.callee,
                    "parameter_names": def.params.clone(),
                    "argument_names": call.args.clone(),
                    "match_kind": match_kind,
                }),
                language_citation_status: citation_status,
            },
        })
    } else {
        None
    }
}

// ---------- Definition extraction (IR) ----------
//
// Top-level only for Rust (mirrors the v0.5.x root-level `function_item`
// walk): impl methods are not registered as definitions. Python
// additionally registers class methods (F4b). For both languages, a
// definition with any `Unsupported` parameter shape (`*args`, `**kwargs`,
// tuple patterns, the `/` and `*` separators, …) is rejected outright,
// and a `_`-prefixed plain parameter rejects the whole definition.

fn extract_rust_fn_defs(file: &IrFile) -> Option<Vec<(String, FnDef)>> {
    if file.parse_recovered {
        return None;
    }
    Some(
        file.fns
            .iter()
            .filter(|f| !f.is_method)
            .filter_map(ir_fn_to_def)
            .collect(),
    )
}

fn extract_python_fn_defs(file: &IrFile) -> Option<Vec<(String, FnDef)>> {
    if file.parse_recovered {
        return None;
    }
    Some(file.fns.iter().filter_map(ir_fn_to_def).collect())
}

/// Build a [`FnDef`] from an [`IrFn`], dropping the implicit `self` /
/// `cls` receiver and rejecting (returning `None`) any definition that
/// carries an unmodellable parameter shape or a `_`-prefixed parameter,
/// matching the v0.5.x `parse_*_fn_def` conservatism byte-for-byte.
fn ir_fn_to_def(f: &IrFn) -> Option<(String, FnDef)> {
    let mut params: Vec<String> = Vec::new();
    for p in &f.params {
        match p.kind {
            // Drop the conventional Python `self` / `cls` receiver before
            // arity checks (Rust top-level fns carry no receiver).
            ParamKind::Receiver => continue,
            // `*args` / `**kwargs` / tuple patterns / separators — reject
            // the entire definition rather than produce a wrong arity.
            ParamKind::Unsupported => return None,
            ParamKind::Plain => {
                if p.name.starts_with('_') {
                    return None;
                }
                params.push(p.name.clone());
            }
        }
    }
    Some((
        f.name.clone(),
        FnDef {
            params,
            location: ir_loc_to_core(&f.location),
        },
    ))
}

// ---------- Call-site extraction (IR) ----------
//
// Walk every function body (including impl / class methods — the
// v0.5.x raw walk visited the whole tree, so calls inside methods are
// in scope even though the method itself is not a Rust definition) and
// collect the call sites that match the v0 swap-candidate shape.

fn extract_rust_call_sites(file: &IrFile) -> Option<Vec<CallSite>> {
    extract_call_sites(file, Language::Rust)
}

fn extract_python_call_sites(file: &IrFile) -> Option<Vec<CallSite>> {
    extract_call_sites(file, Language::Python)
}

fn extract_call_sites(file: &IrFile, lang: Language) -> Option<Vec<CallSite>> {
    if file.parse_recovered {
        return None;
    }
    let mut out: Vec<CallSite> = Vec::new();
    for f in &file.fns {
        walk_block_calls(&f.body, lang, &mut out);
    }
    Some(out)
}

fn walk_block_calls(block: &IrBlock, lang: Language, out: &mut Vec<CallSite>) {
    for stmt in &block.statements {
        match &stmt.kind {
            IrStmtKind::Call(call) => {
                consider_call(call, lang, out);
                walk_args(&call.args, lang, out);
            }
            IrStmtKind::Let { value } | IrStmtKind::Assign { value } => {
                if let Some(v) = value {
                    walk_expr_calls(v, lang, out);
                }
            }
            IrStmtKind::Return(value) | IrStmtKind::Raise(value) => {
                if let Some(v) = value {
                    walk_expr_calls(v, lang, out);
                }
            }
            IrStmtKind::Assert(cond) => walk_expr_calls(cond, lang, out),
            IrStmtKind::DivergentCall { args, .. } => walk_args(args, lang, out),
            IrStmtKind::If(if_stmt) => walk_if_calls(if_stmt, lang, out),
            IrStmtKind::While(w) => {
                walk_expr_calls(&w.condition, lang, out);
                walk_block_calls(&w.body, lang, out);
            }
            IrStmtKind::Loop(l) => walk_block_calls(&l.body, lang, out),
            IrStmtKind::For(f) => {
                walk_expr_calls(&f.iterable, lang, out);
                walk_block_calls(&f.body, lang, out);
            }
            IrStmtKind::Match(m) => {
                walk_expr_calls(&m.scrutinee, lang, out);
                for arm in &m.arms {
                    walk_expr_calls(&arm.body, lang, out);
                }
            }
            IrStmtKind::With(wi) => {
                for cm in &wi.context_managers {
                    walk_expr_calls(cm, lang, out);
                }
                walk_block_calls(&wi.body, lang, out);
            }
            IrStmtKind::Try(t) => {
                walk_block_calls(&t.body, lang, out);
                for h in &t.handlers {
                    walk_block_calls(h, lang, out);
                }
                if let Some(o) = &t.orelse {
                    walk_block_calls(o, lang, out);
                }
                if let Some(fb) = &t.finalbody {
                    walk_block_calls(fb, lang, out);
                }
            }
            IrStmtKind::Break(_)
            | IrStmtKind::Continue(_)
            | IrStmtKind::HoistedItem { .. }
            | IrStmtKind::Other { .. } => {}
        }
    }
}

fn walk_expr_calls(expr: &IrExpr, lang: Language, out: &mut Vec<CallSite>) {
    match &expr.kind {
        IrExprKind::Call(call) => {
            consider_call(call, lang, out);
            walk_args(&call.args, lang, out);
        }
        IrExprKind::Return(inner) | IrExprKind::Raise(inner) => {
            if let Some(e) = inner {
                walk_expr_calls(e, lang, out);
            }
        }
        IrExprKind::Block(b) => walk_block_calls(b, lang, out),
        IrExprKind::If(if_stmt) => walk_if_calls(if_stmt, lang, out),
        IrExprKind::Match(m) => {
            walk_expr_calls(&m.scrutinee, lang, out);
            for arm in &m.arms {
                walk_expr_calls(&arm.body, lang, out);
            }
        }
        IrExprKind::Loop(l) => walk_block_calls(&l.body, lang, out),
        IrExprKind::DivergentCall { args, .. } => walk_args(args, lang, out),
        IrExprKind::Ident(_)
        | IrExprKind::Path(_)
        | IrExprKind::Literal(_)
        | IrExprKind::Break(_)
        | IrExprKind::Continue(_)
        | IrExprKind::Other { .. } => {}
    }
}

fn walk_if_calls(if_stmt: &IrIfStmt, lang: Language, out: &mut Vec<CallSite>) {
    walk_expr_calls(&if_stmt.condition, lang, out);
    walk_block_calls(&if_stmt.consequence, lang, out);
    if let Some(alt) = &if_stmt.alternative {
        walk_block_calls(alt, lang, out);
    }
}

fn walk_args(args: &[IrExpr], lang: Language, out: &mut Vec<CallSite>) {
    for a in args {
        walk_expr_calls(a, lang, out);
    }
}

/// Push a [`CallSite`] for `call` iff it matches the v0 swap-candidate
/// shape: a bare-identifier callee (Rust) / bare-identifier or
/// `self.` / `cls.` method callee (Python) whose arguments are all bare
/// identifiers. Any non-identifier argument disqualifies the whole call.
fn consider_call(call: &IrCallSite, lang: Language, out: &mut Vec<CallSite>) {
    let callee = match lang {
        Language::Rust => rust_callee_name(&call.callee),
        Language::Python => python_callee_name(&call.callee),
    };
    let Some(callee) = callee else { return };

    let mut args: Vec<String> = Vec::with_capacity(call.args.len());
    for a in &call.args {
        match &a.kind {
            IrExprKind::Ident(name) => args.push(name.clone()),
            // keyword arguments, splats, literals, attribute access,
            // nested calls — anything other than a bare identifier
            // disqualifies the call from v0 swap analysis.
            _ => return,
        }
    }

    out.push(CallSite {
        callee,
        args,
        location: ir_loc_to_core(&call.location),
    });
}

/// A Rust callee is in scope only when it is a single bare identifier
/// (`foo`), mirroring v0.5.x `function.kind() == "identifier"`. Scoped
/// paths (`a::b`) and method / field calls (`a.b`) are rejected.
fn rust_callee_name(path: &IrPath) -> Option<String> {
    if path.receiver.is_empty() && path.segments.len() == 1 && path.raw == path.segments[0] {
        Some(path.segments[0].clone())
    } else {
        None
    }
}

/// A Python callee is in scope when it is a bare identifier (`foo`) or a
/// single-segment `self.` / `cls.` method (`self.foo` / `cls.foo`),
/// mirroring v0.5.x `parse_python_call` (identifier or
/// `python_self_method_name`). Deeper attribute access
/// (`self.x.y`, `obj.foo`) is rejected.
fn python_callee_name(path: &IrPath) -> Option<String> {
    if path.segments.len() != 1 {
        return None;
    }
    match path.receiver.as_slice() {
        [] => Some(path.segments[0].clone()),
        [recv] if recv == "self" || recv == "cls" => Some(path.segments[0].clone()),
        _ => None,
    }
}

// ---------- Shared helpers ----------

/// Project an [`crate::ir::Location`] (6 fields, byte offsets included)
/// onto the [`crate::core::Location`] (4 fields) the [`Finding`] surface
/// uses. Dropping the byte offsets keeps the emitted finding shape
/// byte-identical with the v0.5.x output.
fn ir_loc_to_core(loc: &crate::ir::Location) -> Location {
    Location {
        file: loc.file.to_path_buf(),
        start_line: loc.start_line,
        start_col: loc.start_col,
        end_line: loc.end_line,
        end_col: loc.end_col,
    }
}
