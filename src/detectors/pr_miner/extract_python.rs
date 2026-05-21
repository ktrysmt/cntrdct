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

use crate::core::{Language, Location, ParsedFile};

use super::Transaction;

/// Extract one `Transaction` per top-level function definition in `file`.
/// Files that fail to parse, or whose root has any parse error, return
/// an empty vector (silent skip per spec N5 / F1).
pub fn extract(file: &ParsedFile) -> Vec<Transaction> {
    let mut parser = tree_sitter::Parser::new();
    if parser
        .set_language(&crate::parsers::parser_for(Language::Python).ts_language())
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
            out.insert(name.clone());
            // F4e-i: when the call sits directly under a `with_clause` /
            // `with_item` AND its head identifier is on the canonical
            // resource-pair list, also synthesise the matching cleanup
            // identifier. Python's `with X(...) as Y:` contract runs
            // `X.__exit__` at block exit, which for the recognised
            // pairs invokes the documented cleanup method. Synthesis
            // (rather than dropping the open / acquire) preserves both
            // directions of the paired-API rule:
            //   {open} -> {close} satisfied by the synthetic close.
            //   {close} -> {open} satisfied by the original open when
            //   the body also writes a defensive explicit close.
            if python_call_is_under_with_clause(node) {
                if let Some(cleanup) = python_context_manager_cleanup(&name) {
                    out.insert(cleanup.to_string());
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_items(child, source, out);
    }
}

/// True when `call`'s direct parent is a `with_clause` or `with_item`
/// — Python's `with EXPR as NAME:` shape, where EXPR is the
/// context-manager call. The `__exit__` discharge is implicit; treat
/// the call as if it carried both the open / acquire and the matching
/// close / release for the purposes of paired-API mining. Returns
/// false when `call` has no parent (root node) or when the parent is
/// not a `with_*` node.
fn python_call_is_under_with_clause(call: tree_sitter::Node) -> bool {
    let mut current = call.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "with_clause" | "with_item" => return true,
            // `as_pattern` wraps `<call> as <name>` — its parent should
            // still be a `with_item`, so keep walking.
            "as_pattern" => current = parent.parent(),
            _ => return false,
        }
    }
    false
}

/// F4e-i pair table. Maps a context-manager creation call head to
/// its conventional `__exit__` cleanup identifier. Heads not on
/// this list produce no synthesis; they leave `items` unchanged.
fn python_context_manager_cleanup(head: &str) -> Option<&'static str> {
    match head {
        "open" => Some("close"),
        "acquire" => Some("release"),
        _ => None,
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
        // `pkg.mod.fn` or `obj.method` — take the rightmost attribute,
        // but only when the receiver (`object`) is itself an identifier
        // chain. F4e-ii: `pieces[-1].write(...)` has a `subscript`
        // receiver; `get_handler().write(...)` has a `call` receiver.
        // Both describe writes against transient / computed objects
        // whose type pr-miner cannot resolve, so conflating them with
        // file/socket/lock idioms inflates FP without recovering TP.
        "attribute" => {
            let object = node.child_by_field_name("object")?;
            if !python_is_identifier_chain(object) {
                return None;
            }
            node.child_by_field_name("attribute")
                .map(|n| text(n, source).to_string())
        }
        _ => None,
    }
}

