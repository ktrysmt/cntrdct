//! cntrdct-config — `cntrdct.toml` parsing, in-source suppression scanning,
//! and the filter that combines both with a `Vec<Finding>` produced by Layer 1.
//!
//! Spec: `cntrdct/docs/spec/suppression-v0.md` (T2-7).
//!
//! Surface:
//! - [`Config`] mirrors the TOML schema and is `Default` so callers without a
//!   config file get the empty-overrides baseline.
//! - [`Config::discover_in`] looks for `cntrdct.toml` in the supplied scan
//!   root and returns `Ok(None)` when the file is absent (a missing file is
//!   not an error).
//! - [`apply`] performs the actual filter / remap pass: path globs, attribute
//!   suppressions found via tree-sitter, per-detector enable, per-detector
//!   severity remap.
//!
//! In-source suppression syntax:
//!
//! - Rust: `#[cntrdct::allow(detector_id, …)]` on the item containing the
//!   finding. Empty argument list (`#[cntrdct::allow()]`) suppresses every
//!   detector for that item.
//! - Python (Q-9): `# cntrdct: allow(detector_id, …)` line comment.
//!   Trailing form (`code()  # cntrdct: allow(arg-swap)`) suppresses
//!   findings whose `start_line` equals the comment's line. Standalone
//!   form (a whole-line comment) suppresses the next non-comment
//!   sibling statement / definition's full span — mirroring the Rust
//!   attribute-precedes-item shape at line granularity. `# cntrdct: allow()`
//!   is the Python catch-all.

#![deny(missing_docs)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::{Finding, Language, Severity};
use crate::ir::IrFile;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use thiserror::Error;

/// Errors raised while loading or applying a config file.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// I/O failure reading the config file.
    #[error("io error reading {path}: {source}")]
    Io {
        /// File the error occurred against.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// TOML parse failure.
    #[error("toml parse error in {path}: {source}")]
    Parse {
        /// File the error occurred against.
        path: PathBuf,
        /// Underlying serde / TOML error.
        #[source]
        source: toml::de::Error,
    },
    /// One of the configured glob patterns is invalid.
    #[error("invalid glob `{pattern}`: {source}")]
    Glob {
        /// Pattern string that failed to compile.
        pattern: String,
        /// Underlying globset error.
        #[source]
        source: globset::Error,
    },
}

/// Default config file name expected at the scan-root.
pub const CONFIG_FILE: &str = "cntrdct.toml";

/// Parsed `cntrdct.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Per-detector overrides keyed by `detector_id`.
    #[serde(default)]
    pub detectors: HashMap<String, DetectorOverride>,
    /// Path-based include / exclude globs.
    #[serde(default)]
    pub paths: PathRules,
    /// Per-language overrides keyed by canonical language name
    /// (e.g. `"rust"`, `"python"`). Spec: M-5 (`docs/spec/multilang-v0.md`).
    ///
    /// Two effects:
    /// - `enabled = false` causes the file walker to skip files of that
    ///   language (discovery control).
    /// - `suppress = ["<id>", ...]` drops findings whose primary file is in
    ///   this language and whose `detector_id` is in the list. Equivalent
    ///   in spirit to `[detectors.<id>] enabled = false` but scoped to a
    ///   single language so a detector can stay on for Rust while being
    ///   silenced on Python (or vice versa).
    #[serde(default)]
    pub languages: HashMap<String, LanguageOverride>,
}

/// Per-detector overrides.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorOverride {
    /// `Some(false)` drops every finding from this detector.
    /// `None` and `Some(true)` are both treated as enabled.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Remap the detector's `raw_severity` on every emitted finding.
    #[serde(default)]
    pub severity: Option<SeverityName>,
}

/// Per-language overrides.
///
/// Section is optional; absent / empty means "every language enabled, no
/// per-language suppression". Unknown language keys are accepted but
/// ineffective — the walker only consults the override for languages
/// `cntrdct-parsers` already knows about, so a typo (e.g. `[languages.ruby]`)
/// does not silently disable scanning. (Detection of typos is out of scope
/// for v0; consider a `cntrdct check` lint later.)
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageOverride {
    /// `Some(false)` instructs the file walker to skip files of this language.
    /// `None` and `Some(true)` are both treated as enabled.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Detector IDs whose findings are dropped on files of this language.
    /// Empty / absent = no per-language suppression.
    #[serde(default)]
    pub suppress: Vec<String>,
}

