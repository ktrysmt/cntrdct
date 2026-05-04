//! cntrdct-parsers — the multi-language seam shared across detectors,
//! the CLI, and the corpus tooling.
//!
//! Spec: `cntrdct/docs/spec/multilang-v0.md`.
//!
//! Owns three things:
//!
//! 1. The [`Language`] enum (every variant cntrdct currently supports).
//! 2. [`detect_language`] — extension → language mapping for the CLI
//!    file walker and any other code that needs to assign a parser.
//! 3. [`ParserProvider`] + [`parser_for`] — a thin wrapper around the
//!    per-language tree-sitter language constructor so detectors stop
//!    depending on `tree_sitter_rust` / `tree_sitter_python` directly.
//!
//! The crate is intentionally minimal. Per-language detector logic
//! (terminator sets, doc-comment patterns, attribute syntax) lives in
//! the detectors themselves; this crate only owns the language
//! identity and the parsing entry point.

#![deny(missing_docs)]

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Languages cntrdct can parse and analyse.
///
/// Marked `#[non_exhaustive]` so downstream `match` expressions must
/// declare a default arm. New variants land one at a time as the
/// M-series adds language support; rebuilds against this crate must
/// continue to compile when a variant is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Language {
    /// Rust source (`.rs`).
    Rust,
    /// Python source (`.py`, `.pyi`).
    Python,
}

impl Language {
    /// Every variant defined today, in declaration order. Useful for
    /// the CLI walker's default "discover everything" behaviour.
    pub fn all() -> &'static [Language] {
        &[Language::Rust, Language::Python]
    }

    /// Canonical lowercase name used in `ParsedFile.language` strings,
    /// `cntrdct.toml` keys, and SARIF output.
    ///
    /// While `ParsedFile.language` remains a `String` (M-1 phase 4a),
    /// detectors and config code should compare against this rather
    /// than hand-rolled literals.
    pub fn canonical_name(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
        }
    }

    /// Inverse of [`canonical_name`]: parses the lowercase name back
    /// into a variant. Returns `None` for any string that does not
    /// name a currently-supported language.
    pub fn from_canonical_name(name: &str) -> Option<Language> {
        match name {
            "rust" => Some(Language::Rust),
            "python" => Some(Language::Python),
            _ => None,
        }
    }
}

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
    fn all_listed_in_canonical_order() {
        assert_eq!(Language::all(), &[Language::Rust, Language::Python]);
    }

    #[test]
    fn canonical_name_round_trip() {
        for &lang in Language::all() {
            let name = lang.canonical_name();
            assert_eq!(Language::from_canonical_name(name), Some(lang));
        }
    }

    #[test]
    fn from_canonical_name_rejects_unknown() {
        assert_eq!(Language::from_canonical_name("rust"), Some(Language::Rust));
        assert_eq!(
            Language::from_canonical_name("python"),
            Some(Language::Python)
        );
        assert_eq!(Language::from_canonical_name("Rust"), None);
        assert_eq!(Language::from_canonical_name(""), None);
        assert_eq!(Language::from_canonical_name("javascript"), None);
    }

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

    #[test]
    fn language_serializes_as_canonical_lowercase() {
        let json = serde_json::to_string(&Language::Rust).unwrap();
        assert_eq!(json, "\"rust\"");
        let json = serde_json::to_string(&Language::Python).unwrap();
        assert_eq!(json, "\"python\"");
    }

    #[test]
    fn language_deserializes_from_canonical_lowercase() {
        let lang: Language = serde_json::from_str("\"rust\"").unwrap();
        assert_eq!(lang, Language::Rust);
        let lang: Language = serde_json::from_str("\"python\"").unwrap();
        assert_eq!(lang, Language::Python);
    }
}
