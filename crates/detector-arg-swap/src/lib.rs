//! arg-swap detector — Rice-style param/arg name matching for binary calls.
//!
//! Spec: `cntrdct/docs/spec/arg-swap-v0.md`.
//!
//! Algorithm:
//! 1. Parse each Rust file with tree-sitter.
//! 2. Across all files in the scan, build a map from function name to
//!    definitions (collecting parameter names).
//! 3. Walk each file's call_expression nodes whose function operand is a
//!    plain `identifier` and whose arguments are all plain `identifier`s.
//! 4. For binary callees with a unique 2-arg definition, check whether the
//!    argument identifier multiset is the swap permutation of the parameter
//!    name multiset (case-insensitive).
//! 5. Emit a Finding for each clean swap.

use std::collections::HashMap;

use cntrdct_core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding,
    Location, ParsedFile, Severity,
};

static CITATIONS: &[Citation] = &[
    Citation {
        key: "li-zhou-fse-2005",
        authors: "Z. Li, Y. Zhou",
        title: "PR-Miner: Automatically Extracting Implicit Programming Rules and Detecting Violations in Large Software Code",
        venue: "ESEC/FSE 2005",
        year: 2005,
        doi: None,
        url: None,
    },
    Citation {
        key: "rice-icse-2017",
        authors: "A. Rice, E. Aftandilian, C. Jaspan, E. Johnston, M. Pradel, Y. Arroyo-Paredes",
        title: "Detecting Argument Selection Defects",
        venue: "ICSE 2017",
        year: 2017,
        doi: None,
        url: None,
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

    fn supported_languages(&self) -> &'static [&'static str] {
        &["rust"]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut defs_by_name: HashMap<String, Vec<FnDef>> = HashMap::new();
        for file in ctx.files {
            if file.language != "rust" {
                continue;
            }
            if let Some(defs) = extract_fn_defs(file) {
                for (name, def) in defs {
                    defs_by_name.entry(name).or_default().push(def);
                }
            }
        }

        let mut findings: Vec<Finding> = Vec::new();
        for file in ctx.files {
            if file.language != "rust" {
                continue;
            }
            let Some(calls) = extract_call_sites(file) else {
                continue;
            };
            for call in calls {
                if call.args.len() != 2 {
                    continue;
                }
                let Some(candidates) = defs_by_name.get(&call.callee) else {
                    continue;
                };
                let matching: Vec<&FnDef> =
                    candidates.iter().filter(|d| d.params.len() == 2).collect();
                if matching.len() != 1 {
                    continue;
                }
                let def = matching[0];

                let a0 = call.args[0].to_lowercase();
                let a1 = call.args[1].to_lowercase();
                let p0 = def.params[0].to_lowercase();
                let p1 = def.params[1].to_lowercase();

                let identity = a0 == p0 && a1 == p1;
                let swapped = a0 == p1 && a1 == p0;

                if swapped && !identity {
                    findings.push(Finding {
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
                            citation_keys: vec!["li-zhou-fse-2005", "rice-icse-2017"],
                            raw: serde_json::json!({
                                "callee": call.callee,
                                "parameter_names": def.params.clone(),
                                "argument_names": call.args.clone(),
                            }),
                        },
                    });
                }
            }
        }

        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
        });

        Ok(findings)
    }
}

fn extract_fn_defs(file: &ParsedFile) -> Option<Vec<(String, FnDef)>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::language()).ok()?;
    let tree = parser.parse(&file.source, None)?;
    let root = tree.root_node();
    if root.has_error() {
        return None;
    }

    let mut defs = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_item" {
            if let Some(entry) = parse_fn_def(child, file) {
                defs.push(entry);
            }
        }
    }
    Some(defs)
}

fn parse_fn_def(node: tree_sitter::Node, file: &ParsedFile) -> Option<(String, FnDef)> {
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
        let param_name = extract_pattern_identifier(pattern, &file.source)?;
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

fn extract_pattern_identifier(node: tree_sitter::Node, source: &str) -> Option<String> {
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

fn extract_call_sites(file: &ParsedFile) -> Option<Vec<CallSite>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::language()).ok()?;
    let tree = parser.parse(&file.source, None)?;
    let root = tree.root_node();
    if root.has_error() {
        return None;
    }

    let mut calls = Vec::new();
    walk_for_calls(root, file, &mut calls);
    Some(calls)
}

fn walk_for_calls(
    node: tree_sitter::Node,
    file: &ParsedFile,
    out: &mut Vec<CallSite>,
) {
    if node.kind() == "call_expression" {
        if let Some(call) = parse_call(node, file) {
            out.push(call);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_calls(child, file, out);
    }
}

fn parse_call(node: tree_sitter::Node, file: &ParsedFile) -> Option<CallSite> {
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

fn node_text(node: tree_sitter::Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

fn node_location(file: &ParsedFile, node: tree_sitter::Node) -> Location {
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
