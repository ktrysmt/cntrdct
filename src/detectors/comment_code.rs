//! comment-code detector — pattern-based comment/implementation mismatch.
//!
//! Spec: `cntrdct/docs/spec/comment-code-v0.md`.
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A).
//! IR migration (ir-v0.md §F4): consumes `IrFn.{leading_doc,
//! return_type_text, decorators, body, location}`, `IrBlock.statements`,
//! `IrStmtKind::{Raise, Return, Call, If, While, With}`,
//! `IrDecorator.{name_path}`, `IrFn.body.location.{start_byte, end_byte}`
//! sliced against `IrFile.source` for Pattern B body-marker substring.
//!
//! Algorithm (Rust):
//! 1. Walk `IrFile.fns` filtered to top-level (`is_method == false`,
//!    matching the v0.5.x "root children only" walk that skipped
//!    impl/trait methods).
//! 2. For each function with a non-empty `IrFn.leading_doc`, apply
//!    Pattern A / B / C against the rendered doc text,
//!    `IrFn.return_type_text`, the body source slice, and
//!    `IrFn.decorators`.
//! 3. Emit one Finding per match.
//!
//! Algorithm (Python):
//! 1. Walk `IrFile.fns` filtered to top-level. Class methods come in
//!    with `is_method == true` and are skipped.
//! 2. The converter populates `IrFn.leading_doc` with the docstring
//!    text when the body's first statement is a bare string literal
//!    (strip-quotes logic mirrors v0.5.x).
//! 3. Apply py-raises (doc claims raise but no `IrStmtKind::Raise` in
//!    the function body and no factory-shape value return) and
//!    py-deprecated (doc says deprecated but no `@deprecated`-style
//!    decorator) per `IrFn.body` walk and `IrFn.decorators`. Python
//!    findings carry `LanguageCitationStatus::Unconfirmed` per
//!    `docs/surveys/comment-code-python-2026-05.md`.

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::{IrBlock, IrDecorator, IrExpr, IrExprKind, IrFile, IrFn, IrStmt, IrStmtKind};
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
        &[
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::Tsx,
            Language::Go,
        ]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = ctx
            .files
            .par_iter()
            .filter(|f| {
                matches!(
                    f.language,
                    Language::Rust
                        | Language::Python
                        | Language::TypeScript
                        | Language::Tsx
                        | Language::Go
                )
            })
            .flat_map_iter(|file| {
                let mut local = Vec::new();
                if file.parse_recovered {
                    return local;
                }
                match file.language {
                    Language::Rust => collect_rust_findings(file, &mut local),
                    Language::Python => collect_python_findings(file, &mut local),
                    Language::TypeScript | Language::Tsx => {
                        collect_typescript_findings(file, &mut local)
                    }
                    Language::Go => collect_go_findings(file, &mut local),
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

// ---------- Rust scan ----------

fn collect_rust_findings(file: &IrFile, out: &mut Vec<Finding>) {
    for ir_fn in &file.fns {
        // v0.5.x walked only root children — impl methods (is_method ==
        // true) were never inspected. Preserve that semantic by skipping
        // methods here. Nested functions never appear in IrFile.fns.
        if ir_fn.is_method {
            continue;
        }
        let Some(doc) = ir_fn.leading_doc.as_deref() else {
            continue;
        };
        if doc.is_empty() {
            continue;
        }
        let doc_lc = doc.to_lowercase();

        if let Some(trigger) = pattern_a_match(ir_fn, &doc_lc) {
            out.push(make_finding(file, ir_fn, "A", trigger));
        }
        if let Some(trigger) = pattern_b_match(file, ir_fn, &doc_lc) {
            out.push(make_finding(file, ir_fn, "B", trigger));
        }
        if let Some(trigger) = pattern_c_match(ir_fn, &doc_lc) {
            out.push(make_finding(file, ir_fn, "C", trigger));
        }
    }
}

fn pattern_a_match(ir_fn: &IrFn, doc_lc: &str) -> Option<&'static str> {
    let trigger = PATTERN_A_TRIGGERS
        .iter()
        .find(|p| doc_lc.contains(*p))
        .copied()?;
    let return_type_text = ir_fn.return_type_text.as_deref().unwrap_or("");
    if return_type_text.contains("Result") || return_type_text.contains("Option") {
        return None;
    }
    Some(trigger)
}

fn pattern_b_match(file: &IrFile, ir_fn: &IrFn, doc_lc: &str) -> Option<&'static str> {
    if !doc_lc.contains("panic") {
        return None;
    }
    let body_loc = &ir_fn.body.location;
    let start = body_loc.start_byte as usize;
    let end = body_loc.end_byte as usize;
    let source: &str = &file.source;
    if start > end || end > source.len() {
        return None;
    }
    let body_text = &source[start..end];
    for marker in PATTERN_B_BODY_MARKERS {
        if body_text.contains(marker) {
            return None;
        }
    }
    Some("panic")
}

fn pattern_c_match(ir_fn: &IrFn, doc_lc: &str) -> Option<&'static str> {
    if !doc_lc.contains("deprecated") {
        return None;
    }
    if has_rust_deprecated_attribute(&ir_fn.decorators) {
        return None;
    }
    Some("deprecated")
}