/// Path-based filter rules.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathRules {
    /// Findings whose primary file matches any `exclude` glob are dropped.
    /// Exclusion always wins over inclusion.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// When non-empty, findings whose primary file does NOT match any
    /// `include` glob are dropped. Empty means "include everything".
    #[serde(default)]
    pub include: Vec<String>,
}

/// Severity names accepted in `cntrdct.toml`. Mirrors `crate::core::Severity`
/// but is independently `Deserialize` so the surface stays declarative.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityName {
    /// Informational.
    Info,
    /// Note.
    Note,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

impl From<SeverityName> for Severity {
    fn from(s: SeverityName) -> Self {
        match s {
            SeverityName::Info => Severity::Info,
            SeverityName::Note => Severity::Note,
            SeverityName::Warning => Severity::Warning,
            SeverityName::Error => Severity::Error,
        }
    }
}

impl Config {
    /// Read a `cntrdct.toml` from `path`. Always returns an error on missing
    /// file — see [`Config::discover_in`] for the silent-fallback variant.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let body = fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        toml::from_str(&body).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            source: e,
        })
    }

    /// Look for `<root>/cntrdct.toml`. Returns `Ok(None)` when no such file
    /// exists; an existing-but-unreadable file produces `Err`.
    pub fn discover_in(root: &Path) -> Result<Option<Self>, ConfigError> {
        let candidate = root.join(CONFIG_FILE);
        if !candidate.exists() {
            return Ok(None);
        }
        Self::load_from(&candidate).map(Some)
    }

    /// `true` if the file walker should scan files of `lang`. Defaults to
    /// `true` when no override is present (the implicit "everything on"
    /// stance — explicit opt-out is required to disable a language).
    pub fn language_enabled(&self, lang: Language) -> bool {
        self.languages
            .get(lang.canonical_name())
            .and_then(|o| o.enabled)
            .unwrap_or(true)
    }

    /// `true` if the supplied `detector_id` is listed under
    /// `[languages.<canonical>] suppress = [...]` for `lang`. Returns
    /// `false` when no such override exists.
    pub fn language_suppresses_detector(&self, lang: Language, detector_id: &str) -> bool {
        self.languages
            .get(lang.canonical_name())
            .map(|o| o.suppress.iter().any(|d| d == detector_id))
            .unwrap_or(false)
    }
}

/// In-source suppression discovered for a single item.
///
/// `detector_ids` is `None` for the catch-all `#[cntrdct::allow()]` form and
/// `Some(ids)` for explicit `#[cntrdct::allow(<id>, ...)]`.
#[derive(Debug, Clone)]
pub struct AttributeSuppression {
    /// Detectors suppressed for the item; `None` means "all detectors".
    pub detector_ids: Option<Vec<String>>,
    /// Inclusive 1-based start line of the suppressed item.
    pub start_line: u32,
    /// Inclusive 1-based end line of the suppressed item.
    pub end_line: u32,
}

/// Walk `file` and collect every in-source suppression. Dispatches on
/// `file.language`:
///
/// - [`Language::Rust`]: collect `#[cntrdct::allow(...)]` attributes
///   on top-level items (function, struct, enum, impl, trait, mod).
///   The attribute must appear immediately above the item in the same
///   parse tree; nested or detached attributes are ignored.
/// - [`Language::Python`]: collect `# cntrdct: allow(...)` line
///   comments. See module-level docs for the trailing / standalone
///   semantics.
pub fn collect_attribute_suppressions(file: &IrFile) -> Vec<AttributeSuppression> {
    match file.language {
        Language::Rust => collect_rust_suppressions(file),
        Language::Python => collect_python_suppressions(file),
        // TypeScript suppression syntax (R-2.d) is not yet modelled;
        // no attribute-based suppressions are collected for TS files.
        Language::TypeScript => Vec::new(),
        // Go suppression syntax is not yet modelled (R-3); no
        // attribute-based suppressions are collected for Go files.
        Language::Go => Vec::new(),
    }
}

