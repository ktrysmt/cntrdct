//! cntrdct-core: shared types and traits for evidence-based contradiction detection.
//!
//! Design constraints (P1-P5):
//! - P1: every Detector must reference prior art. `register_detector` rejects
//!   detectors with empty `citations()`.
//! - P2: preregistration metadata is carried via `DetectorConfig::preregistration_id`.
//! - P3: only `Adjudicator` may invoke an LLM. `Detector` implementations must be
//!   deterministic; randomness or LLM calls inside `detect` violate this contract.
//! - P4: empirical priors do not belong in adjudicator prompts. Statistical priors
//!   live in `Ranker` implementations and are derived from labelled corpora.
//! - P5: severities map to IEEE 1044-2009 categories at SARIF emission time. SARIF
//!   formatting itself lives in the (future) `sarif` crate.
//!
//! # Example
//!
//! Implementing a minimal detector and registering it under the P1 constraint:
//!
//! ```
//! use cntrdct::core::{
//!     Citation, DetectContext, Detector, DetectorError, Finding, Language,
//!     register_detector,
//! };
//!
//! struct Demo;
//!
//! static CITES: &[Citation] = &[Citation {
//!     key: "demo-2026",
//!     authors: "Demo et al.",
//!     title: "Demo paper",
//!     venue: "Demo venue",
//!     year: 2026,
//!     doi: None,
//!     url: None,
//!     languages: &[Language::Rust],
//! }];
//!
//! impl Detector for Demo {
//!     fn id(&self) -> &'static str { "demo" }
//!     fn name(&self) -> &'static str { "Demo" }
//!     fn citations(&self) -> &'static [Citation] { CITES }
//!     fn supported_languages(&self) -> &'static [Language] { &[Language::Rust] }
//!     fn detect(&self, _: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
//!         Ok(vec![])
//!     }
//! }
//!
//! register_detector(&Demo).unwrap();
//! ```

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ---------- Language (multi-language identity, M-1 / M-6) ----------

/// Languages cntrdct can analyse.
///
/// Lives in `cntrdct-core` (rather than `cntrdct-parsers`) so the
/// `Citation::languages` field and the `LanguageCitationStatus` flag
/// on `Evidence` can reference it without dragging tree-sitter into
/// the core dependency graph. `cntrdct-parsers` re-exports this type
/// and adds the parser-provider machinery.
///
/// Marked `#[non_exhaustive]` so downstream `match` expressions must
/// declare a default arm; new variants land one at a time as the
/// M-series adds language support.
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
    /// Every variant defined today, in declaration order.
    pub fn all() -> &'static [Language] {
        &[Language::Rust, Language::Python]
    }

    /// Canonical lowercase name used in `ParsedFile.language` strings,
    /// `cntrdct.toml` keys, and SARIF output.
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

// ---------- Citation (P1) ----------

/// A bibliographic reference attached to a detector.
///
/// Every `Detector::citations()` entry must resolve to an entry in the
/// workspace `CITATIONS.md`. `register_detector` rejects detectors that
/// return an empty slice (P1 enforcement).
///
/// `languages` declares which languages the citation is grounded in
/// per `docs/spec/citations-policy.md`. An empty slice means the
/// citation is general / methodological (Wilson lower bound papers,
/// IEEE 1044-2009, etc.) and does not satisfy any per-language
/// requirement on its own.
#[derive(Debug, Clone, Serialize)]
pub struct Citation {
    /// Stable identifier used to cross-reference `CITATIONS.md`.
    pub key: &'static str,
    /// Authors of the cited work, in display order.
    pub authors: &'static str,
    /// Title of the cited work.
    pub title: &'static str,
    /// Publication venue (conference, journal, or standards body).
    pub venue: &'static str,
    /// Publication year.
    pub year: u16,
    /// Optional Digital Object Identifier.
    pub doi: Option<&'static str>,
    /// Optional canonical URL for the work.
    pub url: Option<&'static str>,
    /// Languages the citation is grounded in. Empty for general /
    /// methodological references.
    pub languages: &'static [Language],
}

// ---------- Finding ----------

/// A source-code location associated with a finding.
#[derive(Debug, Clone, Serialize)]
pub struct Location {
    /// Path to the file containing the location.
    pub file: PathBuf,
    /// 1-based line number of the first character of the span.
    pub start_line: u32,
    /// 1-based column number of the first character of the span.
    pub start_col: u32,
    /// 1-based line number of the character immediately after the span.
    pub end_line: u32,
    /// 1-based column number of the character immediately after the span.
    pub end_col: u32,
}

/// Detector-supplied severity. Mapped to IEEE 1044-2009 levels by the SARIF emitter (P5).
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Severity {
    /// Lowest level; informational signal that does not warrant action.
    Info,
    /// Slightly higher than `Info`; flagged for awareness.
    Note,
    /// Likely defect; should usually be addressed.
    Warning,
    /// Definite defect; must be addressed.
    Error,
}

