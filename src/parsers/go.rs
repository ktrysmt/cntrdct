//! Go tree-sitter provider + [`crate::ir::IrFile`] converter (R-3).
//!
//! Spec: `docs/spec/ir-v0.md` §F1, §F2, §F3 (the language-agnostic IR
//! contract) and the R-3 Go pilot in `REBUILD.md`. The converter walks
//! the Go tree-sitter AST emitted by `tree_sitter_go::language()` and
//! materialises the IR nodes the cross-cutting detectors consume. Per
//! ir-v0.md §F2, `to_ir` is total over recognised shapes: unknown
//! statement / expression nodes fall back to
//! [`crate::ir::IrStmtKind::Other`] / [`crate::ir::IrExprKind::Other`]
//! with the tree-sitter `Node::kind()` discriminator + a [`NodeRef`] for
//! raw-tree recovery.
//!
//! Scope: the `language()` grammar covers `.go`. Only top-level
//! `function_declaration` and `method_declaration` items become
//! [`IrFn`]s; `func_literal` closures stay nested and remain reachable to
//! the raw-tree detectors (arg-swap Pattern B, pr-miner, clone-drift's
//! function-rooted token walk) but not to an IR-only walk.
//!
//! v0 modelling notes (documented limitations, all safe under the
//! "unknown shape → Other" total-conversion contract):
//!
//! - `expression_switch_statement` / `type_switch_statement` /
//!   `select_statement` / `defer_statement` / `go_statement` /
//!   `labeled_statement` are recorded as [`crate::ir::IrStmtKind::Other`];
//!   calls nested inside them stay reachable to the raw-tree detectors but
//!   not to an IR-only walk. Closing this is future work, not a
//!   correctness issue.
//! - Go's single `for` construct (condition / range / C-style / infinite)
//!   is modelled as [`crate::ir::IrForStmt`] and never contributes a block
//!   terminator. An infinite `for {}` with no `break` is divergent in
//!   principle, but v0 does not classify it as a terminator
//!   (precision-first; mirrors the TypeScript loop treatment).
//! - Go has no `throw` / `raise`; divergence is expressed through
//!   `panic(...)` / `os.Exit(...)` / `log.Fatal*(...)`, classified as
//!   [`crate::ir::DivergentKind`] calls (see [`go_divergent_kind`]).
//! - Unlike Rust/Python there is no v0.5.x byte-identical pinning corpus
//!   for Go, so the normaliser / classifier choices here are anchored only
//!   by the IR golden fixtures (ir-v0.md §F6 T4), not by a prior capture.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{build_ir_shell, Language, ParserProvider};
use crate::ir::{
    BranchMergeKind, DivergentKind, HoistedItemKind, IrBlock, IrCallSite, IrComment, IrCommentKind,
    IrConvertError, IrExpr, IrExprKind, IrFile, IrFn, IrForStmt, IrIfStmt, IrLabel, IrLiteral,
    IrParam, IrPath, IrStmt, IrStmtKind, IrTerminator, Location, NodeRef, NormalisedToken,
    ParamKind,
};

/// Provider for Go source (`*.go`).
pub struct GoParserProvider;

impl ParserProvider for GoParserProvider {
    fn language(&self) -> Language {
        Language::Go
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_go::language()
    }

    fn to_ir(
        &self,
        tree: tree_sitter::Tree,
        source: Arc<str>,
        path: PathBuf,
    ) -> Result<IrFile, IrConvertError> {
        let mut shell = build_ir_shell(self, &tree, source, path)?;
        // One `Arc<Path>` per file, shared by reference into every node's
        // `Location` (R-1.c'' path (a)): clone the Arc (refcount bump)
        // instead of deep-copying the path string per node.
        let path_arc: Arc<Path> = Arc::from(shell.path.as_path());
        let (fns, top_level_comments) = {
            let cv = Converter {
                source: shell.source.as_ref(),
                path: &path_arc,
            };
            cv.convert_root(tree.root_node())?
        };
        shell.fns = fns;
        shell.top_level_comments = top_level_comments;
        // `tree` drops here — IrFile keeps no reference to it. R1
        // mitigation: detectors reparse via IrFile::raw_tree on demand.
        Ok(shell)
    }
}

// ---------- Go divergent-call classification ----------

