//! Rust tree-sitter provider + [`crate::ir::IrFile`] converter.
//!
//! Spec: `docs/spec/ir-v0.md` §F1, §F2, §F3. The converter walks the
//! Rust tree-sitter AST emitted by `tree_sitter_rust::language()` and
//! materialises the IR nodes the cross-cutting detectors consume.
//! Per ir-v0.md §F2, `to_ir` is total over recognised shapes: unknown
//! statement / expression nodes fall back to
//! [`crate::ir::IrStmtKind::Other`] / [`crate::ir::IrExpr::Other`] with
//! the tree-sitter `Node::kind()` discriminator + a [`NodeRef`] for
//! raw-tree recovery. [`crate::ir::IrConvertError::StructuralInvariant`]
//! is reserved for invariants we expect tree-sitter-rust to honour
//! (e.g. a `function_item` having a `name` field); these are programmer
//! errors per ir-v0.md §F2 / R9 and should never fire on real source.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{build_ir_shell, Language, ParserProvider};
use crate::ir::{
    BranchMergeKind, DivergentKind, HoistedItemKind, IrBlock, IrCallSite, IrComment, IrCommentKind,
    IrConvertError, IrDecorator, IrExpr, IrFile, IrFn, IrIfStmt, IrLabel, IrLiteral, IrLoopStmt,
    IrMatchArm, IrMatchStmt, IrParam, IrPath, IrStmt, IrStmtKind, IrTerminator, IrWhileStmt,
    Location, NodeRef, NormalisedToken, ParamKind,
};

/// Provider for Rust source (`*.rs`).
pub struct RustParserProvider;

impl ParserProvider for RustParserProvider {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_rust::language()
    }

    fn to_ir(
        &self,
        tree: tree_sitter::Tree,
        source: Arc<str>,
        path: PathBuf,
    ) -> Result<IrFile, IrConvertError> {
        let mut shell = build_ir_shell(self, &tree, source, path)?;
        let (fns, top_level_comments) = {
            let cv = Converter {
                source: shell.source.as_ref(),
                path: shell.path.as_path(),
            };
            cv.convert_root(tree.root_node())?
        };
        shell.fns = fns;
        shell.top_level_comments = top_level_comments;
        // `tree` drops here — IrFile keeps no reference to it. R1
        // mitigation: language-specific detectors reparse via
        // IrFile::raw_tree on demand.
        Ok(shell)
    }
}

// ---------- Converter ----------

