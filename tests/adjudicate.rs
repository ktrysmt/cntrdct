//! Integration tests for the `cntrdct scan --adjudicate` flow.
//!
//! Spec: `cntrdct/docs/spec/adjudicator-v0.md`.
//!
//! These tests NEVER hit the live Anthropic Messages API. They exercise the
//! adjudicator orchestration via:
//!   1) the in-process `Adjudicator` trait with mock implementations, and
//!   2) the CLI binary against a `mockito` HTTP server, with the API URL
//!      injected via the `ANTHROPIC_API_URL_OVERRIDE` env var that `main.rs`
//!      honours specifically for tests.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use cntrdct::adjudicator::{
    AdjudicatorError, AnthropicAdjudicator, HttpClient, MockResponse, ADJUDICATOR_CITATIONS,
    ANTHROPIC_VERSION, DEFAULT_MODEL,
};
use cntrdct::core::{AdjudicationResult, AdjudicationVerdict, RankedFinding};
use cntrdct::{adjudicate_top_n, rank_with_calibration, scan};
use serde_json::{json, Value};
use std::sync::Mutex;
use tempfile::tempdir;

const FN_BASE: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 {
            result.push(item * 2);
        }
    }
    result
}
"#;

const FN_DRIFTED: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 && item < 100 {
            result.push(item * 2);
        }
    }
    result
}
"#;

fn make_drift_dir() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    for (name, src) in [
        ("a.rs", FN_BASE),
        ("b.rs", FN_BASE),
        ("c.rs", FN_BASE),
        ("d.rs", FN_BASE),
        ("e.rs", FN_DRIFTED),
    ] {
        fs::write(dir.path().join(name), src).unwrap();
    }
    dir
}

// ---------- In-process mock adjudicator ----------

struct CannedClient {
    response: Mutex<Result<Value, AdjudicatorError>>,
}

impl CannedClient {
    fn ok(v: Value) -> Self {
        Self {
            response: Mutex::new(Ok(v)),
        }
    }
}

impl HttpClient for CannedClient {
    fn post_json(
        &self,
        _url: &str,
        _headers: &[(String, String)],
        _body: &Value,
    ) -> Result<Value, AdjudicatorError> {
        match &*self.response.lock().unwrap() {
            Ok(v) => Ok(v.clone()),
            Err(e) => Err(AdjudicatorError::Http(e.to_string())),
        }
    }
}

fn anthropic_text(text: &str) -> Value {
    json!({
        "id": "msg_test",
        "type": "message",
        "role": "assistant",
        "model": DEFAULT_MODEL,
        "content": [{"type": "text", "text": text}],
        "stop_reason": "end_turn",
    })
}

#[test]
fn adjudicate_top_n_populates_only_top_findings() {
    // Build several findings (we'll use the same scan path) then call the
    // helper with top_n smaller than the count.
    let dir = make_drift_dir();
    let findings = scan(dir.path()).expect("scan");
    let mut ranked: Vec<RankedFinding> = rank_with_calibration(findings, true, None).expect("rank");
    assert!(!ranked.is_empty());

    // Pad ranked so we can verify top-N partitioning.
    let template = ranked[0].clone();
    while ranked.len() < 3 {
        ranked.push(template.clone());
    }

    let mock_response = anthropic_text(
        "{\"verdict\":\"LikelyTruePositive\",\"confidence\":0.9,\"rationale\":\"r\",\"calibration_tag\":\"T1.5\"}",
    );
    let client = CannedClient::ok(mock_response);
    let adj = AnthropicAdjudicator::new(client, "test-key".to_string());

    adjudicate_top_n(&mut ranked, &adj, 1).expect("adjudicate ok");

    assert!(ranked[0].adjudication.is_some(), "top finding adjudicated");
    for r in &ranked[1..] {
        assert!(r.adjudication.is_none(), "tail must remain un-adjudicated");
    }

    let a = ranked[0].adjudication.as_ref().unwrap();
    assert!(matches!(a.verdict, AdjudicationVerdict::LikelyTruePositive));
    assert_eq!(a.calibration_tag.as_deref(), Some("T1.5"));
}

#[test]
fn adjudicator_citations_static_lists_spiess() {
    assert!(ADJUDICATOR_CITATIONS
        .iter()
        .any(|c| c.key == "spiess-icse-2025"));
}

// ---------- CLI binary against mock HTTP server ----------

fn cntrdct_bin() -> PathBuf {
    // Cargo sets CARGO_BIN_EXE_<name> for [[bin]] targets when running tests.
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
}

#[test]
fn cli_adjudicate_without_api_key_skips_silently_with_stderr_note() {
    let dir = make_drift_dir();
    let out = Command::new(cntrdct_bin())
        .arg("scan")
        .arg(dir.path())
        .arg("--adjudicate")
        .arg("--no-calibration")
        .env_remove("ANTHROPIC_API_KEY")
        .output()
        .expect("spawn cntrdct");
    assert!(out.status.success(), "scan must succeed even without key");

    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("ANTHROPIC_API_KEY not set"),
        "expected silent-skip note on stderr, got: {}",
        stderr
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    let arr = v.as_array().expect("array");
    for entry in arr {
        // adjudication field must be absent (skipped via skip_serializing_if)
        assert!(
            entry.get("adjudication").is_none(),
            "adjudication should be omitted when key absent: {}",
            entry
        );
    }
}

#[test]
fn cli_adjudicate_with_mock_server_populates_top_n() {
    let mut server = mockito::Server::new();

    let body = anthropic_text(
        "{\"verdict\":\"LikelyTruePositive\",\"confidence\":0.91,\"rationale\":\"clear drift\",\"calibration_tag\":\"T1.5\"}",
    );

    let _m = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body.to_string())
        .expect_at_least(1)
        .create();

    let url = format!("{}/v1/messages", server.url());

    let dir = make_drift_dir();
    let out = Command::new(cntrdct_bin())
        .arg("scan")
        .arg(dir.path())
        .arg("--adjudicate")
        .arg("--no-calibration")
        .env("ANTHROPIC_API_KEY", "sk-test-not-real")
        .env("ANTHROPIC_API_URL_OVERRIDE", &url)
        .output()
        .expect("spawn cntrdct");

    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.success(),
        "scan must succeed; stderr: {}",
        stderr
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout is JSON; err {}; body {}", e, stdout));
    let arr = v.as_array().expect("array").clone();
    assert!(!arr.is_empty(), "scan should produce at least one finding");

    // The first (top-ranked) entry must carry an adjudication object.
    let top = &arr[0];
    let adj = top
        .get("adjudication")
        .unwrap_or_else(|| panic!("top finding missing adjudication: {}", top));
    assert_eq!(adj["verdict"], "LikelyTruePositive");
    assert_eq!(adj["confidence"], 0.91);
    assert_eq!(adj["rationale"], "clear drift");
    assert_eq!(adj["calibration_tag"], "T1.5");

    // The mock should never have leaked the API key into the spawned
    // process's stderr (defence in depth).
    assert!(
        !stderr.contains("sk-test-not-real"),
        "API key leaked to stderr: {}",
        stderr
    );

    // Reference unused symbols so unused_imports does not fire when only
    // some tests are compiled.
    let _ = ANTHROPIC_VERSION;
    let _ = AdjudicationResult {
        verdict: AdjudicationVerdict::Uncertain,
        confidence: 0.5,
        rationale: String::new(),
        calibration_tag: None,
        calibrated_confidence: None,
    };
    let _ = MockResponse {
        url: String::new(),
        headers: vec![],
        body: Value::Null,
    };
}