/// Maps a call-site callee's raw text to a [`DivergentKind`]. v0 models
/// the three Go terminators whose call never returns: `panic(...)` unwinds
/// the goroutine, `os.Exit(...)` terminates the process, and the `log`
/// package's `Fatal` family calls `os.Exit(1)` after writing.
fn go_divergent_kind(callee_text: &str) -> Option<DivergentKind> {
    match callee_text.trim() {
        "panic" => Some(DivergentKind::Panic),
        "os.Exit" => Some(DivergentKind::GoOsExit),
        "log.Fatal" | "log.Fatalln" | "log.Fatalf" => Some(DivergentKind::LogFatal),
        _ => None,
    }
}

// ---------- Converter ----------

struct Converter<'a> {
    source: &'a str,
    path: &'a Arc<Path>,
}

impl<'a> Converter<'a> {
    fn convert_root(
        &self,
        root: tree_sitter::Node<'a>,
    ) -> Result<(Vec<IrFn>, Vec<IrComment>), IrConvertError> {
        let mut fns: Vec<IrFn> = Vec::new();
        let mut top_level_comments: Vec<IrComment> = Vec::new();

        let mut cursor = root.walk();
        let children: Vec<tree_sitter::Node> = root.children(&mut cursor).collect();
        for (idx, node) in children.iter().enumerate() {
            match node.kind() {
                "function_declaration" => {
                    let leading_doc = collect_go_leading_doc(&children, idx, self.source);
                    let f = self.convert_function(*node, false, leading_doc);
                    fns.push(f);
                }
                "method_declaration" => {
                    let leading_doc = collect_go_leading_doc(&children, idx, self.source);
                    let f = self.convert_function(*node, true, leading_doc);
                    fns.push(f);
                }
                "comment" => {
                    if let Some(c) = convert_go_comment(*node, self.source, self.path) {
                        top_level_comments.push(c);
                    }
                }
                _ => {}
            }
        }
        Ok((fns, top_level_comments))
    }

    /// Convert a `function_declaration` / `method_declaration` into an
    /// [`IrFn`]. The Go receiver of a method lives in the separate
    /// `receiver` field, not in `parameters`, so `params` carries only the
    /// real positional arguments and `is_method` records the method-ness;
    /// arg-swap's `ir_fn_to_def` needs no receiver entry to drop.
    fn convert_function(
        &self,
        node: tree_sitter::Node<'a>,
        is_method: bool,
        leading_doc: Option<String>,
    ) -> IrFn {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();

        let params = match node.child_by_field_name("parameters") {
            Some(p) => self.convert_go_params(p),
            None => Vec::new(),
        };

        let return_type_text = node
            .child_by_field_name("result")
            .map(|n| self.text(n).trim().to_string());

        let body = match node.child_by_field_name("body") {
            Some(b) if b.kind() == "block" => self.convert_go_block(b),
            _ => empty_block(self.path, node),
        };

        let mut normalised_tokens = Vec::new();
        walk_normalize_go(node, &mut normalised_tokens);

        IrFn {
            name,
            params,
            body,
            return_type_text,
            // Go has no decorator / attribute syntax that binds to a
            // function definition.
            decorators: Vec::new(),
            is_method,
            leading_doc,
            normalised_tokens,
            location: node_location(self.path, node),
        }
    }