struct Converter<'a> {
    source: &'a str,
    path: &'a Path,
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
                "function_item" => {
                    let leading_doc = collect_rust_leading_doc(&children, idx, self.source);
                    let decorators =
                        collect_rust_preceding_attributes(&children, idx, self.source, self.path);
                    let ir_fn = self.convert_function(*node, false, leading_doc, decorators)?;
                    fns.push(ir_fn);
                }
                "impl_item" => {
                    self.collect_impl_methods(*node, &mut fns)?;
                }
                "line_comment" | "block_comment" => {
                    if let Some(comment) = convert_rust_comment(*node, self.source, self.path, None)
                    {
                        top_level_comments.push(comment);
                    }
                }
                _ => {}
            }
        }
        Ok((fns, top_level_comments))
    }

    fn collect_impl_methods(
        &self,
        impl_node: tree_sitter::Node<'a>,
        out: &mut Vec<IrFn>,
    ) -> Result<(), IrConvertError> {
        let Some(body) = impl_node.child_by_field_name("body") else {
            return Ok(());
        };
        let mut cursor = body.walk();
        let children: Vec<tree_sitter::Node> = body.children(&mut cursor).collect();
        for (idx, node) in children.iter().enumerate() {
            if node.kind() != "function_item" {
                continue;
            }
            let leading_doc = collect_rust_leading_doc(&children, idx, self.source);
            let decorators =
                collect_rust_preceding_attributes(&children, idx, self.source, self.path);
            let ir_fn = self.convert_function(*node, true, leading_doc, decorators)?;
            out.push(ir_fn);
        }
        Ok(())
    }

    fn convert_function(
        &self,
        node: tree_sitter::Node<'a>,
        is_method: bool,
        leading_doc: Option<String>,
        decorators: Vec<IrDecorator>,
    ) -> Result<IrFn, IrConvertError> {
        let name_node = node.child_by_field_name("name").ok_or_else(|| {
            IrConvertError::StructuralInvariant {
                kind: "function_item",
                message: "missing `name` field".to_string(),
            }
        })?;
        let name = self.text(name_node).to_string();

        let params = match node.child_by_field_name("parameters") {
            Some(params_node) => self.convert_rust_params(params_node, is_method),
            None => Vec::new(),
        };

        let return_type_text = node
            .child_by_field_name("return_type")
            .map(|n| self.text(n).to_string());

        let body = match node.child_by_field_name("body") {
            Some(b) => self.convert_rust_block(b),
            None => empty_block(self.path, node),
        };

        // R2 (ir-v0.md): clone-drift normalises every top-level function
        // exactly once. Root the token sequence at the whole
        // `function_item` so the signature prefix participates in the
        // n-gram set, matching v0.5.x `walk_normalize_rust(function_item)`
        // byte-for-byte.
        let mut normalised_tokens = Vec::new();
        walk_normalize_rust(node, &mut normalised_tokens);

        Ok(IrFn {
            name,
            params,
            body,
            return_type_text,
            decorators,
            is_method,
            leading_doc,
            normalised_tokens,
            location: node_location(self.path, node),
        })
    }

    fn convert_rust_params(
        &self,
        params_node: tree_sitter::Node<'a>,
        is_method: bool,
    ) -> Vec<IrParam> {
        let mut out = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            let (name, kind) = match child.kind() {
                "self_parameter" => (
                    self.text(child).to_string(),
                    if is_method {
                        ParamKind::Receiver
                    } else {
                        // `self` in a free fn position should never
                        // occur; treat defensively as Unsupported.
                        ParamKind::Unsupported
                    },
                ),
                "parameter" => match child.child_by_field_name("pattern") {
                    Some(pat) => match extract_rust_pattern_identifier(pat, self.source) {
                        Some(n) => (n, ParamKind::Plain),
                        None => (self.text(child).to_string(), ParamKind::Unsupported),
                    },
                    None => (self.text(child).to_string(), ParamKind::Unsupported),
                },
                _ => (self.text(child).to_string(), ParamKind::Unsupported),
            };
            out.push(IrParam {
                name,
                kind,
                location: node_location(self.path, child),
            });
        }
        out
    }

    fn convert_rust_block(&self, block: tree_sitter::Node<'a>) -> IrBlock {
        // Collect statements (with attribute carry-through), tail
        // expression handling, normalised tokens, and terminator.
        let mut cursor = block.walk();
        let raw_children: Vec<tree_sitter::Node> = block.children(&mut cursor).collect();

        let mut statements: Vec<IrStmt> = Vec::new();
        let mut pending_attributes: Vec<IrDecorator> = Vec::new();

        for child in raw_children.iter() {
            if !child.is_named() {
                continue;
            }
            match child.kind() {
                "line_comment" | "block_comment" => continue,
                "attribute_item" | "inner_attribute_item" => {
                    if let Some(dec) = convert_rust_attribute(*child, self.source, self.path) {
                        pending_attributes.push(dec);
                    }
                    continue;
                }
                "empty_statement" => {
                    pending_attributes.clear();
                    continue;
                }
                _ => {}
            }

            let kind = self.classify_rust_stmt(*child);
            let attributes = std::mem::take(&mut pending_attributes);
            statements.push(IrStmt {
                kind,
                attributes,
                location: node_location(self.path, *child),
            });
        }

        let terminator = compute_block_terminator(&statements);

        // Block-rooted token count for F2b's intra-fn if-branch size
        // gate (ir-v0.md R2: count only, not the vector).
        let mut block_tokens = Vec::new();
        walk_normalize_rust(block, &mut block_tokens);

        IrBlock {
            statements,
            terminator,
            normalised_token_count: block_tokens.len(),
            location: node_location(self.path, block),
        }
    }

    fn classify_rust_stmt(&self, node: tree_sitter::Node<'a>) -> IrStmtKind {
        // Item-shaped declarations sit in a block as hoisted items.
        if let Some(kind) = rust_hoisted_kind(node.kind()) {
            return IrStmtKind::HoistedItem {
                kind,
                node_ref: node_ref(node),
            };
        }

        match node.kind() {
            "let_declaration" => IrStmtKind::Other {
                node_kind: "let_declaration",
                node_ref: node_ref(node),
            },
            "expression_statement" => {
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                match inner {
                    Some(inner) => self.classify_rust_expression_as_stmt(inner),
                    None => IrStmtKind::Other {
                        node_kind: "expression_statement",
                        node_ref: node_ref(node),
                    },
                }
            }
            "if_expression" => IrStmtKind::If(self.convert_rust_if(node)),
            "match_expression" => IrStmtKind::Match(self.convert_rust_match(node)),
            "while_expression" => IrStmtKind::While(self.convert_rust_while(node)),
            "loop_expression" => IrStmtKind::Loop(self.convert_rust_loop(node)),
            "return_expression" => {
                IrStmtKind::Return(self.convert_rust_optional_value(node, &["loop_label"]))
            }
            "break_expression" => IrStmtKind::Break(self.rust_label(node)),
            "continue_expression" => IrStmtKind::Continue(self.rust_label(node)),
            "macro_invocation" => match rust_macro_terminator_kind(node, self.source) {
                Some(kind) => IrStmtKind::DivergentCall {
                    kind,
                    args: Vec::new(),
                },
                None => IrStmtKind::Other {
                    node_kind: "macro_invocation",
                    node_ref: node_ref(node),
                },
            },
            "call_expression" => match self.convert_rust_call_site(node) {
                Some(call) => IrStmtKind::Call(call),
                None => IrStmtKind::Other {
                    node_kind: "call_expression",
                    node_ref: node_ref(node),
                },
            },
            other => IrStmtKind::Other {
                node_kind: static_kind_str(other),
                node_ref: node_ref(node),
            },
        }
    }

    fn classify_rust_expression_as_stmt(&self, inner: tree_sitter::Node<'a>) -> IrStmtKind {
        match inner.kind() {
            "return_expression" => {
                IrStmtKind::Return(self.convert_rust_optional_value(inner, &["loop_label"]))
            }
            "break_expression" => IrStmtKind::Break(self.rust_label(inner)),
            "continue_expression" => IrStmtKind::Continue(self.rust_label(inner)),
            "if_expression" => IrStmtKind::If(self.convert_rust_if(inner)),
            "match_expression" => IrStmtKind::Match(self.convert_rust_match(inner)),
            "while_expression" => IrStmtKind::While(self.convert_rust_while(inner)),
            "loop_expression" => IrStmtKind::Loop(self.convert_rust_loop(inner)),
            "macro_invocation" => {
                if is_rust_assert_macro(inner, self.source) {
                    let cond = self.first_macro_token_argument(inner);
                    return IrStmtKind::Assert(cond);
                }
                match rust_macro_terminator_kind(inner, self.source) {
                    Some(kind) => IrStmtKind::DivergentCall {
                        kind,
                        args: Vec::new(),
                    },
                    None => IrStmtKind::Other {
                        node_kind: "macro_invocation",
                        node_ref: node_ref(inner),
                    },
                }
            }
            "call_expression" => match self.convert_rust_call_site(inner) {
                Some(call) => IrStmtKind::Call(call),
                None => IrStmtKind::Other {
                    node_kind: "call_expression",
                    node_ref: node_ref(inner),
                },
            },
            other => IrStmtKind::Other {
                node_kind: static_kind_str(other),
                node_ref: node_ref(inner),
            },
        }
    }

    fn rust_label(&self, node: tree_sitter::Node<'a>) -> Option<IrLabel> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "loop_label" {
                let mut inner = child.walk();
                for c in child.children(&mut inner) {
                    if c.kind() == "identifier" {
                        return Some(IrLabel::Named(self.text(c).to_string()));
                    }
                }
            }
        }
        Some(IrLabel::Unlabelled)
    }

    fn convert_rust_optional_value(
        &self,
        node: tree_sitter::Node<'a>,
        skip_kinds: &[&str],
    ) -> Option<IrExpr> {
        let mut cursor = node.walk();
        let value = node
            .children(&mut cursor)
            .find(|c| c.is_named() && !skip_kinds.contains(&c.kind()));
        value.map(|v| self.convert_rust_expr(v))
    }

    fn convert_rust_if(&self, node: tree_sitter::Node<'a>) -> IrIfStmt {
        let condition = node
            .child_by_field_name("condition")
            .map(|c| self.convert_rust_expr(c))
            .unwrap_or_else(|| IrExpr::Other {
                node_kind: "missing_condition",
                node_ref: node_ref(node),
            });
        let consequence_node = node.child_by_field_name("consequence");
        let consequence = consequence_node
            .map(|c| self.convert_rust_block(c))
            .unwrap_or_else(|| empty_block(self.path, node));
        let alternative_block = node
            .child_by_field_name("alternative")
            .and_then(rust_find_else_block);
        let alternative = alternative_block.map(|b| self.convert_rust_block(b));
        let terminator = self.compute_rust_if_branch_merge(node);
        IrIfStmt {
            condition,
            consequence,
            alternative,
            terminator,
            location: node_location(self.path, node),
        }
    }

    fn convert_rust_match(&self, node: tree_sitter::Node<'a>) -> IrMatchStmt {
        let scrutinee = node
            .child_by_field_name("value")
            .map(|v| self.convert_rust_expr(v))
            .unwrap_or_else(|| IrExpr::Other {
                node_kind: "missing_scrutinee",
                node_ref: node_ref(node),
            });
        let mut arms: Vec<IrMatchArm> = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() != "match_arm" {
                    continue;
                }
                let body_expr = child
                    .child_by_field_name("value")
                    .map(|v| self.convert_rust_expr(v))
                    .unwrap_or_else(|| IrExpr::Other {
                        node_kind: "missing_arm_value",
                        node_ref: node_ref(child),
                    });
                arms.push(IrMatchArm {
                    body: body_expr,
                    location: node_location(self.path, child),
                });
            }
        }
        let terminator = self.compute_rust_match_branch_merge(node);
        IrMatchStmt {
            scrutinee,
            arms,
            terminator,
            location: node_location(self.path, node),
        }
    }

    fn convert_rust_while(&self, node: tree_sitter::Node<'a>) -> IrWhileStmt {
        let condition = node
            .child_by_field_name("condition")
            .map(|c| self.convert_rust_expr(c))
            .unwrap_or_else(|| IrExpr::Other {
                node_kind: "missing_condition",
                node_ref: node_ref(node),
            });
        let body = node
            .child_by_field_name("body")
            .map(|b| self.convert_rust_block(b))
            .unwrap_or_else(|| empty_block(self.path, node));
        IrWhileStmt {
            condition,
            body,
            location: node_location(self.path, node),
        }
    }

    fn convert_rust_loop(&self, node: tree_sitter::Node<'a>) -> IrLoopStmt {
        let label = rust_loop_self_label_from_node(node, self.source);
        let body_node = rust_first_block_child(node);
        let body = body_node
            .map(|b| self.convert_rust_block(b))
            .unwrap_or_else(|| empty_block(self.path, node));
        let has_break_to_self = match body_node {
            Some(b) => rust_has_break_targeting_self(b, label.as_deref(), 0, self.source),
            None => false,
        };
        IrLoopStmt {
            label: label.map(IrLabel::Named),
            body,
            has_break_to_self,
            location: node_location(self.path, node),
        }
    }

    fn convert_rust_expr(&self, node: tree_sitter::Node<'a>) -> IrExpr {
        match node.kind() {
            "identifier" | "type_identifier" | "field_identifier" => {
                IrExpr::Ident(self.text(node).to_string())
            }
            "scoped_identifier" => IrExpr::Path(self.convert_rust_path(node)),
            "field_expression" => IrExpr::Path(self.convert_rust_path(node)),
            "integer_literal" => IrExpr::Literal(IrLiteral::Int(parse_rust_int(self.text(node)))),
            "float_literal" => IrExpr::Literal(IrLiteral::Float),
            "string_literal" | "raw_string_literal" => {
                let raw = self.text(node);
                IrExpr::Literal(IrLiteral::String {
                    is_empty: rust_string_is_empty(raw),
                })
            }
            "char_literal" => IrExpr::Literal(IrLiteral::Char),
            "boolean_literal" => {
                let text = self.text(node);
                IrExpr::Literal(IrLiteral::Bool(text == "true"))
            }
            "call_expression" => match self.convert_rust_call_site(node) {
                Some(call) => IrExpr::Call(Box::new(call)),
                None => IrExpr::Other {
                    node_kind: "call_expression",
                    node_ref: node_ref(node),
                },
            },
            "macro_invocation" => match rust_macro_terminator_kind(node, self.source) {
                Some(kind) => IrExpr::DivergentCall {
                    kind,
                    args: Vec::new(),
                },
                None => IrExpr::Other {
                    node_kind: "macro_invocation",
                    node_ref: node_ref(node),
                },
            },
            "block" => IrExpr::Block(Box::new(self.convert_rust_block(node))),
            "if_expression" => IrExpr::If(Box::new(self.convert_rust_if(node))),
            "match_expression" => IrExpr::Match(Box::new(self.convert_rust_match(node))),
            "loop_expression" => IrExpr::Loop(Box::new(self.convert_rust_loop(node))),
            "return_expression" => IrExpr::Return(
                self.convert_rust_optional_value(node, &["loop_label"])
                    .map(Box::new),
            ),
            "break_expression" => IrExpr::Break(self.rust_label(node)),
            "continue_expression" => IrExpr::Continue(self.rust_label(node)),
            "parenthesized_expression" => {
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                match inner {
                    Some(inner) => self.convert_rust_expr(inner),
                    None => IrExpr::Other {
                        node_kind: "parenthesized_expression",
                        node_ref: node_ref(node),
                    },
                }
            }
            other => IrExpr::Other {
                node_kind: static_kind_str(other),
                node_ref: node_ref(node),
            },
        }
    }

    fn convert_rust_path(&self, node: tree_sitter::Node<'a>) -> IrPath {
        let raw = self.text(node).to_string();
        let mut segments: Vec<String> = Vec::new();
        rust_collect_path_segments(node, self.source, &mut segments);
        IrPath {
            receiver: Vec::new(),
            segments,
            raw,
        }
    }

    fn convert_rust_call_site(&self, node: tree_sitter::Node<'a>) -> Option<IrCallSite> {
        let function = node.child_by_field_name("function")?;
        let arguments = node.child_by_field_name("arguments")?;
        let callee = match function.kind() {
            "identifier" | "scoped_identifier" | "field_expression" | "type_identifier"
            | "generic_function" => self.convert_rust_path(function),
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
            args.push(self.convert_rust_expr(child));
        }
        Some(IrCallSite {
            callee,
            args,
            location: node_location(self.path, node),
        })
    }

    fn first_macro_token_argument(&self, macro_node: tree_sitter::Node<'a>) -> IrExpr {
        // `assert!(cond)` / `assert!(cond, "msg")`: pick the first
        // expression-like token from the token tree. Production
        // tree-sitter-rust models the args as `token_tree`; we
        // search for the first contained primitive literal /
        // identifier and treat it as the condition expression.
        let Some(args) = macro_node.child_by_field_name("arguments") else {
            return IrExpr::Other {
                node_kind: "macro_invocation",
                node_ref: node_ref(macro_node),
            };
        };
        let mut cursor = args.walk();
        for child in args.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            return self.convert_rust_expr(child);
        }
        IrExpr::Other {
            node_kind: "token_tree",
            node_ref: node_ref(args),
        }
    }

    fn compute_rust_if_branch_merge(&self, if_expr: tree_sitter::Node<'a>) -> Option<IrTerminator> {
        let consequence = if_expr.child_by_field_name("consequence")?;
        let alternative = if_expr.child_by_field_name("alternative")?;
        rust_expression_diverges(consequence, self.source)?;
        rust_alternative_diverges(alternative, self.source)?;
        Some(IrTerminator::BranchMerge {
            kind: BranchMergeKind::IfBranchesDiverge,
        })
    }

    fn compute_rust_match_branch_merge(
        &self,
        match_expr: tree_sitter::Node<'a>,
    ) -> Option<IrTerminator> {
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
            rust_expression_diverges(value, self.source)?;
        }
        Some(IrTerminator::BranchMerge {
            kind: BranchMergeKind::MatchArmsDiverge,
        })
    }

    fn text(&self, node: tree_sitter::Node<'a>) -> &'a str {
        &self.source[node.byte_range()]
    }
}

