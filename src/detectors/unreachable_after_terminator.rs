//! unreachable-after-terminator detector — flag statements that follow a
//! divergent statement inside the same block.
//!
//! Spec: `cntrdct/docs/spec/unreachable-after-terminator-v0.md`.
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A).

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, ParsedFile, Severity,
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
                match file.language {
                    Language::Rust => scan_rust(file, &mut local),
                    Language::Python => scan_python(file, &mut local),
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

// ---------- Rust scan ----------

fn scan_rust(file: &ParsedFile, findings: &mut Vec<Finding>) {
    let mut parser = tree_sitter::Parser::new();
    let lang = crate::parsers::parser_for(Language::Rust).ts_language();
    if parser.set_language(&lang).is_err() {
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
    walk_rust(root, file, findings);
}

fn walk_rust(node: tree_sitter::Node, file: &ParsedFile, findings: &mut Vec<Finding>) {
    if node.kind() == "block" {
        analyze_rust_block(node, file, findings);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_rust(child, file, findings);
    }
}

fn analyze_rust_block(block: tree_sitter::Node, file: &ParsedFile, findings: &mut Vec<Finding>) {
    if is_rust_suppressed(block, &file.source) {
        return;
    }

    let stmts: Vec<tree_sitter::Node> = {
        let mut cursor = block.walk();
        block
            .children(&mut cursor)
            .filter(|c| is_rust_block_statement(*c))
            .collect()
    };

    for (i, stmt) in stmts.iter().enumerate() {
        if let Some(kind) = rust_terminator_kind(*stmt, &file.source) {
            // F4b: a cfg-gated statement is conditional and does NOT
            // qualify as a terminator. Skip and keep scanning so the
            // canonical complementary-cfg-pair idiom (each branch is
            // its own cfg-gated return) produces no finding.
            if is_cfg_gated_statement(*stmt, &file.source) {
                continue;
            }
            let following = stmts.len() - i - 1;
            if following == 0 {
                return;
            }
            let follower = stmts[i + 1];
            findings.push(build_finding(
                file,
                follower,
                *stmt,
                kind,
                following,
                LanguageCitationStatus::Confirmed,
            ));
            return;
        }
    }
}

/// True when `stmt` is preceded (within its block) by one or more
/// `#[cfg(...)]` attribute_items. `cfg_attr(...)` does NOT count: that
/// form conditionally applies an inner attribute while the statement
/// itself runs unconditionally.
fn is_cfg_gated_statement(stmt: tree_sitter::Node, source: &str) -> bool {
    let mut sib = stmt.prev_named_sibling();
    while let Some(s) = sib {
        if s.kind() != "attribute_item" {
            break;
        }
        if attribute_item_is_cfg(s, source) {
            return true;
        }
        sib = s.prev_named_sibling();
    }
    false
}

/// True when an `attribute_item`'s first identifier (after `#[` or `#![`)
/// is exactly `cfg`. Distinguishes `#[cfg(...)]` from the unrelated
/// `#[cfg_attr(...)]` form.
fn attribute_item_is_cfg(attr: tree_sitter::Node, source: &str) -> bool {
    let raw = match attr.utf8_text(source.as_bytes()) {
        Ok(s) => s,
        Err(_) => return false,
    };
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
    first_ident == "cfg"
}

fn is_rust_block_statement(node: tree_sitter::Node) -> bool {
    if !node.is_named() {
        return false;
    }
    // F4c: item declarations inside a block are hoisted by the
    // compiler — they do not execute in source order, so a `fn`
    // (or any other item) appearing after a `return` is NOT
    // unreachable code. Filter all item kinds plus the existing
    // attribute/comment exclusions and the no-op empty statement.
    !matches!(
        node.kind(),
        "inner_attribute_item"
            | "attribute_item"
            | "line_comment"
            | "block_comment"
            | "empty_statement"
            | "function_item"
            | "function_signature_item"
            | "mod_item"
            | "foreign_mod_item"
            | "struct_item"
            | "union_item"
            | "enum_item"
            | "type_item"
            | "const_item"
            | "static_item"
            | "trait_item"
            | "impl_item"
            | "use_declaration"
            | "extern_crate_declaration"
            | "associated_type"
            | "macro_definition"
    )
}

fn rust_terminator_kind(stmt: tree_sitter::Node, source: &str) -> Option<&'static str> {
    if stmt.kind() != "expression_statement" {
        return None;
    }
    let mut cursor = stmt.walk();
    let inner = stmt.children(&mut cursor).find(|c| c.is_named())?;
    match inner.kind() {
        "return_expression" => Some("return"),
        "break_expression" => Some("break"),
        "continue_expression" => Some("continue"),
        "macro_invocation" => rust_macro_terminator_name(inner, source),
        _ => None,
    }
}

fn rust_macro_terminator_name(call: tree_sitter::Node, source: &str) -> Option<&'static str> {
    let macro_node = call.child_by_field_name("macro")?;
    let text = macro_node.utf8_text(source.as_bytes()).ok()?;
    let last = text.rsplit("::").next().unwrap_or(text);
    TERMINATOR_MACROS.iter().copied().find(|&m| m == last)
}

