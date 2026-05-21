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
    // F4d-ii: call_expression whose argument list contains a divergent
    // expression. The arguments are evaluated left-to-right, so any
    // following argument (or the call itself, when the divergent
    // expression is the last argument) is unreachable. macro_invocation
    // is excluded — tree-sitter-rust does not re-parse macro token
    // trees as Rust expressions, so `panic!(return)`-style cases are
    // not visible.
    if node.kind() == "call_expression" {
        analyze_rust_call_args(node, file, findings);
    }
    // F4d-iii: return / break with a divergent return value. The
    // value is evaluated before the surrounding control transfer
    // takes effect, so the outer return / break is itself unreachable.
    if matches!(node.kind(), "return_expression" | "break_expression") {
        analyze_rust_divergent_carrier(node, file, findings);
    }
    // F4d-iv: if-expression whose condition is a divergent
    // expression. The consequence block is unreachable from the
    // outside since the condition never produces a value.
    if node.kind() == "if_expression" {
        analyze_rust_if_condition(node, file, findings);
    }
    // Closure bodies and async blocks introduce a hard break-target
    // boundary (Rust forbids `break` from escaping them). Descend for
    // the other rules — F4d-i / ii / iii / iv still apply inside —
    // but stop recursion here so the outer `walk_rust` continues with
    // unchanged scope; the break-target search inside F4d-v handles
    // the boundary itself via `rust_has_break_targeting_self`.
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
    // F4d-i: a bare if_expression / match_expression that sits as a
    // block statement is itself the candidate (no expression_statement
    // wrapper because no trailing `;` is required for brace-bound
    // expressions). Treat both forms uniformly.
    let inner = match stmt.kind() {
        "expression_statement" => {
            let mut cursor = stmt.walk();
            let child = stmt.children(&mut cursor).find(|c| c.is_named());
            child?
        }
        "if_expression" | "match_expression" => stmt,
        _ => return None,
    };
    match inner.kind() {
        "return_expression" => Some("return"),
        "break_expression" => Some("break"),
        "continue_expression" => Some("continue"),
        "macro_invocation" => rust_macro_terminator_name(inner, source),
        // F4d-i: branch-merge. An if / match whose every branch ends
        // in a divergent expression is itself divergent — any
        // statement that follows in the enclosing block is
        // unreachable.
        "if_expression" => rust_if_all_branches_diverge(inner, source),
        "match_expression" => rust_match_all_arms_diverge(inner, source),
        // F4d-v: a bare `loop { ... }` whose body never `break`s out
        // of it diverges — the loop never terminates, so any statement
        // following the loop in the enclosing block is unreachable.
        // The targeting analysis lives in `rust_loop_diverges`.
        "loop_expression" => rust_loop_diverges(inner, source),
        _ => None,
    }
}

// ---------- F4d divergent expression classifier ----------

/// True iff evaluating `expr` always diverges (never produces a value).
/// Returns the canonical terminator-kind string for the divergence so
/// the surrounding emission carries a useful `terminator_kind`.
///
/// Recursion follows the AST hierarchy; tree-sitter trees are finite
/// so termination is guaranteed.
fn rust_expression_diverges(expr: tree_sitter::Node, source: &str) -> Option<&'static str> {
    match expr.kind() {
        "return_expression" => Some("return"),
        "break_expression" => Some("break"),
        "continue_expression" => Some("continue"),
        "macro_invocation" => rust_macro_terminator_name(expr, source),
        "block" => rust_block_diverges(expr, source),
        "if_expression" => rust_if_all_branches_diverge(expr, source),
        "match_expression" => rust_match_all_arms_diverge(expr, source),
        "loop_expression" => rust_loop_diverges(expr, source),
        _ => None,
    }
}