// ---------- IrStmt → IrTerminator block-level merge ----------

fn compute_block_terminator(statements: &[IrStmt]) -> Option<IrTerminator> {
    // Scan in source order; the first divergent statement determines
    // the block's terminator (everything after it is unreachable).
    // ir-v0.md §F1 only requires `Some` when every reachable path
    // through the block ends in a divergent expression; in v0 the
    // straight-line definition (first terminator wins) is sufficient
    // because the cross-cutting detector
    // (`unreachable-after-terminator`) uses this signal to classify
    // the block's own outer position.
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
        IrStmtKind::Assert(IrExpr::Literal(IrLiteral::Bool(false))) => {
            Some(IrTerminator::AssertFalse)
        }
        IrStmtKind::Assert(_) => None,
        IrStmtKind::If(if_stmt) => if_stmt.terminator,
        IrStmtKind::Match(match_stmt) => match_stmt.terminator,
        IrStmtKind::Loop(loop_stmt) => {
            if loop_stmt.has_break_to_self {
                None
            } else {
                Some(IrTerminator::LoopNoBreak)
            }
        }
        _ => None,
    }
}

// ---------- Rust attribute / decorator extraction ----------

fn collect_rust_preceding_attributes(
    siblings: &[tree_sitter::Node],
    idx: usize,
    source: &str,
    path: &Path,
) -> Vec<IrDecorator> {
    let mut out: Vec<IrDecorator> = Vec::new();
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let n = siblings[i];
        match n.kind() {
            "line_comment" | "block_comment" => continue,
            "attribute_item" | "inner_attribute_item" => {
                if let Some(dec) = convert_rust_attribute(n, source, path) {
                    out.push(dec);
                }
            }
            _ => break,
        }
    }
    out.reverse();
    out
}

