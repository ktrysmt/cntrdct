//! Helper binary used during phase 4 development to capture a sample
//! `build_prompt` output. Not shipped; lives in `examples/` so `cargo run
//! --example show_prompt -p cntrdct-adjudicator-llm` always works.

use cntrdct_core::{
    AnomalyClass, Evidence, Finding, Location, RankedFinding, Severity,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let f = Finding {
        detector_id: "clone-drift".to_string(),
        primary: Location {
            file: PathBuf::from("src/foo.rs"),
            start_line: 42,
            start_col: 1,
            end_line: 60,
            end_col: 1,
        },
        related: vec![Location {
            file: PathBuf::from("src/bar.rs"),
            start_line: 7,
            start_col: 1,
            end_line: 25,
            end_col: 1,
        }],
        message: "function diverged from 3 similar siblings".to_string(),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["cordy-roy-icpc-2008", "krinke-icsm-2007"],
            raw: json!({"group_size": 4, "similarity_threshold": 0.5}),
        },
    };
    let rf = RankedFinding {
        finding: f,
        posterior_tp: Some(0.6),
        wilson_lower: Some(0.4),
        rank_score: 1.0,
        adjudication: None,
    };
    let prompt = cntrdct_adjudicator_llm::__sample_build_prompt(&rf, &HashMap::new());
    println!("{}", prompt);
}