fn rust_block_diverges(block: tree_sitter::Node, source: &str) -> Option<&'static str> {
    let stmts: Vec<tree_sitter::Node> = {
        let mut cursor = block.walk();
        block
            .children(&mut cursor)
            .filter(|c| is_rust_block_statement(*c))
            .collect()
    };
    if stmts.is_empty() {
        return None;
    }
    for stmt in &stmts {
        if let Some(k) = rust_terminator_kind(*stmt, source) {
            return Some(k);
        }
    }
    // Tail position: the last named child may be an expression rather
    // than a statement. Its divergence determines the block's.
    let last = *stmts.last().expect("checked non-empty above");
    rust_expression_diverges(last, source)
}

fn rust_if_all_branches_diverge(if_expr: tree_sitter::Node, source: &str) -> Option<&'static str> {
    let consequence = if_expr.child_by_field_name("consequence")?;
    let alternative = if_expr.child_by_field_name("alternative")?;
    rust_expression_diverges(consequence, source)?;
    rust_alternative_diverges(alternative, source)?;
    Some("if-branches-diverge")
}

fn rust_alternative_diverges(alt: tree_sitter::Node, source: &str) -> Option<&'static str> {
    // else_clause wraps either a `block` (else { ... }) or another
    // `if_expression` (else if ...). Find the first named child that
    // is one of these and delegate.
    let mut cursor = alt.walk();
    for child in alt.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if matches!(child.kind(), "block" | "if_expression") {
            return rust_expression_diverges(child, source);
        }
    }
    None
}

fn rust_match_all_arms_diverge(
    match_expr: tree_sitter::Node,
    source: &str,
) -> Option<&'static str> {
    let body = match_expr.child_by_field_name("body")?;
    let mut cursor = body.walk();
    let arms: Vec<tree_sitter::Node> = body
        .children(&mut cursor)
        .filter(|c| c.kind() == "match_arm")
        .collect();
    if arms.is_empty() {
        return None;
    }
    for arm in &arms {
        let value = arm.child_by_field_name("value")?;
        rust_expression_diverges(value, source)?;
    }
    Some("match-arms-diverge")
}

/// F4d-v: a `loop_expression` diverges iff no `break_expression`
/// inside its body targets this same loop. The targeting analysis
/// resolves labelled `break 'name` against the loop's own label and
/// unlabelled `break` against the innermost enclosing loop-like
/// construct (`loop_expression` / `while_expression` / `for_expression`).
fn rust_loop_diverges(loop_node: tree_sitter::Node, source: &str) -> Option<&'static str> {
    let body = rust_loop_body(loop_node)?;
    let self_label = rust_loop_label(loop_node, source);
    if rust_has_break_targeting_self(body, self_label.as_deref(), 0, source) {
        None
    } else {
        Some("loop-no-break")
    }
}

/// Locate a `loop_expression`'s body block. tree-sitter-rust does not
/// expose a `body` field for `loop_expression`, so iterate children
/// and pick the first `block`.
fn rust_loop_body(loop_node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut cursor = loop_node.walk();
    let found = loop_node
        .children(&mut cursor)
        .find(|c| c.kind() == "block");
    found
}

/// Return the bare identifier name of a `loop_expression`'s label
/// (e.g. `'outer` -> `"outer"`), or `None` for an unlabelled loop.
/// The `label` child node has the shape `label { identifier }`.
fn rust_loop_label(loop_node: tree_sitter::Node, source: &str) -> Option<String> {
    rust_label_identifier(loop_node, source)
}

/// Return the target identifier of a `break_expression`'s label
/// (e.g. `break 'outer` -> `"outer"`), or `None` for an unlabelled
/// break.
fn rust_break_label(break_node: tree_sitter::Node, source: &str) -> Option<String> {
    rust_label_identifier(break_node, source)
}

fn rust_label_identifier(parent: tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = parent.walk();
    for child in parent.children(&mut cursor) {
        if child.kind() != "label" {
            continue;
        }
        let mut inner = child.walk();
        for inner_child in child.children(&mut inner) {
            if inner_child.kind() == "identifier" {
                return inner_child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(str::to_owned);
            }
        }
    }
    None
}

