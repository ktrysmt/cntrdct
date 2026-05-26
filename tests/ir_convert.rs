//! ir-v0.md §F6 T4 — IR golden fixtures.
//!
//! For a handful of canonical sources per language (impl/class-bearing,
//! nested calls, nested if/match), serialise the converted [`IrFile`]
//! to JSON and pin against a golden file. The serialisation projects
//! to [`cntrdct::ir::SerializableIrFile`] (strips `raw_tree` and
//! `source`, which are reproducible from the fixture path) per
//! ir-v0.md §F6 T4.
//!
//! Re-blessing: set the `CNTRDCT_BLESS=1` environment variable to
//! overwrite the golden JSON with the current converter output. Use
//! sparingly — every blessing widens the converter's trusted surface.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use cntrdct::ir::SerializableIrFile;
use cntrdct::parsers::{parser_for, Language};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ir")
}

fn run_case(lang: Language, case: &str, source_ext: &str) {
    let source_name = format!("{case}.{source_ext}");
    let source_path = fixture_root().join(language_dir(lang)).join(&source_name);
    let golden_path = fixture_root()
        .join(language_dir(lang))
        .join(format!("{case}.json"));
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", source_path.display()));

    let provider = parser_for(lang);
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&provider.ts_language())
        .expect("set language");
    let tree = parser
        .parse(source.as_str(), None)
        .expect("parse source fixture");
    let ir = provider
        .to_ir(tree, Arc::from(source.as_str()), PathBuf::from(source_name))
        .expect("to_ir succeeds on clean fixture");

    let projection: SerializableIrFile = (&ir).into();
    let mut actual = serde_json::to_string_pretty(&projection).expect("serialize");
    actual.push('\n');

    if std::env::var("CNTRDCT_BLESS").is_ok() {
        fs::write(&golden_path, &actual)
            .unwrap_or_else(|e| panic!("write golden {}: {e}", golden_path.display()));
        return;
    }

    let expected = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
        panic!(
            "missing golden {} — re-run with CNTRDCT_BLESS=1 to capture: {e}",
            golden_path.display()
        )
    });

    assert_eq!(
        actual,
        expected,
        "IR golden mismatch for {}; re-run with CNTRDCT_BLESS=1 to update",
        source_path.display()
    );
}

fn language_dir(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::Python => "python",
        _ => unreachable!("unsupported language under §F6 T4 in v0"),
    }
}

#[test]
fn t4_rust_impl_methods() {
    run_case(Language::Rust, "impl_methods", "rs");
}

#[test]
fn t4_rust_nested_calls() {
    run_case(Language::Rust, "nested_calls", "rs");
}

#[test]
fn t4_rust_nested_if_match() {
    run_case(Language::Rust, "nested_if_match", "rs");
}

#[test]
fn t4_python_class_methods() {
    run_case(Language::Python, "class_methods", "py");
}

#[test]
fn t4_python_nested_calls() {
    run_case(Language::Python, "nested_calls", "py");
}

#[test]
fn t4_python_nested_if_match() {
    run_case(Language::Python, "nested_if_match", "py");
}
