//! ir-v0.md §F6 T5 / §F3 — Location equality per IR node kind.
//!
//! For every IR-bearing node kind, the IR `Location` 6 fields must
//! equal `tree_sitter::Node::{start_position(), end_position(),
//! start_byte(), end_byte()}` plus the +1 line/column offset.
//!
//! Coverage matrix (per §F3):
//!
//! - `IrFn`, `IrParam`, `IrDecorator` — top-level functions, class
//!   methods (Python), impl methods (Rust), decorators / attributes.
//! - `IrBlock`, `IrComment` — function bodies, leading docs / line
//!   comments.
//! - `IrCallSite` — nested calls.
//! - `IrIfStmt`, `IrWhileStmt`, `IrLoopStmt`, `IrMatchStmt`,
//!   `IrWithStmt` — nested control-flow forms (Rust + Python where
//!   applicable).
//! - `IrStmtKind::Other`, `IrExpr::Other` — escape hatch retains
//!   correct location for raw-tree consumers.

use std::path::PathBuf;
use std::sync::Arc;

use cntrdct::ir::{IrBlock, IrExpr, IrFile, IrStmtKind};
use cntrdct::parsers::{parser_for, Language};

fn to_ir(lang: Language, source: &str, path: &str) -> (tree_sitter::Tree, IrFile) {
    let provider = parser_for(lang);
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&provider.ts_language())
        .expect("set language");
    let tree = parser.parse(source, None).expect("parse");
    // Build a second tree for tree-sitter cross-check (the one inside
    // IrFile is moved into Arc and we can also read it via raw_tree).
    let ir_tree = parser.parse(source, None).expect("parse");
    let ir = provider
        .to_ir(ir_tree, Arc::from(source), PathBuf::from(path))
        .expect("to_ir");
    (tree, ir)
}

fn first_descendant<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = first_descendant(child, kind) {
            return Some(found);
        }
    }
    None
}

fn assert_location_matches(loc: &cntrdct::ir::Location, ts: tree_sitter::Node<'_>, file: &str) {
    assert_eq!(loc.file.to_str().unwrap(), file);
    assert_eq!(loc.start_line, ts.start_position().row as u32 + 1);
    assert_eq!(loc.start_col, ts.start_position().column as u32 + 1);
    assert_eq!(loc.end_line, ts.end_position().row as u32 + 1);
    assert_eq!(loc.end_col, ts.end_position().column as u32 + 1);
    assert_eq!(loc.start_byte, ts.start_byte() as u32);
    assert_eq!(loc.end_byte, ts.end_byte() as u32);
}

// ---------- Rust ----------

#[test]
fn rust_fn_location_matches_tree_sitter() {
    let src = "fn foo(a: i32) -> i32 { a + 1 }\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_fn = first_descendant(tree.root_node(), "function_item").unwrap();
    assert_location_matches(&ir.fns[0].location, ts_fn, "a.rs");
}

#[test]
fn rust_param_location_matches_tree_sitter() {
    let src = "fn foo(alpha: i32) {}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_param = first_descendant(tree.root_node(), "parameter").unwrap();
    assert_location_matches(&ir.fns[0].params[0].location, ts_param, "a.rs");
}

#[test]
fn rust_block_location_matches_tree_sitter() {
    let src = "fn foo() { bar(); }\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_block = first_descendant(tree.root_node(), "block").unwrap();
    assert_location_matches(&ir.fns[0].body.location, ts_block, "a.rs");
}

#[test]
fn rust_call_site_location_matches_tree_sitter() {
    let src = "fn foo() {\n    bar(x, y);\n}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_call = first_descendant(tree.root_node(), "call_expression").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let call = match &stmts[0].kind {
        IrStmtKind::Call(c) => c,
        other => panic!("expected Call, got {other:?}"),
    };
    assert_location_matches(&call.location, ts_call, "a.rs");
}

#[test]
fn rust_if_stmt_location_matches_tree_sitter() {
    let src = "fn foo() {\n    if true { 1 } else { 2 };\n}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_if = first_descendant(tree.root_node(), "if_expression").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let if_stmt = match &stmts[0].kind {
        IrStmtKind::If(s) => s,
        other => panic!("expected If, got {other:?}"),
    };
    assert_location_matches(&if_stmt.location, ts_if, "a.rs");
}

#[test]
fn rust_while_stmt_location_matches_tree_sitter() {
    let src = "fn foo() {\n    while true { bar(); }\n}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_while = first_descendant(tree.root_node(), "while_expression").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let while_stmt = match &stmts[0].kind {
        IrStmtKind::While(s) => s,
        other => panic!("expected While, got {other:?}"),
    };
    assert_location_matches(&while_stmt.location, ts_while, "a.rs");
}