fn collect_rust_suppressions(file: &IrFile) -> Vec<AttributeSuppression> {
    if file.parse_recovered {
        return vec![];
    }
    let raw_tree = file.raw_tree();
    let root = raw_tree.root_node();

    let mut out = Vec::new();
    let mut cursor = root.walk();
    let children: Vec<tree_sitter::Node> = root.children(&mut cursor).collect();

    for (idx, node) in children.iter().enumerate() {
        if !is_top_level_item(node) {
            continue;
        }
        if let Some(supp) = scan_preceding_attributes(&children, idx, &file.source, node) {
            out.push(supp);
        }
        // Also check inner attributes attached to the item itself
        // (for example, mod foo { #![cntrdct::allow(...)] }).
        if let Some(supp) = scan_inner_attributes(node, &file.source) {
            out.push(supp);
        }
    }

    out
}

/// Q-9: collect `# cntrdct: allow(...)` line-comment suppressions from a
/// Python source file.
///
/// Two recognised forms:
///
/// - Trailing comment (`code()  # cntrdct: allow(<id>)`): suppression
///   range is the single line carrying the comment. Detected by
///   inspecting the bytes between the start of the line and the
///   comment's start byte; if any non-whitespace byte is present, the
///   comment is treated as trailing.
/// - Standalone comment (whole line is the comment): suppression range
///   spans the next named sibling whose kind is not `comment`. This
///   mirrors the Rust pattern where the `#[cntrdct::allow(...)]`
///   attribute applies to the immediately following item; intervening
///   blank lines and additional `# cntrdct: allow(...)` lines are
///   tolerated and stack onto the same target.
///
/// Empty argument list (`# cntrdct: allow()`) is the catch-all that
/// suppresses every detector on the suppression range, matching the
/// Rust attribute's empty-form semantics.
fn collect_python_suppressions(file: &IrFile) -> Vec<AttributeSuppression> {
    let raw_tree = file.raw_tree();
    let root = raw_tree.root_node();
    // Unlike Rust's hard `has_error` bail, Python source with a single
    // misindented stretch can still carry well-formed suppression
    // comments earlier in the file. tree-sitter recovers locally;
    // walking the tree and collecting comment nodes is safe even with
    // partial errors. We only bail when no tree was returned at all.
    let mut comments: Vec<tree_sitter::Node> = Vec::new();
    collect_python_comment_nodes(root, &mut comments);

    let mut out = Vec::new();
    for comment in comments {
        let Some(parsed) = parse_python_allow_comment(&comment, &file.source) else {
            continue;
        };
        if is_python_trailing_comment(&comment, &file.source) {
            let line = comment.start_position().row as u32 + 1;
            out.push(AttributeSuppression {
                detector_ids: match parsed {
                    ParsedAllow::All => None,
                    ParsedAllow::List(ids) => Some(ids),
                },
                start_line: line,
                end_line: line,
            });
        } else if let Some(target) = next_non_comment_named_sibling(comment) {
            let start = target.start_position().row as u32 + 1;
            let end = target.end_position().row as u32 + 1;
            out.push(AttributeSuppression {
                detector_ids: match parsed {
                    ParsedAllow::All => None,
                    ParsedAllow::List(ids) => Some(ids),
                },
                start_line: start,
                end_line: end,
            });
        }
    }

    out
}

fn collect_python_comment_nodes<'a>(
    node: tree_sitter::Node<'a>,
    out: &mut Vec<tree_sitter::Node<'a>>,
) {
    if node.kind() == "comment" {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_python_comment_nodes(child, out);
    }
}

fn is_python_trailing_comment(comment: &tree_sitter::Node, source: &str) -> bool {
    let bytes = source.as_bytes();
    let comment_start = comment.start_byte();
    // Walk back to the start of the line.
    let mut line_start = comment_start;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    bytes[line_start..comment_start]
        .iter()
        .any(|b| !b.is_ascii_whitespace())
}

fn next_non_comment_named_sibling(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut sib = node.next_named_sibling();
    while let Some(s) = sib {
        if s.kind() != "comment" {
            return Some(s);
        }
        sib = s.next_named_sibling();
    }
    None
}

