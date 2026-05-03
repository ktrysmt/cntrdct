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
//! In-source suppression syntax: `#[cntrdct::allow(detector_id, …)]` on the
//! item containing the finding. An empty argument list (`#[cntrdct::allow()]`)
//! suppresses every detector for that item.

#![deny(missing_docs)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use cntrdct_core::{Finding, ParsedFile, Severity};
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

/// Severity names accepted in `cntrdct.toml`. Mirrors `cntrdct_core::Severity`
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

/// Walk `file` and collect every `#[cntrdct::allow(...)]` suppression on a
/// top-level item (function, struct, enum, impl, trait, mod). The attribute
/// must appear immediately above the item in the same parse tree; nested or
/// detached attributes are ignored.
pub fn collect_attribute_suppressions(file: &ParsedFile) -> Vec<AttributeSuppression> {
    if file.language != "rust" {
        return vec![];
    }
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_rust::language()).is_err() {
        return vec![];
    }
    let Some(tree) = parser.parse(&file.source, None) else {
        return vec![];
    };
    let root = tree.root_node();
    if root.has_error() {
        return vec![];
    }

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
    files: &[ParsedFile],
    findings: Vec<Finding>,
) -> Result<Vec<Finding>, ConfigError> {
    let exclude_set = build_globset(&config.paths.exclude)?;
    let include_set = build_globset(&config.paths.include)?;
    let include_active = !config.paths.include.is_empty();

    let suppressions: HashMap<PathBuf, Vec<AttributeSuppression>> = files
        .iter()
        .map(|f| (f.path.clone(), collect_attribute_suppressions(f)))
        .collect();

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
    use cntrdct_core::{AnomalyClass, Evidence, Finding, Location, Severity};
    use std::path::PathBuf;

    fn parsed_file(name: &str, body: &str) -> ParsedFile {
        ParsedFile {
            path: PathBuf::from(name),
            language: "rust".to_string(),
            source: body.to_string(),
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