#[test]
fn rust_loop_stmt_location_matches_tree_sitter() {
    let src = "fn foo() {\n    loop { break; }\n}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_loop = first_descendant(tree.root_node(), "loop_expression").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let loop_stmt = match &stmts[0].kind {
        IrStmtKind::Loop(s) => s,
        other => panic!("expected Loop, got {other:?}"),
    };
    assert_location_matches(&loop_stmt.location, ts_loop, "a.rs");
}

#[test]
fn rust_match_stmt_location_matches_tree_sitter() {
    let src = "fn foo(x: i32) {\n    match x { _ => () };\n}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_match = first_descendant(tree.root_node(), "match_expression").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let match_stmt = match &stmts[0].kind {
        IrStmtKind::Match(s) => s,
        other => panic!("expected Match, got {other:?}"),
    };
    assert_location_matches(&match_stmt.location, ts_match, "a.rs");
}

#[test]
fn rust_decorator_location_matches_tree_sitter() {
    let src = "#[deprecated]\nfn foo() {}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_attr = first_descendant(tree.root_node(), "attribute_item").unwrap();
    assert_location_matches(&ir.fns[0].decorators[0].location, ts_attr, "a.rs");
}

#[test]
fn rust_comment_location_matches_tree_sitter() {
    let src = "// stand-alone\nfn foo() {}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_comment = first_descendant(tree.root_node(), "line_comment").unwrap();
    assert_eq!(ir.top_level_comments.len(), 1);
    assert_location_matches(&ir.top_level_comments[0].location, ts_comment, "a.rs");
}

#[test]
fn rust_stmt_kind_other_location_matches_tree_sitter() {
    let src = "fn foo() {\n    let x = 1;\n}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_let = first_descendant(tree.root_node(), "let_declaration").unwrap();
    let stmts = &ir.fns[0].body.statements;
    assert!(matches!(stmts[0].kind, IrStmtKind::Other { .. }));
    assert_location_matches(&stmts[0].location, ts_let, "a.rs");
}

#[test]
fn rust_expr_other_location_matches_tree_sitter() {
    // `1 + 2` is a binary_expression — modelled as IrExpr::Other in v0
    // (no IrExpr::Binary in the spec). Use it via a return value.
    let src = "fn foo() -> i32 {\n    return 1 + 2;\n}\n";
    let (tree, ir) = to_ir(Language::Rust, src, "a.rs");
    let ts_bin = first_descendant(tree.root_node(), "binary_expression").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let expr = match &stmts[0].kind {
        IrStmtKind::Return(Some(e)) => e,
        other => panic!("expected Return, got {other:?}"),
    };
    let node_kind = match expr {
        IrExpr::Other { node_kind, .. } => *node_kind,
        other => panic!("expected Other, got {other:?}"),
    };
    assert_eq!(node_kind, "binary_expression");
    // Confirm via raw_tree resolve that the ref points at the same node.
    if let IrExpr::Other { node_ref, .. } = expr {
        let resolved = ir.resolve(node_ref).expect("ref resolves");
        assert_eq!(resolved.range(), ts_bin.range());
    }
}

// ---------- Python ----------

#[test]
fn python_fn_location_matches_tree_sitter() {
    let src = "def foo(a):\n    return a\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_fn = first_descendant(tree.root_node(), "function_definition").unwrap();
    assert_location_matches(&ir.fns[0].location, ts_fn, "a.py");
}

#[test]
fn python_param_location_matches_tree_sitter() {
    let src = "def foo(alpha):\n    pass\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    // The first named child of `parameters` is the identifier itself
    // for a bare positional parameter.
    let params = first_descendant(tree.root_node(), "parameters").unwrap();
    let ts_param = {
        let mut cursor = params.walk();
        let found = params
            .children(&mut cursor)
            .find(|c| c.kind() == "identifier");
        found.expect("identifier param")
    };
    assert_location_matches(&ir.fns[0].params[0].location, ts_param, "a.py");
}

#[test]
fn python_block_location_matches_tree_sitter() {
    let src = "def foo():\n    bar()\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_block = first_descendant(tree.root_node(), "block").unwrap();
    assert_location_matches(&ir.fns[0].body.location, ts_block, "a.py");
}

