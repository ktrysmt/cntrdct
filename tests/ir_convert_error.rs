//! ir-v0.md §F6 T2 — `IrConvertError` variant coverage.
//!
//! Drives `ParserProvider::to_ir` against deliberately-malformed
//! inputs and asserts the converter returns the documented variant:
//!
//! - T2a `LanguageMismatch`: a tree parsed against one language is
//!   handed to a provider for another language.
//! - T2b `EmptySource`: blank / whitespace-only source.
//! - T2c `StructuralInvariant`: variant shape + Display formatting.
//!   The fault-injection end-to-end test (driving `cntrdct scan` and
//!   asserting tracing log emission) lives downstream of R-1.c, when
//!   detectors switch to consuming `IrFile` and the
//!   skip-and-continue policy goes live. R-1.b ships the variant
//!   contract; this test pins it.

use std::path::PathBuf;
use std::sync::Arc;

use cntrdct::ir::IrConvertError;
use cntrdct::parsers::{parser_for, Language};

fn parse(lang: Language, source: &str) -> tree_sitter::Tree {
    let provider = parser_for(lang);
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&provider.ts_language())
        .expect("set language");
    parser.parse(source, None).expect("parse")
}

#[test]
fn t2a_language_mismatch_when_rust_provider_sees_python_tree() {
    let py_tree = parse(Language::Python, "def main(): pass\n");
    let provider = parser_for(Language::Rust);
    let err = provider
        .to_ir(
            py_tree,
            Arc::from("def main(): pass\n"),
            PathBuf::from("a.py"),
        )
        .expect_err("language-mismatched tree must error");
    match err {
        IrConvertError::LanguageMismatch { expected, actual } => {
            assert_eq!(expected, Language::Rust);
            assert!(
                !actual.is_empty(),
                "actual language string must carry diagnosis info"
            );
        }
        other => panic!("expected LanguageMismatch, got {other:?}"),
    }
}

#[test]
fn t2a_language_mismatch_when_python_provider_sees_rust_tree() {
    let rs_tree = parse(Language::Rust, "fn main() {}\n");
    let provider = parser_for(Language::Python);
    let err = provider
        .to_ir(rs_tree, Arc::from("fn main() {}\n"), PathBuf::from("a.rs"))
        .expect_err("language-mismatched tree must error");
    assert!(matches!(err, IrConvertError::LanguageMismatch { .. }));
}

#[test]
fn t2b_empty_source_rust() {
    let tree = parse(Language::Rust, "\n");
    let provider = parser_for(Language::Rust);
    let err = provider
        .to_ir(tree, Arc::from("\n"), PathBuf::from("blank.rs"))
        .expect_err("blank source must error");
    assert!(matches!(err, IrConvertError::EmptySource));
}

#[test]
fn t2b_empty_source_python_whitespace_only() {
    let tree = parse(Language::Python, "   \n   \n");
    let provider = parser_for(Language::Python);
    let err = provider
        .to_ir(tree, Arc::from("   \n   \n"), PathBuf::from("blank.py"))
        .expect_err("whitespace-only source must error");
    assert!(matches!(err, IrConvertError::EmptySource));
}

#[test]
fn t2c_structural_invariant_variant_carries_kind_and_message() {
    // Programmer-error variant — pin its payload + Display so a
    // detector that surfaces it (R-1.c) can route on the kind.
    let err = IrConvertError::StructuralInvariant {
        kind: "function_item",
        message: "missing `name` field".to_string(),
    };
    let rendered = format!("{err}");
    assert!(rendered.contains("function_item"));
    assert!(rendered.contains("missing `name`"));
    match err {
        IrConvertError::StructuralInvariant { kind, message } => {
            assert_eq!(kind, "function_item");
            assert_eq!(message, "missing `name` field");
        }
        other => panic!("expected StructuralInvariant, got {other:?}"),
    }
}
