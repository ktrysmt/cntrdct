//! Go function-body call-site extraction (R-3.e, pr-miner Go opt-in).
//!
//! Walks top-level functions of a parsed Go source and emits one
//! `Transaction` per function. Each `Transaction` records the distinct
//! callee names found inside the function's body, reduced to the last
//! identifier segment (`obj.Method()` → `Method`, `pkg.sub.Fn()` → `Fn`),
//! matching the Rust / Python / TypeScript extractors and Li-Zhou's
//! "function name only" formulation (spec R2).
//!
//! Top-level functions for Go are `function_declaration` and
//! `method_declaration` (the receiver lives in a separate field and does
//! not affect call-item extraction). `func_literal` closures are nested
//! and are walked as part of their enclosing function's body, matching the
//! "root children only" Python walk.
//!
//! There is no Go analogue of Python's `with` resource block, so the
//! F4e-i context-manager cleanup synthesis is intentionally omitted.
//!
//! Per the R-3.f survey, Go pr-miner findings ship with
//! `LanguageCitationStatus::Unconfirmed` — set in `crate::make_finding`
//! from the transaction's `language` field, which this module tags
//! `Language::Go`.

use std::collections::BTreeSet;
use std::path::Path;

use crate::core::{Language, Location};
use crate::ir::IrFile;

use super::Transaction;

/// Extract one `Transaction` per top-level function definition in `file`.
/// Files that fail to parse return an empty vector (silent skip per
/// spec N5 / F1).
pub fn extract(file: &IrFile) -> Vec<Transaction> {
    if file.parse_recovered {
        return Vec::new();
    }
    let raw_tree = file.raw_tree();
    let root = raw_tree.root_node();

    let mut out = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "method_declaration" => {
                push_function(child, file, &mut out);
            }
            _ => {}
        }
    }
    out
}

fn push_function(fn_node: tree_sitter::Node, file: &IrFile, out: &mut Vec<Transaction>) {
    let Some(body) = fn_node.child_by_field_name("body") else {
        return;
    };
    let mut items: BTreeSet<String> = BTreeSet::new();
    collect_call_items(body, &file.source, &mut items);
    out.push(Transaction {
        language: Language::Go,
        items,
        location: node_location(&file.path, fn_node),
    });
}

fn collect_call_items(node: tree_sitter::Node, source: &str, out: &mut BTreeSet<String>) {
    if node.kind() == "call_expression" {
        if let Some(name) = call_head_name(node, source) {
            out.insert(name);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_items(child, source, out);
    }
}

/// Last-segment identifier of a Go `call_expression` head, or `None` for
/// heads dropped in v0 (index / call receivers, type conversions —
/// anything that is not a static identifier path).
fn call_head_name(call: tree_sitter::Node, source: &str) -> Option<String> {
    let function = call.child_by_field_name("function")?;
    head_to_last_segment(function, source)
}

fn head_to_last_segment(node: tree_sitter::Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(text(node, source).to_string()),
        // `pkg.sub.Fn` / `obj.Method` — take the rightmost field, but only
        // when the receiver chain is itself a static identifier chain
        // (drops `arr[0].Push(...)`, `get().Write(...)`, matching the
        // Python F4e-ii conservatism).
        "selector_expression" => {
            let operand = node.child_by_field_name("operand")?;
            if !is_identifier_chain(operand) {
                return None;
            }
            node.child_by_field_name("field")
                .filter(|p| p.kind() == "field_identifier")
                .map(|n| text(n, source).to_string())
        }
        _ => None,
    }
}

/// True when `node` is a chain of selector access terminating in a bare
/// `identifier`. Matches `f`, `pkg.sub`; rejects `arr[0]` (index),
/// `get()` (call), `(a + b)` (binary).
fn is_identifier_chain(node: tree_sitter::Node) -> bool {
    match node.kind() {
        "identifier" => true,
        "selector_expression" => node
            .child_by_field_name("operand")
            .map(is_identifier_chain)
            .unwrap_or(false),
        _ => false,
    }
}

fn text<'a>(node: tree_sitter::Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn node_location(path: &Path, node: tree_sitter::Node) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        file: path.to_path_buf(),
        start_line: start.row as u32 + 1,
        start_col: start.column as u32 + 1,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32 + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn pf(src: &str) -> IrFile {
        let provider = crate::parsers::parser_for(Language::Go);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&provider.ts_language())
            .expect("set go language");
        let tree = parser.parse(src, None).expect("parse go");
        provider
            .to_ir(tree, Arc::from(src), PathBuf::from("a.go"))
            .expect("to_ir")
    }

    #[test]
    fn extracts_simple_calls() {
        let src = "package main\nfunc f() {\n  beginTx()\n  commitTx()\n}\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(
            txns[0].items.iter().cloned().collect::<Vec<_>>(),
            vec!["beginTx".to_string(), "commitTx".to_string()]
        );
    }

    #[test]
    fn handles_selector_calls() {
        let src = "package main\nfunc f() {\n  obj.Lock()\n  pkg.sub.Unlock()\n}\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["Lock".to_string(), "Unlock".to_string()]);
    }

    #[test]
    fn extracts_method_declaration() {
        let src = "package main\nfunc (r *T) f() {\n  beginTx()\n  commitTx()\n}\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].items.len(), 2);
    }

    #[test]
    fn drops_index_receiver_selector_call() {
        let src = "package main\nfunc f() {\n  handlers[0].Write(x)\n  other()\n}\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(!items.contains(&"Write".to_string()));
        assert!(items.contains(&"other".to_string()));
    }

    #[test]
    fn parse_error_yields_empty() {
        let src = "package main\nfunc f( {\n  beginTx()\n";
        let txns = extract(&pf(src));
        assert!(txns.is_empty());
    }

    #[test]
    fn nested_blocks_walk_recursively() {
        let src =
            "package main\nfunc f(c bool) {\n  if c {\n    beginTx()\n  } else {\n    commitTx()\n  }\n}\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["beginTx".to_string(), "commitTx".to_string()]);
    }
}
