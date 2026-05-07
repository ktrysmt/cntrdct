//! SARIF 2.1.0 emitter for cntrdct findings.
//!
//! Spec: `cntrdct/docs/spec/sarif-v0.md`.

use crate::core::{
    AdjudicationResult, AdjudicationVerdict, AnomalyClass, Citation, Detector, Finding,
    LanguageCitationStatus, Location, RankedFinding, Severity,
};
use serde_json::{json, Map, Value};

const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const SARIF_VERSION: &str = "2.1.0";
const TOOL_NAME: &str = "cntrdct";
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const INFORMATION_URI: &str = "https://github.com/ktrysmt/cntrdct";

/// Emit SARIF 2.1.0 without a rules taxonomy. Preserved for backwards compat
/// with callers (and tests) that don't have access to the live `Detector`
/// instances. Prefer `to_sarif_with_rules` for production output.
pub fn to_sarif(findings: &[Finding]) -> Value {
    let results: Vec<Value> = findings.iter().map(finding_to_result).collect();
    json!({
        "$schema": SARIF_SCHEMA,
        "version": SARIF_VERSION,
        "runs": [{
            "tool": {
                "driver": {
                    "name": TOOL_NAME,
                    "version": TOOL_VERSION,
                    "informationUri": INFORMATION_URI
                }
            },
            "results": results
        }]
    })
}

/// Emit SARIF 2.1.0 with `runs[0].tool.driver.rules` populated from the
/// supplied detectors. One rule per detector; each rule's `fullDescription`
/// is built from the detector's primary (first) citation.
pub fn to_sarif_with_rules(findings: &[Finding], detectors: &[&dyn Detector]) -> Value {
    let results: Vec<Value> = findings.iter().map(finding_to_result).collect();
    let rules: Vec<Value> = detectors.iter().map(|d| detector_to_rule(*d)).collect();
    json!({
        "$schema": SARIF_SCHEMA,
        "version": SARIF_VERSION,
        "runs": [{
            "tool": {
                "driver": {
                    "name": TOOL_NAME,
                    "version": TOOL_VERSION,
                    "informationUri": INFORMATION_URI,
                    "rules": rules
                }
            },
            "results": results
        }]
    })
}

pub fn to_sarif_pretty(findings: &[Finding]) -> String {
    serde_json::to_string_pretty(&to_sarif(findings))
        .expect("SARIF JSON value is always serializable")
}

/// Pretty variant of `to_sarif_with_rules`.
pub fn to_sarif_with_rules_pretty(findings: &[Finding], detectors: &[&dyn Detector]) -> String {
    serde_json::to_string_pretty(&to_sarif_with_rules(findings, detectors))
        .expect("SARIF JSON value is always serializable")
}

/// Like `to_sarif_with_rules`, but accepts `RankedFinding` so adjudication
/// metadata can flow through to `result.properties.adjudication` when present.
///
/// Spec: `docs/spec/adjudicator-v0.md` — when a finding has been adjudicated by
/// the Layer 3 LLM adjudicator, the verdict / confidence / rationale /
/// calibration_tag are surfaced in SARIF as a structured property so SARIF
/// consumers (CodeQL viewer, GitHub Code Scanning, etc.) can display them
/// without knowing about cntrdct's bespoke JSON shape.
pub fn to_sarif_with_rules_ranked(ranked: &[RankedFinding], detectors: &[&dyn Detector]) -> Value {
    let results: Vec<Value> = ranked.iter().map(ranked_to_result).collect();
    let rules: Vec<Value> = detectors.iter().map(|d| detector_to_rule(*d)).collect();
    json!({
        "$schema": SARIF_SCHEMA,
        "version": SARIF_VERSION,
        "runs": [{
            "tool": {
                "driver": {
                    "name": TOOL_NAME,
                    "version": TOOL_VERSION,
                    "informationUri": INFORMATION_URI,
                    "rules": rules
                }
            },
            "results": results
        }]
    })
}

/// Pretty variant of `to_sarif_with_rules_ranked`.
pub fn to_sarif_with_rules_pretty_ranked(
    ranked: &[RankedFinding],
    detectors: &[&dyn Detector],
) -> String {
    serde_json::to_string_pretty(&to_sarif_with_rules_ranked(ranked, detectors))
        .expect("SARIF JSON value is always serializable")
}

