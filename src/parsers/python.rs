//! Python tree-sitter provider + [`crate::ir::IrFile`] converter.
//!
//! Spec: `docs/spec/ir-v0.md` §F1, §F2, §F3, §F4e. The converter walks
//! the Python tree-sitter AST emitted by `tree_sitter_python::language()`
//! and materialises the IR nodes the cross-cutting detectors consume.
//! Per ir-v0.md §F2, `to_ir` is total over recognised shapes: unknown
//! statement / expression nodes fall back to
//! [`crate::ir::IrStmtKind::Other`] / [`crate::ir::IrExpr::Other`] with
//! the tree-sitter `Node::kind()` discriminator + a [`NodeRef`] for
//! raw-tree recovery. [`crate::ir::IrConvertError::StructuralInvariant`]
//! is reserved for invariants we expect tree-sitter-python to honour;
//! these are programmer errors per ir-v0.md §F2 / R9 and should never
//! fire on real source.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{build_ir_shell, Language, ParserProvider};
use crate::ir::{
    BranchMergeKind, ConstantBranchKind, DivergentKind, IrBlock, IrCallSite, IrComment,
    IrCommentKind, IrConvertError, IrDecorator, IrExpr, IrFile, IrFn, IrIfStmt, IrLiteral, IrParam,
    IrPath, IrStmt, IrStmtKind, IrTerminator, IrWhileStmt, IrWithStmt, Location, NodeRef,
    NormalisedToken, ParamKind,
};

/// Provider for Python source (`*.py`, `*.pyi`).
pub struct PythonParserProvider;

impl ParserProvider for PythonParserProvider {
    fn language(&self) -> Language {
        Language::Python
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_python::language()
    }

    fn to_ir(
        &self,
        tree: tree_sitter::Tree,
        source: Arc<str>,
        path: PathBuf,
    ) -> Result<IrFile, IrConvertError> {
        let mut shell = build_ir_shell(self, tree, source, path)?;
        let (fns, top_level_comments) = {
            let cv = Converter {
                source: shell.source.as_ref(),
                path: shell.path.as_path(),
            };
            cv.convert_root(shell.raw_tree.root_node())?
        };
        shell.fns = fns;
        shell.top_level_comments = top_level_comments;
        Ok(shell)
    }
}

// ---------- Python exit call classification ----------

