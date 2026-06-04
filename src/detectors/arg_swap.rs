//! arg-swap detector — Rice-style param/arg name matching for binary calls.
//!
//! Spec: `cntrdct/docs/spec/arg-swap-v0.md` (Rust v0).
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A).
//!
//! Algorithm (shared; definitions on IR, call sites on raw tree):
//! 1. Definition extraction reads each [`IrFile`]'s functions from the
//!    IR the converter already produced (parameter lists are losslessly
//!    modelled by `IrFn.params` / `is_method`).
//! 2. Across all files in the scan, build a map from function name to
//!    definitions (collecting parameter names). Rust registers only
//!    top-level functions (`!is_method`, matching the v0.5.x root-level
//!    `function_item` walk); Python additionally registers class methods
//!    (F4b) with the leading `self` / `cls` receiver dropped.
//! 3. Call-site enumeration walks the raw tree-sitter tree
//!    ([`IrFile::raw_tree`]) for every `call_expression` (Rust) / `call`
//!    (Python) node whose callee is a bare identifier (Rust) or a bare
//!    identifier / `self.` / `cls.` method (Python) and whose arguments
//!    are all bare identifiers. See the "Call-site extraction" note
//!    below for why this is a raw walk rather than an IR walk.
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
//! IR migration note (R-1.c'' Path b, then reverted for call sites):
//! the R-1.c'' migration narrowed call enumeration to an IR walk over
//! the converter-materialised shapes (statement position, `let` /
//! assignment RHS, `for` iterable / body, `try` / `with` blocks,
//! `if` / `match` / `while` / `loop` bodies, call-argument nesting).
//! That walk could not reach calls buried in `IrExpr::Other` shapes
//! (`binary_operator`, closures, Python comprehensions / generators /
//! conditional expressions, f-strings, …), so it silently regressed
//! arg-swap recall on real code: a name-correlating swap inside any of
//! those shapes produced no finding where v0.5.x's full-tree walk did.
//! The T1 pinning gate missed the regression because the only such call
//! in the audit / wild corpora (`totalsegmentator_statistics.py:10`) has
//! no argument/parameter name correlation and so fires in neither
//! version. Call-site enumeration is therefore reverted to the v0.5.x
//! full raw-tree walk (this file's "Call-site extraction" section);
//! definition extraction stays on IR. This is the same Pattern-B escape
//! hatch pr-miner keeps (ir-v0.md §F5) — full call-set enumeration is
//! not losslessly representable in the simplified IR.