fn has_rust_deprecated_attribute(decorators: &[IrDecorator]) -> bool {
    // v0.5.x `attribute_item_is_deprecated`: the first identifier of
    // the attribute path must be `deprecated`. `#[deprecated]`,
    // `#[deprecated(note = "...")]`, `#[deprecated = "..."]` all match;
    // `#[inline]`, `#[foo::deprecated]`, etc. do not.
    decorators
        .iter()
        .any(|d| d.name_path.first().map(|s| s.as_str()) == Some("deprecated"))
}

fn make_finding(
    file: &IrFile,
    ir_fn: &IrFn,
    pattern: &'static str,
    trigger: &'static str,
) -> Finding {
    make_finding_with_status(
        file,
        ir_fn,
        pattern,
        trigger,
        LanguageCitationStatus::Confirmed,
    )
}

fn make_finding_with_status(
    file: &IrFile,
    ir_fn: &IrFn,
    pattern: &'static str,
    trigger: &'static str,
    status: LanguageCitationStatus,
) -> Finding {
    Finding {
        detector_id: "comment-code".to_string(),
        primary: ir_location_to_finding(file, &ir_fn.location),
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
        origin: Default::default(),
    }
}

fn ir_location_to_finding(file: &IrFile, loc: &crate::ir::Location) -> Location {
    Location {
        file: file.path.clone(),
        start_line: loc.start_line,
        start_col: loc.start_col,
        end_line: loc.end_line,
        end_col: loc.end_col,
    }
}

// ---------- Python scan (M-3 Pattern A) ----------
//
// The walk + docstring extraction + finding emission share the same
// shape as the Rust path; what differs is the doc-text source
// (`IrFn.leading_doc` is populated by the Python converter from the
// body's first statement when it's a bare string literal), the
// patterns (no Rust-style static return-type Pattern A), and the
// decorator vocabulary. py-raises substitutes Pattern B in spirit;
// py-deprecated mirrors Pattern C.

const PYTHON_RAISES_TRIGGERS: &[&str] = &["raises", "may raise", "throws"];