fn parse_python_allow_comment(node: &tree_sitter::Node, source: &str) -> Option<ParsedAllow> {
    let text = node_text(node, source);
    // Strip the leading `#` and any whitespace; tolerate `## cntrdct:` /
    // shebang-ish forms by collapsing extra `#` characters.
    let body = text.trim_start_matches('#').trim();
    // Accept either `cntrdct: allow(...)` or `cntrdct:allow(...)`.
    let body = body.strip_prefix("cntrdct:")?.trim_start();
    let body = body.strip_prefix("allow")?.trim_start();
    let body = body.strip_prefix('(')?;
    let close = body.find(')')?;
    let args = body[..close].trim();

    if args.is_empty() {
        return Some(ParsedAllow::All);
    }

    let ids: Vec<String> = args
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if ids.is_empty() {
        Some(ParsedAllow::All)
    } else {
        Some(ParsedAllow::List(ids))
    }
}

fn is_top_level_item(node: &tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "function_item"
            | "struct_item"
            | "enum_item"
            | "impl_item"
            | "trait_item"
            | "mod_item"
            | "const_item"
            | "static_item"
            | "type_item"
    )
}

fn scan_preceding_attributes(
    children: &[tree_sitter::Node],
    idx: usize,
    source: &str,
    item: &tree_sitter::Node,
) -> Option<AttributeSuppression> {
    let mut found: Option<Vec<String>> = None;
    let mut empty_form = false;
    let mut walker = idx;
    while walker > 0 {
        walker -= 1;
        let prev = &children[walker];
        if prev.kind() != "attribute_item" {
            break;
        }
        if let Some(parsed) = parse_cntrdct_allow(prev, source) {
            match parsed {
                ParsedAllow::All => empty_form = true,
                ParsedAllow::List(ids) => match &mut found {
                    Some(existing) => existing.extend(ids),
                    None => found = Some(ids),
                },
            }
        }
    }

    if !empty_form && found.is_none() {
        return None;
    }

    let start = item.start_position().row as u32 + 1;
    let end = item.end_position().row as u32 + 1;
    Some(AttributeSuppression {
        detector_ids: if empty_form { None } else { found },
        start_line: start,
        end_line: end,
    })
}

fn scan_inner_attributes(item: &tree_sitter::Node, source: &str) -> Option<AttributeSuppression> {
    let mut cursor = item.walk();
    let mut found: Option<Vec<String>> = None;
    let mut empty_form = false;
    for child in item.children(&mut cursor) {
        if child.kind() != "inner_attribute_item" {
            continue;
        }
        if let Some(parsed) = parse_cntrdct_allow(&child, source) {
            match parsed {
                ParsedAllow::All => empty_form = true,
                ParsedAllow::List(ids) => match &mut found {
                    Some(existing) => existing.extend(ids),
                    None => found = Some(ids),
                },
            }
        }
    }
    if !empty_form && found.is_none() {
        return None;
    }
    let start = item.start_position().row as u32 + 1;
    let end = item.end_position().row as u32 + 1;
    Some(AttributeSuppression {
        detector_ids: if empty_form { None } else { found },
        start_line: start,
        end_line: end,
    })
}

enum ParsedAllow {
    All,
    List(Vec<String>),
}

fn parse_cntrdct_allow(attr_node: &tree_sitter::Node, source: &str) -> Option<ParsedAllow> {
    let text = node_text(attr_node, source);
    // `attribute_item` text includes the surrounding `#[ ... ]`. Inside it
    // tree-sitter exposes an `attribute` child whose first segment is the
    // path. Accept either `cntrdct::allow` or just `allow` if the user has a
    // top-level `use cntrdct;` (the latter we detect lexically — cheap and
    // good enough for v0).
    let stripped = text
        .trim_start_matches('#')
        .trim_start_matches('!')
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();

    let (head, args) = match stripped.split_once('(') {
        Some((h, rest)) => (h.trim(), rest.trim_end_matches(')').trim()),
        None => (stripped.trim(), ""),
    };

    if head != "cntrdct::allow" {
        return None;
    }

    if args.is_empty() {
        return Some(ParsedAllow::All);
    }

    let ids: Vec<String> = args
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if ids.is_empty() {
        Some(ParsedAllow::All)
    } else {
        Some(ParsedAllow::List(ids))
    }
}