    fn convert_go_params(&self, params_node: tree_sitter::Node<'a>) -> Vec<IrParam> {
        let mut out = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            match child.kind() {
                "parameter_declaration" => {
                    // A single `parameter_declaration` may bind several
                    // names sharing one type (`a, b int`); emit one
                    // IrParam per name. An unnamed declaration
                    // (`func(int)`) carries only `type:` — model it as one
                    // `Unsupported` param since arg-swap cannot correlate
                    // a nameless argument.
                    let mut name_cursor = child.walk();
                    let names: Vec<tree_sitter::Node> = child
                        .children_by_field_name("name", &mut name_cursor)
                        .collect();
                    if names.is_empty() {
                        out.push(IrParam {
                            name: self.text(child).to_string(),
                            kind: ParamKind::Unsupported,
                            // Go has no default-parameter syntax (M6).
                            default: None,
                            location: node_location(self.path, child),
                        });
                    } else {
                        for name in names {
                            out.push(IrParam {
                                name: self.text(name).to_string(),
                                kind: ParamKind::Plain,
                                default: None,
                                location: node_location(self.path, name),
                            });
                        }
                    }
                }
                // `xs ...int`: a variadic parameter cannot be reasoned
                // about positionally by arg-swap.
                "variadic_parameter_declaration" => out.push(IrParam {
                    name: self.text(child).to_string(),
                    kind: ParamKind::Unsupported,
                    default: None,
                    location: node_location(self.path, child),
                }),
                _ => {}
            }
        }
        out
    }

    fn convert_go_block(&self, block: tree_sitter::Node<'a>) -> IrBlock {
        let mut cursor = block.walk();
        let raw_children: Vec<tree_sitter::Node> = block.children(&mut cursor).collect();

        let mut statements: Vec<IrStmt> = Vec::new();
        for child in raw_children.iter() {
            if !child.is_named() {
                continue;
            }
            if child.kind() == "comment" {
                continue;
            }
            let kind = self.classify_go_stmt(*child);
            statements.push(IrStmt {
                kind,
                // Go has no Rust-style per-statement attribute chain.
                attributes: Vec::new(),
                location: node_location(self.path, *child),
            });
        }

        let terminator = compute_block_terminator(&statements);

        let mut block_tokens = Vec::new();
        walk_normalize_go(block, &mut block_tokens);

        IrBlock {
            statements,
            terminator,
            normalised_token_count: block_tokens.len(),
            location: node_location(self.path, block),
        }
    }

    fn classify_go_stmt(&self, node: tree_sitter::Node<'a>) -> IrStmtKind {
        match node.kind() {
            "expression_statement" => {
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                match inner {
                    Some(inner) => self.classify_go_expression_as_stmt(inner),
                    None => self.other_stmt(node),
                }
            }
            "return_statement" => IrStmtKind::Return(self.go_return_value(node)),
            "if_statement" => IrStmtKind::If(self.convert_go_if(node)),
            "for_statement" => IrStmtKind::For(self.convert_go_for(node)),
            "short_var_declaration" => IrStmtKind::Let {
                value: self.expression_list_first(node.child_by_field_name("right")),
            },
            "var_declaration" => IrStmtKind::Let {
                value: self.go_spec_value(node),
            },
            "const_declaration" => IrStmtKind::Let {
                value: self.go_spec_value(node),
            },
            "assignment_statement" => IrStmtKind::Assign {
                value: self.expression_list_first(node.child_by_field_name("right")),
            },
            "break_statement" => IrStmtKind::Break(self.go_label(node)),
            "continue_statement" => IrStmtKind::Continue(self.go_label(node)),
            "type_declaration" => IrStmtKind::HoistedItem {
                kind: HoistedItemKind::Type,
                node_ref: node_ref(node),
            },
            _ => self.other_stmt(node),
        }
    }

    fn classify_go_expression_as_stmt(&self, inner: tree_sitter::Node<'a>) -> IrStmtKind {
        match inner.kind() {
            "call_expression" => match self.convert_go_call_site(inner) {
                Some(call) => match go_divergent_kind(&call.callee.raw) {
                    Some(kind) => IrStmtKind::DivergentCall {
                        kind,
                        args: call.args,
                    },
                    None => IrStmtKind::Call(call),
                },
                None => self.other_stmt(inner),
            },
            _ => self.other_stmt(inner),
        }
    }

    fn convert_go_if(&self, node: tree_sitter::Node<'a>) -> IrIfStmt {
        let condition = node
            .child_by_field_name("condition")
            .map(|c| self.convert_go_expr(c))
            .unwrap_or_else(|| other_expr(self.path, node, "missing_condition"));
        let consequence = node
            .child_by_field_name("consequence")
            .map(|c| self.convert_go_block(c))
            .unwrap_or_else(|| empty_block(self.path, node));
        // The `alternative` field is either a `block` (`else { ... }`) or a
        // chained `if_statement` (`else if`). Surface the chained if as a
        // one-statement block so its terminator propagates upward.
        let alternative = node.child_by_field_name("alternative").map(|a| {
            if a.kind() == "block" {
                self.convert_go_block(a)
            } else {
                self.single_stmt_block(a)
            }
        });
        // Branch-merge: an `if` diverges iff both arms diverge.
        let terminator = match (&consequence.terminator, &alternative) {
            (Some(_), Some(alt)) if alt.terminator.is_some() => Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::IfBranchesDiverge,
            }),
            _ => None,
        };
        IrIfStmt {
            condition,
            consequence,
            alternative,
            terminator,
            location: node_location(self.path, node),
        }
    }

    /// Wrap a single non-block statement (e.g. a chained `else if`) into an
    /// [`IrBlock`] so terminator analysis is uniform.
    fn single_stmt_block(&self, stmt: tree_sitter::Node<'a>) -> IrBlock {
        let kind = self.classify_go_stmt(stmt);
        let statements = vec![IrStmt {
            kind,
            attributes: Vec::new(),
            location: node_location(self.path, stmt),
        }];
        let terminator = compute_block_terminator(&statements);
        let mut block_tokens = Vec::new();
        walk_normalize_go(stmt, &mut block_tokens);
        IrBlock {
            statements,
            terminator,
            normalised_token_count: block_tokens.len(),
            location: node_location(self.path, stmt),
        }
    }

    fn convert_go_for(&self, node: tree_sitter::Node<'a>) -> IrForStmt {
        // The loop clause varies: `range_clause` (range over `right`),
        // `for_clause` (C-style; surface the condition), or a bare
        // expression child (the condition). Surface whichever expression we
        // find so calls there stay reachable to an IR walk; the body is
        // kept for nested-call walking.
        let mut iterable: Option<IrExpr> = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "range_clause" => {
                    iterable = child
                        .child_by_field_name("right")
                        .map(|r| self.convert_go_expr(r));
                }
                "for_clause" => {
                    iterable = child
                        .child_by_field_name("condition")
                        .map(|c| self.convert_go_expr(c));
                }
                // A bare expression clause (`for cond { ... }`). `block` is
                // excluded by `is_go_expr_kind`, so the loop body never
                // matches here.
                _ if iterable.is_none() && is_go_expr_kind(child.kind()) => {
                    iterable = Some(self.convert_go_expr(child));
                }
                _ => {}
            }
        }
        let iterable = iterable.unwrap_or_else(|| other_expr(self.path, node, "for_no_iterable"));
        let body = node
            .child_by_field_name("body")
            .map(|b| self.convert_go_block(b))
            .unwrap_or_else(|| empty_block(self.path, node));
        IrForStmt {
            iterable,
            body,
            location: node_location(self.path, node),
        }
    }

    fn convert_go_expr(&self, node: tree_sitter::Node<'a>) -> IrExpr {
        let location = node_location(self.path, node);
        let kind = match node.kind() {
            "identifier" | "field_identifier" | "type_identifier" | "package_identifier" => {
                IrExprKind::Ident(self.text(node).to_string())
            }
            "selector_expression" => IrExprKind::Path(self.convert_go_selector_path(node)),
            "int_literal" => IrExprKind::Literal(parse_go_number(self.text(node))),
            "float_literal" | "imaginary_literal" => IrExprKind::Literal(IrLiteral::Float),
            "interpreted_string_literal" | "raw_string_literal" => {
                IrExprKind::Literal(IrLiteral::String {
                    is_empty: go_string_is_empty(self.text(node)),
                })
            }
            "rune_literal" => IrExprKind::Literal(IrLiteral::Char),
            "true" => IrExprKind::Literal(IrLiteral::Bool(true)),
            "false" => IrExprKind::Literal(IrLiteral::Bool(false)),
            "nil" => IrExprKind::Literal(IrLiteral::None),
            "call_expression" => match self.convert_go_call_site(node) {
                Some(call) => match go_divergent_kind(&call.callee.raw) {
                    Some(kind) => IrExprKind::DivergentCall {
                        kind,
                        args: call.args,
                    },
                    None => IrExprKind::Call(Box::new(call)),
                },
                None => IrExprKind::Other {
                    node_kind: "call_expression",
                    node_ref: node_ref(node),
                },
            },
            "parenthesized_expression" => {
                // Transparent wrapper: return the inner expression with its
                // own location (ir-v0.md §F2 transparent-wrapper note).
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                match inner {
                    Some(inner) => return self.convert_go_expr(inner),
                    None => IrExprKind::Other {
                        node_kind: static_kind_str(node.kind()),
                        node_ref: node_ref(node),
                    },
                }
            }
            other => IrExprKind::Other {
                node_kind: static_kind_str(other),
                node_ref: node_ref(node),
            },
        };
        IrExpr { kind, location }
    }

    /// Convert a `selector_expression` (`a.b.c`) into an [`IrPath`] with the
    /// receiver chain in `receiver` and the final field as the sole
    /// `segments` entry (matching the Python / TypeScript attribute
    /// convention so "last segment" consumers see the method name).
    fn convert_go_selector_path(&self, node: tree_sitter::Node<'a>) -> IrPath {
        let raw = self.text(node).to_string();
        let mut chain: Vec<String> = Vec::new();
        go_collect_selector_chain(node, self.source, &mut chain);
        let (segments, receiver) = match chain.split_last() {
            Some((last, head)) => (vec![last.clone()], head.to_vec()),
            None => (Vec::new(), Vec::new()),
        };
        IrPath {
            receiver,
            segments,
            raw,
        }
    }

    fn convert_go_call_site(&self, node: tree_sitter::Node<'a>) -> Option<IrCallSite> {
        let function = node.child_by_field_name("function")?;
        let arguments = node.child_by_field_name("arguments")?;
        let callee = match function.kind() {
            "identifier" | "field_identifier" => IrPath {
                receiver: Vec::new(),
                segments: vec![self.text(function).to_string()],
                raw: self.text(function).to_string(),
            },
            "selector_expression" => self.convert_go_selector_path(function),
            _ => IrPath {
                receiver: Vec::new(),
                segments: Vec::new(),
                raw: self.text(function).to_string(),
            },
        };
        let mut args: Vec<IrExpr> = Vec::new();
        let mut cursor = arguments.walk();
        for child in arguments.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            args.push(self.convert_go_expr(child));
        }
        Some(IrCallSite {
            callee,
            args,
            location: node_location(self.path, node),
        })
    }

    /// Extract the value of a `return_statement`: its child
    /// `expression_list`'s first named expression (Go multi-value returns
    /// keep only the first for IR purposes — the value is read only for
    /// divergent-return classification, which a multi-return never is).
    fn go_return_value(&self, node: tree_sitter::Node<'a>) -> Option<IrExpr> {
        let mut cursor = node.walk();
        let list = node
            .children(&mut cursor)
            .find(|c| c.kind() == "expression_list")?;
        self.expression_list_first(Some(list))
    }

    fn expression_list_first(&self, list: Option<tree_sitter::Node<'a>>) -> Option<IrExpr> {
        let list = list?;
        let mut cursor = list.walk();
        let first = list.children(&mut cursor).find(|c| c.is_named())?;
        Some(self.convert_go_expr(first))
    }

    /// Value of a `var_declaration` / `const_declaration`: the first
    /// spec's `value` expression list head.
    fn go_spec_value(&self, decl: tree_sitter::Node<'a>) -> Option<IrExpr> {
        let mut cursor = decl.walk();
        for child in decl.children(&mut cursor) {
            if matches!(child.kind(), "var_spec" | "const_spec") {
                if let Some(v) = child.child_by_field_name("value") {
                    return self.expression_list_first(Some(v));
                }
            }
        }
        None
    }

    fn go_label(&self, node: tree_sitter::Node<'a>) -> Option<IrLabel> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "label_name" {
                return Some(IrLabel::Named(self.text(child).to_string()));
            }
        }
        Some(IrLabel::Unlabelled)
    }

    fn other_stmt(&self, node: tree_sitter::Node<'a>) -> IrStmtKind {
        IrStmtKind::Other {
            node_kind: static_kind_str(node.kind()),
            node_ref: node_ref(node),
        }
    }

    fn text(&self, node: tree_sitter::Node<'a>) -> &'a str {
        &self.source[node.byte_range()]
    }
}