fn is_rust_suppressed(node: tree_sitter::Node, source: &str) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            if matches!(child.kind(), "attribute_item" | "inner_attribute_item")
                && rust_attribute_contains(child, source, SUPPRESSION_TOKEN)
            {
                return true;
            }
        }
        let mut sib = n.prev_named_sibling();
        while let Some(s) = sib {
            if s.kind() == "attribute_item" {
                if rust_attribute_contains(s, source, SUPPRESSION_TOKEN) {
                    return true;
                }
                sib = s.prev_named_sibling();
            } else {
                break;
            }
        }
        current = n.parent();
    }
    false
}

fn rust_attribute_contains(attr: tree_sitter::Node, source: &str, token: &str) -> bool {
    attr.utf8_text(source.as_bytes())
        .map(|t| t.contains(token))
        .unwrap_or(false)
}

// ---------- Python scan ----------
//
// Pattern A: the walk + post-terminator-statement detection is shared
// with Rust at the algorithmic level; what differs is the AST node-kind
// vocabulary and the terminator set. tree-sitter-python uses `block`
// for indented bodies (function, class, if/for/while/with/try) the same
// way tree-sitter-rust uses `block` for braced bodies, so the same
// outer recursion structure applies.
//
// Suppression: Q-9 introduced `# cntrdct: allow(<id>)` line-comment
// suppression for Python (mirrors the Rust attribute form at line
// granularity; trailing form covers a single line, standalone form
// covers the next named sibling's span). This detector emits findings
// at AST nodes; the suppression filter in `crate::config::apply` walks
// `# cntrdct: allow(...)` comments via tree-sitter-python and drops
// matches before SARIF emission. Project-level suppression via
// `cntrdct.toml` (T2-7 / M-5) continues to apply.

fn scan_python(file: &ParsedFile, findings: &mut Vec<Finding>) {
    let mut parser = tree_sitter::Parser::new();
    let lang = crate::parsers::parser_for(Language::Python).ts_language();
    if parser.set_language(&lang).is_err() {
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
    walk_python(root, file, findings);
}

fn walk_python(node: tree_sitter::Node, file: &ParsedFile, findings: &mut Vec<Finding>) {
    if node.kind() == "block" {
        analyze_python_block(node, file, findings);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_python(child, file, findings);
    }
}

fn analyze_python_block(block: tree_sitter::Node, file: &ParsedFile, findings: &mut Vec<Finding>) {
    let stmts: Vec<tree_sitter::Node> = {
        let mut cursor = block.walk();
        block
            .children(&mut cursor)
            .filter(|c| is_python_block_statement(*c))
            .collect()
    };

    for (i, stmt) in stmts.iter().enumerate() {
        if let Some(kind) = python_terminator_kind(*stmt, &file.source) {
            let following = stmts.len() - i - 1;
            if following == 0 {
                return;
            }
            let follower = stmts[i + 1];
            findings.push(build_finding(
                file,
                follower,
                *stmt,
                kind,
                following,
                LanguageCitationStatus::Unconfirmed,
            ));
            return;
        }
    }
}

fn is_python_block_statement(node: tree_sitter::Node) -> bool {
    if !node.is_named() {
        return false;
    }
    // Module-level docstrings parse as `expression_statement` containing
    // a `string`; we still treat them as statements. Comments are
    // skipped because they cannot be reached by control flow.
    !matches!(node.kind(), "comment")
}

fn python_terminator_kind(stmt: tree_sitter::Node, source: &str) -> Option<&'static str> {
    match stmt.kind() {
        "return_statement" => Some("return"),
        "raise_statement" => Some("raise"),
        "break_statement" => Some("break"),
        "continue_statement" => Some("continue"),
        "assert_statement" => python_assert_terminator(stmt),
        "expression_statement" => python_expression_statement_terminator(stmt, source),
        _ => None,
    }
}

/// `assert False` (or `assert 0` / `assert None`) raises `AssertionError`
/// unconditionally. Only the literal `False` form is treated as a
/// terminator in v0; constant-folding `0` / `None` is out of scope.
fn python_assert_terminator(stmt: tree_sitter::Node) -> Option<&'static str> {
    let mut cursor = stmt.walk();
    let cond = stmt.children(&mut cursor).find(|c| c.is_named())?;
    if cond.kind() == "false" {
        Some("assert")
    } else {
        None
    }
}

fn python_expression_statement_terminator(
    stmt: tree_sitter::Node,
    source: &str,
) -> Option<&'static str> {
    let mut cursor = stmt.walk();
    let inner = stmt.children(&mut cursor).find(|c| c.is_named())?;
    if inner.kind() != "call" {
        return None;
    }
    python_exit_call_kind(inner, source)
}

fn python_exit_call_kind(call: tree_sitter::Node, source: &str) -> Option<&'static str> {
    let func = call.child_by_field_name("function")?;
    let text = func.utf8_text(source.as_bytes()).ok()?;
    let normalized = text.trim();
    PYTHON_EXIT_FUNCTIONS
        .iter()
        .copied()
        .find(|&name| name == normalized)
}

// ---------- Shared finding construction ----------

fn build_finding(
    file: &ParsedFile,
    follower: tree_sitter::Node,
    terminator: tree_sitter::Node,
    kind: &'static str,
    following_count: usize,
    citation_status: LanguageCitationStatus,
) -> Finding {
    let primary = node_location(file, follower);
    let related = vec![node_location(file, terminator)];
    let terminator_line = related[0].start_line;
    Finding {
        detector_id: "unreachable-after-terminator".to_string(),
        primary,
        related,
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
