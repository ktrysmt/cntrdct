//! ir-v0.md §F6 T3 — `IrFile.parse_recovered` carry-through.
//!
//! A deliberately-broken source per language must:
//!
//! 1. NOT return `EmptySource` (the source is non-empty) /
//!    `LanguageMismatch` (the tree language matches), and
//! 2. produce an `IrFile` whose `parse_recovered == true`.
//!
//! Cross-cutting detectors gate on `parse_recovered` to preserve the
//! v0.5.x "skip files with parse errors" behaviour; the detector-side
//! skip assertion lives downstream of R-1.c, when detectors actually
//! consume `IrFile`. This test guards the converter contract.
//!
//! The LSP integration smoke test (`didChange` with a broken buffer,
//! diagnostics from prior parse remain visible) also lives downstream
//! of R-1.c, when the `scan_buffer` signature migrates to `IrFile`.

use std::path::PathBuf;
use std::sync::Arc;

use cntrdct::parsers::{parser_for, Language};

fn parse_with(lang: Language, source: &str) -> tree_sitter::Tree {
    let provider = parser_for(lang);
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&provider.ts_language())
        .expect("set language");
    parser.parse(source, None).expect("parse")
}

#[test]
fn t3_rust_broken_source_carries_parse_recovered_true() {
    // Unbalanced parens — tree-sitter recovers, root_node().has_error()
    // is true, the converter still returns an IrFile but marks
    // parse_recovered.
    let broken = "fn main( {\n    foo();\n}\n";
    let tree = parse_with(Language::Rust, broken);
    let provider = parser_for(Language::Rust);
    let ir = provider
        .to_ir(tree, Arc::from(broken), PathBuf::from("broken.rs"))
        .expect("converter still produces an IrFile under partial parse");
    assert!(ir.parse_recovered, "syntax error must set parse_recovered");
}

#[test]
fn t3_python_broken_source_carries_parse_recovered_true() {
    let broken = "def main(:\n    pass\n";
    let tree = parse_with(Language::Python, broken);
    let provider = parser_for(Language::Python);
    let ir = provider
        .to_ir(tree, Arc::from(broken), PathBuf::from("broken.py"))
        .expect("converter still produces an IrFile under partial parse");
    assert!(ir.parse_recovered, "syntax error must set parse_recovered");
}

#[test]
fn t3_clean_source_marks_parse_recovered_false() {
    let clean = "fn main() {}\n";
    let tree = parse_with(Language::Rust, clean);
    let provider = parser_for(Language::Rust);
    let ir = provider
        .to_ir(tree, Arc::from(clean), PathBuf::from("clean.rs"))
        .expect("clean parse succeeds");
    assert!(
        !ir.parse_recovered,
        "clean parse must leave parse_recovered=false"
    );
}