/// True for tree-sitter Go node kinds the converter models as an
/// expression (used to pick the bare-condition child of a `for_statement`).
fn is_go_expr_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "field_identifier"
            | "type_identifier"
            | "package_identifier"
            | "selector_expression"
            | "int_literal"
            | "float_literal"
            | "imaginary_literal"
            | "interpreted_string_literal"
            | "raw_string_literal"
            | "rune_literal"
            | "true"
            | "false"
            | "nil"
            | "call_expression"
            | "parenthesized_expression"
            | "binary_expression"
            | "unary_expression"
            | "index_expression"
    )
}

// ---------- Block terminator merge ----------

fn compute_block_terminator(statements: &[IrStmt]) -> Option<IrTerminator> {
    for stmt in statements {
        if let Some(t) = stmt_terminator(stmt) {
            return Some(t);
        }
    }
    None
}

fn stmt_terminator(stmt: &IrStmt) -> Option<IrTerminator> {
    match &stmt.kind {
        IrStmtKind::Return(_) => Some(IrTerminator::Return),
        IrStmtKind::Break(_) => Some(IrTerminator::Break),
        IrStmtKind::Continue(_) => Some(IrTerminator::Continue),
        IrStmtKind::DivergentCall { kind, .. } => Some(IrTerminator::DivergentCall { kind: *kind }),
        IrStmtKind::If(if_stmt) => if_stmt.terminator,
        _ => None,
    }
}