fn convert_rust_attribute(
    node: tree_sitter::Node<'_>,
    source: &str,
    path: &Path,
) -> Option<IrDecorator> {
    let raw = source[node.byte_range()].to_string();
    let after_open = raw
        .trim_start()
        .strip_prefix("#![")
        .or_else(|| raw.trim_start().strip_prefix("#["))?;
    let trimmed = after_open.trim_start();
    // The leading identifier (dotted path for nested macros like
    // `foo::bar(...)`) lives before the first `(`, `=`, whitespace or
    // `]`.
    let head_end = trimmed
        .find(|c: char| c == '(' || c == '=' || c == ']' || c.is_whitespace())
        .unwrap_or(trimmed.len());
    let head = &trimmed[..head_end];
    let name_path: Vec<String> = head
        .split("::")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    Some(IrDecorator {
        raw,
        name_path,
        location: node_location(path, node),
    })
}

// ---------- Rust comment extraction ----------

fn collect_rust_leading_doc(
    siblings: &[tree_sitter::Node],
    idx: usize,
    source: &str,
) -> Option<String> {
    // Walk preceding siblings upward as long as they are `///` line
    // comments (skipping attribute_items between). Returns the
    // rendered doc text (lines joined with `\n`, `///` prefix
    // stripped) or `None` when no `///` comments are present.
    let mut lines: Vec<String> = Vec::new();
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let n = siblings[i];
        if n.kind() != "line_comment" {
            if matches!(n.kind(), "attribute_item" | "inner_attribute_item") {
                continue;
            }
            break;
        }
        let text = &source[n.byte_range()];
        if let Some(rest) = text.strip_prefix("///") {
            let rendered = rest.strip_prefix(' ').unwrap_or(rest);
            lines.push(rendered.trim_end_matches('\n').to_string());
        } else {
            break;
        }
    }
    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