fn python_exit_kind(text: &str) -> Option<DivergentKind> {
    match text.trim() {
        "sys.exit" => Some(DivergentKind::SysExit),
        "sys.abort" => Some(DivergentKind::SysAbort),
        "os._exit" => Some(DivergentKind::OsExit),
        "exit" => Some(DivergentKind::ExitBuiltin),
        "quit" => Some(DivergentKind::QuitBuiltin),
        _ => None,
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
        for child in root.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    let f = self.convert_function(child, &[], false)?;
                    fns.push(f);
                }
                "decorated_definition" => {
                    if let Some(f) = self.convert_decorated_function(child, false)? {
                        fns.push(f);
                    }
                }
                "class_definition" => {
                    self.collect_class_methods(child, &mut fns)?;
                }
                "comment" => {
                    if let Some(c) = convert_python_comment(child, self.source, self.path) {
                        top_level_comments.push(c);
                    }
                }
                _ => {}
            }
        }
        Ok((fns, top_level_comments))
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
        for child in body.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    let f = self.convert_function(child, &[], true)?;
                    out.push(f);
                }
                "decorated_definition" => {
                    if let Some(f) = self.convert_decorated_function(child, true)? {
                        out.push(f);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn convert_decorated_function(
        &self,
        decorated: tree_sitter::Node<'a>,
        is_method: bool,
    ) -> Result<Option<IrFn>, IrConvertError> {
        let mut cursor = decorated.walk();
        let kids: Vec<tree_sitter::Node> = decorated.children(&mut cursor).collect();
        let decorators_nodes: Vec<tree_sitter::Node> = kids
            .iter()
            .filter(|c| c.kind() == "decorator")
            .copied()
            .collect();
        let Some(fn_def) = kids.iter().find(|c| c.kind() == "function_definition") else {
            return Ok(None);
        };
        let f = self.convert_function(*fn_def, &decorators_nodes, is_method)?;
        Ok(Some(f))
    }

    fn convert_function(
        &self,
        node: tree_sitter::Node<'a>,
        decorator_nodes: &[tree_sitter::Node<'a>],
        is_method: bool,
    ) -> Result<IrFn, IrConvertError> {
        let name_node = node.child_by_field_name("name").ok_or_else(|| {
            IrConvertError::StructuralInvariant {
                kind: "function_definition",
                message: "missing `name` field".to_string(),
            }
        })?;
        let name = self.text(name_node).to_string();

        let params = match node.child_by_field_name("parameters") {
            Some(params_node) => self.convert_python_params(params_node, is_method),
            None => Vec::new(),
        };

        let return_type_text = node
            .child_by_field_name("return_type")
            .map(|n| self.text(n).to_string());

        let body = match node.child_by_field_name("body") {
            Some(b) => self.convert_python_block(b),
            None => empty_block(self.path, node),
        };

        let leading_doc = extract_python_docstring(&body, self.source, node);

        let decorators: Vec<IrDecorator> = decorator_nodes
            .iter()
            .filter_map(|d| convert_python_decorator(*d, self.source, self.path))
            .collect();

        Ok(IrFn {
            name,
            params,
            body,
            return_type_text,
            decorators,
            is_method,
            leading_doc,
            location: node_location(self.path, node),
        })
    }

    fn convert_python_params(
        &self,
        params_node: tree_sitter::Node<'a>,
        is_method: bool,
    ) -> Vec<IrParam> {
        let mut out = Vec::new();
        let mut cursor = params_node.walk();
        let mut first = true;
        for child in params_node.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            let (name, kind) = match child.kind() {
                "identifier" => {
                    let text = self.text(child).to_string();
                    if is_method && first && matches!(text.as_str(), "self" | "cls") {
                        (text, ParamKind::Receiver)
                    } else {
                        (text, ParamKind::Plain)
                    }
                }
                "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                    let id = python_first_identifier(child);
                    let text = id
                        .map(|n| self.text(n).to_string())
                        .unwrap_or_else(|| self.text(child).to_string());
                    if is_method && first && matches!(text.as_str(), "self" | "cls") {
                        (text, ParamKind::Receiver)
                    } else {
                        (text, ParamKind::Plain)
                    }
                }
                // `*args` / `**kwargs` / positional-only / keyword-only
                // separators and any other shape the converter cannot
                // model land as Unsupported. arg-swap rejects the
                // entire function definition when any param is
                // Unsupported, matching v0.5.x conservatism.
                _ => (self.text(child).to_string(), ParamKind::Unsupported),
            };
            out.push(IrParam {
                name,
                kind,
                location: node_location(self.path, child),
            });
            first = false;
        }
        out
    }

    fn convert_python_block(&self, block: tree_sitter::Node<'a>) -> IrBlock {
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
            let kind = self.classify_python_stmt(*child);
            statements.push(IrStmt {
                kind,
                // Python attaches decorators to function / class
                // definitions, not arbitrary statements; per-statement
                // contexts therefore carry an empty `attributes`
                // vector per ir-v0.md §F1.
                attributes: Vec::new(),
                location: node_location(self.path, *child),
            });
        }

        let terminator = compute_python_block_terminator(&statements);
        let mut normalised_tokens: Vec<NormalisedToken> = Vec::new();
        walk_normalize_python(block, &mut normalised_tokens);

        IrBlock {
            statements,
            terminator,
            normalised_tokens,
            location: node_location(self.path, block),
        }
    }

    fn classify_python_stmt(&self, node: tree_sitter::Node<'a>) -> IrStmtKind {
        match node.kind() {
            "return_statement" => IrStmtKind::Return(self.first_named_child_as_expr(node)),
            "raise_statement" => IrStmtKind::Raise(self.first_named_child_as_expr(node)),
            "break_statement" => IrStmtKind::Break(None),
            "continue_statement" => IrStmtKind::Continue(None),
            "assert_statement" => {
                let cond = self
                    .first_named_child_as_expr(node)
                    .unwrap_or(IrExpr::Other {
                        node_kind: "assert_statement",
                        node_ref: node_ref(node),
                    });
                IrStmtKind::Assert(cond)
            }
            "if_statement" => IrStmtKind::If(self.convert_python_if(node)),
            "while_statement" => IrStmtKind::While(self.convert_python_while(node)),
            "with_statement" => IrStmtKind::With(self.convert_python_with(node)),
            "expression_statement" => {
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                match inner {
                    Some(inner) if inner.kind() == "call" => {
                        if let Some(kind) = self.python_exit_kind_for_call(inner) {
                            IrStmtKind::DivergentCall {
                                kind,
                                args: self.convert_python_call_args(inner),
                            }
                        } else {
                            match self.convert_python_call_site(inner) {
                                Some(call) => IrStmtKind::Call(call),
                                None => IrStmtKind::Other {
                                    node_kind: "call",
                                    node_ref: node_ref(inner),
                                },
                            }
                        }
                    }
                    Some(inner) => IrStmtKind::Other {
                        node_kind: static_kind_str(inner.kind()),
                        node_ref: node_ref(inner),
                    },
                    None => IrStmtKind::Other {
                        node_kind: "expression_statement",
                        node_ref: node_ref(node),
                    },
                }
            }
            other => IrStmtKind::Other {
                node_kind: static_kind_str(other),
                node_ref: node_ref(node),
            },
        }
    }

    fn first_named_child_as_expr(&self, node: tree_sitter::Node<'a>) -> Option<IrExpr> {
        let mut cursor = node.walk();
        let child = node.children(&mut cursor).find(|c| c.is_named());
        child.map(|c| self.convert_python_expr(c))
    }

    fn convert_python_if(&self, node: tree_sitter::Node<'a>) -> IrIfStmt {
        let condition = match node.child_by_field_name("condition") {
            Some(c) => self.convert_python_expr(c),
            None => IrExpr::Other {
                node_kind: "missing_condition",
                node_ref: node_ref(node),
            },
        };
        let consequence = match node.child_by_field_name("consequence") {
            Some(b) => self.convert_python_block(b),
            None => empty_block(self.path, node),
        };
        // The else_clause (if present) wraps a `block` body.
        let alternative = python_find_else_block(node).map(|b| self.convert_python_block(b));

        // F4e Python constant-condition terminator. The IR layer
        // surfaces ConstantBranchUnreachable here so detectors do not
        // re-parse the condition at scan time.
        let cond_constant = node
            .child_by_field_name("condition")
            .and_then(|c| python_constant_condition(c, self.source));
        let terminator = match cond_constant {
            Some(false) => Some(IrTerminator::ConstantBranchUnreachable {
                kind: ConstantBranchKind::ConstantFalseIf,
            }),
            Some(true) if alternative.is_some() => Some(IrTerminator::ConstantBranchUnreachable {
                kind: ConstantBranchKind::ConstantTrueIfElse,
            }),
            _ => self.python_if_branch_merge(node),
        };

        IrIfStmt {
            condition,
            consequence,
            alternative,
            terminator,
            location: node_location(self.path, node),
        }
    }

    fn convert_python_while(&self, node: tree_sitter::Node<'a>) -> IrWhileStmt {
        let condition = match node.child_by_field_name("condition") {
            Some(c) => self.convert_python_expr(c),
            None => IrExpr::Other {
                node_kind: "missing_condition",
                node_ref: node_ref(node),
            },
        };
        let body = match node.child_by_field_name("body") {
            Some(b) => self.convert_python_block(b),
            None => empty_block(self.path, node),
        };
        IrWhileStmt {
            condition,
            body,
            location: node_location(self.path, node),
        }
    }

    fn convert_python_with(&self, node: tree_sitter::Node<'a>) -> IrWithStmt {
        let mut context_managers: Vec<IrExpr> = Vec::new();
        // tree-sitter-python: with_statement → with_clause → with_item.
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "with_clause" {
                continue;
            }
            let mut inner = child.walk();
            for item in child.children(&mut inner) {
                if item.kind() != "with_item" {
                    continue;
                }
                if let Some(value) = item.child_by_field_name("value") {
                    let expr_node = match value.kind() {
                        "as_pattern" => {
                            // `expr as name` — take the leading expr.
                            let mut ac = value.walk();
                            let found = value.children(&mut ac).find(|c| c.is_named());
                            found.unwrap_or(value)
                        }
                        _ => value,
                    };
                    context_managers.push(self.convert_python_expr(expr_node));
                } else {
                    // Fall back to the first named child of the
                    // with_item itself when no `value` field exists.
                    let mut ic = item.walk();
                    let found = item.children(&mut ic).find(|c| c.is_named());
                    if let Some(c) = found {
                        context_managers.push(self.convert_python_expr(c));
                    }
                }
            }
        }
        let body = match node.child_by_field_name("body") {
            Some(b) => self.convert_python_block(b),
            None => empty_block(self.path, node),
        };
        IrWithStmt {
            context_managers,
            body,
            location: node_location(self.path, node),
        }
    }

    fn convert_python_expr(&self, node: tree_sitter::Node<'a>) -> IrExpr {
        match node.kind() {
            "identifier" => IrExpr::Ident(self.text(node).to_string()),
            "attribute" => IrExpr::Path(self.convert_python_path(node)),
            "true" => IrExpr::Literal(IrLiteral::Bool(true)),
            "false" => IrExpr::Literal(IrLiteral::Bool(false)),
            "none" => IrExpr::Literal(IrLiteral::None),
            "integer" => IrExpr::Literal(IrLiteral::Int(parse_python_int(self.text(node)))),
            "float" => IrExpr::Literal(IrLiteral::Float),
            "string" => IrExpr::Literal(IrLiteral::String {
                is_empty: python_string_is_empty(node),
            }),
            "call" => match self.convert_python_call_site(node) {
                Some(call) => {
                    // Treat exit-family calls as DivergentCall expressions
                    // so detectors can pattern-match them uniformly.
                    if let Some(kind) = self.python_exit_kind_for_call(node) {
                        IrExpr::DivergentCall {
                            kind,
                            args: self.convert_python_call_args(node),
                        }
                    } else {
                        IrExpr::Call(Box::new(call))
                    }
                }
                None => IrExpr::Other {
                    node_kind: "call",
                    node_ref: node_ref(node),
                },
            },
            "parenthesized_expression" => {
                let mut cursor = node.walk();
                let inner = node.children(&mut cursor).find(|c| c.is_named());
                inner
                    .map(|c| self.convert_python_expr(c))
                    .unwrap_or_else(|| IrExpr::Other {
                        node_kind: "parenthesized_expression",
                        node_ref: node_ref(node),
                    })
            }
            other => IrExpr::Other {
                node_kind: static_kind_str(other),
                node_ref: node_ref(node),
            },
        }
    }

    fn convert_python_path(&self, node: tree_sitter::Node<'a>) -> IrPath {
        // Walk `attribute` chains to populate receiver / segments per
        // ir-v0.md §F1. `self.method` → receiver=["self"], segments=["method"].
        // `obj.attr.method` → receiver=["obj","attr"], segments=["method"].
        let raw = self.text(node).to_string();
        let mut chain: Vec<String> = Vec::new();
        python_collect_attribute_chain(node, self.source, &mut chain);
        let (receiver, segments) = if chain.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            let last = chain.pop().expect("non-empty chain");
            (chain, vec![last])
        };
        IrPath {
            receiver,
            segments,
            raw,
        }
    }

    fn convert_python_call_site(&self, node: tree_sitter::Node<'a>) -> Option<IrCallSite> {
        let function = node.child_by_field_name("function")?;
        let callee = match function.kind() {
            "identifier" => IrPath {
                receiver: Vec::new(),
                segments: vec![self.text(function).to_string()],
                raw: self.text(function).to_string(),
            },
            "attribute" => self.convert_python_path(function),
            _ => IrPath {
                receiver: Vec::new(),
                segments: Vec::new(),
                raw: self.text(function).to_string(),
            },
        };
        let args = self.convert_python_call_args(node);
        Some(IrCallSite {
            callee,
            args,
            location: node_location(self.path, node),
        })
    }

    fn convert_python_call_args(&self, call: tree_sitter::Node<'a>) -> Vec<IrExpr> {
        let mut args: Vec<IrExpr> = Vec::new();
        let Some(arguments) = call.child_by_field_name("arguments") else {
            return args;
        };
        let mut cursor = arguments.walk();
        for child in arguments.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            args.push(self.convert_python_expr(child));
        }
        args
    }

    fn python_exit_kind_for_call(&self, call: tree_sitter::Node<'a>) -> Option<DivergentKind> {
        let function = call.child_by_field_name("function")?;
        let text = self.text(function);
        python_exit_kind(text)
    }

    fn python_if_branch_merge(&self, node: tree_sitter::Node<'a>) -> Option<IrTerminator> {
        let consequence = node.child_by_field_name("consequence")?;
        let alternative = python_find_else_block(node)?;
        python_block_diverges(consequence, self.source)?;
        python_block_diverges(alternative, self.source)?;
        Some(IrTerminator::BranchMerge {
            kind: BranchMergeKind::IfBranchesDiverge,
        })
    }

    fn text(&self, node: tree_sitter::Node<'a>) -> &'a str {
        &self.source[node.byte_range()]
    }
}