// ---------- Comment extraction ----------

/// Collect a Go doc comment: the run of `//` line comments immediately
/// (row-adjacent) above the declaration at `idx`. Go's convention is a
/// contiguous block of line comments directly preceding the item; a blank
/// line (row gap) or a non-`//` node breaks the association. Returns the
/// joined text with the `//` prefix stripped, or `None` when absent.
fn collect_go_leading_doc(
    siblings: &[tree_sitter::Node],
    idx: usize,
    source: &str,
) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut next_start_row = siblings[idx].start_position().row;
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let n = siblings[i];
        if n.kind() != "comment" {
            break;
        }
        let raw = &source[n.byte_range()];
        let Some(rest) = raw.strip_prefix("//") else {
            break;
        };
        // Must be immediately above the next collected line (no blank line).
        if n.end_position().row + 1 != next_start_row {
            break;
        }
        let text = rest
            .strip_prefix(' ')
            .unwrap_or(rest)
            .trim_end()
            .to_string();
        lines.push(text);
        next_start_row = n.start_position().row;
    }
    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

fn convert_go_comment(
    node: tree_sitter::Node<'_>,
    source: &str,
    path: &Arc<Path>,
) -> Option<IrComment> {
    let raw = &source[node.byte_range()];
    let (kind, text) = if let Some(rest) = raw.strip_prefix("//") {
        (
            IrCommentKind::GoLine,
            rest.strip_prefix(' ')
                .unwrap_or(rest)
                .trim_end()
                .to_string(),
        )
    } else if let Some(inner) = raw.strip_prefix("/*").and_then(|s| s.strip_suffix("*/")) {
        (IrCommentKind::GoBlock, inner.trim().to_string())
    } else {
        (IrCommentKind::GoBlock, raw.to_string())
    };
    Some(IrComment {
        kind,
        text,
        target: None,
        location: node_location(path, node),
    })
}

