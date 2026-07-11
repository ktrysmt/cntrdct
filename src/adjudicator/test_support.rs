//! Shared test fixtures for the adjudicator modules.
//!
//! Compiled only under `#[cfg(test)]` (see the `mod` declaration in
//! `mod.rs`). Holds the mock HTTP client, canned findings, and the
//! stub-script writer used by the provider modules' unit tests so each
//! fixture lives in exactly one place.

use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::{json, Value};

use crate::core::{
    AnomalyClass, Evidence, Finding, LanguageCitationStatus, Location, RankedFinding, Severity,
};

use super::{AdjudicatorError, HttpClient, DEFAULT_MODEL};

// ---- Mock client ----

pub(crate) struct MockClient {
    pub(crate) response: Mutex<Result<Value, AdjudicatorError>>,
    pub(crate) last_url: Mutex<Option<String>>,
    pub(crate) last_headers: Mutex<Option<Vec<(String, String)>>>,
    pub(crate) last_body: Mutex<Option<Value>>,
}

impl MockClient {
    pub(crate) fn ok(v: Value) -> Self {
        Self {
            response: Mutex::new(Ok(v)),
            last_url: Mutex::new(None),
            last_headers: Mutex::new(None),
            last_body: Mutex::new(None),
        }
    }

    pub(crate) fn err(e: AdjudicatorError) -> Self {
        Self {
            response: Mutex::new(Err(e)),
            last_url: Mutex::new(None),
            last_headers: Mutex::new(None),
            last_body: Mutex::new(None),
        }
    }
}

impl HttpClient for MockClient {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Result<Value, AdjudicatorError> {
        *self.last_url.lock().unwrap() = Some(url.to_string());
        *self.last_headers.lock().unwrap() = Some(headers.to_vec());
        *self.last_body.lock().unwrap() = Some(body.clone());
        // Return a clone of the canned response. The mock holds it in a
        // Mutex so we don't need Sync-without-interior-mutability gymnastics
        // for the trait object.
        let guard = self.response.lock().unwrap();
        match &*guard {
            Ok(v) => Ok(v.clone()),
            Err(e) => Err(AdjudicatorError::Http(e.to_string())),
        }
    }
}

// ---- Fixtures ----

pub(crate) fn make_finding() -> Finding {
    Finding {
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
            language_citation_status: LanguageCitationStatus::Confirmed,
        },
        origin: Default::default(),
    }
}

pub(crate) fn make_ranked(prior: Option<(f64, f64)>) -> RankedFinding {
    let (posterior_tp, wilson_lower, prior_method) = match prior {
        Some((p, w)) => (
            Some(p),
            Some(w),
            Some(crate::calibration::PriorMethod::Wilson),
        ),
        None => (None, None, None),
    };
    RankedFinding {
        finding: make_finding(),
        posterior_tp,
        wilson_lower,
        prior_method,
        rank_score: 1.0,
        adjudication: None,
    }
}

pub(crate) fn anthropic_response(text: &str) -> Value {
    json!({
        "id": "msg_test",
        "type": "message",
        "role": "assistant",
        "model": DEFAULT_MODEL,
        "content": [{"type": "text", "text": text}],
        "stop_reason": "end_turn",
    })
}

// ---- Stub CLI scripts ----

pub(crate) fn write_stub_script(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write stub");
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}