// ---------- IrStmt → IrTerminator block-level merge ----------

fn compute_python_block_terminator(statements: &[IrStmt]) -> Option<IrTerminator> {
    for stmt in statements {
        if let Some(t) = python_stmt_terminator(stmt) {
            return Some(t);
        }
    }
    None
}

fn python_stmt_terminator(stmt: &IrStmt) -> Option<IrTerminator> {
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
        IrStmtKind::Match(stmt) => stmt.terminator,
        _ => None,
    }
}

// ---------- Python control-flow helpers ----------

fn python_find_else_block(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "else_clause" {
            continue;
        }
        let mut inner = child.walk();
        let found = child.children(&mut inner).find(|c| c.kind() == "block");
        if found.is_some() {
            return found;
        }
    }
    None
}

fn python_block_diverges(block: tree_sitter::Node<'_>, source: &str) -> Option<&'static str> {
    let mut cursor = block.walk();
    let stmts: Vec<tree_sitter::Node> = block
        .children(&mut cursor)
        .filter(|c| c.is_named() && c.kind() != "comment")
        .collect();
    if stmts.is_empty() {
        return None;
    }
    for stmt in &stmts {
        if let Some(kind) = python_stmt_diverges(*stmt, source) {
            return Some(kind);
        }
    }
    None
}

fn python_stmt_diverges(stmt: tree_sitter::Node<'_>, source: &str) -> Option<&'static str> {
    match stmt.kind() {
        "return_statement" => Some("return"),
        "raise_statement" => Some("raise"),
        "break_statement" => Some("break"),
        "continue_statement" => Some("continue"),
        "assert_statement" => {
            let mut cursor = stmt.walk();
            let cond = stmt.children(&mut cursor).find(|c| c.is_named())?;
            if cond.kind() == "false" {
                Some("assert")
            } else {
                None
            }
        }
        "expression_statement" => {
            let mut cursor = stmt.walk();
            let inner = stmt.children(&mut cursor).find(|c| c.is_named())?;
            if inner.kind() != "call" {
                return None;
            }
            let func = inner.child_by_field_name("function")?;
            let text = &source[func.byte_range()];
            python_exit_kind(text).map(|_| "exit-call")
        }
        _ => None,
    }
}

// ---------- F4e constant-condition classifier ----------

fn python_constant_condition(node: tree_sitter::Node<'_>, source: &str) -> Option<bool> {
    match node.kind() {
        "false" => Some(false),
        "true" => Some(true),
        "none" => Some(false),
        "integer" => {
            let text = &source[node.byte_range()];
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
            let mut cursor = node.walk();
            let has_content = node
                .children(&mut cursor)
                .any(|c| c.kind() == "string_content");
            Some(has_content)
        }
        _ => None,
    }
}

// ---------- Python helpers ----------

fn python_first_identifier(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find(|c| c.kind() == "identifier");
    found
}

fn python_collect_attribute_chain(
    node: tree_sitter::Node<'_>,
    source: &str,
    out: &mut Vec<String>,
) {
    match node.kind() {
        "identifier" => out.push(source[node.byte_range()].to_string()),
        "attribute" => {
            if let Some(object) = node.child_by_field_name("object") {
                python_collect_attribute_chain(object, source, out);
            }
            if let Some(attr) = node.child_by_field_name("attribute") {
                out.push(source[attr.byte_range()].to_string());
            }
        }
        _ => {
            // Fall back to raw text as the only segment so consumers
            // that care about the last segment still see something.
            out.push(source[node.byte_range()].to_string());
        }
    }
}

fn python_string_is_empty(node: tree_sitter::Node<'_>) -> bool {
    // tree-sitter-python `string` wraps `string_start`, optional
    // `string_content`, and `string_end`. The literal is empty iff
    // no `string_content` named child exists between the delimiters.
    let mut cursor = node.walk();
    let has_content = node
        .children(&mut cursor)
        .any(|c| c.kind() == "string_content");
    !has_content
}

fn parse_python_int(text: &str) -> Option<i128> {
    // ir-v0.md §F1: decimal integer parses cleanly via
    // `i128::from_str_radix(_, 10)`; hex / octal / binary literals
    // stay `None`.
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
    no_underscore.parse::<i128>().ok()
}

// ---------- Python decorator / docstring extraction ----------

fn convert_python_decorator(
    node: tree_sitter::Node<'_>,
    source: &str,
    path: &Path,
) -> Option<IrDecorator> {
    let raw = source[node.byte_range()].to_string();
    let stripped = raw
        .trim_start()
        .strip_prefix('@')
        .unwrap_or(raw.as_str())
        .trim_start();
    // The decorator name path runs up to the first `(` or whitespace.
    let head_end = stripped
        .find(|c: char| c == '(' || c.is_whitespace())
        .unwrap_or(stripped.len());
    let head = &stripped[..head_end];
    let name_path: Vec<String> = head
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

fn extract_python_docstring(
    body: &IrBlock,
    source: &str,
    fn_node: tree_sitter::Node<'_>,
) -> Option<String> {
    // The docstring is the first statement of the function body when
    // it is a bare string literal. We re-walk via the raw tree to
    // recover the source text — IR's IrStmt does not carry the raw
    // bytes for `Other` nodes.
    let body_node = fn_node.child_by_field_name("body")?;
    let mut cursor = body_node.walk();
    let first = body_node.children(&mut cursor).find(|c| c.is_named())?;
    if first.kind() != "expression_statement" {
        return None;
    }
    let mut inner = first.walk();
    let string_node = first.children(&mut inner).find(|c| c.is_named())?;
    if string_node.kind() != "string" {
        return None;
    }
    let raw = string_node.utf8_text(source.as_bytes()).ok()?;
    let stripped = strip_python_string_quotes(raw);
    // Make sure the docstring actually lives at the leading position
    // (i.e. the first IR statement is `Other { node_kind: "string"
    // }` -- a sanity check).
    let _ = body; // signal to keep param for future use
    Some(stripped)
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

fn convert_python_comment(
    node: tree_sitter::Node<'_>,
    source: &str,
    path: &Path,
) -> Option<IrComment> {
    if node.kind() != "comment" {
        return None;
    }
    let raw = &source[node.byte_range()];
    let text = raw
        .strip_prefix('#')
        .map(|s| s.strip_prefix(' ').unwrap_or(s))
        .unwrap_or(raw)
        .trim_end_matches('\n')
        .to_string();
    Some(IrComment {
        kind: IrCommentKind::PythonComment,
        text,
        target: None,
        location: node_location(path, node),
    })
}

// ---------- Normalised tokens (byte-identical with v0.5.x walk_normalize_python) ----------

fn walk_normalize_python(node: tree_sitter::Node<'_>, out: &mut Vec<NormalisedToken>) {
    if !node.is_named() {
        return;
    }
    let kind = node.kind();
    if kind == "comment" {
        return;
    }
    let leaf_token = match kind {
        "identifier" => Some(NormalisedToken::Ident),
        "integer" => Some(NormalisedToken::LitInt),
        "float" => Some(NormalisedToken::LitFloat),
        "string" => Some(NormalisedToken::LitStr),
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
        walk_normalize_python(child, out);
    }
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
        normalised_tokens: Vec::new(),
        location: node_location(path, parent),
    }
}

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
            .set_language(&tree_sitter_python::language())
            .expect("set python language");
        parser.parse(source, None).expect("parse python")
    }

    fn to_ir(source: &str) -> IrFile {
        let tree = parse(source);
        let provider = PythonParserProvider;
        provider
            .to_ir(tree, Arc::from(source), PathBuf::from("a.py"))
            .expect("to_ir succeeds")
    }

    #[test]
    fn converts_top_level_function() {
        let ir = to_ir("def foo(a, b):\n    return a + b\n");
        assert_eq!(ir.fns.len(), 1);
        let f = &ir.fns[0];
        assert_eq!(f.name, "foo");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].kind, ParamKind::Plain);
        assert!(!f.is_method);
    }

    #[test]
    fn marks_class_methods_with_receiver() {
        let src = "\
class C:
    def foo(self, x):
        pass
    @classmethod
    def bar(cls):
        pass
";
        let ir = to_ir(src);
        let methods: Vec<&IrFn> = ir.fns.iter().filter(|f| f.is_method).collect();
        assert_eq!(methods.len(), 2);
        let foo = methods.iter().find(|f| f.name == "foo").unwrap();
        assert_eq!(foo.params[0].kind, ParamKind::Receiver);
        let bar = methods.iter().find(|f| f.name == "bar").unwrap();
        assert_eq!(bar.params[0].kind, ParamKind::Receiver);
        assert_eq!(bar.decorators.len(), 1);
        assert_eq!(bar.decorators[0].name_path, vec!["classmethod".to_string()]);
    }

    #[test]
    fn docstring_is_extracted_as_leading_doc() {
        let src = "def foo():\n    \"\"\"docstring here\"\"\"\n    pass\n";
        let ir = to_ir(src);
        assert_eq!(ir.fns[0].leading_doc.as_deref(), Some("docstring here"));
    }

    #[test]
    fn decorator_name_path_handles_dotted_form() {
        let src = "@warnings.deprecated\ndef foo():\n    pass\n";
        let ir = to_ir(src);
        let decs = &ir.fns[0].decorators;
        assert_eq!(decs.len(), 1);
        assert_eq!(
            decs[0].name_path,
            vec!["warnings".to_string(), "deprecated".to_string()]
        );
    }

    #[test]
    fn self_method_call_has_receiver_self() {
        let src = "def foo(self):\n    self.bar(1)\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::Call(call) => {
                assert_eq!(call.callee.receiver, vec!["self".to_string()]);
                assert_eq!(call.callee.segments, vec!["bar".to_string()]);
                assert_eq!(call.args.len(), 1);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn nested_attribute_call_has_multi_segment_receiver() {
        let src = "def foo():\n    obj.attr.method(x)\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::Call(call) => {
                assert_eq!(
                    call.callee.receiver,
                    vec!["obj".to_string(), "attr".to_string()]
                );
                assert_eq!(call.callee.segments, vec!["method".to_string()]);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn sys_exit_emits_divergent_call_stmt() {
        let src = "def foo():\n    sys.exit(1)\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        assert!(matches!(
            stmts[0].kind,
            IrStmtKind::DivergentCall {
                kind: DivergentKind::SysExit,
                ..
            }
        ));
        assert!(matches!(
            ir.fns[0].body.terminator,
            Some(IrTerminator::DivergentCall {
                kind: DivergentKind::SysExit
            })
        ));
    }

    #[test]
    fn raise_marks_terminator() {
        let src = "def foo():\n    raise ValueError('x')\n";
        let ir = to_ir(src);
        assert!(matches!(
            ir.fns[0].body.terminator,
            Some(IrTerminator::Raise)
        ));
    }

    #[test]
    fn if_false_marks_constant_branch_unreachable() {
        let src = "def foo():\n    if False:\n        return 1\n    return 2\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::If(if_stmt) => {
                assert!(matches!(
                    if_stmt.terminator,
                    Some(IrTerminator::ConstantBranchUnreachable {
                        kind: ConstantBranchKind::ConstantFalseIf
                    })
                ));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn while_unmodified_terminator_is_none() {
        // ConstantFalseWhile is surfaced as an IrTerminator variant
        // but it lives on the inner While body for the F4e detector,
        // not on the IrWhileStmt's body. The outer block terminator
        // is therefore None.
        let src = "def foo():\n    while False:\n        return 1\n";
        let ir = to_ir(src);
        assert!(ir.fns[0].body.terminator.is_none());
    }

    #[test]
    fn with_statement_records_context_managers() {
        let src = "def foo():\n    with open('p') as fp:\n        fp.read()\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::With(with) => {
                assert_eq!(with.context_managers.len(), 1);
                assert_eq!(with.body.statements.len(), 1);
            }
            other => panic!("expected With, got {other:?}"),
        }
    }

    #[test]
    fn star_args_param_is_unsupported() {
        let src = "def foo(a, *args, **kwargs):\n    pass\n";
        let ir = to_ir(src);
        let f = &ir.fns[0];
        assert_eq!(f.params.len(), 3);
        assert_eq!(f.params[0].kind, ParamKind::Plain);
        assert_eq!(f.params[1].kind, ParamKind::Unsupported);
        assert_eq!(f.params[2].kind, ParamKind::Unsupported);
    }

    #[test]
    fn integer_literal_parses_decimal_only() {
        let src = "def foo():\n    return 42\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::Return(Some(IrExpr::Literal(IrLiteral::Int(Some(42))))) => {}
            other => panic!("unexpected: {other:?}"),
        }

        let src = "def foo():\n    return 0xff\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::Return(Some(IrExpr::Literal(IrLiteral::Int(None)))) => {}
            other => panic!("hex literal must produce Int(None), got {other:?}"),
        }
    }

    #[test]
    fn empty_string_literal_marks_is_empty_true() {
        let src = "def foo():\n    return ''\n";
        let ir = to_ir(src);
        let stmts = &ir.fns[0].body.statements;
        match &stmts[0].kind {
            IrStmtKind::Return(Some(IrExpr::Literal(IrLiteral::String { is_empty }))) => {
                assert!(*is_empty);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn location_invariant_holds_for_call_site() {
        let source = "def foo():\n    bar(x, y)\n";
        let ir = to_ir(source);
        let stmts = &ir.fns[0].body.statements;
        let call = match &stmts[0].kind {
            IrStmtKind::Call(c) => c,
            _ => panic!(),
        };
        assert_eq!(call.location.start_line, 2);
        assert_eq!(call.location.start_col, 5);
        let raw =
            &source.as_bytes()[call.location.start_byte as usize..call.location.end_byte as usize];
        assert_eq!(std::str::from_utf8(raw).unwrap(), "bar(x, y)");
    }
}
