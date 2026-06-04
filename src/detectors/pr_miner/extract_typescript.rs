//! TypeScript function-body call-site extraction (R-2.e, pr-miner
//! TypeScript opt-in).
//!
//! Walks top-level functions of a parsed TypeScript source and emits one
//! `Transaction` per function. Each `Transaction` records the distinct
//! callee names found inside the function's body, reduced to the last
//! identifier segment (`obj.method()` → `method`, `pkg.mod.fn()` → `fn`),
//! matching the Rust / Python extractors and Li-Zhou's "function name
//! only" formulation (spec R2).
//!
//! Top-level functions for TypeScript are: `function_declaration`
//! (optionally wrapped in `export_statement`) and
//! `const f = () => {}` / `const f = function () {}` declarators whose
//! initialiser is a function expression. Class methods are out of v0
//! scope here — pr-miner mines module-level functions, matching the
//! "root children only" Python walk.
//!
//! There is no TypeScript analogue of Python's `with` resource block, so
//! the F4e-i context-manager cleanup synthesis is intentionally omitted.
//!
//! Per the R-2.f survey, TypeScript pr-miner findings ship with
//! `LanguageCitationStatus::Unconfirmed` — set in `crate::make_finding`
//! from the transaction's `language` field, which this module tags
//! `Language::TypeScript`.

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
        // `export [default] <decl>` is a transparent wrapper.
        let node = match child.kind() {
            "export_statement" => match child.child_by_field_name("declaration") {
                Some(decl) => decl,
                None => continue,
            },
            _ => child,
        };
        match node.kind() {
            "function_declaration" | "generator_function_declaration" => {
                push_function(node, node, file, &mut out);
            }
            "lexical_declaration" | "variable_declaration" => {
                collect_declared_functions(node, file, &mut out);
            }
            _ => {}
        }
    }
    out
}

fn collect_declared_functions(decl: tree_sitter::Node, file: &IrFile, out: &mut Vec<Transaction>) {
    let mut cursor = decl.walk();
    for child in decl.children(&mut cursor) {
        if child.kind() != "variable_declarator" {
            continue;
        }
        let Some(value) = child.child_by_field_name("value") else {
            continue;
        };
        if !matches!(value.kind(), "arrow_function" | "function_expression") {
            continue;
        }
        // The primary location points at the whole declaration so the
        // violation marker lands on the `const`/`let` line the author
        // wrote, mirroring the Python decorated-definition behaviour.
        push_function(value, decl, file, out);
    }
}

fn push_function(
    fn_node: tree_sitter::Node,
    primary_node: tree_sitter::Node,
    file: &IrFile,
    out: &mut Vec<Transaction>,
) {
    let Some(body) = fn_node.child_by_field_name("body") else {
        return;
    };
    let mut items: BTreeSet<String> = BTreeSet::new();
    collect_call_items(body, &file.source, &mut items);
    out.push(Transaction {
        language: Language::TypeScript,
        items,
        location: node_location(&file.path, primary_node),
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

/// Last-segment identifier of a TypeScript `call_expression` head, or
/// `None` for heads dropped in v0 (computed member access, call /
/// subscript receivers, dynamic dispatch — anything that is not a static
/// identifier path).
fn call_head_name(call: tree_sitter::Node, source: &str) -> Option<String> {
    let function = call.child_by_field_name("function")?;
    head_to_last_segment(function, source)
}

fn head_to_last_segment(node: tree_sitter::Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(text(node, source).to_string()),
        // `pkg.mod.fn` / `obj.method` / `this.method` — take the rightmost
        // property, but only when the receiver chain is itself a static
        // identifier chain (drops `arr[0].push(...)`, `get().write(...)`,
        // matching the Python F4e-ii conservatism).
        "member_expression" => {
            let object = node.child_by_field_name("object")?;
            if !is_identifier_chain(object) {
                return None;
            }
            node.child_by_field_name("property")
                .filter(|p| p.kind() == "property_identifier")
                .map(|n| text(n, source).to_string())
        }
        _ => None,
    }
}

/// True when `node` is a chain of identifier-typed member access
/// terminating in a bare `identifier` or `this`. Matches `f`, `this.foo`,
/// `pkg.mod.sub`; rejects `arr[0]` (subscript), `get()` (call),
/// `(a + b)` (binary).
fn is_identifier_chain(node: tree_sitter::Node) -> bool {
    match node.kind() {
        "identifier" | "this" => true,
        "member_expression" => node
            .child_by_field_name("object")
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
        let provider = crate::parsers::parser_for(Language::TypeScript);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&provider.ts_language())
            .expect("set typescript language");
        let tree = parser.parse(src, None).expect("parse typescript");
        provider
            .to_ir(tree, Arc::from(src), PathBuf::from("a.ts"))
            .expect("to_ir")
    }

    #[test]
    fn extracts_simple_calls() {
        let src = "function f() {\n  beginTx();\n  commitTx();\n}\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(
            txns[0].items.iter().cloned().collect::<Vec<_>>(),
            vec!["beginTx".to_string(), "commitTx".to_string()]
        );
    }

    #[test]
    fn handles_member_calls() {
        let src = "function f() {\n  obj.lock();\n  pkg.mod.unlock();\n}\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["lock".to_string(), "unlock".to_string()]);
    }

    #[test]
    fn extracts_arrow_declarator_function() {
        let src = "const f = () => {\n  beginTx();\n  commitTx();\n};\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].items.len(), 2);
    }

    #[test]
    fn unwraps_export_statement() {
        let src = "export function f() {\n  beginTx();\n  commitTx();\n}\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].items.len(), 2);
    }

    #[test]
    fn drops_subscript_receiver_member_call() {
        let src = "function f() {\n  handlers[0].write(x);\n  other();\n}\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(!items.contains(&"write".to_string()));
        assert!(items.contains(&"other".to_string()));
    }

    #[test]
    fn parse_error_yields_empty() {
        let src = "function f( {\n  beginTx();\n";
        let txns = extract(&pf(src));
        assert!(txns.is_empty());
    }

    #[test]
    fn nested_blocks_walk_recursively() {
        let src =
            "function f(c) {\n  if (c) {\n    beginTx();\n  } else {\n    commitTx();\n  }\n}\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["beginTx".to_string(), "commitTx".to_string()]);
    }
}