/// True iff a `break_expression` exists inside `node` whose target is
/// the loop that owns `self_label`. `nesting_depth` counts the number
/// of loop-like ancestors between `node` and the candidate loop:
/// an unlabelled `break` only targets the candidate when `nesting_depth
/// == 0`. `closure_expression` and `async_block` are not descended
/// into — Rust forbids `break` from escaping either, so any break
/// inside them cannot target an outer loop.
fn rust_has_break_targeting_self(
    node: tree_sitter::Node,
    self_label: Option<&str>,
    nesting_depth: u32,
    source: &str,
) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if matches!(kind, "closure_expression" | "async_block") {
            continue;
        }
        if kind == "break_expression" {
            match rust_break_label(child, source) {
                Some(target) => {
                    if Some(target.as_str()) == self_label {
                        return true;
                    }
                }
                None => {
                    if nesting_depth == 0 {
                        return true;
                    }
                }
            }
        }
        let new_depth = if matches!(
            kind,
            "loop_expression" | "while_expression" | "for_expression"
        ) {
            nesting_depth + 1
        } else {
            nesting_depth
        };
        if rust_has_break_targeting_self(child, self_label, new_depth, source) {
            return true;
        }
    }
    false
}

// ---------- F4d-ii / F4d-iii / F4d-iv emission helpers ----------

fn analyze_rust_call_args(call: tree_sitter::Node, file: &ParsedFile, findings: &mut Vec<Finding>) {
    let Some(args_node) = call.child_by_field_name("arguments") else {
        return;
    };
    let mut cursor = args_node.walk();
    let args: Vec<tree_sitter::Node> = args_node
        .children(&mut cursor)
        .filter(|c| {
            c.is_named()
                && !matches!(
                    c.kind(),
                    "attribute_item" | "line_comment" | "block_comment"
                )
        })
        .collect();
    for (i, arg) in args.iter().enumerate() {
        if let Some(kind) = rust_expression_diverges(*arg, &file.source) {
            // Subsequent argument is unreachable iff one exists;
            // otherwise the call as a whole is unreachable (the
            // function is never invoked because the argument
            // evaluation diverges first).
            let follower = if i + 1 < args.len() {
                args[i + 1]
            } else {
                call
            };
            let following_count = args.len().saturating_sub(i + 1).max(1) as u32;
            findings.push(build_finding(
                file,
                follower,
                *arg,
                kind,
                following_count as usize,
                LanguageCitationStatus::Confirmed,
            ));
            return;
        }
    }
}

fn analyze_rust_divergent_carrier(
    expr: tree_sitter::Node,
    file: &ParsedFile,
    findings: &mut Vec<Finding>,
) {
    // For `return EXPR` or `break EXPR`, the inner value is evaluated
    // before the surrounding control transfer takes effect. If the
    // value itself diverges, the outer return / break never runs.
    let mut cursor = expr.walk();
    let value = expr
        .children(&mut cursor)
        .find(|c| c.is_named() && c.kind() != "loop_label");
    let Some(value) = value else { return };
    let Some(kind) = rust_expression_diverges(value, &file.source) else {
        return;
    };
    findings.push(build_finding(
        file,
        expr,
        value,
        kind,
        1,
        LanguageCitationStatus::Confirmed,
    ));
}