fn convert_rust_comment(
    node: tree_sitter::Node<'_>,
    source: &str,
    path: &Path,
    target: Option<NodeRef>,
) -> Option<IrComment> {
    let raw = &source[node.byte_range()];
    let (kind, text) = match node.kind() {
        "line_comment" => {
            if let Some(rest) = raw.strip_prefix("///") {
                (
                    IrCommentKind::RustDocLine,
                    rest.strip_prefix(' ').unwrap_or(rest).to_string(),
                )
            } else if let Some(rest) = raw.strip_prefix("//!") {
                // Inner doc comments use the same delimiter set as `///`
                // but document the enclosing item; we still record them
                // as RustDocLine so comment-code can see them. The
                // distinction lives in the source delimiter; v0
                // detectors do not branch on it.
                (
                    IrCommentKind::RustDocLine,
                    rest.strip_prefix(' ').unwrap_or(rest).to_string(),
                )
            } else if let Some(rest) = raw.strip_prefix("//") {
                (
                    IrCommentKind::RustLine,
                    rest.strip_prefix(' ').unwrap_or(rest).to_string(),
                )
            } else {
                (IrCommentKind::RustLine, raw.to_string())
            }
        }
        "block_comment" => {
            if let Some(inner) = raw.strip_prefix("/**").and_then(|s| s.strip_suffix("*/")) {
                (IrCommentKind::RustDocBlock, inner.trim().to_string())
            } else if let Some(inner) = raw.strip_prefix("/*").and_then(|s| s.strip_suffix("*/")) {
                (IrCommentKind::RustBlock, inner.trim().to_string())
            } else {
                (IrCommentKind::RustBlock, raw.to_string())
            }
        }
        _ => return None,
    };
    Some(IrComment {
        kind,
        text,
        target,
        location: node_location(path, node),
    })
}

// ---------- Rust helper functions ----------

fn extract_rust_pattern_identifier(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(source[node.byte_range()].to_string()),
        "mut_pattern" | "ref_pattern" => {
            let mut cursor = node.walk();
            for c in node.children(&mut cursor) {
                if c.kind() == "identifier" {
                    return Some(source[c.byte_range()].to_string());
                }
            }
            None
        }
        _ => None,
    }
}

fn rust_collect_path_segments(node: tree_sitter::Node<'_>, source: &str, out: &mut Vec<String>) {
    match node.kind() {
        "identifier" | "type_identifier" | "field_identifier" => {
            out.push(source[node.byte_range()].to_string());
        }
        "scoped_identifier" => {
            if let Some(path) = node.child_by_field_name("path") {
                rust_collect_path_segments(path, source, out);
            }
            if let Some(name) = node.child_by_field_name("name") {
                rust_collect_path_segments(name, source, out);
            }
        }
        "field_expression" => {
            // `obj.method` — only the field name lands as a segment so
            // pr-miner-style "last segment" consumers see `method`.
            if let Some(field) = node.child_by_field_name("field") {
                out.push(source[field.byte_range()].to_string());
            }
        }
        "generic_function" => {
            if let Some(inner) = node.child_by_field_name("function") {
                rust_collect_path_segments(inner, source, out);
            }
        }
        _ => {
            // Fall back to raw text as a single segment.
            out.push(source[node.byte_range()].to_string());
        }
    }
}

