//! TypeScript tree-sitter provider + [`crate::ir::IrFile`] converter.
//!
//! Spec: `docs/spec/ir-v0.md` §F1, §F2, §F3 (the language-agnostic IR
//! contract) and the R-2 TypeScript pilot in `REBUILD.md`. The converter
//! walks the TypeScript tree-sitter AST emitted by
//! `tree_sitter_typescript::language_typescript()` and materialises the
//! IR nodes the cross-cutting detectors consume. Per ir-v0.md §F2,
//! `to_ir` is total over recognised shapes: unknown statement /
//! expression nodes fall back to [`crate::ir::IrStmtKind::Other`] /
//! [`crate::ir::IrExprKind::Other`] with the tree-sitter `Node::kind()`
//! discriminator + a [`NodeRef`] for raw-tree recovery.
//!
//! Scope: the `language_typescript()` grammar covers `.ts` / `.mts` /
//! `.cts`. JSX-bearing `.tsx` is out of scope for v0 (see
//! [`Language::TypeScript`] docs for the grammar-ambiguity rationale).
//!
//! v0 modelling notes (documented limitations, all safe under the
//! "unknown shape → Other" total-conversion contract):
//!
//! - `switch_statement` and `do_statement`-style constructs that do not
//!   map cleanly onto an IR control-flow node are recorded as
//!   [`crate::ir::IrStmtKind::Other`]; calls nested inside them stay
//!   reachable to the raw-tree detectors (arg-swap Pattern B, pr-miner,
//!   clone-drift's function-rooted token walk) but not to an IR-only
//!   walk. Closing this is future work, not a correctness issue.
//! - Unlike Rust/Python there is no v0.5.x byte-identical pinning corpus
//!   for TypeScript, so the normaliser / classifier choices here are
//!   anchored only by the IR golden fixtures (ir-v0.md §F6 T4), not by a
//!   prior capture.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{build_ir_shell, Language, ParserProvider};
use crate::ir::{
    BranchMergeKind, DivergentKind, HoistedItemKind, IrBlock, IrCallSite, IrComment, IrCommentKind,
    IrConvertError, IrDecorator, IrExpr, IrExprKind, IrFile, IrFn, IrForStmt, IrIfStmt, IrLabel,
    IrLiteral, IrParam, IrPath, IrStmt, IrStmtKind, IrTerminator, IrWhileStmt, Location, NodeRef,
    NormalisedToken, ParamKind,
};

/// Provider for TypeScript source (`*.ts`, `*.mts`, `*.cts`).
pub struct TypeScriptParserProvider;

impl ParserProvider for TypeScriptParserProvider {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::language_typescript()
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

// ---------- TypeScript divergent-call classification ----------

/// Maps a call-site callee's raw text to a [`DivergentKind`]. v0 models
/// the one canonical Node terminator, `process.exit(...)`; the Node
/// runtime kills the process so control never returns past the call,
/// mirroring Python's `sys.exit`.
fn typescript_divergent_kind(callee_text: &str) -> Option<DivergentKind> {
    match callee_text.trim() {
        "process.exit" => Some(DivergentKind::ProcessExit),
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
            // `export [default] <decl>` is a transparent wrapper around
            // the real declaration; unwrap it and treat the inner node as
            // if it sat at top level (carrying the export_statement's own
            // preceding comments as leading doc).
            let (target, doc_idx) = match node.kind() {
                "export_statement" => match node.child_by_field_name("declaration") {
                    Some(decl) => (decl, idx),
                    None => continue,
                },
                _ => (*node, idx),
            };
            self.collect_top_level_item(
                target,
                &children,
                doc_idx,
                &mut fns,
                &mut top_level_comments,
            )?;
        }
        Ok((fns, top_level_comments))
    }

