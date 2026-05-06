//! comment-code detector — pattern-based comment/implementation mismatch.
//!
//! Spec: `cntrdct/docs/spec/comment-code-v0.md`.
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A).
//!
//! Algorithm (Rust):
//! 1. Parse each Rust file with tree-sitter; skip files with parse errors.
//! 2. For each top-level `function_item`, gather the immediately preceding
//!    `///` line-comment block into a single rendered doc string.
//! 3. Apply three hardcoded checks (Pattern A/B/C) against the rendered text,
//!    the function's return type text, body source, and attribute set.
//! 4. Emit one Finding per match.
//!
//! Algorithm (Python):
//! 1. Parse each Python file with tree-sitter-python; skip parse errors.
//! 2. For each top-level `function_definition` (including those wrapped in
//!    `decorated_definition`), extract the docstring as the first statement
//!    of the function body when that statement is a bare string literal.
//! 3. Apply two checks: py-raises (doc claims a raise but body has no
//!    `raise_statement`) and py-deprecated (doc says deprecated but no
//!    `@deprecated`-style decorator on the function).
//! 4. Emit one Finding per match. Python findings carry
//!    `LanguageCitationStatus::Unconfirmed` per
//!    `docs/surveys/comment-code-python-2026-05.md`.

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, ParsedFile, Severity,
};
use rayon::prelude::*;

static CITATIONS: &[Citation] = &[
    Citation {
        key: "tan-sosp-2007",
        authors: "L. Tan, D. Yuan, G. Krishna, Y. Zhou",
        title: "/*iComment: Bugs or Bad Comments?*/",
        venue: "SOSP 2007",
        year: 2007,
        doi: None,
        url: None,
        languages: &[Language::Rust],
    },
    Citation {
        key: "tan-pldi-2011",
        authors: "L. Tan, Y. Zhou, Y. Padioleau",
        title: "aComment: Mining Annotations from Comments and Code to Detect Interrupt-related Concurrency Bugs",
        venue: "PLDI 2011",
        year: 2011,
        doi: None,
        url: None,
        languages: &[Language::Rust],
    },
];

const PATTERN_A_TRIGGERS: &[&str] = &[
    "returns err",
    "returns result",
    "may fail",
    "fallible",
    "returns option",
    "may return none",
];

const PATTERN_B_BODY_MARKERS: &[&str] = &[
    "panic!",
    "unwrap",
    "expect(",
    "unreachable!",
    "assert!",
    "assert_eq!",
    "assert_ne!",
    "todo!",
    "unimplemented!",
    "debug_assert",
];

#[derive(Debug, Default)]
pub struct CommentCode;

impl CommentCode {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for CommentCode {
    fn id(&self) -> &'static str {
        "comment-code"
    }

    fn name(&self) -> &'static str {
        "Comment/Code Mismatch"
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
                match file.language {
                    Language::Rust => collect_rust_findings(file, &mut local),
                    Language::Python => collect_python_findings(file, &mut local),
                }
                local
            })
            .collect();

        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
        });

        Ok(findings)
    }
}

fn collect_rust_findings(file: &ParsedFile, out: &mut Vec<Finding>) {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_rust::language()).is_err() {
        return;
    }
    let tree = match parser.parse(&file.source, None) {
        Some(t) => t,
        None => return,
    };
    let root = tree.root_node();
    if root.has_error() {
        return;
    }

    let mut cursor = root.walk();
    let children: Vec<tree_sitter::Node> = root.children(&mut cursor).collect();
    for (idx, child) in children.iter().enumerate() {
        if child.kind() != "function_item" {
            continue;
        }
        let doc = collect_preceding_doc(&children, idx, &file.source);
        if doc.is_empty() {
            continue;
        }
        let doc_lc = doc.to_lowercase();

        if let Some(trigger) = pattern_a_match(*child, &file.source, &doc_lc) {
            out.push(make_finding(file, *child, "A", trigger));
        }
        if let Some(trigger) = pattern_b_match(*child, &file.source, &doc_lc) {
            out.push(make_finding(file, *child, "B", trigger));
        }
        if let Some(trigger) = pattern_c_match(&children, idx, &file.source, &doc_lc) {
            out.push(make_finding(file, *child, "C", trigger));
        }
    }
}

/// Walk preceding siblings of `children[idx]` upward as long as they are
/// `///` line comments. Returns the rendered doc text (lines joined with
/// `\n`, prefix stripped). `//!` and plain `//` comments are ignored.
fn collect_preceding_doc(children: &[tree_sitter::Node], idx: usize, source: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let node = children[i];
        if node.kind() != "line_comment" {
            // Allow attribute items to sit between the doc block and the fn
            // (e.g. `/// docs\n#[inline]\nfn ...`). Skip them and keep walking.
            if node.kind() == "attribute_item" {
                continue;
            }
            break;
        }
        let text = &source[node.byte_range()];
        if let Some(rest) = text.strip_prefix("///") {
            let rendered = rest.strip_prefix(' ').unwrap_or(rest);
            lines.push(rendered.to_string());
        } else {
            break;
        }
    }
    lines.reverse();
    lines.join("\n")
}

