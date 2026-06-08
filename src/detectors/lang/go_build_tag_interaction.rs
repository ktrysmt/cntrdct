//! build-tag-interaction-go detector — flag a Go `//go:build` constraint
//! that is unsatisfiable because it requires a build tag and its negation
//! in the same conjunction (e.g. `//go:build linux && !linux`). The file
//! then never builds for any configuration — the Go analogue of the Rust
//! `config-interaction` `cfg(all(X, not(X)))` contradiction.
//!
//! Language-specific detector under the post-R-1 two-tier layout
//! (`src/detectors/lang/`). Go-only by construction: it parses the Go
//! build-constraint comment grammar. Like `rust_config_interaction` and
//! `python_unreachable_except` it reads source directly (Pattern B,
//! ir-v0.md §F5) — the constraint lives in leading comments, so no AST is
//! needed.
//!
//! Spec: `docs/spec/build-tag-interaction-go-v0.md`.
//!
//! Scope (v0, contradiction only): only the modern `//go:build` line is
//! analysed, and only when its expression contains NO `||` and no negated
//! parenthesis `!(…)` (a De Morgan disjunction). Within that pure-
//! conjunction subset, a tag appearing both positively and under an odd
//! number of `!` makes the whole constraint false. Anything outside the
//! subset is INDETERMINATE and never flagged (precision-first). The legacy
//! `// +build` form and cross-line `//go:build` / `// +build` mismatch are
//! explicit non-goals.

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::IrFile;
use rayon::prelude::*;
use std::collections::HashSet;

static CITATIONS: &[Citation] = &[
    Citation {
        key: "tartler-eurosys-2011",
        authors: "B. Tartler, D. Lohmann, J. Sincero, W. Schröder-Preikschat",
        title: "Feature consistency in compile-time-configurable system software: facing the Linux 10,000 feature problem",
        venue: "EuroSys 2011",
        year: 2011,
        doi: None,
        url: None,
        // Canonical dead-block / inconsistent-feature anomaly class. The Go
        // `//go:build` mechanism is the moral analogue of the C `#ifdef` /
        // KConfig system the paper studies; grounds the concept, not Go.
        languages: &[],
    },
    Citation {
        key: "nadi-icse-2014",
        authors: "S. Nadi, T. Berger, C. Kästner, K. Czarnecki",
        title: "Mining configuration constraints: Static analyses and empirical results",
        venue: "ICSE 2014",
        year: 2014,
        doi: None,
        url: None,
        // Empirical evidence that contradictory configuration predicates
        // recur in production code (subjects: Linux / KConfig). Grounds the
        // concept; Go coverage is Unconfirmed (see survey).
        languages: &[],
    },
];

#[derive(Debug, Default)]
pub struct GoBuildTagInteraction;

impl GoBuildTagInteraction {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for GoBuildTagInteraction {
    fn id(&self) -> &'static str {
        "build-tag-interaction-go"
    }

    fn name(&self) -> &'static str {
        "Go Build Tag Interaction"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [Language] {
        &[Language::Go]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = ctx
            .files
            .par_iter()
            .filter(|f| f.language == Language::Go)
            .flat_map_iter(scan_file)
            .collect();
        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
                .then(a.primary.start_col.cmp(&b.primary.start_col))
        });
        Ok(findings)
    }
}

fn scan_file(file: &IrFile) -> Vec<Finding> {
    if file.parse_recovered {
        return Vec::new();
    }
    let mut out = Vec::new();
    // The `//go:build` line must precede the `package` clause. Scan leading
    // lines only; stop at the package declaration so a `//go:build`-looking
    // string deeper in the file is never misread.
    for (line_idx, raw_line) in file.source.split_inclusive('\n').enumerate() {
        let line = raw_line.trim_end_matches('\n');
        let trimmed = line.trim_start();
        if trimmed.starts_with("package ") || trimmed == "package" {
            break;
        }
        if let Some(expr) = trimmed.strip_prefix("//go:build ") {
            if let Some(conflict) = contradictory_tag(expr) {
                let line_no = (line_idx + 1) as u32;
                let end_col = (line.chars().count() + 1) as u32;
                out.push(make_finding(file, line_no, end_col, &conflict, expr.trim()));
            }
            // Only one `//go:build` line is valid per Go file.
            break;
        }
    }
    out
}