/// IEEE 1044-2009 §5.4 anomaly classification.
///
/// Citation: `ieee-1044-2009` (see `CITATIONS.md` Layer 4). The standard lists
/// classes Logic, Interface, Data, Description, Documentation, Standards,
/// Performance, and Other; `Description` and `Documentation` are merged here
/// under `Documentation` for cntrdct's purposes.
///
/// Each unit variant serializes as its PascalCase name (e.g., `"Logic"`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnomalyClass {
    /// Defect in the program's logic (e.g., contradictory branches).
    Logic,
    /// Defect at an interface boundary (e.g., swapped arguments).
    Interface,
    /// Defect in data shape or values.
    Data,
    /// Defect in documentation or comments relative to behaviour.
    Documentation,
    /// Performance regression or pessimisation.
    Performance,
    /// Violation of a coding standard.
    Standards,
    /// Anomaly that does not fit the other categories.
    Other,
}

/// Strength of the per-language citation grounding for a `Finding`.
///
/// Per `docs/spec/citations-policy.md`, every detector ships with at
/// least one citation overall (P1, hard-gated). Per-language citation
/// is best-effort: when the survey for a (detector, language) pair
/// returns no qualifying paper, the language extension still ships
/// and each emitted finding declares
/// [`LanguageCitationStatus::Unconfirmed`] so SARIF consumers can
/// weigh indirectly-grounded findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageCitationStatus {
    /// At least one of the detector's citations is grounded in the
    /// finding's source language per
    /// `citations-policy.md` clauses (a), (b), or (c).
    Confirmed,
    /// All cited works ground a different language; the survey
    /// returned no per-language match. The detector still applies
    /// because the underlying concept transfers, but the grounding
    /// is indirect.
    Unconfirmed,
}

/// Evidence supporting a `Finding`.
#[derive(Debug, Clone, Serialize)]
pub struct Evidence {
    /// Citation keys referenced by the detector for this finding.
    pub citation_keys: Vec<&'static str>,
    /// Detector-defined raw evidence payload (kept opaque to the core).
    pub raw: serde_json::Value,
    /// Per-language citation grounding for this finding. Detectors
    /// set this to [`LanguageCitationStatus::Confirmed`] when at
    /// least one of their citations covers `Finding`'s source
    /// language; [`LanguageCitationStatus::Unconfirmed`] when the
    /// language is supported via concept transfer rather than a
    /// language-specific citation.
    pub language_citation_status: LanguageCitationStatus,
}

/// A single detector finding before ranking.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Identifier of the detector that produced this finding.
    pub detector_id: String,
    /// Primary location the finding refers to.
    pub primary: Location,
    /// Additional related locations (e.g., the sibling that drifted).
    pub related: Vec<Location>,
    /// Human-readable description of the finding.
    pub message: String,
    /// Detector-supplied severity (mapped at SARIF emission per P5).
    pub raw_severity: Severity,
    /// IEEE 1044-2009 anomaly classification supplied by the detector.
    pub anomaly_class: AnomalyClass,
    /// Supporting evidence (citations + opaque payload).
    pub evidence: Evidence,
}

// ---------- Parser context ----------

/// One source file presented to a detector run.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    /// Filesystem path of the source file.
    pub path: PathBuf,
    /// Language of the source file. Set by the file walker via
    /// `crate::parsers::detect_language` and consumed by detectors to
    /// dispatch per-language scan logic.
    pub language: Language,
    /// File contents as UTF-8.
    pub source: String,
}

/// Aggregate statistics over the corpus passed to a detector run.
#[derive(Debug, Default)]
pub struct CorpusStats {
    /// Number of files in the corpus.
    pub file_count: usize,
    /// Sum of source lines across the corpus.
    pub total_loc: usize,
}

/// Detector configuration provided per-run.
#[derive(Debug, Clone, Default)]
pub struct DetectorConfig {
    /// Preregistration identifier (P2). `Some` when the run is part of a
    /// declared empirical study.
    pub preregistration_id: Option<String>,
    /// Detector-defined options (kept opaque to the core).
    pub options: serde_json::Value,
}

/// Inputs handed to `Detector::detect`.
#[derive(Debug)]
pub struct DetectContext<'a> {
    /// Files in scope for this detection run.
    pub files: &'a [ParsedFile],
    /// Corpus-level statistics.
    pub stats: &'a CorpusStats,
    /// Per-run configuration.
    pub config: &'a DetectorConfig,
}

// ---------- Errors ----------