fn ranked_to_result(rf: &RankedFinding) -> Value {
    let mut result = finding_to_result(&rf.finding);
    if let Some(adj) = &rf.adjudication {
        if let Some(props) = result.get_mut("properties").and_then(|v| v.as_object_mut()) {
            props.insert("adjudication".to_string(), adjudication_to_value(adj));
        }
    }
    result
}

fn adjudication_to_value(a: &AdjudicationResult) -> Value {
    let mut obj = Map::new();
    obj.insert(
        "verdict".to_string(),
        Value::String(verdict_to_str(a.verdict).to_string()),
    );
    obj.insert("confidence".to_string(), json!(a.confidence));
    obj.insert("rationale".to_string(), Value::String(a.rationale.clone()));
    if let Some(tag) = &a.calibration_tag {
        obj.insert("calibration_tag".to_string(), Value::String(tag.clone()));
    }
    Value::Object(obj)
}

fn verdict_to_str(v: AdjudicationVerdict) -> &'static str {
    match v {
        AdjudicationVerdict::LikelyTruePositive => "LikelyTruePositive",
        AdjudicationVerdict::LikelyFalsePositive => "LikelyFalsePositive",
        AdjudicationVerdict::Uncertain => "Uncertain",
    }
}

fn detector_to_rule(d: &dyn Detector) -> Value {
    let mut rule = Map::new();
    rule.insert("id".to_string(), Value::String(d.id().to_string()));
    rule.insert("shortDescription".to_string(), json!({ "text": d.name() }));

    if let Some(primary) = d.citations().first() {
        rule.insert(
            "fullDescription".to_string(),
            json!({ "text": citation_to_full_description(primary) }),
        );
        if let Some(url) = primary.url {
            rule.insert("helpUri".to_string(), Value::String(url.to_string()));
        }
    }

    Value::Object(rule)
}

fn citation_to_full_description(c: &Citation) -> String {
    format!("{}. {}. {} ({}).", c.authors, c.title, c.venue, c.year)
}

fn finding_to_result(f: &Finding) -> Value {
    json!({
        "ruleId": f.detector_id,
        "level": severity_to_level(f.raw_severity),
        "message": { "text": f.message },
        "locations": [physical_location(&f.primary)],
        "relatedLocations": f.related.iter().map(physical_location).collect::<Vec<_>>(),
        "properties": {
            "citationKeys": f.evidence.citation_keys,
            "anomalyClass": anomaly_class_to_str(f.anomaly_class),
            "languageCitationStatus": language_citation_status_to_str(f.evidence.language_citation_status),
            "raw": f.evidence.raw
        }
    })
}

fn severity_to_level(s: Severity) -> &'static str {
    match s {
        Severity::Info => "none",
        Severity::Note => "note",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

/// IEEE 1044-2009 §5.4 anomaly class as a plain SARIF property string.
///
/// We deliberately do not delegate to `serde_json::to_value(class)` so that the
/// SARIF surface-level vocabulary is decoupled from the core enum's serde
/// representation: a future serde rename or tagging change in core would
/// silently shift the SARIF property if we relied on it.
fn anomaly_class_to_str(c: AnomalyClass) -> &'static str {
    match c {
        AnomalyClass::Logic => "Logic",
        AnomalyClass::Interface => "Interface",
        AnomalyClass::Data => "Data",
        AnomalyClass::Documentation => "Documentation",
        AnomalyClass::Performance => "Performance",
        AnomalyClass::Standards => "Standards",
        AnomalyClass::Other => "Other",
    }
}

/// Per-language citation grounding flag, surfaced as a SARIF property.
///
/// Per `docs/spec/citations-policy.md`, `Confirmed` means at least one
/// of the detector's citations is grounded in the finding's source
/// language; `Unconfirmed` means the support transferred via concept
/// rather than a language-specific citation. Consumers (downstream
/// dashboards, reviewers) can choose to weight unconfirmed findings
/// lower without changing the underlying detector's behaviour.
fn language_citation_status_to_str(status: LanguageCitationStatus) -> &'static str {
    match status {
        LanguageCitationStatus::Confirmed => "Confirmed",
        LanguageCitationStatus::Unconfirmed => "Unconfirmed",
    }
}

fn physical_location(loc: &Location) -> Value {
    json!({
        "physicalLocation": {
            "artifactLocation": {
                "uri": loc.file.to_string_lossy()
            },
            "region": {
                "startLine": loc.start_line,
                "startColumn": loc.start_col,
                "endLine": loc.end_line,
                "endColumn": loc.end_col
            }
        }
    })
}