/// Return the tag that appears both positively and negatively in a
/// pure-conjunction `//go:build` expression, or `None` if the expression
/// is satisfiable / outside the decidable v0 subset.
fn contradictory_tag(expr: &str) -> Option<String> {
    let tokens = tokenize(expr)?;
    // Out of the decidable subset: any `||` (disjunction) makes the
    // single-conjunction reasoning unsound, so bail (INDETERMINATE).
    if tokens.iter().any(|t| matches!(t, Token::Or)) {
        return None;
    }
    let mut positive: HashSet<String> = HashSet::new();
    let mut negative: HashSet<String> = HashSet::new();
    let mut pending_negations = 0usize;
    for tok in &tokens {
        match tok {
            Token::Not => pending_negations += 1,
            Token::LParen => {
                // `!( … )` is a De Morgan disjunction — outside the v0
                // decidable subset. A non-negated paren is fine (still a
                // conjunction) and contributes no atom itself.
                if pending_negations % 2 == 1 {
                    return None;
                }
                pending_negations = 0;
            }
            Token::RParen => {}
            Token::And => pending_negations = 0,
            Token::Or => unreachable!("handled above"),
            Token::Tag(name) => {
                if pending_negations % 2 == 1 {
                    negative.insert(name.clone());
                } else {
                    positive.insert(name.clone());
                }
                pending_negations = 0;
            }
        }
    }
    let mut conflict: Vec<&String> = positive.intersection(&negative).collect();
    conflict.sort();
    conflict.first().map(|s| (*s).clone())
}

#[derive(Debug, PartialEq)]
enum Token {
    And,
    Or,
    Not,
    LParen,
    RParen,
    Tag(String),
}

/// Tokenise a `//go:build` expression. Returns `None` on any character
/// outside the known grammar (treated as INDETERMINATE — never flagged).
fn tokenize(expr: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.trim().chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => i += 1,
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '!' => {
                tokens.push(Token::Not);
                i += 1;
            }
            '&' => {
                if chars.get(i + 1) == Some(&'&') {
                    tokens.push(Token::And);
                    i += 2;
                } else {
                    return None;
                }
            }
            '|' => {
                if chars.get(i + 1) == Some(&'|') {
                    tokens.push(Token::Or);
                    i += 2;
                } else {
                    return None;
                }
            }
            _ if is_tag_char(c) => {
                let start = i;
                while i < chars.len() && is_tag_char(chars[i]) {
                    i += 1;
                }
                tokens.push(Token::Tag(chars[start..i].iter().collect()));
            }
            _ => return None,
        }
    }
    Some(tokens)
}

/// Go build tags are letters, digits, `_`, and `.` (per `go/build`).
fn is_tag_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

fn make_finding(file: &IrFile, line: u32, end_col: u32, conflict_tag: &str, expr: &str) -> Finding {
    Finding {
        detector_id: "build-tag-interaction-go".to_string(),
        primary: Location {
            file: file.path.clone(),
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col,
        },
        related: Vec::new(),
        message: format!(
            "//go:build constraint is unsatisfiable: tag `{conflict_tag}` is required both positively and negatively (`{expr}`) — the file never builds"
        ),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["tartler-eurosys-2011", "nadi-icse-2014"],
            raw: serde_json::json!({
                "kind": "go-build-tag-contradiction",
                "constraint": expr,
                "conflicting_tag": conflict_tag,
            }),
            // No Go-subject peer-reviewed grounding (survey:
            // docs/surveys/build-tag-interaction-go-2026-06.md); the
            // concept keys carry the citation.
            language_citation_status: LanguageCitationStatus::Unconfirmed,
        },
        origin: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contradiction(expr: &str) -> Option<String> {
        contradictory_tag(expr)
    }

    #[test]
    fn flags_simple_contradiction() {
        assert_eq!(contradiction("linux && !linux"), Some("linux".to_string()));
        assert_eq!(
            contradiction("!windows && windows"),
            Some("windows".to_string())
        );
    }

    #[test]
    fn flags_contradiction_nested_in_conjunction() {
        assert_eq!(
            contradiction("(linux && amd64) && !linux"),
            Some("linux".to_string())
        );
        assert_eq!(
            contradiction("cgo && unix && !unix && amd64"),
            Some("unix".to_string())
        );
    }

    #[test]
    fn satisfiable_constraint_is_clean() {
        assert_eq!(contradiction("linux && amd64"), None);
        assert_eq!(contradiction("linux && !windows"), None);
        assert_eq!(contradiction("!cgo"), None);
        assert_eq!(contradiction("darwin && (amd64 || arm64)"), None);
    }

    #[test]
    fn disjunction_is_indeterminate() {
        // `||` leaves the single-conjunction reasoning unsound; never flag.
        assert_eq!(contradiction("linux || !linux"), None);
        assert_eq!(contradiction("(linux && !linux) || amd64"), None);
    }

    #[test]
    fn de_morgan_negated_paren_is_indeterminate() {
        // `!(linux && amd64)` == `!linux || !amd64` — a disjunction.
        assert_eq!(contradiction("linux && !(linux && amd64)"), None);
    }

    #[test]
    fn double_negation_is_positive() {
        // `!!linux` is positive `linux`; with `!linux` that is a contradiction.
        assert_eq!(
            contradiction("!!linux && !linux"),
            Some("linux".to_string())
        );
        // `!!linux && linux` is satisfiable.
        assert_eq!(contradiction("!!linux && linux"), None);
    }

    #[test]
    fn unknown_grammar_is_indeterminate() {
        assert_eq!(contradiction("linux & amd64"), None);
        assert_eq!(contradiction("linux + amd64"), None);
    }
}