/// Errors returned by `Detector::detect` and registration.
#[derive(Debug, Error)]
pub enum DetectorError {
    /// Source file failed to parse.
    #[error("parse error: {0}")]
    Parse(String),
    /// I/O failure while reading sources.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Configuration was rejected (e.g., P1 violation in `register_detector`).
    #[error("invalid configuration: {0}")]
    Config(String),
}

// ---------- Detector trait (Layer 1) ----------

/// Layer 1 deterministic detector trait.
///
/// Implementations must be pure with respect to their inputs (P3): no LLM
/// calls, no randomness, no I/O beyond what is supplied via `DetectContext`.
pub trait Detector: Send + Sync {
    /// Stable, machine-readable identifier (e.g., `"clone-drift"`).
    fn id(&self) -> &'static str;
    /// Human-readable display name.
    fn name(&self) -> &'static str;
    /// Bibliographic references justifying this detector (P1).
    fn citations(&self) -> &'static [Citation];
    /// Languages this detector supports (e.g., `&[Language::Rust]`).
    fn supported_languages(&self) -> &'static [Language];
    /// Run the detector and return its findings.
    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError>;
}

// ---------- Ranker trait (Layer 2) ----------

/// A `Finding` enriched with Layer 2 ranking and optional Layer 3 adjudication.
#[derive(Debug, Clone, Serialize)]
pub struct RankedFinding {
    /// The underlying detector finding.
    pub finding: Finding,
    /// `None` when no labelled corpus is available (v0). Becomes `Some(p)` once
    /// calibration data ships.
    pub posterior_tp: Option<f64>,
    /// `None` when no labelled corpus is available (v0).
    pub wilson_lower: Option<f64>,
    /// Which 95% lower-bound method produced `wilson_lower`. `None`
    /// when the ranker had no calibration data for this finding (and
    /// therefore left `wilson_lower` as `None`). Per Q-11, this lets
    /// downstream consumers tell whether the value came from the
    /// Wilson formula (`n >= 30`) or the small-sample Bayes-Laplace
    /// fallback (`n < 30`) — the switching itself is opaque to the
    /// finding, but the auditable label is not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_method: Option<crate::calibration::PriorMethod>,
    /// Final ranking score used to order the output.
    pub rank_score: f64,
    /// Layer 3 LLM adjudication. `None` unless `--adjudicate` was requested
    /// AND the adjudicator successfully ran for this finding.
    ///
    /// Per design constraint P3, only `Adjudicator` implementations may
    /// populate this field; detectors and rankers must leave it as `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjudication: Option<AdjudicationResult>,
}

/// Layer 2 ranker trait.
pub trait Ranker: Send + Sync {
    /// Convert raw findings into ranked findings, in output order.
    fn rank(&self, findings: Vec<Finding>) -> Vec<RankedFinding>;
}

// ---------- Adjudicator trait (Layer 3) ----------

/// Layer 3 adjudication verdict.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum AdjudicationVerdict {
    /// Adjudicator believes the finding is most likely a true positive.
    LikelyTruePositive,
    /// Adjudicator believes the finding is most likely a false positive.
    LikelyFalsePositive,
    /// Adjudicator could not commit to either direction.
    Uncertain,
}

/// Result of running the Layer 3 adjudicator on a single finding.
#[derive(Debug, Clone, Serialize)]
pub struct AdjudicationResult {
    /// Verdict produced by the adjudicator.
    pub verdict: AdjudicationVerdict,
    /// Adjudicator-reported confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Free-form rationale supplied by the adjudicator.
    pub rationale: String,
    /// Optional calibration tag (e.g., the prompt template version).
    pub calibration_tag: Option<String>,
}

/// Layer 3 adjudicator trait. The only layer permitted to invoke an LLM (P3).
pub trait Adjudicator: Send + Sync {
    /// Adjudicate a ranked finding, returning a verdict + rationale.
    fn adjudicate(&self, finding: &RankedFinding) -> Result<AdjudicationResult, DetectorError>;
}

// ---------- Registration helper (P1 enforcement) ----------

