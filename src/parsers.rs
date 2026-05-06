//! cntrdct-parsers — the multi-language seam shared across detectors,
//! the CLI, and the corpus tooling.
//!
//! Spec: `cntrdct/docs/spec/multilang-v0.md`.
//!
//! Owns:
//!
//! 1. [`detect_language`] — extension → language mapping for the CLI
//!    file walker and any other code that needs to assign a parser.
//! 2. [`ParserProvider`] + [`parser_for`] — a thin wrapper around the
//!    per-language tree-sitter language constructor so detectors stop
//!    depending on `tree_sitter_rust` / `tree_sitter_python` directly.
//!
//! [`Language`] itself lives in `cntrdct-core` so `Citation::languages`
//! and `Evidence::language_citation_status` can reference it without
//! pulling tree-sitter into the core dependency graph. This crate
//! re-exports it for callers that only depend on `cntrdct-parsers`.
//!
//! Per-language detector logic (terminator sets, doc-comment patterns,
//! attribute syntax) lives in the detectors themselves; this crate
//! only owns the parsing entry point.

#![deny(missing_docs)]

use std::path::Path;

pub use crate::core::Language;

/// Map a path's extension to a [`Language`]. Returns `None` for
/// extensions cntrdct does not analyse, including extension-less
/// files. Shebang inspection is intentionally out of scope (see
/// multilang-v0.md F2).
pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension().and_then(|s| s.to_str())?;
    match ext {
        "rs" => Some(Language::Rust),
        "py" | "pyi" => Some(Language::Python),
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
}

/// Provider for Rust source.
pub struct RustParserProvider;

impl ParserProvider for RustParserProvider {
    fn language(&self) -> Language {
        Language::Rust
    }
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_rust::language()
    }
}

/// Provider for Python source.
pub struct PythonParserProvider;

impl ParserProvider for PythonParserProvider {
    fn language(&self) -> Language {
        Language::Python
    }
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_python::language()
    }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
    }
}