fn collect_python_findings(file: &IrFile, out: &mut Vec<Finding>) {
    for ir_fn in &file.fns {
        // v0 only inspects module-top-level def. Class methods come in
        // with is_method == true and are skipped here.
        if ir_fn.is_method {
            continue;
        }
        let Some(doc) = ir_fn.leading_doc.as_deref() else {
            continue;
        };
        let doc_lc = doc.to_lowercase();

        if let Some(trigger) = python_pattern_raises(&doc_lc, &ir_fn.body) {
            out.push(make_finding_with_status(
                file,
                ir_fn,
                "py-raises",
                trigger,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
        if let Some(trigger) = python_pattern_deprecated(&doc_lc, &ir_fn.decorators) {
            out.push(make_finding_with_status(
                file,
                ir_fn,
                "py-deprecated",
                trigger,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    }
}

// ---------- TypeScript (R-2.d) ----------

/// JSDoc / prose markers that claim the function throws. The TypeScript
/// convention is the JSDoc `@throws` (alias `@exception`) tag; the prose
/// forms catch hand-written docs that describe throwing without the tag.
const TYPESCRIPT_THROWS_TRIGGERS: &[&str] = &[
    "@throws",
    "@throw",
    "@exception",
    "throws ",
    "throws an",
    "may throw",
    "will throw",
];

/// TypeScript analogue of `collect_python_findings`. v0 ships the
/// `ts-throws` pattern (doc claims the function throws but the body has
/// no `throw`), mirroring py-raises. Findings carry
/// `LanguageCitationStatus::Unconfirmed` per the R-2.f survey.
fn collect_typescript_findings(file: &IrFile, out: &mut Vec<Finding>) {
    for ir_fn in &file.fns {
        // Mirror the Rust/Python "top-level only" walk; class methods
        // arrive with is_method == true and are skipped in v0.
        if ir_fn.is_method {
            continue;
        }
        let Some(doc) = ir_fn.leading_doc.as_deref() else {
            continue;
        };
        let doc_lc = doc.to_lowercase();
        if let Some(trigger) = typescript_pattern_throws(&doc_lc, &ir_fn.body) {
            out.push(make_finding_with_status(
                file,
                ir_fn,
                "ts-throws",
                trigger,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    }
}

fn typescript_pattern_throws(doc_lc: &str, body: &IrBlock) -> Option<&'static str> {
    let trigger = TYPESCRIPT_THROWS_TRIGGERS
        .iter()
        .find(|p| doc_lc.contains(*p))
        .copied()?;
    // The IR maps `throw` onto `IrStmtKind::Raise`, so the
    // language-agnostic `body_contains_raise` walk applies directly.
    if body_contains_raise(body) {
        return None;
    }
    // Factory-shape suppression, same as py-raises: a function that
    // returns the result of a call (e.g. `return makeError(x)`) is
    // delegating, not making a direct no-throw claim.
    if body_returns_call_expression(body) {
        return None;
    }
    Some(trigger)
}

// ---------- Go (R-3.d) ----------

/// Doc / prose markers that claim the function panics. Go has no `@throws`
/// tag; the convention is prose in the doc comment ("panics if …").
const GO_PANICS_TRIGGERS: &[&str] = &[
    "panics",
    "will panic",
    "may panic",
    "panic if",
    "panic when",
];

/// Go analogue of `collect_typescript_findings`. v0 ships the `go-panics`
/// pattern (doc claims the function panics but the body has no `panic` /
/// `log.Fatal` / `os.Exit` divergent call), the Go counterpart of
/// `ts-throws` / `py-raises`. Findings carry
/// `LanguageCitationStatus::Unconfirmed` per the R-3.f survey.
fn collect_go_findings(file: &IrFile, out: &mut Vec<Finding>) {
    for ir_fn in &file.fns {
        // Top-level only, mirroring the other languages; methods arrive
        // with is_method == true and are skipped in v0.
        if ir_fn.is_method {
            continue;
        }
        let Some(doc) = ir_fn.leading_doc.as_deref() else {
            continue;
        };
        let doc_lc = doc.to_lowercase();
        if let Some(trigger) = go_pattern_panics(&doc_lc, &ir_fn.body) {
            out.push(make_finding_with_status(
                file,
                ir_fn,
                "go-panics",
                trigger,
                LanguageCitationStatus::Unconfirmed,
            ));
        }
    }
}

fn go_pattern_panics(doc_lc: &str, body: &IrBlock) -> Option<&'static str> {
    let trigger = GO_PANICS_TRIGGERS
        .iter()
        .find(|p| doc_lc.contains(*p))
        .copied()?;
    // Go expresses divergence through `panic(...)` / `os.Exit(...)` /
    // `log.Fatal*(...)`, modelled as `IrStmtKind::DivergentCall`; a body
    // that has one is not contradicting its doc.
    if body_contains_divergent_call(body) {
        return None;
    }
    // Factory-shape suppression, same as ts-throws / py-raises: a function
    // that returns the result of a call is delegating, not making a direct
    // no-panic claim.
    if body_returns_call_expression(body) {
        return None;
    }
    Some(trigger)
}

/// Recursively scan `block` for any `IrStmtKind::DivergentCall` (Go
/// `panic` / `os.Exit` / `log.Fatal*`). Walks into the Go IR's
/// compound-statement shapes (`If`, `For`). Calls inside nested closures
/// are `IrStmtKind::Other` and are not recursed into — a panic inside an
/// inner closure belongs to that scope.
fn body_contains_divergent_call(block: &IrBlock) -> bool {
    block.statements.iter().any(stmt_contains_divergent_call)
}

fn stmt_contains_divergent_call(stmt: &IrStmt) -> bool {
    match &stmt.kind {
        IrStmtKind::DivergentCall { .. } => true,
        IrStmtKind::If(if_stmt) => {
            body_contains_divergent_call(&if_stmt.consequence)
                || if_stmt
                    .alternative
                    .as_ref()
                    .map(body_contains_divergent_call)
                    .unwrap_or(false)
        }
        IrStmtKind::For(for_stmt) => body_contains_divergent_call(&for_stmt.body),
        _ => false,
    }
}

fn python_pattern_raises(doc_lc: &str, body: &IrBlock) -> Option<&'static str> {
    let trigger = PYTHON_RAISES_TRIGGERS
        .iter()
        .find(|p| doc_lc.contains(*p))
        .copied()?;
    if body_contains_raise(body) {
        return None;
    }
    // F5b: factory-shape suppression. The body is treated as a
    // factory when both:
    //   (i) the body has no `raise` (already checked above), and
    //   (ii) at least one `return_statement` returns a `call`
    //        expression (e.g. `return _Validator(x)`).
    // Returning a non-call value (slice / subscript / literal /
    // identifier) does NOT qualify — `return buf[:4]` style functions
    // are direct claims, not factories. A bare `return` or
    // `return None` does not qualify either.
    if body_returns_call_expression(body) {
        return None;
    }
    Some(trigger)
}

/// Recursively scan `block` for any `IrStmtKind::Raise`. Walks into
/// the IR's known compound-statement shapes (`If`, `While`, `With`,
/// `Match`, `Loop`). Bodies of nested function / class / lambda
/// definitions are represented as `IrStmtKind::Other` and are NOT
/// recursed into — a raise inside an inner def belongs to that scope.
fn body_contains_raise(block: &IrBlock) -> bool {
    block.statements.iter().any(stmt_contains_raise)
}

fn stmt_contains_raise(stmt: &IrStmt) -> bool {
    match &stmt.kind {
        IrStmtKind::Raise(_) => true,
        IrStmtKind::If(if_stmt) => {
            body_contains_raise(&if_stmt.consequence)
                || if_stmt
                    .alternative
                    .as_ref()
                    .map(body_contains_raise)
                    .unwrap_or(false)
        }
        IrStmtKind::While(while_stmt) => body_contains_raise(&while_stmt.body),
        IrStmtKind::With(with_stmt) => body_contains_raise(&with_stmt.body),
        IrStmtKind::Match(match_stmt) => match_stmt
            .arms
            .iter()
            .any(|arm| expr_contains_raise(&arm.body)),
        IrStmtKind::Loop(loop_stmt) => body_contains_raise(&loop_stmt.body),
        _ => false,
    }
}

fn expr_contains_raise(expr: &IrExpr) -> bool {
    match &expr.kind {
        IrExprKind::Raise(_) => true,
        IrExprKind::Block(b) => body_contains_raise(b),
        IrExprKind::If(if_stmt) => {
            body_contains_raise(&if_stmt.consequence)
                || if_stmt
                    .alternative
                    .as_ref()
                    .map(body_contains_raise)
                    .unwrap_or(false)
        }
        IrExprKind::Match(m) => m.arms.iter().any(|arm| expr_contains_raise(&arm.body)),
        IrExprKind::Loop(l) => body_contains_raise(&l.body),
        _ => false,
    }
}

/// True when `block` contains at least one `return_statement` whose
/// immediate returned expression is a `call` node (i.e. the function
/// returns the result of a function / constructor call). Walks into
/// the IR's known compound-statement shapes (`If`, `While`, `With`,
/// `Match`, `Loop`). Returns inside nested function / class / lambda
/// definitions are represented as `IrStmtKind::Other` and are NOT
/// recursed into — a return inside an inner def belongs to that scope.
fn body_returns_call_expression(block: &IrBlock) -> bool {
    block.statements.iter().any(stmt_returns_call_expression)
}

fn stmt_returns_call_expression(stmt: &IrStmt) -> bool {
    match &stmt.kind {
        IrStmtKind::Return(value) => matches!(
            value,
            Some(IrExpr {
                kind: IrExprKind::Call(_),
                ..
            })
        ),
        IrStmtKind::If(if_stmt) => {
            body_returns_call_expression(&if_stmt.consequence)
                || if_stmt
                    .alternative
                    .as_ref()
                    .map(body_returns_call_expression)
                    .unwrap_or(false)
        }
        IrStmtKind::While(while_stmt) => body_returns_call_expression(&while_stmt.body),
        IrStmtKind::With(with_stmt) => body_returns_call_expression(&with_stmt.body),
        IrStmtKind::Match(match_stmt) => match_stmt
            .arms
            .iter()
            .any(|arm| expr_returns_call_expression(&arm.body)),
        IrStmtKind::Loop(loop_stmt) => body_returns_call_expression(&loop_stmt.body),
        _ => false,
    }
}

fn expr_returns_call_expression(expr: &IrExpr) -> bool {
    match &expr.kind {
        IrExprKind::Return(value) => matches!(
            value.as_deref(),
            Some(IrExpr {
                kind: IrExprKind::Call(_),
                ..
            })
        ),
        IrExprKind::Block(b) => body_returns_call_expression(b),
        IrExprKind::If(if_stmt) => {
            body_returns_call_expression(&if_stmt.consequence)
                || if_stmt
                    .alternative
                    .as_ref()
                    .map(body_returns_call_expression)
                    .unwrap_or(false)
        }
        IrExprKind::Match(m) => m
            .arms
            .iter()
            .any(|arm| expr_returns_call_expression(&arm.body)),
        IrExprKind::Loop(l) => body_returns_call_expression(&l.body),
        _ => false,
    }
}

fn python_pattern_deprecated(doc_lc: &str, decorators: &[IrDecorator]) -> Option<&'static str> {
    if !doc_lc.contains("deprecated") {
        return None;
    }
    if decorators.iter().any(decorator_is_deprecated) {
        return None;
    }
    // F5c: distinguish `.. deprecated::` directive shapes. A directive
    // whose body opens with reST emphasis (`*X*`) or literal markup
    // (`` `X` `` / ` ``X`` `) is parameter / item-level; otherwise
    // function-level. The body may be on the same line as the
    // directive header or on a subsequent indented continuation line
    // when the header has no inline body. If every directive in the
    // doc is parameter-level, suppress; if any function-level
    // directive coexists, fire.
    let mut found_param_level = false;
    let lines: Vec<&str> = doc_lc.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        match classify_deprecated_directive(line, &lines, idx) {
            DeprecatedDirectiveClass::FunctionLevel => return Some("deprecated"),
            DeprecatedDirectiveClass::ParameterLevel => found_param_level = true,
            DeprecatedDirectiveClass::None => {}
        }
    }
    if found_param_level {
        return None;
    }
    Some("deprecated")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeprecatedDirectiveClass {
    FunctionLevel,
    ParameterLevel,
    None,
}

/// Classify a `.. deprecated::` directive starting at `lines[idx]`.
///
/// The directive body is taken from the inline portion (text after
/// `.. deprecated:: VERSION` on the same line) when present;
/// otherwise the first non-blank continuation line is used. A reST
/// directive's continuation lines are indented relative to the
/// directive header — we accept any deeper indentation as
/// continuation, mirroring how Sphinx itself parses these blocks.
///
/// Function-level: body is empty or starts with neither `*` nor `` ` ``.
/// Parameter / item-level: body starts with `*` (reST emphasis) or
/// `` ` `` (reST literal — single or double backtick).
/// None: the header line is not a `.. deprecated::` directive.
fn classify_deprecated_directive(
    line: &str,
    lines: &[&str],
    idx: usize,
) -> DeprecatedDirectiveClass {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix("..") else {
        return DeprecatedDirectiveClass::None;
    };
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix("deprecated::") else {
        return DeprecatedDirectiveClass::None;
    };
    let rest = rest.trim_start();
    let mut splitter = rest.splitn(2, char::is_whitespace);
    let _version = splitter.next();
    let inline_body = splitter
        .next()
        .map(|s| s.trim_start())
        .unwrap_or("")
        .trim_end();

    let header_indent = line.len() - line.trim_start().len();
    let body = if inline_body.is_empty() {
        // Look ahead for the first non-blank line whose indentation
        // is strictly greater than the header's: that's the
        // directive's continuation body. Stop at any line at the
        // header's indentation or shallower (the directive ended).
        let mut found: Option<&str> = None;
        for next in lines.iter().skip(idx + 1) {
            if next.trim().is_empty() {
                continue;
            }
            let next_indent = next.len() - next.trim_start().len();
            if next_indent <= header_indent {
                break;
            }
            found = Some(next.trim_start());
            break;
        }
        found.unwrap_or("").trim_end()
    } else {
        inline_body
    };

    if body.is_empty() {
        return DeprecatedDirectiveClass::FunctionLevel;
    }
    if body.starts_with('*') || body.starts_with('`') {
        DeprecatedDirectiveClass::ParameterLevel
    } else {
        DeprecatedDirectiveClass::FunctionLevel
    }
}

/// True when `decorator.name_path` ends in `deprecated`. Matches the
/// bare identifier `deprecated`, `warnings.deprecated`,
/// `typing_extensions.deprecated`, and any other dotted path whose
/// final segment is `deprecated` (e.g. `mypkg.compat.deprecated`).
/// Decorator factories like `@deprecated("reason")` are accepted
/// because `name_path` is parsed before any `(`.
fn decorator_is_deprecated(decorator: &IrDecorator) -> bool {
    decorator.name_path.last().map(|s| s.as_str()) == Some("deprecated")
}