/// Validate that a detector satisfies P1 (non-empty citations).
///
/// Returns `Err(DetectorError::Config)` if `d.citations()` is empty.
pub fn register_detector(d: &dyn Detector) -> Result<(), DetectorError> {
    if d.citations().is_empty() {
        return Err(DetectorError::Config(format!(
            "detector {} has no citations (P1 violation)",
            d.id()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static GOOD_CITES: &[Citation] = &[Citation {
        key: "test-2026",
        authors: "Test",
        title: "Test",
        venue: "Test",
        year: 2026,
        doi: None,
        url: None,
        languages: &[Language::Rust],
    }];

    struct Bad;
    impl Detector for Bad {
        fn id(&self) -> &'static str {
            "bad"
        }
        fn name(&self) -> &'static str {
            "Bad"
        }
        fn citations(&self) -> &'static [Citation] {
            &[]
        }
        fn supported_languages(&self) -> &'static [Language] {
            &[Language::Rust]
        }
        fn detect(&self, _: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
            Ok(vec![])
        }
    }

    struct Good;
    impl Detector for Good {
        fn id(&self) -> &'static str {
            "good"
        }
        fn name(&self) -> &'static str {
            "Good"
        }
        fn citations(&self) -> &'static [Citation] {
            GOOD_CITES
        }
        fn supported_languages(&self) -> &'static [Language] {
            &[Language::Rust]
        }
        fn detect(&self, _: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
            Ok(vec![])
        }
    }

    #[test]
    fn p1_violation_rejected() {
        assert!(register_detector(&Bad).is_err());
    }

    #[test]
    fn p1_satisfied_accepted() {
        assert!(register_detector(&Good).is_ok());
    }

    #[test]
    fn anomaly_class_serializes_as_pascal_case_string() {
        let finding = Finding {
            detector_id: "demo".to_string(),
            primary: Location {
                file: PathBuf::from("a.rs"),
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            },
            related: vec![],
            message: "demo".to_string(),
            raw_severity: Severity::Warning,
            anomaly_class: AnomalyClass::Logic,
            evidence: Evidence {
                citation_keys: vec!["test-2026"],
                raw: serde_json::Value::Null,
                language_citation_status: LanguageCitationStatus::Confirmed,
            },
        };
        let json = serde_json::to_string(&finding).expect("serializes");
        assert!(
            json.contains("\"anomaly_class\":\"Logic\""),
            "expected `anomaly_class` field as plain string, got: {}",
            json
        );
    }

    fn make_finding() -> Finding {
        Finding {
            detector_id: "demo".to_string(),
            primary: Location {
                file: PathBuf::from("a.rs"),
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            },
            related: vec![],
            message: "demo".to_string(),
            raw_severity: Severity::Warning,
            anomaly_class: AnomalyClass::Logic,
            evidence: Evidence {
                citation_keys: vec!["test-2026"],
                raw: serde_json::Value::Null,
                language_citation_status: LanguageCitationStatus::Confirmed,
            },
        }
    }

    #[test]
    fn ranked_finding_omits_adjudication_when_none() {
        let rf = RankedFinding {
            finding: make_finding(),
            posterior_tp: None,
            wilson_lower: None,
            prior_method: None,
            rank_score: 1.0,
            adjudication: None,
        };
        let json = serde_json::to_string(&rf).expect("serializes");
        assert!(
            !json.contains("\"adjudication\""),
            "field must be omitted when None: {}",
            json
        );
        assert!(
            !json.contains("\"prior_method\""),
            "prior_method must be omitted when None: {}",
            json
        );
    }

    #[test]
    fn ranked_finding_serializes_adjudication_object_when_some() {
        let rf = RankedFinding {
            finding: make_finding(),
            posterior_tp: Some(0.6),
            wilson_lower: Some(0.4),
            prior_method: Some(crate::calibration::PriorMethod::Wilson),
            rank_score: 1.0,
            adjudication: Some(AdjudicationResult {
                verdict: AdjudicationVerdict::LikelyTruePositive,
                confidence: 0.82,
                rationale: "matches drift pattern".to_string(),
                calibration_tag: Some("T1.5".to_string()),
            }),
        };
        let json = serde_json::to_string(&rf).expect("serializes");
        assert!(
            json.contains("\"adjudication\":{"),
            "must contain adjudication object: {}",
            json
        );
        assert!(
            json.contains("\"verdict\":\"LikelyTruePositive\""),
            "got: {}",
            json
        );
        assert!(json.contains("\"confidence\":0.82"), "got: {}", json);
        assert!(
            json.contains("\"rationale\":\"matches drift pattern\""),
            "got: {}",
            json
        );
        assert!(
            json.contains("\"calibration_tag\":\"T1.5\""),
            "got: {}",
            json
        );
    }

    #[test]
    fn anomaly_class_variants_serialize_as_their_names() {
        let cases: &[(AnomalyClass, &str)] = &[
            (AnomalyClass::Logic, "\"Logic\""),
            (AnomalyClass::Interface, "\"Interface\""),
            (AnomalyClass::Data, "\"Data\""),
            (AnomalyClass::Documentation, "\"Documentation\""),
            (AnomalyClass::Performance, "\"Performance\""),
            (AnomalyClass::Standards, "\"Standards\""),
            (AnomalyClass::Other, "\"Other\""),
        ];
        for (variant, expected) in cases {
            let s = serde_json::to_string(variant).expect("serializes");
            assert_eq!(
                &s, expected,
                "variant {:?} must serialize to {}",
                variant, expected
            );
        }
    }
}