/// True when `node` is a chain of identifier-typed attribute access
/// terminating in a bare `identifier`. Examples that match:
/// `f`, `self.foo`, `pkg.mod.sub`. Examples that do not:
/// `pieces[-1]` (subscript), `get()` (call), `(a + b)` (binary op).
fn python_is_identifier_chain(node: tree_sitter::Node) -> bool {
    match node.kind() {
        "identifier" => true,
        "attribute" => node
            .child_by_field_name("object")
            .map(python_is_identifier_chain)
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

    // ---------- F4e-i: context-manager cleanup synthesis ----------

    #[test]
    fn t_prm_fp_1_with_open_synthesises_close() {
        // canonical nbrmd FP shape: `with open(path) as fp:` body uses
        // fp but never syntactically closes it. Synthesis of `close`
        // satisfies the {open} -> {close} rule even without a
        // syntactic close() in the body.
        let src = "def f(path):\n    with open(path) as fp:\n        data = fp.read()\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(
            items.contains(&"open".to_string()),
            "F4e-i must NOT drop open; got {:?}",
            items
        );
        assert!(
            items.contains(&"close".to_string()),
            "F4e-i must synthesise close for with-managed open; got {:?}",
            items
        );
    }

    #[test]
    fn t_prm_fp_4_with_open_then_explicit_close_keeps_both() {
        // carla_import regression pin: `with open(p) as fh: ...; fh.close()`
        // (defensive double-close). The explicit close() must NOT be
        // accidentally dropped by F4e-i; both open and close stay.
        let src = "def f(p):\n    with open(p) as fh:\n        fh.write(x)\n        fh.close()\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(
            items.contains(&"open".to_string()),
            "open must remain in items; got {:?}",
            items
        );
        assert!(
            items.contains(&"close".to_string()),
            "explicit close must remain in items; got {:?}",
            items
        );
    }

    #[test]
    fn t_prm_fp_5_with_acquire_synthesises_release() {
        // Lock acquisition idiom: `with lock.acquire() as l:` runs
        // __exit__ → release. The synthesis table pairs acquire ->
        // release so paired-API rules are satisfied without a
        // syntactic release() in the body.
        let src = "def f(lock):\n    with lock.acquire() as l:\n        do_stuff()\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(
            items.contains(&"acquire".to_string()),
            "acquire must remain in items; got {:?}",
            items
        );
        assert!(
            items.contains(&"release".to_string()),
            "F4e-i must synthesise release for with-managed acquire; got {:?}",
            items
        );
    }

    #[test]
    fn t_prm_fp_6_non_pair_with_call_no_synthesis() {
        // Non-recognised context-manager head: `with session() as s:` —
        // not in the v0 pair table. Items must contain `session` but
        // NO synthesised cleanup.
        let src = "def f():\n    with session() as s:\n        do_stuff()\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(
            items.contains(&"session".to_string()),
            "session must be in items; got {:?}",
            items
        );
        assert!(
            !items.contains(&"close".to_string()),
            "no synthesis for unrecognised context manager; got {:?}",
            items
        );
        assert!(
            !items.contains(&"release".to_string()),
            "no synthesis for unrecognised context manager; got {:?}",
            items
        );
    }

    // ---------- F4e-ii: drop attribute calls with complex receivers ----------

    #[test]
    fn t_prm_fp_2_subscript_receiver_attribute_dropped() {
        // canonical rosrust FP shape: `pieces[-1].write(...)` is a
        // StringIO write — pr-miner cannot distinguish it from a
        // file handle write. Drop the call entirely.
        let src = "def f():\n    pieces[-1].write(x)\n    other_call()\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(
            !items.contains(&"write".to_string()),
            "F4e-ii must drop write on subscript receiver; got {:?}",
            items
        );
        assert!(
            items.contains(&"other_call".to_string()),
            "non-attribute call must remain; got {:?}",
            items
        );
    }

    #[test]
    fn t_prm_fp_3_call_receiver_attribute_dropped() {
        // Chained-call receiver: `get_handler().write(x)`. The receiver
        // is itself a call expression — drop the .write item.
        let src = "def f():\n    get_handler().write(x)\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        // get_handler IS a bare-identifier call; it stays. write is
        // an attribute call with a call receiver; it does not.
        assert!(
            items.contains(&"get_handler".to_string()),
            "get_handler must remain; got {:?}",
            items
        );
        assert!(
            !items.contains(&"write".to_string()),
            "F4e-ii must drop attribute call with call receiver; got {:?}",
            items
        );
    }

    #[test]
    fn t_prm_fp_7_identifier_chain_attribute_kept() {
        // Identifier-chain receivers (`self.foo.lock(...)`) MUST stay.
        // Only non-identifier-chain receivers (subscript, call,
        // arithmetic) are dropped.
        let src = "def f(self):\n    self.foo.lock(x)\n";
        let txns = extract(&pf(src));
        let items: Vec<String> = txns[0].items.iter().cloned().collect();
        assert!(
            items.contains(&"lock".to_string()),
            "identifier-chain receiver must be kept; got {:?}",
            items
        );
    }
}
