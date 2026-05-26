//! arg-swap detector — Rice-style param/arg name matching for binary calls.
//!
//! Spec: `cntrdct/docs/spec/arg-swap-v0.md` (Rust v0).
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A).
//!
//! Algorithm (Rust):
//! 1. Parse each Rust file with tree-sitter.
//! 2. Across all files in the scan, build a map from function name to
//!    definitions (collecting parameter names) for top-level `fn` items.
//! 3. Walk each file's `call_expression` nodes whose function operand is a
//!    plain `identifier` and whose arguments are all plain `identifier`s.
//! 4. For binary callees with a unique 2-arg definition, check whether the
//!    argument identifier multiset is the swap permutation of the parameter
//!    name multiset (case-insensitive).
//! 5. Emit a Finding for each clean swap.
//!
//! Algorithm (Python):
//! Same shape as Rust. Definitions are top-level `function_definition`
//! (including those wrapped in `decorated_definition`). Calls are bare
//! `call` nodes whose `function` field is an `identifier` and whose
//! `argument_list` contains exactly two bare `identifier` arguments
//! (keyword arguments, splats, and non-identifier expressions disqualify
//! the call). Python findings carry
//! `LanguageCitationStatus::Confirmed` grounded by Allamanis et al.
//! NeurIPS 2021 (PyBugLab + PyPIBugs); see
//! `docs/surveys/arg-swap-python-2026-05.md` for the survey. Rust
//! definitions never match Python calls and vice versa: each pipeline
//! runs over its own language's files only.

use std::collections::HashMap;

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::IrFile;
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

// ---------- Rust extraction ----------

fn extract_rust_fn_defs(file: &IrFile) -> Option<Vec<(String, FnDef)>> {
    if file.parse_recovered {
        return None;
    }
    let raw_tree = file.raw_tree();
    let root = raw_tree.root_node();
    let mut defs = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_item" {
            if let Some(entry) = parse_rust_fn_def(child, file) {
                defs.push(entry);
            }
        }
    }
    Some(defs)
}

fn parse_rust_fn_def(node: tree_sitter::Node, file: &IrFile) -> Option<(String, FnDef)> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, &file.source);

    let params_node = node.child_by_field_name("parameters")?;
    let mut params: Vec<String> = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if child.kind() != "parameter" {
            continue;
        }
        let pattern = child.child_by_field_name("pattern")?;
        let param_name = extract_rust_pattern_identifier(pattern, &file.source)?;
        if param_name.starts_with('_') {
            return None;
        }
        params.push(param_name);
    }

    Some((
        name,
        FnDef {
            params,
            location: node_location(file, node),
        },
    ))
}

fn extract_rust_pattern_identifier(node: tree_sitter::Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "mut_pattern" | "ref_pattern" => {
            let mut cursor = node.walk();
            for c in node.children(&mut cursor) {
                if c.kind() == "identifier" {
                    return Some(node_text(c, source));
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_rust_call_sites(file: &IrFile) -> Option<Vec<CallSite>> {
    if file.parse_recovered {
        return None;
    }
    let raw_tree = file.raw_tree();
    let root = raw_tree.root_node();
    let mut calls = Vec::new();
    walk_rust_for_calls(root, file, &mut calls);
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

// ---------- Python extraction ----------
//
// Pattern A reuse: the algorithm shape (collect 2-arg top-level defs,
// walk all call sites, match by name and check the swap permutation)
// is identical. Differences are the AST-node vocabulary and the
// parameter / argument shapes Python admits. v0 conservatively
// excludes any def that mentions `*args`, `**kwargs`, the `/` and `*`
// separators, or whose parameter list contains anything other than a
// plain `identifier` / `typed_parameter` / `default_parameter` /
// `typed_default_parameter`. Calls with keyword arguments, splats, or
// non-identifier expressions are skipped on the call side.
//
// Top-level only is intentional: methods inside a `class_definition`,
// closures inside another function body, and `lambda` expressions are
// out of v0 scope (mirrors the Rust path which only walks direct
// children of the module root).

fn extract_python_fn_defs(file: &IrFile) -> Option<Vec<(String, FnDef)>> {
    if file.parse_recovered {
        return None;
    }
    let raw_tree = file.raw_tree();
    let root = raw_tree.root_node();
    let mut defs = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(entry) = parse_python_fn_def(child, file, false) {
                    defs.push(entry);
                }
            }
            "decorated_definition" => {
                if let Some(entry) = parse_python_decorated_def(child, file, false) {
                    defs.push(entry);
                }
            }
            // F4b (added 2026-05-21): walk into class_definition bodies
            // so methods become candidates for swap resolution. Methods
            // are parsed with `is_method = true`; the `self` parameter
            // is filtered out before arity checks so a 2-positional
            // method matches the existing 2-arg call shape.
            "class_definition" => {
                collect_python_class_methods(child, file, &mut defs);
            }
            _ => {}
        }
    }
    Some(defs)
}

fn collect_python_class_methods(
    class_node: tree_sitter::Node,
    file: &IrFile,
    defs: &mut Vec<(String, FnDef)>,
) {
    let Some(body) = class_node.child_by_field_name("body") else {
        return;
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(entry) = parse_python_fn_def(child, file, true) {
                    defs.push(entry);
                }
            }
            "decorated_definition" => {
                if let Some(entry) = parse_python_decorated_def(child, file, true) {
                    defs.push(entry);
                }
            }
            _ => {}
        }
    }
}

