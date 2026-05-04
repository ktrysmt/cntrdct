//! Integration tests for the SARIF emitter v0 spec.

use std::path::PathBuf;

use cntrdct_core::{
    AdjudicationResult, AdjudicationVerdict, AnomalyClass, Citation, DetectContext, Detector,
    DetectorError, Evidence, Finding, Language, LanguageCitationStatus, Location, RankedFinding,
    Severity,
};
use cntrdct_sarif::{to_sarif, to_sarif_pretty, to_sarif_with_rules, to_sarif_with_rules_ranked};

static CD_CITATIONS: &[Citation] = &[
    Citation {
        key: "cordy-roy-icpc-2008",
        authors: "J.R. Cordy, C.K. Roy",
        title: "The NiCad Clone Detector",
        venue: "ICPC",
        year: 2008,
        doi: None,
        url: Some("https://example.invalid/nicad"),
        languages: &[Language::Rust],
    },
    Citation {
        key: "bettenburg-msr-2009",
        authors: "N. Bettenburg et al.",
        title: "An Empirical Study on Inconsistent Changes to Code Clones",
        venue: "MSR",
        year: 2009,
        doi: None,
        url: None,
        languages: &[Language::Rust],
    },
];

static AS_CITATIONS: &[Citation] = &[Citation {
    key: "rice-icse-2017",
    authors: "A. Rice et al.",
    title: "Detecting Argument Selection Defects",
    venue: "ICSE",
    year: 2017,
    doi: None,
    url: None,
    languages: &[Language::Rust],
}];

struct FakeCloneDrift;
impl Detector for FakeCloneDrift {
    fn id(&self) -> &'static str {
        "clone-drift"
    }
    fn name(&self) -> &'static str {
        "Clone Drift"
    }
    fn citations(&self) -> &'static [Citation] {
        CD_CITATIONS
    }
    fn supported_languages(&self) -> &'static [&'static str] {
        &["rust"]
    }
    fn detect(&self, _: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        Ok(vec![])
    }
}

struct FakeArgSwap;
impl Detector for FakeArgSwap {
    fn id(&self) -> &'static str {
        "arg-swap"
    }
    fn name(&self) -> &'static str {
        "Argument Swap"
    }
    fn citations(&self) -> &'static [Citation] {
        AS_CITATIONS
    }
    fn supported_languages(&self) -> &'static [&'static str] {
        &["rust"]
    }
    fn detect(&self, _: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        Ok(vec![])
    }
}

fn loc(file: &str, sl: u32, sc: u32, el: u32, ec: u32) -> Location {
    Location {
        file: PathBuf::from(file),
        start_line: sl,
        start_col: sc,
        end_line: el,
        end_col: ec,
    }
}

fn make_finding(severity: Severity) -> Finding {
    make_finding_with_class(severity, AnomalyClass::Logic)
}

fn make_finding_with_class(severity: Severity, class: AnomalyClass) -> Finding {
    Finding {
        detector_id: "clone-drift".to_string(),
        primary: loc("a.rs", 10, 5, 15, 8),
        related: vec![loc("b.rs", 1, 1, 2, 2)],
        message: "function diverged".to_string(),
        raw_severity: severity,
        anomaly_class: class,
        evidence: Evidence {
            citation_keys: vec!["cordy-roy-icpc-2008", "bettenburg-msr-2009"],
            raw: serde_json::json!({"group_size": 5}),
            language_citation_status: LanguageCitationStatus::Confirmed,
        },
    }
}

#[test]
fn t1_empty_findings_yield_minimal_sarif() {
    let s = to_sarif(&[]);
    assert_eq!(s["version"], "2.1.0");
    assert_eq!(s["runs"][0]["tool"]["driver"]["name"], "cntrdct");
    let results = s["runs"][0]["results"].as_array().unwrap();
    assert!(results.is_empty());
}

#[test]
fn t2_finding_becomes_warning_result() {
    let f = make_finding(Severity::Warning);
    let s = to_sarif(&[f]);
    let r = &s["runs"][0]["results"][0];
    assert_eq!(r["ruleId"], "clone-drift");
    assert_eq!(r["level"], "warning");
    assert_eq!(r["message"]["text"], "function diverged");
}