use std::collections::HashMap;

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::{IrFile, IrFn, ParamKind};
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
        &[Language::Rust, Language::Python, Language::TypeScript]
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
        // R-2.d: TypeScript reuses the same name-correlation pipeline —
        // IR definition extraction + a raw-tree call walk (Pattern B).
        // Unconfirmed until the R-2.f survey grounds a TypeScript citation.
        findings.extend(run_pipeline(
            ctx,
            Language::TypeScript,
            extract_typescript_fn_defs,
            extract_typescript_call_sites,
            LanguageCitationStatus::Unconfirmed,
            &["li-zhou-fse-2005", "rice-icse-2017"],
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

// ---------- Call-site extraction (raw tree-sitter) ----------
//
// arg-swap must enumerate EVERY call site in the file, including calls
// nested in expression shapes the converter leaves as `IrExpr::Other`
// (binary operators, closures, Python comprehensions / generators /
// conditional expressions, f-strings, …). The structured IR does not
// materialise those, so an IR-only walk silently drops their calls (the
// R-1.c'' regression — see the module header). Call enumeration is
// therefore a raw tree-sitter walk over `IrFile::raw_tree()`, byte-for-
// byte the v0.5.x traversal; this is the Pattern-B escape hatch pr-miner
// uses (ir-v0.md §F5). Definition extraction stays on IR because
// parameter lists ARE losslessly modelled (`IrFn.params` / `is_method`).
//
// The `parse_recovered` guard reproduces the v0.5.x `root.has_error()`
// skip: the converter sets `parse_recovered` from exactly that flag
// (`src/parsers/mod.rs`).

fn extract_rust_call_sites(file: &IrFile) -> Option<Vec<CallSite>> {
    if file.parse_recovered {
        return None;
    }
    let tree = file.raw_tree();
    let mut calls = Vec::new();
    walk_rust_for_calls(tree.root_node(), file, &mut calls);
    Some(calls)
}

fn walk_rust_for_calls(node: tree_sitter::Node, file: &IrFile, out: &mut Vec<CallSite>) {
    if node.kind() == "call_expression" {
        if let Some(call) = parse_rust_call(node, file) {
            out.push(call);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_rust_for_calls(child, file, out);
    }
}

/// A Rust call is in scope only when its function operand is a single
/// bare identifier (`foo(...)`); scoped paths (`a::b`) and method /
/// field calls (`a.b`) are rejected. Every argument must be a bare
/// `identifier` — any other shape disqualifies the call from v0 swap
/// analysis.
fn parse_rust_call(node: tree_sitter::Node, file: &IrFile) -> Option<CallSite> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "identifier" {
        return None;
    }
    let callee = node_text(function, &file.source);

    let arguments = node.child_by_field_name("arguments")?;
    let mut args = Vec::new();
    let mut cursor = arguments.walk();
    for child in arguments.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if child.kind() == "identifier" {
            args.push(node_text(child, &file.source));
        } else {
            return None;
        }
    }

    Some(CallSite {
        callee,
        args,
        location: node_location(file, node),
    })
}

fn extract_python_call_sites(file: &IrFile) -> Option<Vec<CallSite>> {
    if file.parse_recovered {
        return None;
    }
    let tree = file.raw_tree();
    let mut calls = Vec::new();
    walk_python_for_calls(tree.root_node(), file, &mut calls);
    Some(calls)
}

fn walk_python_for_calls(node: tree_sitter::Node, file: &IrFile, out: &mut Vec<CallSite>) {
    if node.kind() == "call" {
        if let Some(call) = parse_python_call(node, file) {
            out.push(call);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_python_for_calls(child, file, out);
    }
}

/// A Python call is in scope when its function operand is a bare
/// identifier (`foo(...)`) or a single-segment `self.` / `cls.` method
/// (`self.foo(...)` / `cls.foo(...)`, F3b); the receiver is dropped and
/// the attribute identifier becomes the callee, matching how the method
/// is registered under F4b. Deeper attribute access (`self.x.y(...)`,
/// `obj.foo(...)`) is out of v0 scope. Every argument must be a bare
/// `identifier`.
fn parse_python_call(node: tree_sitter::Node, file: &IrFile) -> Option<CallSite> {
    let function = node.child_by_field_name("function")?;
    let callee = match function.kind() {
        "identifier" => node_text(function, &file.source),
        "attribute" => python_self_method_name(function, &file.source)?,
        _ => return None,
    };

    let arguments = node.child_by_field_name("arguments")?;
    let mut args = Vec::new();
    let mut cursor = arguments.walk();
    for child in arguments.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if child.kind() == "identifier" {
            args.push(node_text(child, &file.source));
        } else {
            // keyword_argument, list_splat, dictionary_splat, literals,
            // attribute access, nested calls — anything other than a
            // bare identifier disqualifies the call from v0 swap analysis.
            return None;
        }
    }

    Some(CallSite {
        callee,
        args,
        location: node_location(file, node),
    })
}

/// Extract the method name from a `self.<name>` or `cls.<name>`
/// attribute node, or return `None` for any other receiver shape.
fn python_self_method_name(attribute: tree_sitter::Node, source: &str) -> Option<String> {
    let object = attribute.child_by_field_name("object")?;
    let attr = attribute.child_by_field_name("attribute")?;
    if object.kind() != "identifier" || attr.kind() != "identifier" {
        return None;
    }
    let receiver = node_text(object, source);
    if receiver != "self" && receiver != "cls" {
        return None;
    }
    Some(node_text(attr, source))
}

// ---------- TypeScript extraction (R-2.d) ----------

/// TypeScript definitions mirror Python: every IR function (top-level,
/// class method, and `const f = () => {}` declarator) is a candidate.
/// `ir_fn_to_def` drops the explicit `this` receiver and rejects
/// rest / destructuring / `_`-prefixed parameter shapes.
fn extract_typescript_fn_defs(file: &IrFile) -> Option<Vec<(String, FnDef)>> {
    if file.parse_recovered {
        return None;
    }
    Some(file.fns.iter().filter_map(ir_fn_to_def).collect())
}

fn extract_typescript_call_sites(file: &IrFile) -> Option<Vec<CallSite>> {
    if file.parse_recovered {
        return None;
    }
    let tree = file.raw_tree();
    let mut calls = Vec::new();
    walk_typescript_for_calls(tree.root_node(), file, &mut calls);
    Some(calls)
}

fn walk_typescript_for_calls(node: tree_sitter::Node, file: &IrFile, out: &mut Vec<CallSite>) {
    if node.kind() == "call_expression" {
        if let Some(call) = parse_typescript_call(node, file) {
            out.push(call);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_typescript_for_calls(child, file, out);
    }
}

/// A TypeScript call is in scope when its function operand is a bare
/// identifier (`foo(...)`) or a single-segment `this.<name>(...)` method
/// call (the receiver is dropped and the property name becomes the
/// callee, mirroring how methods register as definitions). Deeper member
/// access (`a.b.c(...)`, `obj.foo(...)`) is out of v0 scope. Every
/// argument must be a bare `identifier`.
fn parse_typescript_call(node: tree_sitter::Node, file: &IrFile) -> Option<CallSite> {
    let function = node.child_by_field_name("function")?;
    let callee = match function.kind() {
        "identifier" => node_text(function, &file.source),
        "member_expression" => typescript_this_method_name(function, &file.source)?,
        _ => return None,
    };

    let arguments = node.child_by_field_name("arguments")?;
    let mut args = Vec::new();
    let mut cursor = arguments.walk();
    for child in arguments.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if child.kind() == "identifier" {
            args.push(node_text(child, &file.source));
        } else {
            // spread_element, literals, member access, nested calls,
            // object / array literals — anything other than a bare
            // identifier disqualifies the call from v0 swap analysis.
            return None;
        }
    }

    Some(CallSite {
        callee,
        args,
        location: node_location(file, node),
    })
}

/// Extract the method name from a `this.<name>` member expression, or
/// return `None` for any other receiver shape (matching the Python
/// `self.` / `cls.` restriction).
fn typescript_this_method_name(member: tree_sitter::Node, source: &str) -> Option<String> {
    let object = member.child_by_field_name("object")?;
    let property = member.child_by_field_name("property")?;
    if object.kind() != "this" || property.kind() != "property_identifier" {
        return None;
    }
    Some(node_text(property, source))
}

// ---------- Shared helpers ----------

/// Source text of a raw tree-sitter node.
fn node_text(node: tree_sitter::Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

/// Build a [`Location`] from a raw tree-sitter node, mirroring the
/// v0.5.x `node_location` (1-based line / column, no byte offsets) so
/// the emitted finding shape stays byte-identical.
fn node_location(file: &IrFile, node: tree_sitter::Node) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        file: file.path.clone(),
        start_line: start.row as u32 + 1,
        start_col: start.column as u32 + 1,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32 + 1,
    }
}

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