fn parse_python_decorated_def(
    decorated: tree_sitter::Node,
    file: &IrFile,
    is_method: bool,
) -> Option<(String, FnDef)> {
    let mut cursor = decorated.walk();
    let fn_def = decorated
        .children(&mut cursor)
        .find(|c| c.kind() == "function_definition")?;
    parse_python_fn_def(fn_def, file, is_method)
}

fn parse_python_fn_def(
    node: tree_sitter::Node,
    file: &IrFile,
    is_method: bool,
) -> Option<(String, FnDef)> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, &file.source);

    let params_node = node.child_by_field_name("parameters")?;
    let mut params: Vec<String> = Vec::new();
    let mut cursor = params_node.walk();
    let mut first = true;
    for child in params_node.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        let param_name = match child.kind() {
            "identifier" => node_text(child, &file.source),
            "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                let mut ic = child.walk();
                let kids: Vec<tree_sitter::Node> = child.children(&mut ic).collect();
                let id = kids.iter().find(|c| c.kind() == "identifier")?;
                node_text(*id, &file.source)
            }
            // *args, **kwargs, /, *, and any other parameter shape we
            // don't model — reject the entire definition rather than
            // silently producing a wrong arity match.
            _ => return None,
        };
        // F4b: drop the implicit `self` (or `cls`) receiver of a
        // class method before applying the `_`-prefix rejection rule.
        // A leading `self`/`cls` is a Python convention — not a real
        // semantic parameter for swap analysis.
        if is_method && first && matches!(param_name.as_str(), "self" | "cls") {
            first = false;
            continue;
        }
        first = false;
        if param_name.starts_with('_') {
            return None;
        }
        params.push(param_name);
    }

    Some((
        name,
        FnDef {
            params,
            location: node_location(file, node),
        },
    ))
}

fn extract_python_call_sites(file: &IrFile) -> Option<Vec<CallSite>> {
    if file.parse_recovered {
        return None;
    }
    let raw_tree = file.raw_tree();
    let root = raw_tree.root_node();
    let mut calls = Vec::new();
    walk_python_for_calls(root, file, &mut calls);
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

fn parse_python_call(node: tree_sitter::Node, file: &IrFile) -> Option<CallSite> {
    let function = node.child_by_field_name("function")?;
    // F3b (added 2026-05-21): accept `self.<name>(...)` and
    // `cls.<name>(...)` method calls in addition to bare-identifier
    // calls. The receiver name is dropped; the attribute's
    // identifier becomes the callee, mirroring how the method is
    // registered in the per-file definition table under F4b.
    // Deeper attribute access (`self.x.y(...)`, `obj.foo(...)`) is
    // still out of v0 scope — only the single-segment `self.foo` /
    // `cls.foo` shapes are admitted.
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
            // attribute access, nested calls — anything other than a bare
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

/// Extract the method name from a `self.<name>` or `cls.<name>`
/// attribute node, or return None for any other receiver shape.
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

// ---------- Shared helpers ----------

fn node_text(node: tree_sitter::Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

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