// ---------- Path / selector chain ----------

fn go_collect_selector_chain(node: tree_sitter::Node<'_>, source: &str, out: &mut Vec<String>) {
    match node.kind() {
        "selector_expression" => {
            if let Some(operand) = node.child_by_field_name("operand") {
                go_collect_selector_chain(operand, source, out);
            }
            if let Some(field) = node.child_by_field_name("field") {
                out.push(source[field.byte_range()].to_string());
            }
        }
        "identifier" | "field_identifier" | "package_identifier" | "type_identifier" => {
            out.push(source[node.byte_range()].to_string());
        }
        _ => {
            // Index / call receivers etc.: keep the raw text as a single
            // chain segment so callers still see a non-empty receiver.
            out.push(source[node.byte_range()].to_string());
        }
    }
}

// ---------- Normalised tokens ----------

fn walk_normalize_go(node: tree_sitter::Node<'_>, out: &mut Vec<NormalisedToken>) {
    if !node.is_named() {
        return;
    }
    let kind = node.kind();
    if kind == "comment" {
        return;
    }
    let leaf_token = match kind {
        "identifier" | "field_identifier" | "type_identifier" | "package_identifier"
        | "label_name" => Some(NormalisedToken::Ident),
        "int_literal" => Some(NormalisedToken::LitInt),
        "float_literal" | "imaginary_literal" => Some(NormalisedToken::LitFloat),
        "interpreted_string_literal" | "raw_string_literal" => Some(NormalisedToken::LitStr),
        "rune_literal" => Some(NormalisedToken::LitChar),
        "true" | "false" => Some(NormalisedToken::LitBool),
        _ => None,
    };
    if let Some(tok) = leaf_token {
        out.push(tok);
        return;
    }
    out.push(NormalisedToken::Kind(kind));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_normalize_go(child, out);
    }
}

