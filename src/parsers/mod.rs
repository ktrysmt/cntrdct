//! cntrdct-parsers — the multi-language seam shared across detectors,
//! the CLI, and the corpus tooling.
//!
//! Spec: `docs/spec/multilang-v0.md` for the original `ParserProvider`
//! seam; `docs/spec/ir-v0.md` §F2 for the `to_ir` extension that lands
//! in R-1.b.
//!
//! Owns:
//!
//! 1. [`detect_language`] — extension → language mapping for the CLI
//!    file walker and any other code that needs to assign a parser.
//! 2. [`ParserProvider`] + [`parser_for`] — a thin wrapper around the
//!    per-language tree-sitter language constructor so detectors stop
//!    depending on `tree_sitter_rust` / `tree_sitter_python` directly.
//!    Each provider also converts a `tree_sitter::Tree` into an
//!    [`crate::ir::IrFile`] via [`ParserProvider::to_ir`].
//!
//! [`Language`] itself lives in `cntrdct-core` so `Citation::languages`
//! and `Evidence::language_citation_status` can reference it without
//! pulling tree-sitter into the core dependency graph. This module
//! re-exports it for callers that only depend on `cntrdct-parsers`.
//!
//! Per-language parser logic (tree-sitter language object, IR
//! conversion) lives in [`rust`] and [`python`] submodules. Detector
//! logic (terminator sets, doc-comment patterns, attribute syntax)
//! stays in the detectors themselves; this module only owns the
//! parsing entry point.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use crate::core::Language;
use crate::ir::{IrConvertError, IrFile, IrStmt, IrTerminator};

pub mod go;
pub mod python;
pub mod rust;
pub mod typescript;

pub use go::GoParserProvider;
pub use python::PythonParserProvider;
pub use rust::RustParserProvider;
pub use typescript::{TsxParserProvider, TypeScriptParserProvider};

/// Map a path's extension to a [`Language`]. Returns `None` for
/// extensions cntrdct does not analyse, including extension-less
/// files. Shebang inspection is intentionally out of scope (see
/// multilang-v0.md F2).
pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension().and_then(|s| s.to_str())?;
    match ext {
        "rs" => Some(Language::Rust),
        "py" | "pyi" => Some(Language::Python),
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" => Some(Language::Tsx),
        "go" => Some(Language::Go),
        _ => None,
    }
}

/// Per-language parsing entry point.
///
/// The trait is object-safe so [`parser_for`] can return a boxed
/// instance without monomorphising the call site. Implementations are
/// `Send + Sync`; tree-sitter language constructors are pure functions
/// returning `'static` data.
pub trait ParserProvider: Send + Sync {
    /// The language this provider parses.
    fn language(&self) -> Language;
    /// The tree-sitter language object passed to
    /// `tree_sitter::Parser::set_language`.
    fn ts_language(&self) -> tree_sitter::Language;
    /// Convert a parsed tree-sitter tree into an [`IrFile`].
    ///
    /// Per ir-v0.md §F2: the caller parses with
    /// `tree_sitter::Parser::set_language` followed by
    /// `parser.parse(source, None)` and hands the resulting `Tree` to
    /// `to_ir`. The converter walks the tree to build IR but does not
    /// retain it — language-specific detectors recover the tree via
    /// [`IrFile::raw_tree`], which reparses from
    /// [`IrFile::source`] on demand (R1 mitigation). `source` is
    /// shared as `Arc<str>` so IR nodes can reference it without
    /// cloning.
    ///
    /// `to_ir` is total over the supplied tree's recognised shapes;
    /// unknown nodes become [`crate::ir::IrStmtKind::Other`] or
    /// [`crate::ir::IrExpr::Other`] with `node_kind` + `NodeRef`.
    /// [`IrFile::parse_recovered`] is set from
    /// `tree.root_node().has_error()`.
    ///
    /// Production-runtime behaviour for the three
    /// [`IrConvertError`] variants is documented on the enum.
    fn to_ir(
        &self,
        tree: tree_sitter::Tree,
        source: Arc<str>,
        path: PathBuf,
    ) -> Result<IrFile, IrConvertError>;
}

/// Build an [`IrFile`] shell with the language / source / tree
/// metadata populated. Used by every per-language provider (Rust,
/// Python, TypeScript, `.tsx`, Go) as the prelude to structural
/// conversion, which fills `fns` / `top_level_comments` in place.
pub(crate) fn build_ir_shell<P: ParserProvider + ?Sized>(
    provider: &P,
    tree: &tree_sitter::Tree,
    source: Arc<str>,
    path: PathBuf,
) -> Result<IrFile, IrConvertError> {
    let expected = provider.ts_language();
    let mismatch_actual = {
        let actual = tree.language();
        if *actual != expected {
            Some(format!("{:?}", *actual))
        } else {
            None
        }
    };
    if let Some(actual) = mismatch_actual {
        return Err(IrConvertError::LanguageMismatch {
            expected: provider.language(),
            actual,
        });
    }
    if source.trim().is_empty() {
        return Err(IrConvertError::EmptySource);
    }
    let parse_recovered = tree.root_node().has_error();
    Ok(IrFile {
        path,
        language: provider.language(),
        source,
        fns: Vec::new(),
        top_level_comments: Vec::new(),
        parse_recovered,
    })
}