#[test]
fn python_call_site_location_matches_tree_sitter() {
    let src = "def foo():\n    bar(x, y)\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_call = first_descendant(tree.root_node(), "call").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let call = match &stmts[0].kind {
        IrStmtKind::Call(c) => c,
        other => panic!("expected Call, got {other:?}"),
    };
    assert_location_matches(&call.location, ts_call, "a.py");
}

#[test]
fn python_if_stmt_location_matches_tree_sitter() {
    let src = "def foo(x):\n    if x:\n        return 1\n    else:\n        return 2\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_if = first_descendant(tree.root_node(), "if_statement").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let if_stmt = match &stmts[0].kind {
        IrStmtKind::If(s) => s,
        other => panic!("expected If, got {other:?}"),
    };
    assert_location_matches(&if_stmt.location, ts_if, "a.py");
}

#[test]
fn python_while_stmt_location_matches_tree_sitter() {
    let src = "def foo():\n    while True:\n        bar()\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_while = first_descendant(tree.root_node(), "while_statement").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let while_stmt = match &stmts[0].kind {
        IrStmtKind::While(s) => s,
        other => panic!("expected While, got {other:?}"),
    };
    assert_location_matches(&while_stmt.location, ts_while, "a.py");
}

#[test]
fn python_with_stmt_location_matches_tree_sitter() {
    let src = "def foo():\n    with open('p') as fp:\n        fp.read()\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_with = first_descendant(tree.root_node(), "with_statement").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let with = match &stmts[0].kind {
        IrStmtKind::With(s) => s,
        other => panic!("expected With, got {other:?}"),
    };
    assert_location_matches(&with.location, ts_with, "a.py");
}

#[test]
fn python_decorator_location_matches_tree_sitter() {
    let src = "@deprecated\ndef foo():\n    pass\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_dec = first_descendant(tree.root_node(), "decorator").unwrap();
    assert_location_matches(&ir.fns[0].decorators[0].location, ts_dec, "a.py");
}

#[test]
fn python_comment_location_matches_tree_sitter() {
    let src = "# free comment\ndef foo():\n    pass\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_c = first_descendant(tree.root_node(), "comment").unwrap();
    assert_eq!(ir.top_level_comments.len(), 1);
    assert_location_matches(&ir.top_level_comments[0].location, ts_c, "a.py");
}

#[test]
fn python_stmt_kind_other_location_matches_tree_sitter() {
    // `x = 1` parses as an `assignment` inside `expression_statement`;
    // the outer statement maps to IrStmtKind::Other (no Assign variant
    // in the spec).
    let src = "def foo():\n    x = 1\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_expr = first_descendant(tree.root_node(), "assignment").unwrap();
    let stmts = &ir.fns[0].body.statements;
    match &stmts[0].kind {
        IrStmtKind::Other { node_ref, .. } => {
            let resolved = ir.resolve(node_ref).expect("ref resolves");
            assert_eq!(resolved.range(), ts_expr.range());
        }
        other => panic!("expected Other, got {other:?}"),
    }
}

#[test]
fn python_expr_other_location_matches_tree_sitter() {
    // `a + b` is a `binary_operator` — IrExpr::Other in v0.
    let src = "def foo(a, b):\n    return a + b\n";
    let (tree, ir) = to_ir(Language::Python, src, "a.py");
    let ts_bin = first_descendant(tree.root_node(), "binary_operator").unwrap();
    let stmts = &ir.fns[0].body.statements;
    let expr = match &stmts[0].kind {
        IrStmtKind::Return(Some(e)) => e,
        other => panic!("expected Return, got {other:?}"),
    };
    if let IrExpr::Other { node_ref, .. } = expr {
        let resolved = ir.resolve(node_ref).expect("ref resolves");
        assert_eq!(resolved.range(), ts_bin.range());
    } else {
        panic!("expected IrExpr::Other, got {expr:?}");
    }
}

// ---------- Sanity: block normalised tokens still byte-identical ----------

#[test]
fn block_normalised_tokens_capture_function_body() {
    // T5 is location-focused but we cross-check that the F1
    // normalised_tokens field exists and is non-empty for both
    // languages so detectors that consume it (clone-drift) see
    // structural tokens.
    let (_t, rs_ir) = to_ir(Language::Rust, "fn foo() { let x = 1; x }\n", "a.rs");
    assert!(!body_tokens(&rs_ir.fns[0].body).is_empty());
    let (_t, py_ir) = to_ir(Language::Python, "def foo():\n    return 1\n", "a.py");
    assert!(!body_tokens(&py_ir.fns[0].body).is_empty());
}

fn body_tokens(b: &IrBlock) -> &[cntrdct::ir::NormalisedToken] {
    &b.normalised_tokens
}