fn rust_hoisted_kind(node_kind: &str) -> Option<HoistedItemKind> {
    Some(match node_kind {
        "function_item" | "function_signature_item" => HoistedItemKind::Function,
        "mod_item" | "foreign_mod_item" => HoistedItemKind::Mod,
        "struct_item" | "union_item" | "enum_item" | "type_item" => HoistedItemKind::Type,
        "const_item" | "static_item" => HoistedItemKind::Const,
        "trait_item" | "impl_item" | "associated_type" => HoistedItemKind::Trait,
        "use_declaration" | "extern_crate_declaration" => HoistedItemKind::Use,
        "macro_definition" => HoistedItemKind::Macro,
        _ => return None,
    })
}

fn rust_find_else_block(alternative: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut cursor = alternative.walk();
    for child in alternative.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if child.kind() == "block" {
            return Some(child);
        }
        if child.kind() == "if_expression" {
            // else-if chain — surface it as the alternative so it
            // converts as a sub-block via recursion at the outer
            // walker.
            return None;
        }
    }
    None
}

fn rust_first_block_child(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find(|c| c.kind() == "block");
    found
}

fn rust_loop_self_label_from_node(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "loop_label" {
            continue;
        }
        let mut inner = child.walk();
        for inner_child in child.children(&mut inner) {
            if inner_child.kind() == "identifier" {
                return Some(source[inner_child.byte_range()].to_string());
            }
        }
    }
    None
}

fn rust_break_label_text(node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "loop_label" {
            continue;
        }
        let mut inner = child.walk();
        for inner_child in child.children(&mut inner) {
            if inner_child.kind() == "identifier" {
                return Some(source[inner_child.byte_range()].to_string());
            }
        }
    }
    None
}

