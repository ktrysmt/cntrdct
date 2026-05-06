//! Rust function-body call-site extraction (spec F2 Rust row).
//!
//! Walks top-level `function_item` nodes of a parsed Rust source and emits
//! one `Transaction` per function. Each `Transaction` records the distinct
//! callee names found inside the function's body. Names are reduced to the
//! last identifier segment (e.g. `a.lock()` -> `lock`, `std::vec::Vec::new()`
//! -> `new`); this matches Li-Zhou's "function name only" formulation per
//! spec R2.

use std::collections::BTreeSet;
use std::path::Path;

use crate::core::{Language, Location, ParsedFile};

use super::Transaction;

/// Extract one `Transaction` per top-level `function_item` in `file`. Files
/// that fail to parse, or whose root has any parse error, return an empty
/// vector (silent skip per spec N5 / F1).
pub fn extract(file: &ParsedFile) -> Vec<Transaction> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_rust::language()).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(&file.source, None) else {
        return Vec::new();
    };
    let root = tree.root_node();
    if root.has_error() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() != "function_item" {
            continue;
        }
        let body = match child.child_by_field_name("body") {
            Some(b) => b,
            None => continue,
        };
        let mut items: BTreeSet<String> = BTreeSet::new();
        collect_call_items(body, &file.source, &mut items);
        out.push(Transaction {
            language: Language::Rust,
            items,
            location: node_location(&file.path, child),
        });
    }
    out
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

/// Return the last-segment identifier of the call's head, or `None` for
/// heads we drop in v0 (closures, dynamic dispatch, indexed calls, macro
/// calls — anything that is not a static identifier path).
fn call_head_name(call: tree_sitter::Node, source: &str) -> Option<String> {
    let function = call.child_by_field_name("function")?;
    head_to_last_segment(function, source)
}

fn head_to_last_segment(node: tree_sitter::Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" | "type_identifier" => Some(text(node, source).to_string()),
        // `a::b::c()` — take last identifier of the path.
        "scoped_identifier" => node
            .child_by_field_name("name")
            .map(|n| text(n, source).to_string()),
        // `a.b()` — take the field name.
        "field_expression" => node
            .child_by_field_name("field")
            .map(|n| text(n, source).to_string()),
        // `f::<T>()` — recurse into the inner function.
        "generic_function" => node
            .child_by_field_name("function")
            .and_then(|n| head_to_last_segment(n, source)),
        _ => None,
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
    use crate::core::Language;
    use std::path::PathBuf;

    fn pf(src: &str) -> ParsedFile {
        ParsedFile {
            path: PathBuf::from("a.rs"),
            language: Language::Rust,
            source: src.to_string(),
        }
    }

    #[test]
    fn extracts_simple_calls() {
        let src = "fn f() { acquire(); release(); }\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(
            txns[0].items.iter().cloned().collect::<Vec<_>>(),
            vec!["acquire".to_string(), "release".to_string()]
        );
    }

    #[test]
    fn deduplicates_repeated_calls() {
        let src = "fn f() { acquire(); acquire(); release(); }\n";
        let txns = extract(&pf(src));
        assert_eq!(txns[0].items.len(), 2);
    }

    #[test]
    fn drops_closure_heads() {
        let src = "fn f() { (|| 1)(); release(); }\n";
        let txns = extract(&pf(src));
        assert_eq!(
            txns[0].items.iter().cloned().collect::<Vec<_>>(),
            vec!["release".to_string()]
        );
    }

    #[test]
    fn handles_paths_and_methods() {
        let src = "fn f() { std::mem::swap(&mut a, &mut b); obj.lock(); }\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(items.contains(&"swap".to_string()));
        assert!(items.contains(&"lock".to_string()));
    }

    #[test]
    fn parse_error_yields_empty() {
        let src = "fn f( { unbalanced }\n";
        let txns = extract(&pf(src));
        assert!(txns.is_empty());
    }

    #[test]
    fn empty_body_yields_empty_items() {
        let src = "fn f() {}\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert!(txns[0].items.is_empty());
    }

    #[test]
    fn nested_blocks_walk_recursively() {
        let src = "fn f() { if true { acquire(); } else { release(); } }\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["acquire".to_string(), "release".to_string()]);
    }
}