#[test]
fn t3_severity_round_trip() {
    let cases: &[(Severity, &str)] = &[
        (Severity::Info, "none"),
        (Severity::Note, "note"),
        (Severity::Warning, "warning"),
        (Severity::Error, "error"),
    ];
    for (sev, expected) in cases {
        let f = make_finding(*sev);
        let s = to_sarif(&[f]);
        assert_eq!(
            s["runs"][0]["results"][0]["level"], *expected,
            "severity {:?} should map to {}",
            sev, expected
        );
    }
}

#[test]
fn t4_schema_field_present() {
    let s = to_sarif(&[]);
    assert!(s["$schema"].is_string(), "$schema must be a string");
}

#[test]
fn t5_location_fields_propagate() {
    let f = make_finding(Severity::Warning);
    let s = to_sarif(&[f]);
    let r = &s["runs"][0]["results"][0];

    let primary = &r["locations"][0]["physicalLocation"];
    assert_eq!(primary["artifactLocation"]["uri"], "a.rs");
    assert_eq!(primary["region"]["startLine"], 10);
    assert_eq!(primary["region"]["startColumn"], 5);
    assert_eq!(primary["region"]["endLine"], 15);
    assert_eq!(primary["region"]["endColumn"], 8);

    let related = &r["relatedLocations"][0]["physicalLocation"];
    assert_eq!(related["artifactLocation"]["uri"], "b.rs");
    assert_eq!(related["region"]["startLine"], 1);
    assert_eq!(related["region"]["endColumn"], 2);
}

#[test]
fn t6_pretty_string_is_valid_json() {
    let f = make_finding(Severity::Warning);
    let s = to_sarif_pretty(&[f]);
    let parsed: serde_json::Value =
        serde_json::from_str(&s).expect("pretty output must be valid JSON");
    assert_eq!(parsed["version"], "2.1.0");
    assert_eq!(parsed["runs"][0]["results"][0]["ruleId"], "clone-drift");
}

#[test]
fn t7_citation_keys_propagate() {
    let f = make_finding(Severity::Warning);
    let s = to_sarif(&[f]);
    let keys = &s["runs"][0]["results"][0]["properties"]["citationKeys"];
    let arr = keys.as_array().expect("citationKeys must be array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0], "cordy-roy-icpc-2008");
    assert_eq!(arr[1], "bettenburg-msr-2009");
}

#[test]
fn t8_legacy_to_sarif_has_no_rules_array() {
    let s = to_sarif(&[]);
    let driver = &s["runs"][0]["tool"]["driver"];
    // The legacy emitter is preserved for backwards compat: no rules taxonomy.
    assert!(
        driver.get("rules").is_none(),
        "legacy to_sarif must not emit a rules array; got {:?}",
        driver.get("rules")
    );
}

#[test]
fn t9_with_rules_emits_rule_per_detector() {
    let cd = FakeCloneDrift;
    let as_ = FakeArgSwap;
    let detectors: Vec<&dyn Detector> = vec![&cd, &as_];
    let s = to_sarif_with_rules(&[], &detectors);

    let rules = s["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("driver.rules must be an array");
    assert_eq!(rules.len(), 2, "one rule per registered detector");

    let ids: Vec<&str> = rules
        .iter()
        .map(|r| r["id"].as_str().expect("rule.id must be string"))
        .collect();
    assert!(ids.contains(&"clone-drift"));
    assert!(ids.contains(&"arg-swap"));
}

#[test]
fn t10_rule_short_description_is_detector_name() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let s = to_sarif_with_rules(&[], &detectors);
    let rule = &s["runs"][0]["tool"]["driver"]["rules"][0];
    assert_eq!(rule["id"], "clone-drift");
    assert_eq!(rule["shortDescription"]["text"], "Clone Drift");
}

#[test]
fn t11_rule_full_description_is_primary_citation_string() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let s = to_sarif_with_rules(&[], &detectors);
    let rule = &s["runs"][0]["tool"]["driver"]["rules"][0];
    assert_eq!(
        rule["fullDescription"]["text"],
        "J.R. Cordy, C.K. Roy. The NiCad Clone Detector. ICPC (2008)."
    );
}

