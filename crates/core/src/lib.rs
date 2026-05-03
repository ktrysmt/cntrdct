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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ---------- Citation (P1) ----------

#[derive(Debug, Clone, Serialize)]
pub struct Citation {
    pub key: &'static str,
    pub authors: &'static str,
    pub title: &'static str,
    pub venue: &'static str,
    pub year: u16,
    pub doi: Option<&'static str>,
    pub url: Option<&'static str>,
}

// ---------- Finding ----------

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub file: PathBuf,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Severity {
    Info,
    Note,
    Warning,
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
    Logic,
    Interface,
    Data,
    Documentation,
    Performance,
    Standards,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evidence {
    pub citation_keys: Vec<&'static str>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub detector_id: String,
    pub primary: Location,
    pub related: Vec<Location>,
    pub message: String,
    pub raw_severity: Severity,
    /// IEEE 1044-2009 anomaly classification supplied by the detector.
    pub anomaly_class: AnomalyClass,
    pub evidence: Evidence,
}

// ---------- Parser context ----------

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub language: String,
    pub source: String,
}

#[derive(Debug, Default)]
pub struct CorpusStats {
    pub file_count: usize,
    pub total_loc: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DetectorConfig {
    pub preregistration_id: Option<String>,
    pub options: serde_json::Value,
}

#[derive(Debug)]
pub struct DetectContext<'a> {
    pub files: &'a [ParsedFile],
    pub stats: &'a CorpusStats,
    pub config: &'a DetectorConfig,
}

// ---------- Errors ----------

#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid configuration: {0}")]
    Config(String),
}

// ---------- Detector trait (Layer 1) ----------

pub trait Detector: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn citations(&self) -> &'static [Citation];
    fn supported_languages(&self) -> &'static [&'static str];
    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError>;
}

// ---------- Ranker trait (Layer 2) ----------

#[derive(Debug, Clone, Serialize)]
pub struct RankedFinding {
    pub finding: Finding,
    /// `None` when no labelled corpus is available (v0). Becomes `Some(p)` once
    /// calibration data ships.
    pub posterior_tp: Option<f64>,
    /// `None` when no labelled corpus is available (v0).
    pub wilson_lower: Option<f64>,
    pub rank_score: f64,
    /// Layer 3 LLM adjudication. `None` unless `--adjudicate` was requested
    /// AND the adjudicator successfully ran for this finding.
    ///
    /// Per design constraint P3, only `Adjudicator` implementations may
    /// populate this field; detectors and rankers must leave it as `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjudication: Option<AdjudicationResult>,
}

pub trait Ranker: Send + Sync {
    fn rank(&self, findings: Vec<Finding>) -> Vec<RankedFinding>;
}

// ---------- Adjudicator trait (Layer 3) ----------

#[derive(Debug, Clone, Copy, Serialize)]
pub enum AdjudicationVerdict {
    LikelyTruePositive,
    LikelyFalsePositive,
    Uncertain,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdjudicationResult {
    pub verdict: AdjudicationVerdict,
    pub confidence: f64,
    pub rationale: String,
    pub calibration_tag: Option<String>,
}

pub trait Adjudicator: Send + Sync {
    fn adjudicate(&self, finding: &RankedFinding) -> Result<AdjudicationResult, DetectorError>;
}

// ---------- Registration helper (P1 enforcement) ----------

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
    }];

    struct Bad;
    impl Detector for Bad {
        fn id(&self) -> &'static str { "bad" }
        fn name(&self) -> &'static str { "Bad" }
        fn citations(&self) -> &'static [Citation] { &[] }
        fn supported_languages(&self) -> &'static [&'static str] { &["*"] }
        fn detect(&self, _: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
            Ok(vec![])
        }
    }

    struct Good;
    impl Detector for Good {
        fn id(&self) -> &'static str { "good" }
        fn name(&self) -> &'static str { "Good" }
        fn citations(&self) -> &'static [Citation] { GOOD_CITES }
        fn supported_languages(&self) -> &'static [&'static str] { &["*"] }
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
            },
        }
    }

    #[test]
    fn ranked_finding_omits_adjudication_when_none() {
        let rf = RankedFinding {
            finding: make_finding(),
            posterior_tp: None,
            wilson_lower: None,
            rank_score: 1.0,
            adjudication: None,
        };
        let json = serde_json::to_string(&rf).expect("serializes");
        assert!(
            !json.contains("\"adjudication\""),
            "field must be omitted when None: {}",
            json
        );
    }

    #[test]
    fn ranked_finding_serializes_adjudication_object_when_some() {
        let rf = RankedFinding {
            finding: make_finding(),
            posterior_tp: Some(0.6),
            wilson_lower: Some(0.4),
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
        assert!(json.contains("\"verdict\":\"LikelyTruePositive\""), "got: {}", json);
        assert!(json.contains("\"confidence\":0.82"), "got: {}", json);
        assert!(json.contains("\"rationale\":\"matches drift pattern\""), "got: {}", json);
        assert!(json.contains("\"calibration_tag\":\"T1.5\""), "got: {}", json);
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
            assert_eq!(&s, expected, "variant {:?} must serialize to {}", variant, expected);
        }
    }
}