    fn collect_top_level_item(
        &self,
        node: tree_sitter::Node<'a>,
        siblings: &[tree_sitter::Node<'a>],
        idx: usize,
        fns: &mut Vec<IrFn>,
        top_level_comments: &mut Vec<IrComment>,
    ) -> Result<(), IrConvertError> {
        match node.kind() {
            "function_declaration" | "generator_function_declaration" | "function_signature" => {
                let leading_doc = collect_ts_leading_doc(siblings, idx, self.source);
                let f = self.convert_named_function(node, false, leading_doc, Vec::new())?;
                fns.push(f);
            }
            "class_declaration" | "abstract_class_declaration" => {
                self.collect_class_methods(node, fns)?;
            }
            "lexical_declaration" | "variable_declaration" => {
                self.collect_declared_functions(node, siblings, idx, false, fns)?;
            }
            "comment" => {
                if let Some(c) = convert_ts_comment(node, self.source, self.path) {
                    top_level_comments.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn collect_class_methods(
        &self,
        class_node: tree_sitter::Node<'a>,
        out: &mut Vec<IrFn>,
    ) -> Result<(), IrConvertError> {
        let Some(body) = class_node.child_by_field_name("body") else {
            return Ok(());
        };
        let mut cursor = body.walk();
        let children: Vec<tree_sitter::Node> = body.children(&mut cursor).collect();
        for (idx, node) in children.iter().enumerate() {
            if node.kind() != "method_definition" {
                continue;
            }
            let leading_doc = collect_ts_leading_doc(&children, idx, self.source);
            let decorators =
                collect_ts_preceding_decorators(&children, idx, self.source, self.path);
            let f = self.convert_named_function(*node, true, leading_doc, decorators)?;
            out.push(f);
        }
        Ok(())
    }

    /// Extract any `const f = () => {}` / `const f = function () {}`
    /// declarators whose value is a function expression and surface them
    /// as named [`IrFn`]s. Other declarators (plain values) are ignored
    /// at the definition level.
    fn collect_declared_functions(
        &self,
        decl_node: tree_sitter::Node<'a>,
        siblings: &[tree_sitter::Node<'a>],
        idx: usize,
        is_method: bool,
        out: &mut Vec<IrFn>,
    ) -> Result<(), IrConvertError> {
        let mut cursor = decl_node.walk();
        for child in decl_node.children(&mut cursor) {
            if child.kind() != "variable_declarator" {
                continue;
            }
            let Some(value) = child.child_by_field_name("value") else {
                continue;
            };
            if !matches!(value.kind(), "arrow_function" | "function_expression") {
                continue;
            }
            let name = child
                .child_by_field_name("name")
                .map(|n| self.text(n).to_string())
                .unwrap_or_default();
            let leading_doc = collect_ts_leading_doc(siblings, idx, self.source);
            let f =
                self.convert_function_body_node(value, name, is_method, leading_doc, Vec::new());
            out.push(f);
        }
        Ok(())
    }

    /// Convert a `function_declaration` / `method_definition` (named via
    /// the node's `name` field) into an [`IrFn`].
    fn convert_named_function(
        &self,
        node: tree_sitter::Node<'a>,
        is_method: bool,
        leading_doc: Option<String>,
        decorators: Vec<IrDecorator>,
    ) -> Result<IrFn, IrConvertError> {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        Ok(self.convert_function_body_node(node, name, is_method, leading_doc, decorators))
    }

    /// Shared body conversion for any function-bearing node
    /// (`function_declaration`, `method_definition`, `arrow_function`,
    /// `function_expression`). `name` is supplied by the caller because
    /// arrow / function expressions carry their name on the enclosing
    /// declarator, not on the node itself.
    fn convert_function_body_node(
        &self,
        node: tree_sitter::Node<'a>,
        name: String,
        is_method: bool,
        leading_doc: Option<String>,
        decorators: Vec<IrDecorator>,
    ) -> IrFn {
        let params = match node.child_by_field_name("parameters") {
            Some(p) => self.convert_ts_params(p),
            None => Vec::new(),
        };

        let return_type_text = node.child_by_field_name("return_type").map(|n| {
            // `return_type` is a `type_annotation` node spelled `: T`;
            // strip the leading colon + whitespace to store just `T`,
            // matching the Rust converter's inner-type convention.
            let raw = self.text(n);
            raw.trim_start_matches(':').trim().to_string()
        });

        let body = match node.child_by_field_name("body") {
            Some(b) if b.kind() == "statement_block" => self.convert_ts_block(b),
            // Arrow concise body (`(x) => expr`): the expression is the
            // implicit return value. Model it as a one-statement block
            // holding a `return expr` so the implicit return participates
            // in terminator analysis and any nested calls stay reachable.
            Some(expr) => self.concise_arrow_block(node, expr),
            None => empty_block(self.path, node),
        };

        let mut normalised_tokens = Vec::new();
        walk_normalize_ts(node, &mut normalised_tokens);

        IrFn {
            name,
            params,
            body,
            return_type_text,
            decorators,
            is_method,
            leading_doc,
            normalised_tokens,
            location: node_location(self.path, node),
        }
    }

    fn concise_arrow_block(
        &self,
        arrow: tree_sitter::Node<'a>,
        expr: tree_sitter::Node<'a>,
    ) -> IrBlock {
        let value = self.convert_ts_expr(expr);
        let stmt = IrStmt {
            kind: IrStmtKind::Return(Some(value)),
            attributes: Vec::new(),
            location: node_location(self.path, expr),
        };
        let statements = vec![stmt];
        let terminator = compute_block_terminator(&statements);
        let mut block_tokens = Vec::new();
        walk_normalize_ts(expr, &mut block_tokens);
        IrBlock {
            statements,
            terminator,
            normalised_token_count: block_tokens.len(),
            location: node_location(self.path, arrow),
        }
    }

    fn convert_ts_params(&self, params_node: tree_sitter::Node<'a>) -> Vec<IrParam> {
        let mut out = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            let (name, kind) = match child.kind() {
                "required_parameter" | "optional_parameter" => {
                    match child.child_by_field_name("pattern") {
                        Some(pat) => match pat.kind() {
                            "identifier" => (self.text(pat).to_string(), ParamKind::Plain),
                            // TS explicit `this` parameter is a typing
                            // device, never a real positional argument.
                            "this" => (self.text(pat).to_string(), ParamKind::Receiver),
                            // Rest / destructuring patterns cannot be
                            // reasoned about positionally by arg-swap.
                            _ => (self.text(child).to_string(), ParamKind::Unsupported),
                        },
                        None => (self.text(child).to_string(), ParamKind::Unsupported),
                    }
                }
                _ => (self.text(child).to_string(), ParamKind::Unsupported),
            };
            // `function f(a, b = 10)`: tree-sitter-typescript exposes the
            // default expression under the `value` field of the parameter
            // node (M6). `b?: number` carries no `value`, so default stays
            // None.
            let default = child
                .child_by_field_name("value")
                .map(|n| self.text(n).trim().to_string());
            out.push(IrParam {
                name,
                kind,
                default,
                location: node_location(self.path, child),
            });
        }
        out
    }

    fn convert_ts_block(&self, block: tree_sitter::Node<'a>) -> IrBlock {
        let mut cursor = block.walk();
        let raw_children: Vec<tree_sitter::Node> = block.children(&mut cursor).collect();

        let mut statements: Vec<IrStmt> = Vec::new();
        for child in raw_children.iter() {
            if !child.is_named() {
                continue;
            }
            match child.kind() {
                "comment" | "empty_statement" => continue,
                _ => {}
            }
            let kind = self.classify_ts_stmt(*child);
            statements.push(IrStmt {
                kind,
                // TypeScript has no Rust-style per-statement attribute
                // chain; suppression decorators bind to declarations, not
                // arbitrary statements.
                attributes: Vec::new(),
                location: node_location(self.path, *child),
            });
        }

        let terminator = compute_block_terminator(&statements);

        let mut block_tokens = Vec::new();
        walk_normalize_ts(block, &mut block_tokens);

        IrBlock {
            statements,
            terminator,
            normalised_token_count: block_tokens.len(),
            location: node_location(self.path, block),
        }
    }

    fn classify_ts_stmt(&self, node: tree_sitter::Node<'a>) -> IrStmtKind {
        match node.kind() {
            "expression_statement" => {
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                match inner {
                    Some(inner) => self.classify_ts_expression_as_stmt(inner),
                    None => self.other_stmt(node),
                }
            }
            "return_statement" => IrStmtKind::Return(self.first_named_value(node)),
            "throw_statement" => IrStmtKind::Raise(self.first_named_value(node)),
            "if_statement" => IrStmtKind::If(self.convert_ts_if(node)),
            "while_statement" => IrStmtKind::While(self.convert_ts_while(node)),
            // `do { ... } while (c)` runs the body at least once; model it
            // as a while for body reachability (it never terminates the
            // enclosing block).
            "do_statement" => IrStmtKind::While(self.convert_ts_while(node)),
            "for_statement" => IrStmtKind::For(self.convert_ts_c_for(node)),
            "for_in_statement" => IrStmtKind::For(self.convert_ts_for_in(node)),
            "break_statement" => IrStmtKind::Break(self.ts_label(node)),
            "continue_statement" => IrStmtKind::Continue(self.ts_label(node)),
            "lexical_declaration" | "variable_declaration" => IrStmtKind::Let {
                value: self.first_declarator_value(node),
            },
            "function_declaration" | "generator_function_declaration" | "function_signature" => {
                IrStmtKind::HoistedItem {
                    kind: HoistedItemKind::Function,
                    node_ref: node_ref(node),
                }
            }
            "class_declaration" | "abstract_class_declaration" => IrStmtKind::HoistedItem {
                kind: HoistedItemKind::Type,
                node_ref: node_ref(node),
            },
            "import_statement" => IrStmtKind::HoistedItem {
                kind: HoistedItemKind::Use,
                node_ref: node_ref(node),
            },
            _ => self.other_stmt(node),
        }
    }

    fn classify_ts_expression_as_stmt(&self, inner: tree_sitter::Node<'a>) -> IrStmtKind {
        match inner.kind() {
            "call_expression" => match self.convert_ts_call_site(inner) {
                Some(call) => match typescript_divergent_kind(&call.callee.raw) {
                    Some(kind) => IrStmtKind::DivergentCall {
                        kind,
                        args: call.args,
                    },
                    None => IrStmtKind::Call(call),
                },
                None => self.other_stmt(inner),
            },
            // `await foo()` in statement position: unwrap the transparent
            // await wrapper and reclassify the inner expression.
            "await_expression" => {
                let mut cursor = inner.walk();
                let awaited = inner.children(&mut cursor).find(|c| c.is_named());
                match awaited {
                    Some(awaited) => self.classify_ts_expression_as_stmt(awaited),
                    None => self.other_stmt(inner),
                }
            }
            "assignment_expression" => IrStmtKind::Assign {
                value: inner
                    .child_by_field_name("right")
                    .map(|r| self.convert_ts_expr(r)),
            },
            _ => self.other_stmt(inner),
        }
    }

    fn convert_ts_if(&self, node: tree_sitter::Node<'a>) -> IrIfStmt {
        let condition = node
            .child_by_field_name("condition")
            .map(|c| self.convert_ts_expr(c))
            .unwrap_or_else(|| other_expr(self.path, node, "missing_condition"));
        let consequence = node
            .child_by_field_name("consequence")
            .map(|c| self.block_from_stmt_or_block(c))
            .unwrap_or_else(|| empty_block(self.path, node));
        let alternative = node
            .child_by_field_name("alternative")
            .map(|a| self.convert_else_clause(a));
        // Branch-merge: an `if` diverges iff both arms diverge. Reuse the
        // already-computed per-block terminators (the alternative block of
        // an `else if` carries the nested if's terminator via recursion).
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

    fn convert_else_clause(&self, else_clause: tree_sitter::Node<'a>) -> IrBlock {
        // `else_clause` wraps either a `statement_block` or a chained
        // `if_statement` (`else if`). Surface the chained if as a
        // one-statement block so its terminator propagates upward.
        let mut cursor = else_clause.walk();
        for child in else_clause.children(&mut cursor) {
            match child.kind() {
                "statement_block" => return self.convert_ts_block(child),
                "if_statement" => return self.single_stmt_block(child),
                _ => {}
            }
        }
        empty_block(self.path, else_clause)
    }

    fn block_from_stmt_or_block(&self, node: tree_sitter::Node<'a>) -> IrBlock {
        if node.kind() == "statement_block" {
            self.convert_ts_block(node)
        } else {
            self.single_stmt_block(node)
        }
    }

    /// Wrap a single non-block statement (e.g. a braceless `if (c) return;`
    /// body) into an [`IrBlock`] so terminator analysis is uniform.
    fn single_stmt_block(&self, stmt: tree_sitter::Node<'a>) -> IrBlock {
        let kind = self.classify_ts_stmt(stmt);
        let statements = vec![IrStmt {
            kind,
            attributes: Vec::new(),
            location: node_location(self.path, stmt),
        }];
        let terminator = compute_block_terminator(&statements);
        let mut block_tokens = Vec::new();
        walk_normalize_ts(stmt, &mut block_tokens);
        IrBlock {
            statements,
            terminator,
            normalised_token_count: block_tokens.len(),
            location: node_location(self.path, stmt),
        }
    }

    fn convert_ts_while(&self, node: tree_sitter::Node<'a>) -> IrWhileStmt {
        let condition = node
            .child_by_field_name("condition")
            .map(|c| self.convert_ts_expr(c))
            .unwrap_or_else(|| other_expr(self.path, node, "missing_condition"));
        let body = node
            .child_by_field_name("body")
            .map(|b| self.block_from_stmt_or_block(b))
            .unwrap_or_else(|| empty_block(self.path, node));
        IrWhileStmt {
            condition,
            body,
            location: node_location(self.path, node),
        }
    }

    fn convert_ts_for_in(&self, node: tree_sitter::Node<'a>) -> IrForStmt {
        let iterable = node
            .child_by_field_name("right")
            .map(|r| self.convert_ts_expr(r))
            .unwrap_or_else(|| other_expr(self.path, node, "missing_iterable"));
        let body = node
            .child_by_field_name("body")
            .map(|b| self.block_from_stmt_or_block(b))
            .unwrap_or_else(|| empty_block(self.path, node));
        IrForStmt {
            iterable,
            body,
            location: node_location(self.path, node),
        }
    }

    fn convert_ts_c_for(&self, node: tree_sitter::Node<'a>) -> IrForStmt {
        // C-style `for (init; cond; inc)` has no single iterable; surface
        // the condition as the iterable expression so calls there stay
        // reachable, and keep the body for nested call walking.
        let iterable = node
            .child_by_field_name("condition")
            .map(|c| self.convert_ts_expr(c))
            .unwrap_or_else(|| other_expr(self.path, node, "c_for_no_iterable"));
        let body = node
            .child_by_field_name("body")
            .map(|b| self.block_from_stmt_or_block(b))
            .unwrap_or_else(|| empty_block(self.path, node));
        IrForStmt {
            iterable,
            body,
            location: node_location(self.path, node),
        }
    }

    fn convert_ts_expr(&self, node: tree_sitter::Node<'a>) -> IrExpr {
        let location = node_location(self.path, node);
        let kind = match node.kind() {
            "identifier"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "type_identifier" => IrExprKind::Ident(self.text(node).to_string()),
            "this" => IrExprKind::Ident("this".to_string()),
            "member_expression" => IrExprKind::Path(self.convert_ts_member_path(node)),
            "number" => IrExprKind::Literal(parse_ts_number(self.text(node))),
            "string" => IrExprKind::Literal(IrLiteral::String {
                is_empty: ts_string_is_empty(node),
            }),
            "true" => IrExprKind::Literal(IrLiteral::Bool(true)),
            "false" => IrExprKind::Literal(IrLiteral::Bool(false)),
            // TS `null` / `undefined` have no dedicated IrLiteral; the
            // closest "nullish constant" is the Python-origin `None`.
            "null" | "undefined" => IrExprKind::Literal(IrLiteral::None),
            "call_expression" => match self.convert_ts_call_site(node) {
                Some(call) => match typescript_divergent_kind(&call.callee.raw) {
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
            "parenthesized_expression" | "await_expression" => {
                // Transparent wrappers: return the inner expression with
                // its own location (ir-v0.md §F2 transparent-wrapper note).
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                match inner {
                    Some(inner) => return self.convert_ts_expr(inner),
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

    /// Convert a `member_expression` (`a.b.c`) into an [`IrPath`] with the
    /// receiver chain in `receiver` and the final property as the sole
    /// `segments` entry, matching the Python attribute convention so
    /// pr-miner-style "last segment" consumers see the method name.
    fn convert_ts_member_path(&self, node: tree_sitter::Node<'a>) -> IrPath {
        let raw = self.text(node).to_string();
        let mut chain: Vec<String> = Vec::new();
        ts_collect_member_chain(node, self.source, &mut chain);
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

    fn convert_ts_call_site(&self, node: tree_sitter::Node<'a>) -> Option<IrCallSite> {
        let function = node.child_by_field_name("function")?;
        let arguments = node.child_by_field_name("arguments")?;
        let callee = match function.kind() {
            "identifier" | "property_identifier" => IrPath {
                receiver: Vec::new(),
                segments: vec![self.text(function).to_string()],
                raw: self.text(function).to_string(),
            },
            "member_expression" => self.convert_ts_member_path(function),
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
            args.push(self.convert_ts_expr(child));
        }
        Some(IrCallSite {
            callee,
            args,
            location: node_location(self.path, node),
        })
    }

    fn first_named_value(&self, node: tree_sitter::Node<'a>) -> Option<IrExpr> {
        let mut cursor = node.walk();
        let found = node.children(&mut cursor).find(|c| c.is_named());
        found.map(|v| self.convert_ts_expr(v))
    }

    fn first_declarator_value(&self, decl_node: tree_sitter::Node<'a>) -> Option<IrExpr> {
        let mut cursor = decl_node.walk();
        for child in decl_node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(v) = child.child_by_field_name("value") {
                    return Some(self.convert_ts_expr(v));
                }
            }
        }
        None
    }

    fn ts_label(&self, node: tree_sitter::Node<'a>) -> Option<IrLabel> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "statement_identifier" {
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
        IrStmtKind::Raise(_) => Some(IrTerminator::Raise),
        IrStmtKind::Break(_) => Some(IrTerminator::Break),
        IrStmtKind::Continue(_) => Some(IrTerminator::Continue),
        IrStmtKind::DivergentCall { kind, .. } => Some(IrTerminator::DivergentCall { kind: *kind }),
        IrStmtKind::If(if_stmt) => if_stmt.terminator,
        _ => None,
    }
}

// ---------- Comment extraction ----------

fn collect_ts_leading_doc(
    siblings: &[tree_sitter::Node],
    idx: usize,
    source: &str,
) -> Option<String> {
    // Walk preceding siblings upward over `comment` / `decorator` nodes;
    // the nearest `/** ... */` JSDoc block becomes the function's leading
    // doc (rendered with the `/**`, `*/` and per-line ` * ` stripped).
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let n = siblings[i];
        match n.kind() {
            "comment" => {
                let raw = &source[n.byte_range()];
                if raw.starts_with("/**") {
                    return Some(render_jsdoc(raw));
                }
                // A non-doc comment between the declaration and any doc
                // block breaks the association, matching the Rust
                // converter's "nearest contiguous doc" behaviour.
                return None;
            }
            "decorator" => continue,
            _ => break,
        }
    }
    None
}

fn render_jsdoc(raw: &str) -> String {
    let inner = raw
        .strip_prefix("/**")
        .and_then(|s| s.strip_suffix("*/"))
        .unwrap_or(raw);
    let mut lines: Vec<String> = Vec::new();
    for line in inner.lines() {
        let trimmed = line.trim_start();
        let stripped = trimmed.strip_prefix('*').unwrap_or(trimmed);
        let rendered = stripped.strip_prefix(' ').unwrap_or(stripped);
        lines.push(rendered.trim_end().to_string());
    }
    // Drop leading / trailing blank lines produced by the `/**\n` and
    // `\n */` framing.
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn collect_ts_preceding_decorators(
    siblings: &[tree_sitter::Node],
    idx: usize,
    source: &str,
    path: &Arc<Path>,
) -> Vec<IrDecorator> {
    let mut out: Vec<IrDecorator> = Vec::new();
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let n = siblings[i];
        match n.kind() {
            "comment" => continue,
            "decorator" => {
                if let Some(dec) = convert_ts_decorator(n, source, path) {
                    out.push(dec);
                }
            }
            _ => break,
        }
    }
    out.reverse();
    out
}

fn convert_ts_decorator(
    node: tree_sitter::Node<'_>,
    source: &str,
    path: &Arc<Path>,
) -> Option<IrDecorator> {
    let raw = source[node.byte_range()].to_string();
    // `@foo` / `@foo.bar` / `@foo(...)`: the name path is everything after
    // the `@` up to the first `(` or whitespace, split on `.`.
    let after_at = raw.trim_start().strip_prefix('@')?;
    let head_end = after_at
        .find(|c: char| c == '(' || c.is_whitespace())
        .unwrap_or(after_at.len());
    let name_path: Vec<String> = after_at[..head_end]
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    Some(IrDecorator {
        raw,
        name_path,
        location: node_location(path, node),
    })
}

fn convert_ts_comment(
    node: tree_sitter::Node<'_>,
    source: &str,
    path: &Arc<Path>,
) -> Option<IrComment> {
    let raw = &source[node.byte_range()];
    let (kind, text) = if let Some(rest) = raw.strip_prefix("//") {
        (
            IrCommentKind::TypeScriptLine,
            rest.strip_prefix(' ')
                .unwrap_or(rest)
                .trim_end()
                .to_string(),
        )
    } else if raw.starts_with("/**") && raw.len() > 4 {
        (IrCommentKind::TypeScriptDocBlock, render_jsdoc(raw))
    } else if let Some(inner) = raw.strip_prefix("/*").and_then(|s| s.strip_suffix("*/")) {
        (IrCommentKind::TypeScriptBlock, inner.trim().to_string())
    } else {
        (IrCommentKind::TypeScriptBlock, raw.to_string())
    };
    Some(IrComment {
        kind,
        text,
        target: None,
        location: node_location(path, node),
    })
}

// ---------- Path / member chain ----------

fn ts_collect_member_chain(node: tree_sitter::Node<'_>, source: &str, out: &mut Vec<String>) {
    match node.kind() {
        "member_expression" => {
            if let Some(object) = node.child_by_field_name("object") {
                ts_collect_member_chain(object, source, out);
            }
            if let Some(property) = node.child_by_field_name("property") {
                out.push(source[property.byte_range()].to_string());
            }
        }
        "identifier" | "property_identifier" | "this" | "shorthand_property_identifier" => {
            out.push(source[node.byte_range()].to_string());
        }
        _ => {
            // Computed access / call receivers etc.: keep the raw text as
            // a single chain segment so callers still see a non-empty
            // receiver.
            out.push(source[node.byte_range()].to_string());
        }
    }
}

// ---------- Normalised tokens ----------

fn walk_normalize_ts(node: tree_sitter::Node<'_>, out: &mut Vec<NormalisedToken>) {
    if !node.is_named() {
        return;
    }
    let kind = node.kind();
    if kind == "comment" {
        return;
    }
    let leaf_token = match kind {
        "identifier"
        | "property_identifier"
        | "shorthand_property_identifier"
        | "type_identifier"
        | "statement_identifier" => Some(NormalisedToken::Ident),
        // TypeScript's lexer emits a single `number` node for ints and
        // floats; fold both to LitInt (the structural clone signal does
        // not depend on the numeric subtype).
        "number" => Some(NormalisedToken::LitInt),
        "string" | "template_string" => Some(NormalisedToken::LitStr),
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
        walk_normalize_ts(child, out);
    }
}

// ---------- Literal parsing ----------

fn parse_ts_number(text: &str) -> IrLiteral {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.starts_with("0o") || lower.starts_with("0b") {
        // Non-decimal radix: ir-v0.md F1 keeps these as `Int(None)`.
        return IrLiteral::Int(None);
    }
    if trimmed.ends_with('n') {
        // BigInt literal (`10n`): treat as a non-decimal-parsing int.
        return IrLiteral::Int(None);
    }
    if trimmed.contains('.') || lower.contains('e') {
        return IrLiteral::Float;
    }
    let no_sep: String = trimmed.chars().filter(|c| *c != '_').collect();
    IrLiteral::Int(no_sep.parse::<i128>().ok())
}

fn ts_string_is_empty(node: tree_sitter::Node<'_>) -> bool {
    // A `string` node with no `string_fragment` / escape children between
    // its delimiters is empty.
    let mut cursor = node.walk();
    let has_content = node
        .children(&mut cursor)
        .any(|c| matches!(c.kind(), "string_fragment" | "escape_sequence"));
    !has_content
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
            .set_language(&tree_sitter_typescript::language_typescript())
            .expect("set typescript language");
        let tree = parser.parse(source, None).expect("parse typescript");
        TypeScriptParserProvider
            .to_ir(tree, Arc::from(source), PathBuf::from("a.ts"))
            .expect("to_ir succeeds")
    }

    #[test]
    fn converts_top_level_function() {
        let ir = to_ir("function add(a: number, b: number): number { return a + b; }\n");
        assert_eq!(ir.fns.len(), 1);
        let f = &ir.fns[0];
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].kind, ParamKind::Plain);
        assert!(!f.is_method);
        assert_eq!(f.return_type_text.as_deref(), Some("number"));
    }

    #[test]
    fn default_parameter_captures_default_literal() {
        // M6: `b = expr` captures the trimmed default; `c?: T` (optional,
        // no value) and a required param stay None.
        let ir = to_ir("function f(a: number, b: number = 10, c?: string): void {}\n");
        let f = &ir.fns[0];
        assert_eq!(f.params.len(), 3);
        assert_eq!(f.params[0].default, None);
        assert_eq!(f.params[1].default.as_deref(), Some("10"));
        assert_eq!(f.params[2].default, None);
    }

    #[test]
    fn marks_class_methods_as_methods() {
        let ir =
            to_ir("class Foo {\n  bar(x: number): void { this.baz(x); }\n  static qux() {}\n}\n");
        let methods: Vec<&IrFn> = ir.fns.iter().filter(|f| f.is_method).collect();
        assert_eq!(methods.len(), 2);
        let bar = methods.iter().find(|f| f.name == "bar").unwrap();
        assert_eq!(bar.params[0].name, "x");
    }

    #[test]
    fn extracts_arrow_and_function_expressions() {
        let ir = to_ir(
            "const f = (a: string, b: string) => { return a + b; };\nexport const g = () => 1;\n",
        );
        let names: Vec<&str> = ir.fns.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"f"));
        assert!(names.contains(&"g"));
        // Concise-body arrow `g` gets an implicit `return 1`.
        let g = ir.fns.iter().find(|f| f.name == "g").unwrap();
        assert!(matches!(
            g.body.statements[0].kind,
            IrStmtKind::Return(Some(_))
        ));
    }

    #[test]
    fn unwraps_export_statement() {
        let ir = to_ir(
            "export function f() {}\nexport default function h() {}\nexport class C { m() {} }\n",
        );
        let names: Vec<&str> = ir.fns.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"f"));
        assert!(names.contains(&"h"));
        assert!(names.contains(&"m"));
    }

    #[test]
    fn jsdoc_becomes_leading_doc() {
        let ir = to_ir("/**\n * adds two numbers\n * @param a first\n */\nfunction add(a, b) { return a + b; }\n");
        assert_eq!(
            ir.fns[0].leading_doc.as_deref(),
            Some("adds two numbers\n@param a first")
        );
    }

    #[test]
    fn call_site_records_callee_and_args() {
        let ir = to_ir("function m() {\n  foo(1, 2);\n  obj.method(a, b);\n}\n");
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
                assert_eq!(call.callee.segments, vec!["method".to_string()]);
            }
            other => panic!("expected member Call, got {other:?}"),
        }
    }

    #[test]
    fn throw_is_a_raise_terminator() {
        let ir = to_ir("function t() { throw new Error('e'); foo(); }\n");
        assert_eq!(ir.fns[0].body.terminator, Some(IrTerminator::Raise));
    }

    #[test]
    fn process_exit_is_divergent() {
        let ir = to_ir("function t() { process.exit(1); foo(); }\n");
        assert!(matches!(
            ir.fns[0].body.terminator,
            Some(IrTerminator::DivergentCall {
                kind: DivergentKind::ProcessExit
            })
        ));
    }

    #[test]
    fn if_both_branches_diverge_merges() {
        let ir = to_ir("function c(x) {\n  if (x) { return 1; } else { throw new Error('e'); }\n  other();\n}\n");
        assert!(matches!(
            ir.fns[0].body.terminator,
            Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::IfBranchesDiverge
            })
        ));
    }

    #[test]
    fn nested_function_declaration_is_hoisted() {
        let ir = to_ir("function outer() { function inner() {} inner(); }\n");
        let outer = ir.fns.iter().find(|f| f.name == "outer").unwrap();
        assert!(matches!(
            outer.body.statements[0].kind,
            IrStmtKind::HoistedItem {
                kind: HoistedItemKind::Function,
                ..
            }
        ));
    }

    #[test]
    fn comments_classify_by_delimiter() {
        let ir = to_ir("// line\n/* block */\nconst x = 1;\n");
        let kinds: Vec<IrCommentKind> = ir.top_level_comments.iter().map(|c| c.kind).collect();
        assert!(kinds.contains(&IrCommentKind::TypeScriptLine));
        assert!(kinds.contains(&IrCommentKind::TypeScriptBlock));
    }

    #[test]
    fn parse_recovered_set_on_syntax_error() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::language_typescript())
            .unwrap();
        let broken = "function f( {\n";
        let tree = parser.parse(broken, None).unwrap();
        let ir = TypeScriptParserProvider
            .to_ir(tree, Arc::from(broken), PathBuf::from("a.ts"))
            .expect("to_ir succeeds on partial parse");
        assert!(ir.parse_recovered);
    }

    #[test]
    fn location_invariant_for_call_site() {
        let source = "function f() {\n  bar(x, y);\n}\n";
        let ir = to_ir(source);
        let call = match &ir.fns[0].body.statements[0].kind {
            IrStmtKind::Call(c) => c,
            _ => panic!(),
        };
        assert_eq!(call.location.start_line, 2);
        assert_eq!(call.location.start_col, 3);
        let raw =
            &source.as_bytes()[call.location.start_byte as usize..call.location.end_byte as usize];
        assert_eq!(std::str::from_utf8(raw).unwrap(), "bar(x, y)");
    }
}