fn rust_has_break_targeting_self(
    node: tree_sitter::Node<'_>,
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
            match rust_break_label_text(child, source) {
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

fn rust_expression_diverges(node: tree_sitter::Node<'_>, source: &str) -> Option<&'static str> {
    // Mirrors `unreachable_after_terminator::rust_expression_diverges`
    // closely; returns `Some(kind_str)` when evaluating `node` always
    // diverges. The returned string discriminator is not surfaced in
    // IR — the converter only consumes the `Option<_>` flag.
    match node.kind() {
        "return_expression" => Some("return"),
        "break_expression" => Some("break"),
        "continue_expression" => Some("continue"),
        "macro_invocation" => rust_macro_terminator_name(node, source),
        "block" => rust_block_diverges(node, source),
        "if_expression" => {
            let consequence = node.child_by_field_name("consequence")?;
            let alternative = node.child_by_field_name("alternative")?;
            rust_expression_diverges(consequence, source)?;
            rust_alternative_diverges(alternative, source)?;
            Some("if-branches-diverge")
        }
        "match_expression" => {
            let body = node.child_by_field_name("body")?;
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
        "loop_expression" => {
            let body = rust_first_block_child(node)?;
            let label = rust_loop_self_label_from_node(node, source);
            if rust_has_break_targeting_self(body, label.as_deref(), 0, source) {
                None
            } else {
                Some("loop-no-break")
            }
        }
        _ => None,
    }
}

fn rust_block_diverges(block: tree_sitter::Node<'_>, source: &str) -> Option<&'static str> {
    let mut cursor = block.walk();
    let stmts: Vec<tree_sitter::Node> = block
        .children(&mut cursor)
        .filter(|c| {
            c.is_named()
                && !matches!(
                    c.kind(),
                    "inner_attribute_item"
                        | "attribute_item"
                        | "line_comment"
                        | "block_comment"
                        | "empty_statement"
                )
        })
        .collect();
    if stmts.is_empty() {
        return None;
    }
    for stmt in &stmts {
        let inner = match stmt.kind() {
            "expression_statement" => {
                let mut sc = stmt.walk();
                let found = stmt.children(&mut sc).find(|c| c.is_named());
                found
            }
            _ => Some(*stmt),
        };
        let Some(inner) = inner else { continue };
        if let Some(kind) = rust_expression_diverges(inner, source) {
            return Some(kind);
        }
    }
    None
}

fn rust_alternative_diverges(alt: tree_sitter::Node<'_>, source: &str) -> Option<&'static str> {
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

fn rust_macro_terminator_name(call: tree_sitter::Node<'_>, source: &str) -> Option<&'static str> {
    let macro_node = call.child_by_field_name("macro")?;
    let text = &source[macro_node.byte_range()];
    let last = text.rsplit("::").next().unwrap_or(text);
    static TERMINATORS: &[(&str, &str)] = &[
        ("panic", "panic"),
        ("unreachable", "unreachable"),
        ("todo", "todo"),
        ("unimplemented", "unimplemented"),
        ("abort", "abort"),
        ("exit", "exit"),
    ];
    TERMINATORS
        .iter()
        .find(|(name, _)| *name == last)
        .map(|(_, kind)| *kind)
}

fn rust_macro_terminator_kind(call: tree_sitter::Node<'_>, source: &str) -> Option<DivergentKind> {
    let macro_node = call.child_by_field_name("macro")?;
    let text = &source[macro_node.byte_range()];
    let last = text.rsplit("::").next().unwrap_or(text);
    Some(match last {
        "panic" => DivergentKind::Panic,
        "unreachable" => DivergentKind::Unreachable,
        "todo" => DivergentKind::Todo,
        "unimplemented" => DivergentKind::Unimplemented,
        "abort" => DivergentKind::Abort,
        "exit" => DivergentKind::Exit,
        _ => return None,
    })
}

fn is_rust_assert_macro(call: tree_sitter::Node<'_>, source: &str) -> bool {
    let Some(macro_node) = call.child_by_field_name("macro") else {
        return false;
    };
    let text = &source[macro_node.byte_range()];
    let last = text.rsplit("::").next().unwrap_or(text);
    matches!(last, "assert" | "assert_eq" | "assert_ne" | "debug_assert")
}

// ---------- Normalised tokens (byte-identical with v0.5.x walk_normalize_rust) ----------

fn walk_normalize_rust(node: tree_sitter::Node<'_>, out: &mut Vec<NormalisedToken>) {
    if !node.is_named() {
        return;
    }
    let kind = node.kind();
    if kind == "line_comment" || kind == "block_comment" {
        return;
    }
    let leaf_token = match kind {
        "identifier"
        | "type_identifier"
        | "field_identifier"
        | "shorthand_field_identifier"
        | "scoped_identifier"
        | "scoped_type_identifier" => Some(NormalisedToken::Ident),
        "integer_literal" => Some(NormalisedToken::LitInt),
        "float_literal" => Some(NormalisedToken::LitFloat),
        "string_literal" | "raw_string_literal" => Some(NormalisedToken::LitStr),
        "char_literal" => Some(NormalisedToken::LitChar),
        "boolean_literal" => Some(NormalisedToken::LitBool),
        _ => None,
    };
    if let Some(tok) = leaf_token {
        out.push(tok);
        return;
    }
    out.push(NormalisedToken::Kind(kind));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_normalize_rust(child, out);
    }
}

// ---------- Literal parsing ----------

fn parse_rust_int(text: &str) -> Option<i128> {
    // ir-v0.md §F1: decimal integer parses cleanly via
    // `i128::from_str_radix(_, 10)`; hex / octal / binary literals
    // stay `None`. Strip Rust's `_` separators and any type suffix
    // (`123u32`, `5i64`) before parsing.
    let trimmed = text.trim();
    if trimmed.starts_with("0x")
        || trimmed.starts_with("0X")
        || trimmed.starts_with("0o")
        || trimmed.starts_with("0O")
        || trimmed.starts_with("0b")
        || trimmed.starts_with("0B")
    {
        return None;
    }
    let no_underscore: String = trimmed.chars().filter(|c| *c != '_').collect();
    let digits_end = no_underscore
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(no_underscore.len());
    no_underscore[..digits_end].parse::<i128>().ok()
}

fn rust_string_is_empty(raw: &str) -> bool {
    // Strip the leading `b` / `c` / `r#"` markers and check whether the
    // delimited content is empty.
    let stripped = raw.trim_start_matches(['b', 'c', 'r', '#']);
    if let Some(inner) = stripped.strip_prefix('"').and_then(|s| s.rsplit_once('"')) {
        return inner.0.is_empty();
    }
    false
}

// ---------- Generic helpers ----------

fn node_location(path: &Path, node: tree_sitter::Node<'_>) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        file: path.to_path_buf(),
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

fn empty_block(path: &Path, parent: tree_sitter::Node<'_>) -> IrBlock {
    IrBlock {
        statements: Vec::new(),
        terminator: None,
        normalised_token_count: 0,
        location: node_location(path, parent),
    }
}

/// Force a `&str` returned by `Node::kind()` (already `&'static str`
/// for every grammar cntrdct links against) into the `&'static str`
/// slot `IrStmtKind::Other` / `IrExpr::Other` demand. tree-sitter's
/// signature is `&'static str`, so this is a no-op cast that documents
/// the lifetime expectation at the call site.
fn static_kind_str(s: &'static str) -> &'static str {
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn parse(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::language())
            .expect("set rust language");
        parser.parse(source, None).expect("parse rust")
    }

    fn to_ir(source: &str) -> IrFile {
        let tree = parse(source);
        let provider = RustParserProvider;
        provider
            .to_ir(tree, Arc::from(source), PathBuf::from("a.rs"))
            .expect("to_ir succeeds")
    }

    #[test]
    fn converts_top_level_function() {
        let ir = to_ir("fn foo(a: i32, b: i32) -> i32 { a + b }\n");
        assert_eq!(ir.fns.len(), 1);
        let f = &ir.fns[0];
        assert_eq!(f.name, "foo");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[1].name, "b");
        assert_eq!(f.params[0].kind, ParamKind::Plain);
        assert!(!f.is_method);
        // tree-sitter-rust's `return_type` field child is the type
        // expression itself (`i32`), not the `-> i32` syntactic group.
        // The IR mirrors v0.5.x's `source[rt.byte_range()]` behaviour.
        assert_eq!(f.return_type_text.as_deref(), Some("i32"));
    }

    #[test]
    fn marks_impl_methods_as_methods_with_receiver() {
        let ir = to_ir("struct S;\nimpl S {\n    fn bar(&self, x: i32) {}\n    fn baz() {}\n}\n");
        let methods: Vec<&IrFn> = ir.fns.iter().filter(|f| f.is_method).collect();
        assert_eq!(methods.len(), 2);
        let bar = methods.iter().find(|f| f.name == "bar").unwrap();
        assert!(matches!(bar.params[0].kind, ParamKind::Receiver));
        assert_eq!(bar.params[1].name, "x");
    }

    #[test]
    fn leading_doc_strips_triple_slash() {
        let ir = to_ir("/// returns result\n/// of the computation\nfn foo() {}\n");
        assert_eq!(
            ir.fns[0].leading_doc.as_deref(),
            Some("returns result\nof the computation")
        );
    }

    #[test]
    fn decorators_capture_outer_attributes() {
        let ir = to_ir("#[deprecated]\n#[inline]\nfn foo() {}\n");
        let decs = &ir.fns[0].decorators;
        assert_eq!(decs.len(), 2);
        assert_eq!(decs[0].name_path, vec!["deprecated".to_string()]);
        assert_eq!(decs[1].name_path, vec!["inline".to_string()]);
    }

    #[test]
    fn top_level_comments_capture_free_standing_comments() {
        let ir = to_ir("// freestanding\nfn foo() {}\n");
        // The `// freestanding` is not a `///` doc comment, so it
        // stays in top_level_comments rather than fn.leading_doc.
        assert_eq!(ir.fns[0].leading_doc, None);
        assert_eq!(ir.top_level_comments.len(), 1);
        assert_eq!(ir.top_level_comments[0].kind, IrCommentKind::RustLine);
    }

    #[test]
    fn normalised_tokens_are_byte_identical_with_v0_walk() {
        // The IR walk must produce the same NormalisedToken sequence
        // (modulo NormalisedToken::Kind vs string-form) as v0.5.x's
        // walk_normalize_rust(function_item). Spot-check via a small fn.
        let ir = to_ir("fn foo(a: i32) -> i32 { let x = 1; x + 2 }\n");
        let toks = &ir.fns[0].normalised_tokens;
        // Function-item-rooted: starts with `Kind("function_item")` and
        // contains the two int literal placeholders.
        assert!(matches!(toks[0], NormalisedToken::Kind("function_item")));
        assert!(toks.iter().any(|t| matches!(t, NormalisedToken::LitInt)));
    }

    #[test]
    fn terminator_panic_macro_marks_div_call() {
        let ir = to_ir("fn foo() { panic!(); foo(); }\n");
        let body = &ir.fns[0].body;
        assert!(matches!(
            body.terminator,
            Some(IrTerminator::DivergentCall {
                kind: DivergentKind::Panic
            })
        ));
    }

    #[test]
    fn terminator_if_branches_diverge() {
        let ir = to_ir("fn foo() {\n    if true { return; } else { panic!(); }\n    other();\n}\n");
        let body = &ir.fns[0].body;
        assert!(matches!(
            body.terminator,
            Some(IrTerminator::BranchMerge {
                kind: BranchMergeKind::IfBranchesDiverge
            })
        ));
    }

    #[test]
    fn loop_no_break_marks_terminator() {
        let ir = to_ir("fn foo() { loop {} unreachable_after(); }\n");
        let body = &ir.fns[0].body;
        assert!(matches!(body.terminator, Some(IrTerminator::LoopNoBreak)));
    }

    #[test]
    fn call_site_records_callee_and_arg_count() {
        let ir = to_ir("fn foo() { bar(1, 2); }\n");
        let stmts = &ir.fns[0].body.statements;
        assert_eq!(stmts.len(), 1);
        match &stmts[0].kind {
            IrStmtKind::Call(call) => {
                assert_eq!(call.callee.segments, vec!["bar".to_string()]);
                assert_eq!(call.args.len(), 2);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn hoisted_function_item_in_block() {
        let ir = to_ir("fn outer() { fn inner() {} inner(); }\n");
        let stmts = &ir.fns[0].body.statements;
        // First statement must be the hoisted inner function.
        assert!(matches!(
            stmts[0].kind,
            IrStmtKind::HoistedItem {
                kind: HoistedItemKind::Function,
                ..
            }
        ));
    }

    #[test]
    fn cfg_attribute_attaches_to_statement() {
        let ir = to_ir("fn foo() {\n    #[cfg(unix)]\n    return;\n}\n");
        let stmts = &ir.fns[0].body.statements;
        assert_eq!(stmts.len(), 1);
        // The `#[cfg(unix)]` attribute precedes the return; it must
        // be carried on the `return` statement's `attributes`.
        assert_eq!(stmts[0].attributes.len(), 1);
        assert_eq!(stmts[0].attributes[0].name_path, vec!["cfg".to_string()]);
    }

    #[test]
    fn parse_recovered_still_returns_shell() {
        // Broken Rust source — to_ir still produces an IrFile with
        // parse_recovered=true and (best-effort) any fns it could
        // recover.
        let tree = parse("fn foo( {\n");
        let provider = RustParserProvider;
        let ir = provider
            .to_ir(tree, Arc::from("fn foo( {\n"), PathBuf::from("a.rs"))
            .expect("to_ir succeeds on partial parse");
        assert!(ir.parse_recovered);
    }

    #[test]
    fn location_invariant_holds_for_call_site() {
        let source = "fn foo() {\n    bar(x, y);\n}\n";
        let ir = to_ir(source);
        let stmts = &ir.fns[0].body.statements;
        let call = match &stmts[0].kind {
            IrStmtKind::Call(c) => c,
            _ => panic!(),
        };
        // tree-sitter places `bar(x, y)` on row 1 (0-based) at col 4
        // (0-based). The IR Location is 1-based; the call sits on
        // line 2, col 5.
        assert_eq!(call.location.start_line, 2);
        assert_eq!(call.location.start_col, 5);
        // Byte range pinned per F3.
        let raw =
            &source.as_bytes()[call.location.start_byte as usize..call.location.end_byte as usize];
        assert_eq!(std::str::from_utf8(raw).unwrap(), "bar(x, y)");
    }
}