fn analyze_rust_if_condition(
    if_expr: tree_sitter::Node,
    file: &ParsedFile,
    findings: &mut Vec<Finding>,
) {
    let Some(condition) = if_expr.child_by_field_name("condition") else {
        return;
    };
    let Some(kind) = rust_expression_diverges(condition, &file.source) else {
        return;
    };
    let Some(consequence) = if_expr.child_by_field_name("consequence") else {
        return;
    };
    findings.push(build_finding(
        file,
        consequence,
        condition,
        kind,
        1,
        LanguageCitationStatus::Confirmed,
    ));
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
    // F4e: constant-condition `if` / `while` branch reachability.
    // CodeQL's `UnreachableCode` query flags the body of
    // `while False:` / `while 0:` and the unreachable arm of
    // `if False: ... else: ...` / `if True: ... else: ...`. The
    // classifier `python_constant_condition` recognises only the
    // four literal forms named in F4e (bool / integer / None /
    // empty string); other shapes fall back to indeterminate.
    if node.kind() == "while_statement" {
        analyze_python_while_constant(node, file, findings);
    }
    if node.kind() == "if_statement" {
        analyze_python_if_constant(node, file, findings);
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

/// F4e classifier — evaluate the truthiness of a Python condition
/// expression at parse time. Returns `Some(true)` for truthy
/// constants, `Some(false)` for falsy constants, and `None` for any
/// expression that is not a recognised literal (identifier, call,
/// boolean operator, parenthesised expression, etc. all return
/// `None`). v0 recognises four literal forms:
///
/// - `false` / `true` keyword tokens.
/// - `integer` literal whose text parses to exactly `0` (falsy) or
///   any other integer (truthy). Hex, binary, octal forms are NOT
///   accepted in v0; only base-10 `0` is recognised as falsy.
/// - `none` keyword token (falsy).
/// - `string` whose surface text contains no characters between its
///   delimiters (falsy). Triple-quoted and prefixed strings (`b""`,
///   `r""`, `f""`) are also accepted at the kind level; the inner
///   emptiness check is identical.
fn python_constant_condition(node: tree_sitter::Node, source: &str) -> Option<bool> {
    match node.kind() {
        "false" => Some(false),
        "true" => Some(true),
        "none" => Some(false),
        "integer" => {
            let text = node.utf8_text(source.as_bytes()).ok()?;
            // Base-10 only: `0`, `00`, etc. produce falsy; non-zero
            // integers produce truthy. Hex / binary / octal literals
            // (`0x0`, `0b0`, `0o0`) are explicitly excluded in v0.
            let trimmed = text.trim();
            if trimmed.starts_with("0x")
                || trimmed.starts_with("0X")
                || trimmed.starts_with("0b")
                || trimmed.starts_with("0B")
                || trimmed.starts_with("0o")
                || trimmed.starts_with("0O")
            {
                return None;
            }
            let value: i128 = trimmed.replace('_', "").parse().ok()?;
            Some(value != 0)
        }
        "string" => {
            // tree-sitter-python `string` wraps `string_start`,
            // optional `string_content`, and `string_end`. Empty iff
            // the only named children are start/end (no content).
            let mut cursor = node.walk();
            let has_content = node
                .children(&mut cursor)
                .any(|c| c.kind() == "string_content");
            Some(has_content)
        }
        _ => None,
    }
}

fn analyze_python_while_constant(
    while_node: tree_sitter::Node,
    file: &ParsedFile,
    findings: &mut Vec<Finding>,
) {
    let mut cursor = while_node.walk();
    let mut named = while_node.children(&mut cursor).filter(|c| c.is_named());
    let condition = match named.next() {
        Some(n) => n,
        None => return,
    };
    if let Some(false) = python_constant_condition(condition, &file.source) {
        let body = match while_node.child_by_field_name("body") {
            Some(b) => b,
            None => return,
        };
        let first_stmt = first_named_stmt_in_python_block(body);
        if let Some(stmt) = first_stmt {
            findings.push(build_finding(
                file,
                stmt,
                condition,
                "constant-false-while",
                1,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    }
}

fn analyze_python_if_constant(
    if_node: tree_sitter::Node,
    file: &ParsedFile,
    findings: &mut Vec<Finding>,
) {
    let if_children: Vec<tree_sitter::Node> = {
        let mut cursor = if_node.walk();
        if_node.children(&mut cursor).collect()
    };
    let condition = if_children.iter().find(|c| c.is_named()).copied();
    let Some(condition) = condition else {
        return;
    };
    let cond_value = python_constant_condition(condition, &file.source);
    let Some(cond_value) = cond_value else {
        return;
    };

    let consequence = match if_node.child_by_field_name("consequence") {
        Some(b) => b,
        None => return,
    };
    let else_clause = if_children
        .iter()
        .find(|c| c.kind() == "else_clause")
        .copied();

    if !cond_value {
        // F4e-ii: `if False:` consequence is unreachable. Two carve-
        // outs match CodeQL's UnreachableCode fixture explicit non-
        // findings: (a) type-checking import guards
        // (`if False: from X import Y`) and (b) the generator-marker
        // idiom (`if False: yield ...`). Both produce no runtime code
        // and are deliberate by-design rather than bugs.
        if python_if_false_body_is_carveout(consequence) {
            return;
        }
        if let Some(stmt) = first_named_stmt_in_python_block(consequence) {
            findings.push(build_finding(
                file,
                stmt,
                condition,
                "constant-false-if",
                1,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    } else if let Some(else_clause) = else_clause {
        // F4e-iii: `if True: ... else: <unreachable>`. The else_clause
        // wraps a block child whose first statement is the unreachable
        // entry. `elif` branches under a truthy `if` are also
        // unreachable but they parse as `elif_clause` siblings of the
        // else; v0 reports only the immediate `else_clause` body to
        // keep the FP surface narrow. Multi-branch widening is a v1
        // non-goal.
        let else_children: Vec<tree_sitter::Node> = {
            let mut inner = else_clause.walk();
            else_clause.children(&mut inner).collect()
        };
        let block = else_children.iter().find(|c| c.kind() == "block").copied();
        if let Some(block) = block {
            if let Some(stmt) = first_named_stmt_in_python_block(block) {
                findings.push(build_finding(
                    file,
                    stmt,
                    condition,
                    "constant-true-if-else",
                    1,
                    LanguageCitationStatus::Unconfirmed,
                ));
            }
        }
    }
}

fn first_named_stmt_in_python_block(block: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut cursor = block.walk();
    let found = block
        .children(&mut cursor)
        .find(|c| c.is_named() && c.kind() != "comment");
    found
}

/// F4e-ii carve-outs. True when every non-comment statement in the
/// `if False:` body is one of the recognised idiomatic shapes:
///
/// - `import_statement` / `import_from_statement` /
///   `future_import_statement` — type-checking import guards
///   (pre-`typing.TYPE_CHECKING` fallback). All statements must be
///   imports; a mixed body (e.g. `if False: import x; print(1)`)
///   does NOT match.
/// - A single statement whose inner expression is `yield_expression`
///   — the generator-marker idiom (CodeQL ODASA-6783). Multiple
///   yield statements still match as long as every statement in the
///   body is a yield expression statement.
///
/// Returns false if the body is empty.
fn python_if_false_body_is_carveout(block: tree_sitter::Node) -> bool {
    let stmts: Vec<tree_sitter::Node> = {
        let mut cursor = block.walk();
        block
            .children(&mut cursor)
            .filter(|c| c.is_named() && c.kind() != "comment")
            .collect()
    };
    if stmts.is_empty() {
        return false;
    }
    let all_imports = stmts.iter().all(|s| {
        matches!(
            s.kind(),
            "import_statement" | "import_from_statement" | "future_import_statement"
        )
    });
    if all_imports {
        return true;
    }
    let all_yields = stmts.iter().all(|s| {
        if s.kind() != "expression_statement" {
            return false;
        }
        let mut inner_cursor = s.walk();
        let inner = s.children(&mut inner_cursor).find(|c| c.is_named());
        matches!(inner.map(|n| n.kind()), Some("yield"))
    });
    all_yields
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
