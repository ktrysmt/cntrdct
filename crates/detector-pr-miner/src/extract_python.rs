//! Python function-body call-site extraction (spec F2 Python row).
//!
//! Walks top-level `function_definition` (and `decorated_definition`
//! wrappers, including async) of a parsed Python source and emits one
//! `Transaction` per function. Each `Transaction` records the distinct
//! callee names found inside the function's body. Names are reduced to
//! the last identifier segment (e.g. `obj.method()` → `method`,
//! `pkg.mod.fn()` → `fn`); this matches the Rust path and Li-Zhou's
//! "function name only" formulation per spec R2.
//!
//! Per docs/surveys/pr-miner-python-2026-05.md, Python findings ship
//! with `LanguageCitationStatus::Unconfirmed` (no qualifying Python
//! citation was found). The status is set in
//! `crate::make_finding` from the violator transaction's `language`
//! field; this module simply tags every transaction it emits as
//! `Language::Python`.

use std::collections::BTreeSet;
use std::path::Path;

use cntrdct_core::{Language, Location, ParsedFile};

use crate::Transaction;

/// Extract one `Transaction` per top-level function definition in `file`.
/// Files that fail to parse, or whose root has any parse error, return
/// an empty vector (silent skip per spec N5 / F1).
pub fn extract(file: &ParsedFile) -> Vec<Transaction> {
    let mut parser = tree_sitter::Parser::new();
    if parser
        .set_language(&tree_sitter_python::language())
        .is_err()
    {
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
        let fn_def = match child.kind() {
            "function_definition" => child,
            "decorated_definition" => match find_inner_function_definition(child) {
                Some(inner) => inner,
                None => continue,
            },
            _ => continue,
        };
        let body = match fn_def.child_by_field_name("body") {
            Some(b) => b,
            None => continue,
        };
        let mut items: BTreeSet<String> = BTreeSet::new();
        collect_call_items(body, &file.source, &mut items);
        // Use the location of the outer `decorated_definition` if the
        // function had decorators, so the violation primary points at the
        // first line the user actually wrote (decorator line). This
        // matches comment-code's behaviour for `@deprecated` checks.
        let primary_node = if child.kind() == "decorated_definition" {
            child
        } else {
            fn_def
        };
        out.push(Transaction {
            language: Language::Python,
            items,
            location: node_location(&file.path, primary_node),
        });
    }
    out
}

fn find_inner_function_definition(decorated: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut cursor = decorated.walk();
    let inner = decorated
        .children(&mut cursor)
        .find(|c| c.kind() == "function_definition");
    inner
}

fn collect_call_items(node: tree_sitter::Node, source: &str, out: &mut BTreeSet<String>) {
    if node.kind() == "call" {
        if let Some(name) = call_head_name(node, source) {
            out.insert(name);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_items(child, source, out);
    }
}

/// Last-segment identifier of a Python `call` node's function head, or
/// `None` for heads we drop in v0 (subscript calls, lambda-call patterns,
/// dynamic dispatch, anything that is not a static identifier path).
fn call_head_name(call: tree_sitter::Node, source: &str) -> Option<String> {
    let function = call.child_by_field_name("function")?;
    head_to_last_segment(function, source)
}

fn head_to_last_segment(node: tree_sitter::Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(text(node, source).to_string()),
        // `pkg.mod.fn` or `obj.method` — take the rightmost attribute.
        "attribute" => node
            .child_by_field_name("attribute")
            .map(|n| text(n, source).to_string()),
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
    use std::path::PathBuf;

    fn pf(src: &str) -> ParsedFile {
        ParsedFile {
            path: PathBuf::from("a.py"),
            language: Language::Python,
            source: src.to_string(),
        }
    }

    #[test]
    fn extracts_simple_calls() {
        let src = "def f():\n    acquire()\n    release()\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(
            txns[0].items.iter().cloned().collect::<Vec<_>>(),
            vec!["acquire".to_string(), "release".to_string()]
        );
    }

    #[test]
    fn handles_attribute_calls() {
        let src = "def f():\n    obj.lock()\n    pkg.mod.unlock()\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["lock".to_string(), "unlock".to_string()]);
    }

    #[test]
    fn deduplicates_repeated_calls() {
        let src = "def f():\n    acquire()\n    acquire()\n    release()\n";
        let txns = extract(&pf(src));
        assert_eq!(txns[0].items.len(), 2);
    }

    #[test]
    fn handles_decorated_definition() {
        let src = "\
@dec
def f():
    acquire()
    release()
";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].items.len(), 2);
    }

    #[test]
    fn handles_async_def_when_decorated() {
        // tree-sitter-python wraps `async def` as a function_definition
        // (top-level) just like a sync def; either way our walker picks
        // it up.
        let src = "async def f():\n    acquire()\n    release()\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].items.len(), 2);
    }

    #[test]
    fn parse_error_yields_empty() {
        // tree-sitter-python is permissive, but a stray indent block is
        // still flagged on the root.
        let src = "def f(:\n    acquire()\n";
        let txns = extract(&pf(src));
        assert!(txns.is_empty());
    }

    #[test]
    fn empty_body_pass_yields_empty_items() {
        let src = "def f():\n    pass\n";
        let txns = extract(&pf(src));
        assert_eq!(txns.len(), 1);
        assert!(txns[0].items.is_empty());
    }

    #[test]
    fn nested_blocks_walk_recursively() {
        let src = "\
def f():
    if True:
        acquire()
    else:
        release()
";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["acquire".to_string(), "release".to_string()]);
    }

    #[test]
    fn drops_subscript_call_heads() {
        let src = "def f():\n    handlers[0]()\n    release()\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert_eq!(items, vec!["release".to_string()]);
    }
}