// ---------- Literal parsing ----------

fn parse_go_number(text: &str) -> IrLiteral {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    // Non-decimal radix (hex `0x`, octal `0o`/`0`, binary `0b`): ir-v0.md
    // F1 keeps these as `Int(None)`.
    if lower.starts_with("0x") || lower.starts_with("0o") || lower.starts_with("0b") {
        return IrLiteral::Int(None);
    }
    let no_sep: String = trimmed.chars().filter(|c| *c != '_').collect();
    if no_sep.len() > 1 && no_sep.starts_with('0') && no_sep.chars().all(|c| c.is_ascii_digit()) {
        // Leading-zero octal (`0755`).
        return IrLiteral::Int(None);
    }
    IrLiteral::Int(no_sep.parse::<i128>().ok())
}

fn go_string_is_empty(text: &str) -> bool {
    let trimmed = text.trim();
    // `""` (interpreted) or ` `` ` (raw) carry no content between their
    // two-character delimiters.
    trimmed.len() <= 2
}

// ---------- Generic helpers ----------

fn node_location(path: &Arc<Path>, node: tree_sitter::Node<'_>) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        file: Arc::clone(path),
        start_line: start.row as u32 + 1,
        start_col: start.column as u32 + 1,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32 + 1,
        start_byte: node.start_byte() as u32,
        end_byte: node.end_byte() as u32,
    }
}

fn node_ref(node: tree_sitter::Node<'_>) -> NodeRef {
    NodeRef {
        range: node.range(),
    }
}

fn empty_block(path: &Arc<Path>, parent: tree_sitter::Node<'_>) -> IrBlock {
    IrBlock {
        statements: Vec::new(),
        terminator: None,
        normalised_token_count: 0,
        location: node_location(path, parent),
    }
}

fn other_expr(path: &Arc<Path>, node: tree_sitter::Node<'_>, node_kind: &'static str) -> IrExpr {
    IrExpr {
        kind: IrExprKind::Other {
            node_kind,
            node_ref: node_ref(node),
        },
        location: node_location(path, node),
    }
}