/// Shared block-terminator rule for the per-language converters
/// (ir-v0.md §F1): scan `statements` in source order; the first
/// statement classified as divergent by the language's
/// `stmt_terminator` determines the block's terminator (everything
/// after it is unreachable). §F1 only requires `Some` when every
/// reachable path through the block ends in a divergent expression; in
/// v0 the straight-line definition (first terminator wins) is
/// sufficient because the cross-cutting detector
/// (`unreachable-after-terminator`) uses this signal to classify the
/// block's own outer position.
///
/// What COUNTS as a terminator stays per-language — Rust classifies
/// `Assert(false)` / `Match` / `Loop`-without-break in addition to the
/// common set, while Go / TypeScript intentionally cover a subset — so
/// each converter supplies its own `stmt_terminator` classifier.
pub(crate) fn first_stmt_terminator(
    statements: &[IrStmt],
    stmt_terminator: impl FnMut(&IrStmt) -> Option<IrTerminator>,
) -> Option<IrTerminator> {
    statements.iter().find_map(stmt_terminator)
}

/// Get the [`ParserProvider`] for a language.
///
/// The returned `Box<dyn ParserProvider>` is cheap to construct
/// (the providers are unit structs); detectors that parse many files
/// in a row can either keep the boxed provider for the duration of
/// their run or call `parser_for` per file.
pub fn parser_for(lang: Language) -> Box<dyn ParserProvider> {
    match lang {
        Language::Rust => Box::new(RustParserProvider),
        Language::Python => Box::new(PythonParserProvider),
        Language::TypeScript => Box::new(TypeScriptParserProvider),
        Language::Tsx => Box::new(TsxParserProvider),
        Language::Go => Box::new(GoParserProvider),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_language_from_extension() {
        assert_eq!(
            detect_language(&PathBuf::from("a.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            detect_language(&PathBuf::from("a.py")),
            Some(Language::Python)
        );
        assert_eq!(
            detect_language(&PathBuf::from("a.pyi")),
            Some(Language::Python)
        );
        assert_eq!(
            detect_language(&PathBuf::from("src/foo/bar.rs")),
            Some(Language::Rust)
        );
        assert_eq!(detect_language(&PathBuf::from("README.md")), None);
        assert_eq!(detect_language(&PathBuf::from("Makefile")), None);
        assert_eq!(detect_language(&PathBuf::from("a.RS")), None);
    }

    #[test]
    fn parser_for_returns_correct_language() {
        for &lang in Language::all() {
            let p = parser_for(lang);
            assert_eq!(p.language(), lang);
        }
    }

    #[test]
    fn parsers_actually_parse_their_language() {
        let mut parser = tree_sitter::Parser::new();
        let p = parser_for(Language::Rust);
        parser
            .set_language(&p.ts_language())
            .expect("set rust language");
        let tree = parser.parse("fn main() {}", None).expect("parse rust");
        assert!(!tree.root_node().has_error());

        let mut parser = tree_sitter::Parser::new();
        let p = parser_for(Language::Python);
        parser
            .set_language(&p.ts_language())
            .expect("set python language");
        let tree = parser
            .parse("def main(): pass\n", None)
            .expect("parse python");
        assert!(!tree.root_node().has_error());

        let mut parser = tree_sitter::Parser::new();
        let p = parser_for(Language::Go);
        parser
            .set_language(&p.ts_language())
            .expect("set go language");
        let tree = parser
            .parse("package main\nfunc main() {}\n", None)
            .expect("parse go");
        assert!(!tree.root_node().has_error());
    }

    fn parse_with(lang: Language, source: &str) -> tree_sitter::Tree {
        let p = parser_for(lang);
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&p.ts_language()).expect("set language");
        parser.parse(source, None).expect("parse")
    }

    #[test]
    fn to_ir_stub_returns_ir_file_for_each_language() {
        for &lang in Language::all() {
            let p = parser_for(lang);
            let source = match lang {
                Language::Rust => "fn main() {}\n",
                Language::Python => "def main():\n    pass\n",
                Language::TypeScript => "function main() {}\n",
                Language::Tsx => "const app = () => <div>{main()}</div>;\n",
                Language::Go => "package main\nfunc main() {}\n",
            };
            let tree = parse_with(lang, source);
            let ir = p
                .to_ir(
                    tree,
                    Arc::from(source),
                    PathBuf::from(format!("a.{}", lang.canonical_name())),
                )
                .expect("to_ir succeeds on clean parse");
            assert_eq!(ir.language, lang);
            assert!(!ir.parse_recovered);
        }
    }

    #[test]
    fn to_ir_returns_empty_source_for_blank_input() {
        let p = parser_for(Language::Rust);
        let tree = parse_with(Language::Rust, "   \n");
        let err = p
            .to_ir(tree, Arc::from("   \n"), PathBuf::from("blank.rs"))
            .expect_err("blank source must error");
        assert!(matches!(err, IrConvertError::EmptySource));
    }

    #[test]
    fn to_ir_returns_language_mismatch_when_tree_is_for_other_language() {
        let p_rust = parser_for(Language::Rust);
        let tree = parse_with(Language::Python, "def main(): pass\n");
        let err = p_rust
            .to_ir(tree, Arc::from("def main(): pass\n"), PathBuf::from("a.py"))
            .expect_err("language-mismatched tree must error");
        match err {
            IrConvertError::LanguageMismatch { expected, .. } => {
                assert_eq!(expected, Language::Rust);
            }
            other => panic!("expected LanguageMismatch, got {other:?}"),
        }
    }

    #[test]
    fn to_ir_marks_parse_recovered_on_syntax_error() {
        let p = parser_for(Language::Python);
        let broken = "def main(:\n    pass\n";
        let tree = parse_with(Language::Python, broken);
        let ir = p
            .to_ir(tree, Arc::from(broken), PathBuf::from("broken.py"))
            .expect("EmptySource not triggered");
        assert!(ir.parse_recovered, "syntax error must set parse_recovered");
    }
}