#[test]
fn t12_help_uri_present_when_citation_url_some() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let s = to_sarif_with_rules(&[], &detectors);
    let rule = &s["runs"][0]["tool"]["driver"]["rules"][0];
    assert_eq!(rule["helpUri"], "https://example.invalid/nicad");
}

#[test]
fn t13_help_uri_omitted_when_citation_url_none() {
    let as_ = FakeArgSwap;
    let detectors: Vec<&dyn Detector> = vec![&as_];
    let s = to_sarif_with_rules(&[], &detectors);
    let rule = &s["runs"][0]["tool"]["driver"]["rules"][0];
    assert!(
        rule.get("helpUri").is_none(),
        "helpUri must be omitted when primary citation has no url; got {:?}",
        rule.get("helpUri")
    );
}

#[test]
fn t14_with_rules_preserves_results_payload() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let f = make_finding(Severity::Warning);
    let s = to_sarif_with_rules(&[f], &detectors);
    let r = &s["runs"][0]["results"][0];
    assert_eq!(r["ruleId"], "clone-drift");
    assert_eq!(r["level"], "warning");
    assert_eq!(r["message"]["text"], "function diverged");
}

#[test]
fn t16_anomaly_class_propagates_through_with_rules() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let f = make_finding_with_class(Severity::Warning, AnomalyClass::Logic);
    let s = to_sarif_with_rules(&[f], &detectors);
    let r = &s["runs"][0]["results"][0];
    assert_eq!(
        r["properties"]["anomalyClass"], "Logic",
        "result.properties.anomalyClass must be the IEEE 1044 class as a plain string"
    );
}

#[test]
fn t17_anomaly_class_propagates_through_legacy_to_sarif() {
    let f = make_finding_with_class(Severity::Warning, AnomalyClass::Interface);
    let s = to_sarif(&[f]);
    let r = &s["runs"][0]["results"][0];
    assert_eq!(r["properties"]["anomalyClass"], "Interface");
}

#[test]
fn t18_existing_properties_payload_preserved() {
    // Defends against future regressions where adding `anomalyClass` accidentally
    // overwrites the citation/raw payload also living under `result.properties`.
    let f = make_finding_with_class(Severity::Warning, AnomalyClass::Logic);
    let s = to_sarif(&[f]);
    let props = &s["runs"][0]["results"][0]["properties"];
    assert!(
        props["citationKeys"].is_array(),
        "citationKeys must still be present"
    );
    assert_eq!(props["citationKeys"][0], "cordy-roy-icpc-2008");
    assert_eq!(props["raw"]["group_size"], 5);
    assert_eq!(props["anomalyClass"], "Logic");
}

fn make_ranked(adj: Option<AdjudicationResult>) -> RankedFinding {
    RankedFinding {
        finding: make_finding_with_class(Severity::Warning, AnomalyClass::Logic),
        posterior_tp: Some(0.6),
        wilson_lower: Some(0.4),
        rank_score: 1.0,
        adjudication: adj,
    }
}

#[test]
fn t19_ranked_without_adjudication_omits_property() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let s = to_sarif_with_rules_ranked(&[make_ranked(None)], &detectors);
    let props = &s["runs"][0]["results"][0]["properties"];
    assert!(props.get("adjudication").is_none(), "must omit when None");
}

#[test]
fn t20_ranked_with_adjudication_surfaces_in_properties() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let adj = AdjudicationResult {
        verdict: AdjudicationVerdict::LikelyTruePositive,
        confidence: 0.85,
        rationale: "matches drift pattern".to_string(),
        calibration_tag: Some("T1.5".to_string()),
    };
    let s = to_sarif_with_rules_ranked(&[make_ranked(Some(adj))], &detectors);
    let a = &s["runs"][0]["results"][0]["properties"]["adjudication"];
    assert_eq!(a["verdict"], "LikelyTruePositive");
    assert_eq!(a["confidence"], 0.85);
    assert_eq!(a["rationale"], "matches drift pattern");
    assert_eq!(a["calibration_tag"], "T1.5");
}

#[test]
fn t15_with_rules_keeps_driver_identity() {
    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];
    let s = to_sarif_with_rules(&[], &detectors);
    let driver = &s["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "cntrdct");
    assert!(driver["version"].is_string());
    assert!(driver["informationUri"].is_string());
}