fn static_kind_str(s: &'static str) -> &'static str {
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::IrCommentKind;

    fn to_ir(source: &str) -> IrFile {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::language())
            .expect("set go language");
        let tree = parser.parse(source, None).expect("parse go");
        GoParserProvider
            .to_ir(tree, Arc::from(source), PathBuf::from("a.go"))
            .expect("to_ir succeeds")
    }

    #[test]
    fn converts_top_level_function() {
        let ir = to_ir("package main\nfunc add(a, b int) int { return a + b }\n");
        assert_eq!(ir.fns.len(), 1);
        let f = &ir.fns[0];
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].kind, ParamKind::Plain);
        assert_eq!(f.params[1].name, "b");
        assert!(!f.is_method);
        assert_eq!(f.return_type_text.as_deref(), Some("int"));
    }

    #[test]
    fn marks_methods_as_methods_receiver_not_in_params() {
        let ir = to_ir("package main\nfunc (r *T) M(x int) error { return nil }\n");
        assert_eq!(ir.fns.len(), 1);
        let f = &ir.fns[0];
        assert_eq!(f.name, "M");
        assert!(f.is_method);
        // The receiver lives in the separate `receiver` field; only the
        // real argument `x` appears in params.
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "x");
    }

    #[test]
    fn variadic_param_is_unsupported() {
        let ir = to_ir("package main\nfunc f(xs ...int) {}\n");
        let f = &ir.fns[0];
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].kind, ParamKind::Unsupported);
    }

    #[test]
    fn doc_line_comments_become_leading_doc() {
        let ir = to_ir(
            "package main\n// add returns the sum.\n// Second line.\nfunc add(a, b int) int { return a + b }\n",
        );
        assert_eq!(
            ir.fns[0].leading_doc.as_deref(),
            Some("add returns the sum.\nSecond line.")
        );
    }

    #[test]
    fn blank_line_breaks_doc_association() {
        let ir = to_ir("package main\n// not a doc\n\nfunc add(a, b int) int { return a + b }\n");
        assert_eq!(ir.fns[0].leading_doc, None);
    }

    #[test]
    fn call_site_records_callee_and_args() {
        let ir = to_ir("package main\nfunc m() {\n  foo(a, b)\n  obj.Method(c, d)\n}\n");
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::Call(call) => {
                assert_eq!(call.callee.segments, vec!["foo".to_string()]);
                assert_eq!(call.args.len(), 2);
            }
            other => panic!("expected Call, got {other:?}"),
        }
        match &stmts[1].kind {
            IrStmtKind::Call(call) => {
                assert_eq!(call.callee.receiver, vec!["obj".to_string()]);
                assert_eq!(call.callee.segments, vec!["Method".to_string()]);
            }
            other => panic!("expected member Call, got {other:?}"),
        }
    }

    #[test]
    fn panic_is_divergent() {
        let ir = to_ir("package main\nfunc t() { panic(\"e\"); foo() }\n");
        assert!(matches!(
            ir.fns[0].body.terminator,
            Some(IrTerminator::DivergentCall {
                kind: DivergentKind::Panic
            })
        ));
    }

    #[test]
    fn os_exit_and_log_fatal_are_divergent() {
        let ir = to_ir("package main\nfunc a() { os.Exit(1) }\nfunc b() { log.Fatal(\"x\") }\n");
        assert!(matches!(
            ir.fns[0].body.terminator,
            Some(IrTerminator::DivergentCall {
                kind: DivergentKind::GoOsExit
            })
        ));
        assert!(matches!(
            ir.fns[1].body.terminator,
            Some(IrTerminator::DivergentCall {
                kind: DivergentKind::LogFatal
            })
        ));
    }

    #[test]
    fn return_is_a_terminator() {
        let ir = to_ir("package main\nfunc t() int { return 1 }\n");
        assert_eq!(ir.fns[0].body.terminator, Some(IrTerminator::Return));
    }

    #[test]
    fn if_both_branches_diverge_merges() {
        let ir = to_ir(
            "package main\nfunc c(x int) int {\n  if x > 0 { return 1 } else { panic(\"e\") }\n  other()\n  return 0\n}\n",
        );
        assert!(matches!(
            ir.fns[0].body.terminator,
            Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::IfBranchesDiverge
            })
        ));
    }

    #[test]
    fn for_loop_is_not_a_terminator() {
        let ir = to_ir("package main\nfunc l() {\n  for i := 0; i < 10; i++ { work() }\n}\n");
        match &ir.fns[0].body.statements[0].kind {
            IrStmtKind::For(_) => {}
            other => panic!("expected For, got {other:?}"),
        }
        assert_eq!(ir.fns[0].body.terminator, None);
    }

    #[test]
    fn short_var_declaration_is_let() {
        let ir = to_ir("package main\nfunc v() {\n  x := compute()\n  _ = x\n}\n");
        assert!(matches!(
            ir.fns[0].body.statements[0].kind,
            IrStmtKind::Let { value: Some(_) }
        ));
    }

    #[test]
    fn comments_classify_by_delimiter() {
        let ir = to_ir("package main\n// line\n/* block */\nfunc f() {}\n");
        let kinds: Vec<IrCommentKind> = ir.top_level_comments.iter().map(|c| c.kind).collect();
        assert!(kinds.contains(&IrCommentKind::GoLine));
        assert!(kinds.contains(&IrCommentKind::GoBlock));
    }

    #[test]
    fn parse_recovered_set_on_syntax_error() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_go::language()).unwrap();
        let broken = "package main\nfunc f( {\n";
        let tree = parser.parse(broken, None).unwrap();
        let ir = GoParserProvider
            .to_ir(tree, Arc::from(broken), PathBuf::from("a.go"))
            .expect("to_ir succeeds on partial parse");
        assert!(ir.parse_recovered);
    }

    #[test]
    fn location_invariant_for_call_site() {
        let source = "package main\nfunc f() {\n  bar(x, y)\n}\n";
        let ir = to_ir(source);
        let call = match &ir.fns[0].body.statements[0].kind {
            IrStmtKind::Call(c) => c,
            _ => panic!(),
        };
        assert_eq!(call.location.start_line, 3);
        let raw =
            &source.as_bytes()[call.location.start_byte as usize..call.location.end_byte as usize];
        assert_eq!(std::str::from_utf8(raw).unwrap(), "bar(x, y)");
    }
}