fn pattern_a_match(node: tree_sitter::Node, source: &str, doc_lc: &str) -> Option<&'static str> {
    let trigger = PATTERN_A_TRIGGERS
        .iter()
        .find(|p| doc_lc.contains(*p))
        .copied()?;

    let return_type_text = match node.child_by_field_name("return_type") {
        Some(rt) => source[rt.byte_range()].to_string(),
        None => String::new(),
    };
    if return_type_text.contains("Result") || return_type_text.contains("Option") {
        return None;
    }
    Some(trigger)
}

fn pattern_b_match(node: tree_sitter::Node, source: &str, doc_lc: &str) -> Option<&'static str> {
    if !doc_lc.contains("panic") {
        return None;
    }
    let body = node.child_by_field_name("body")?;
    let body_text = &source[body.byte_range()];
    for marker in PATTERN_B_BODY_MARKERS {
        if body_text.contains(marker) {
            return None;
        }
    }
    Some("panic")
}

fn pattern_c_match(
    children: &[tree_sitter::Node],
    idx: usize,
    source: &str,
    doc_lc: &str,
) -> Option<&'static str> {
    if !doc_lc.contains("deprecated") {
        return None;
    }
    if preceding_siblings_have_deprecated(children, idx, source) {
        return None;
    }
    Some("deprecated")
}

/// Walk preceding siblings of `children[idx]` and look for `attribute_item`
/// nodes whose source text (after `#[`) begins with the identifier
/// `deprecated`. This catches `#[deprecated]`, `#[deprecated(note = "...")]`,
/// etc., while rejecting unrelated attributes like `#[inline]`. Stops at the
/// first non-comment, non-attribute sibling so we don't pick up attributes
/// that belong to a different item.
fn preceding_siblings_have_deprecated(
    children: &[tree_sitter::Node],
    idx: usize,
    source: &str,
) -> bool {
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let node = children[i];
        match node.kind() {
            "line_comment" => continue,
            "attribute_item" => {
                if attribute_item_is_deprecated(node, source) {
                    return true;
                }
            }
            _ => break,
        }
    }
    false
}

fn attribute_item_is_deprecated(node: tree_sitter::Node, source: &str) -> bool {
    let raw = &source[node.byte_range()];
    // Strip leading `#[` (or `#![` for inner attrs) and whitespace, then
    // check whether the first identifier is exactly `deprecated`.
    let after_open = raw
        .trim_start()
        .strip_prefix("#![")
        .or_else(|| raw.trim_start().strip_prefix("#["));
    let Some(after_open) = after_open else {
        return false;
    };
    let trimmed = after_open.trim_start();
    let first_ident: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    first_ident == "deprecated"
}

fn make_finding(
    file: &ParsedFile,
    node: tree_sitter::Node,
    pattern: &'static str,
    trigger: &'static str,
) -> Finding {
    make_finding_with_status(
        file,
        node,
        pattern,
        trigger,
        LanguageCitationStatus::Confirmed,
    )
}

fn make_finding_with_status(
    file: &ParsedFile,
    node: tree_sitter::Node,
    pattern: &'static str,
    trigger: &'static str,
    status: LanguageCitationStatus,
) -> Finding {
    Finding {
        detector_id: "comment-code".to_string(),
        primary: node_location(file, node),
        related: Vec::new(),
        message: format!(
            "doc comment claims '{}' but implementation does not match",
            trigger
        ),
        raw_severity: Severity::Note,
        anomaly_class: AnomalyClass::Documentation,
        evidence: Evidence {
            citation_keys: vec!["tan-sosp-2007", "tan-pldi-2011"],
            raw: serde_json::json!({
                "pattern": pattern,
                "trigger": trigger,
            }),
            language_citation_status: status,
        },
    }
}

// ---------- Python scan (M-3 Pattern A) ----------
//
// The walk + docstring extraction + finding emission share the same
// shape as the Rust path; what differs is the AST node-kind vocabulary,
// the comment representation (docstrings live INSIDE the function body
// as the first statement, not as preceding siblings), and the available
// patterns. Python lacks Rust's static return-type signal, so the
// Pattern A "Result/Option claim without matching return type" rule
// does not transfer; py-raises substitutes by checking body-level
// `raise_statement` presence (parallel to Rust's Pattern B). py-deprecated
// mirrors Rust's Pattern C with `@deprecated` decorator detection.

const PYTHON_RAISES_TRIGGERS: &[&str] = &["raises", "may raise", "throws"];