fn node_text(node: &tree_sitter::Node, source: &str) -> String {
    let bytes = source.as_bytes();
    let start = node.start_byte();
    let end = node.end_byte();
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// Apply a `Config` plus the in-source attribute suppressions found in
/// `files` to `findings`. Returns the surviving (and possibly remapped)
/// findings in the same input order.
pub fn apply(
    config: &Config,
    files: &[IrFile],
    findings: Vec<Finding>,
) -> Result<Vec<Finding>, ConfigError> {
    let exclude_set = build_globset(&config.paths.exclude)?;
    let include_set = build_globset(&config.paths.include)?;
    let include_active = !config.paths.include.is_empty();

    let suppressions: HashMap<PathBuf, Vec<AttributeSuppression>> = files
        .iter()
        .map(|f| (f.path.clone(), collect_attribute_suppressions(f)))
        .collect();

    let language_by_path: HashMap<PathBuf, Language> =
        files.iter().map(|f| (f.path.clone(), f.language)).collect();

    let mut out = Vec::with_capacity(findings.len());
    for mut finding in findings {
        // Path filter.
        let primary = &finding.primary.file;
        if exclude_set.is_match(primary) {
            continue;
        }
        if include_active && !include_set.is_match(primary) {
            continue;
        }

        // Detector enable.
        if let Some(over) = config.detectors.get(&finding.detector_id) {
            if matches!(over.enabled, Some(false)) {
                continue;
            }
        }

        // Per-language suppression. The lookup goes through the
        // `language_by_path` map built from the supplied `files` slice
        // — callers that pass an empty `files` slice get no per-language
        // filtering (this matches the historical `apply(&cfg, &[], …)` test
        // shape, which we keep working for the suppression-free case).
        if let Some(lang) = language_by_path.get(primary) {
            if config.language_suppresses_detector(*lang, &finding.detector_id) {
                continue;
            }
        }

        // In-source attribute suppression.
        if let Some(sups) = suppressions.get(primary) {
            let suppressed = sups.iter().any(|s| {
                let line = finding.primary.start_line;
                if line < s.start_line || line > s.end_line {
                    return false;
                }
                match &s.detector_ids {
                    None => true,
                    Some(ids) => ids.iter().any(|id| id == &finding.detector_id),
                }
            });
            if suppressed {
                continue;
            }
        }

        // Severity remap (applied after all filters).
        if let Some(over) = config.detectors.get(&finding.detector_id) {
            if let Some(sev) = over.severity {
                finding.raw_severity = sev.into();
            }
        }

        out.push(finding);
    }

    Ok(out)
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, ConfigError> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let g = Glob::new(p).map_err(|e| ConfigError::Glob {
            pattern: p.clone(),
            source: e,
        })?;
        b.add(g);
    }
    b.build().map_err(|e| ConfigError::Glob {
        pattern: "<built>".to_string(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        AnomalyClass, Evidence, Finding, LanguageCitationStatus, Location, Severity,
    };
    use std::path::PathBuf;

    fn parsed_file(name: &str, body: &str) -> IrFile {
        build_ir_for_test(name, Language::Rust, body)
    }

    fn parsed_python(name: &str, body: &str) -> IrFile {
        build_ir_for_test(name, Language::Python, body)
    }

    fn build_ir_for_test(name: &str, lang: Language, body: &str) -> IrFile {
        use std::sync::Arc;
        let provider = crate::parsers::parser_for(lang);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&provider.ts_language())
            .expect("set language");
        let tree = parser.parse(body, None).expect("parse source");
        let source: Arc<str> = Arc::from(body);
        match provider.to_ir(tree, source.clone(), PathBuf::from(name)) {
            Ok(ir) => ir,
            Err(_) => {
                // For tests with empty Python `source: String::new()`
                // (the per-language suppression case) the body is
                // intentionally blank. Construct a minimal IrFile
                // directly so the test can drive `apply()` without
                // tripping the EmptySource gate.
                // Lazy raw_tree: reparse-on-demand from `source` (R1
                // mitigation). The empty-source fallback supplies a
                // tiny placeholder body so the suppression test path
                // does not pass blank input to the parser.
                let placeholder = match lang {
                    Language::Rust => "fn _x() {}\n",
                    Language::Python => "def _x():\n    pass\n",
                    Language::TypeScript => "function _x() {}\n",
                    Language::Go => "package main\nfunc _x() {}\n",
                };
                let source_for_ir: Arc<str> = if body.trim().is_empty() {
                    Arc::from(placeholder)
                } else {
                    Arc::from(body)
                };
                IrFile {
                    path: PathBuf::from(name),
                    language: lang,
                    source: source_for_ir,
                    fns: Vec::new(),
                    top_level_comments: Vec::new(),
                    parse_recovered: false,
                }
            }
        }
    }

    fn finding_at(detector: &str, file: &str, line: u32) -> Finding {
        Finding {
            detector_id: detector.to_string(),
            primary: Location {
                file: PathBuf::from(file),
                start_line: line,
                start_col: 1,
                end_line: line,
                end_col: 1,
            },
            related: vec![],
            message: "demo".to_string(),
            raw_severity: Severity::Warning,
            anomaly_class: AnomalyClass::Logic,
            evidence: Evidence {
                citation_keys: vec![],
                raw: serde_json::Value::Null,
                language_citation_status: LanguageCitationStatus::Confirmed,
            },
        }
    }

    #[test]
    fn empty_config_passes_findings_through() {
        let cfg = Config::default();
        let f = finding_at("clone-drift", "a.rs", 10);
        let out = apply(&cfg, &[], vec![f.clone()]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].detector_id, "clone-drift");
    }

    #[test]
    fn detector_disabled_drops_all_its_findings() {
        let cfg: Config = toml::from_str(
            r#"
            [detectors.clone-drift]
            enabled = false
            "#,
        )
        .unwrap();
        let kept = finding_at("arg-swap", "a.rs", 1);
        let dropped = finding_at("clone-drift", "a.rs", 2);
        let out = apply(&cfg, &[], vec![kept.clone(), dropped]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].detector_id, "arg-swap");
    }

    #[test]
    fn severity_remap_changes_raw_severity_in_place() {
        let cfg: Config = toml::from_str(
            r#"
            [detectors.clone-drift]
            severity = "error"
            "#,
        )
        .unwrap();
        let f = finding_at("clone-drift", "a.rs", 1);
        let out = apply(&cfg, &[], vec![f]).unwrap();
        assert!(matches!(out[0].raw_severity, Severity::Error));
    }

    #[test]
    fn attribute_suppression_for_specific_detector() {
        let body =
            "#[cntrdct::allow(unreachable-after-terminator)]\nfn dead() { return; let _ = 1; }\n";
        let pf = parsed_file("a.rs", body);
        let sups = collect_attribute_suppressions(&pf);
        assert_eq!(sups.len(), 1);
        assert_eq!(
            sups[0].detector_ids.as_deref(),
            Some(&["unreachable-after-terminator".to_string()][..])
        );
        assert_eq!(sups[0].start_line, 2);

        let f = finding_at("unreachable-after-terminator", "a.rs", 2);
        let out = apply(&Config::default(), &[pf], vec![f]).unwrap();
        assert!(out.is_empty(), "attribute should suppress the finding");
    }

    #[test]
    fn attribute_suppression_with_empty_arg_suppresses_all() {
        let body = "#[cntrdct::allow()]\nfn anything() {}\n";
        let pf = parsed_file("a.rs", body);
        let f1 = finding_at("clone-drift", "a.rs", 2);
        let f2 = finding_at("arg-swap", "a.rs", 2);
        let out = apply(&Config::default(), &[pf], vec![f1, f2]).unwrap();
        assert!(out.is_empty(), "catch-all allow should drop everything");
    }

    #[test]
    fn attribute_suppression_does_not_match_other_detectors() {
        let body = "#[cntrdct::allow(arg-swap)]\nfn somewhere() {}\n";
        let pf = parsed_file("a.rs", body);
        let f = finding_at("clone-drift", "a.rs", 2);
        let out = apply(&Config::default(), &[pf], vec![f]).unwrap();
        assert_eq!(out.len(), 1, "non-matching detector_id stays");
    }

    // ---------- Q-9: Python attribute-style suppression ----------

    #[test]
    fn python_trailing_allow_suppresses_finding_on_same_line() {
        let body = "x = copy(src, dst)  # cntrdct: allow(arg-swap)\n";
        let pf = parsed_python("a.py", body);
        let sups = collect_attribute_suppressions(&pf);
        assert_eq!(sups.len(), 1, "expected one suppression; got {:?}", sups);
        assert_eq!(sups[0].start_line, 1);
        assert_eq!(sups[0].end_line, 1);
        assert_eq!(
            sups[0].detector_ids.as_deref(),
            Some(&["arg-swap".to_string()][..])
        );

        let f = finding_at("arg-swap", "a.py", 1);
        let out = apply(&Config::default(), &[pf], vec![f]).unwrap();
        assert!(
            out.is_empty(),
            "trailing comment must drop same-line finding"
        );
    }

    #[test]
    fn python_standalone_allow_covers_following_def_span() {
        let body = "\
# cntrdct: allow(unreachable-after-terminator)
def dead():
    return 1
    print(\"unreachable\")
";
        let pf = parsed_python("a.py", body);
        let sups = collect_attribute_suppressions(&pf);
        assert_eq!(sups.len(), 1, "expected one suppression; got {:?}", sups);
        // Comment is on line 1; `def dead` starts on line 2 and the
        // unreachable `print` lives on line 4. The suppression range
        // must cover line 4.
        assert_eq!(sups[0].start_line, 2);
        assert!(sups[0].end_line >= 4);
        assert_eq!(
            sups[0].detector_ids.as_deref(),
            Some(&["unreachable-after-terminator".to_string()][..])
        );

        let f = finding_at("unreachable-after-terminator", "a.py", 4);
        let out = apply(&Config::default(), &[pf], vec![f]).unwrap();
        assert!(out.is_empty(), "standalone comment must cover def's body");
    }

    #[test]
    fn python_allow_empty_form_is_catch_all() {
        let body = "\
# cntrdct: allow()
def anything():
    return 1
";
        let pf = parsed_python("a.py", body);
        let sups = collect_attribute_suppressions(&pf);
        assert_eq!(sups.len(), 1);
        assert!(
            sups[0].detector_ids.is_none(),
            "empty form means all detectors"
        );

        let f1 = finding_at("clone-drift", "a.py", 2);
        let f2 = finding_at("arg-swap", "a.py", 3);
        let out = apply(&Config::default(), &[pf], vec![f1, f2]).unwrap();
        assert!(
            out.is_empty(),
            "catch-all must drop every detector on the def"
        );
    }

    #[test]
    fn python_allow_does_not_match_other_detectors() {
        let body = "\
# cntrdct: allow(arg-swap)
def helper():
    return 1
";
        let pf = parsed_python("a.py", body);
        let f = finding_at("clone-drift", "a.py", 2);
        let out = apply(&Config::default(), &[pf], vec![f]).unwrap();
        assert_eq!(
            out.len(),
            1,
            "non-matching detector_id must survive the suppression"
        );
    }

    #[test]
    fn python_no_suppression_on_unrelated_comment() {
        let body = "\
# this is just a normal comment
def helper():
    return 1
";
        let pf = parsed_python("a.py", body);
        let sups = collect_attribute_suppressions(&pf);
        assert!(
            sups.is_empty(),
            "regular comments must not register as suppressions"
        );
    }

    #[test]
    fn python_inside_block_standalone_allow_targets_next_statement() {
        // Mirrors the FindBugs-style local suppression: the
        // `# cntrdct: allow(...)` line sits inside a function body and
        // applies to the immediately following statement only.
        let body = "\
def f():
    x = 1
    # cntrdct: allow(unreachable-after-terminator)
    return x
    y = 2
";
        let pf = parsed_python("a.py", body);
        let sups = collect_attribute_suppressions(&pf);
        // The suppression must cover line 4 (the `return x`); the
        // dead `y = 2` on line 5 is outside the targeted statement and
        // therefore stays exposed — matching the Rust attribute model
        // where `#[cntrdct::allow(...)]` covers exactly the next item.
        assert!(
            sups.iter().any(|s| s.start_line <= 4 && s.end_line >= 4),
            "no suppression covers line 4: {:?}",
            sups
        );
    }

    #[test]
    fn exclude_glob_drops_findings_in_matching_paths() {
        let cfg: Config = toml::from_str(
            r#"
            [paths]
            exclude = ["benchmarks/**"]
            "#,
        )
        .unwrap();
        let f1 = finding_at("clone-drift", "src/foo.rs", 10);
        let f2 = finding_at("clone-drift", "benchmarks/corpus/x.rs", 10);
        let out = apply(&cfg, &[], vec![f1.clone(), f2]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].primary.file, f1.primary.file);
    }

    #[test]
    fn include_glob_drops_unmatched() {
        let cfg: Config = toml::from_str(
            r#"
            [paths]
            include = ["src/**"]
            "#,
        )
        .unwrap();
        let in_src = finding_at("clone-drift", "src/foo.rs", 10);
        let out_src = finding_at("clone-drift", "tests/foo.rs", 10);
        let out = apply(&cfg, &[], vec![in_src.clone(), out_src]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].primary.file, in_src.primary.file);
    }

    #[test]
    fn languages_section_default_enables_every_language() {
        let cfg = Config::default();
        assert!(cfg.language_enabled(Language::Rust));
        assert!(cfg.language_enabled(Language::Python));
    }

    #[test]
    fn languages_section_disables_python() {
        let cfg: Config = toml::from_str(
            r#"
            [languages.python]
            enabled = false
            "#,
        )
        .unwrap();
        assert!(cfg.language_enabled(Language::Rust));
        assert!(!cfg.language_enabled(Language::Python));
    }

    #[test]
    fn languages_section_explicit_enable_true_is_identity() {
        let cfg: Config = toml::from_str(
            r#"
            [languages.rust]
            enabled = true
            "#,
        )
        .unwrap();
        assert!(cfg.language_enabled(Language::Rust));
    }

    #[test]
    fn per_language_suppress_drops_findings_only_in_that_language() {
        let cfg: Config = toml::from_str(
            r#"
            [languages.python]
            suppress = ["clone-drift"]
            "#,
        )
        .unwrap();
        let py = build_ir_for_test("a.py", Language::Python, "");
        let rs = build_ir_for_test("a.rs", Language::Rust, "");
        let f_py = finding_at("clone-drift", "a.py", 1);
        let f_rs = finding_at("clone-drift", "a.rs", 1);
        let out = apply(&cfg, &[py, rs], vec![f_py, f_rs]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].primary.file, PathBuf::from("a.rs"));
    }

    #[test]
    fn per_language_suppress_does_not_affect_other_detectors() {
        let cfg: Config = toml::from_str(
            r#"
            [languages.python]
            suppress = ["clone-drift"]
            "#,
        )
        .unwrap();
        let py = build_ir_for_test("a.py", Language::Python, "");
        let f_other = finding_at("arg-swap", "a.py", 1);
        let out = apply(&cfg, &[py], vec![f_other.clone()]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].detector_id, "arg-swap");
    }

    #[test]
    fn language_suppresses_detector_helper() {
        let cfg: Config = toml::from_str(
            r#"
            [languages.python]
            suppress = ["clone-drift", "arg-swap"]
            "#,
        )
        .unwrap();
        assert!(cfg.language_suppresses_detector(Language::Python, "clone-drift"));
        assert!(cfg.language_suppresses_detector(Language::Python, "arg-swap"));
        assert!(!cfg.language_suppresses_detector(Language::Python, "comment-code"));
        assert!(!cfg.language_suppresses_detector(Language::Rust, "clone-drift"));
    }

    #[test]
    fn discover_in_returns_none_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::discover_in(dir.path()).unwrap();
        assert!(cfg.is_none());
    }

    #[test]
    fn discover_in_loads_present_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("cntrdct.toml"),
            r#"
            [detectors.arg-swap]
            enabled = false
            "#,
        )
        .unwrap();
        let cfg = Config::discover_in(dir.path()).unwrap().unwrap();
        assert_eq!(
            cfg.detectors.get("arg-swap").and_then(|o| o.enabled),
            Some(false)
        );
    }
}