fn collect_python_findings(file: &ParsedFile, out: &mut Vec<Finding>) {
    let mut parser = tree_sitter::Parser::new();
    if parser
        .set_language(&tree_sitter_python::language())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(&file.source, None) else {
        return;
    };
    let root = tree.root_node();
    if root.has_error() {
        return;
    }

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => analyze_python_function(child, &[], file, out),
            "decorated_definition" => {
                let mut dcursor = child.walk();
                let kids: Vec<tree_sitter::Node> = child.children(&mut dcursor).collect();
                let decorators: Vec<tree_sitter::Node> = kids
                    .iter()
                    .filter(|c| c.kind() == "decorator")
                    .copied()
                    .collect();
                if let Some(fn_def) = kids.iter().find(|c| c.kind() == "function_definition") {
                    analyze_python_function(*fn_def, &decorators, file, out);
                }
            }
            _ => {}
        }
    }
}

fn analyze_python_function(
    fn_def: tree_sitter::Node,
    decorators: &[tree_sitter::Node],
    file: &ParsedFile,
    out: &mut Vec<Finding>,
) {
    let body = match fn_def.child_by_field_name("body") {
        Some(b) => b,
        None => return,
    };
    let doc = match extract_python_docstring(body, &file.source) {
        Some(d) => d,
        None => return,
    };
    let doc_lc = doc.to_lowercase();

    if let Some(trigger) = python_pattern_raises(&doc_lc, body) {
        out.push(make_finding_with_status(
            file,
            fn_def,
            "py-raises",
            trigger,
            LanguageCitationStatus::Unconfirmed,
        ));
    }
    if let Some(trigger) = python_pattern_deprecated(&doc_lc, decorators, &file.source) {
        out.push(make_finding_with_status(
            file,
            fn_def,
            "py-deprecated",
            trigger,
            LanguageCitationStatus::Unconfirmed,
        ));
    }
}

/// Extract the docstring text from a function `block` body, if the first
/// statement is a bare string literal. Strips an optional Python string
/// prefix (`r`, `b`, `f`, `u`, case-insensitive) and the surrounding
/// quotes (triple or single, matched pair).
fn extract_python_docstring(body_block: tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = body_block.walk();
    let first = body_block.children(&mut cursor).find(|c| c.is_named())?;
    if first.kind() != "expression_statement" {
        return None;
    }
    let mut inner_cursor = first.walk();
    let inner = first.children(&mut inner_cursor).find(|c| c.is_named())?;
    if inner.kind() != "string" {
        return None;
    }
    let raw = inner.utf8_text(source.as_bytes()).ok()?;
    Some(strip_python_string_quotes(raw))
}

fn strip_python_string_quotes(raw: &str) -> String {
    let trimmed = raw.trim();
    let after_prefix = trimmed.trim_start_matches(['r', 'R', 'b', 'B', 'f', 'F', 'u', 'U']);
    if let Some(s) = after_prefix
        .strip_prefix("\"\"\"")
        .and_then(|s| s.strip_suffix("\"\"\""))
    {
        return s.to_string();
    }
    if let Some(s) = after_prefix
        .strip_prefix("'''")
        .and_then(|s| s.strip_suffix("'''"))
    {
        return s.to_string();
    }
    if let Some(s) = after_prefix
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
    {
        return s.to_string();
    }
    if let Some(s) = after_prefix
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
    {
        return s.to_string();
    }
    after_prefix.to_string()
}

fn python_pattern_raises(doc_lc: &str, body: tree_sitter::Node) -> Option<&'static str> {
    let trigger = PYTHON_RAISES_TRIGGERS
        .iter()
        .find(|p| doc_lc.contains(*p))
        .copied()?;
    if body_contains_raise(body) {
        return None;
    }
    Some(trigger)
}

fn body_contains_raise(node: tree_sitter::Node) -> bool {
    if node.kind() == "raise_statement" {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if body_contains_raise(child) {
            return true;
        }
    }
    false
}

fn python_pattern_deprecated(
    doc_lc: &str,
    decorators: &[tree_sitter::Node],
    source: &str,
) -> Option<&'static str> {
    if !doc_lc.contains("deprecated") {
        return None;
    }
    if decorators
        .iter()
        .any(|d| decorator_is_deprecated(*d, source))
    {
        return None;
    }
    Some("deprecated")
}

/// Decide whether a `decorator` node names a `@deprecated`-style marker.
/// Recognises the bare identifier `deprecated`, the dotted forms
/// `warnings.deprecated`, `typing_extensions.deprecated`, and any other
/// dotted path whose final segment is `deprecated` (e.g.
/// `mypkg.compat.deprecated`). Decorator factories like
/// `@deprecated("reason")` are accepted because the name path is taken
/// before any `(`.
fn decorator_is_deprecated(node: tree_sitter::Node, source: &str) -> bool {
    let raw = match node.utf8_text(source.as_bytes()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let stripped = raw
        .trim_start()
        .strip_prefix('@')
        .unwrap_or(raw)
        .trim_start();
    let name_end = stripped
        .find(|c: char| c == '(' || c.is_whitespace())
        .unwrap_or(stripped.len());
    let name_path = &stripped[..name_end];
    let last = name_path.rsplit('.').next().unwrap_or(name_path);
    last == "deprecated"
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
