//! Layer 3 LLM adjudicator for cntrdct findings.
//!
//! Spec: `cntrdct/docs/spec/adjudicator-v0.md`.
//!
//! Per design constraint P3 in `cntrdct-core`, this crate is the SOLE component
//! permitted to invoke an LLM. Detector and Ranker crates remain network-free.
//!
//! Reference: `spiess-icse-2025` — Spiess et al., "Calibration and Correctness
//! of Language Models for Code", ICSE 2025. We do not replicate the paper's
//! experiments; we adopt the verbalised-confidence + per-model calibration tag
//! output schema.

use std::collections::HashMap;
use std::time::Duration;

use cntrdct_core::{
    AdjudicationResult, AdjudicationVerdict, Adjudicator, Citation, DetectorError, RankedFinding,
};
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

// ---------- Constants ----------

pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
pub const DEFAULT_TEMPERATURE: f64 = 0.0;
pub const DEFAULT_MAX_TOKENS: u32 = 1024;
pub const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

// ---------- Citations (Layer 3) ----------

/// Citations for the Layer 3 adjudicator. Mirrors the `## Layer 3` section of
/// `CITATIONS.md`. Surfaced as a static so the citations-consistency test can
/// validate the contract between markdown and code for Layer 3, mirroring how
/// Layer 1 detectors expose `Detector::citations()`.
pub static ADJUDICATOR_CITATIONS: &[Citation] = &[Citation {
    key: "spiess-icse-2025",
    authors: "C. Spiess et al.",
    title: "Calibration and Correctness of Language Models for Code",
    venue: "ICSE",
    year: 2025,
    doi: None,
    url: None,
}];

// ---------- Errors ----------

#[derive(Debug, Error)]
pub enum AdjudicatorError {
    #[error("http error: {0}")]
    Http(String),
    #[error("response missing content[0].text")]
    MissingContent,
    #[error("inner json parse error: {0}")]
    InnerJson(String),
    #[error("inner json missing required field: {0}")]
    MissingField(&'static str),
    #[error("invalid verdict string: {0}")]
    InvalidVerdict(String),
}

impl From<AdjudicatorError> for DetectorError {
    fn from(e: AdjudicatorError) -> Self {
        DetectorError::Config(format!("adjudicator: {}", e))
    }
}

// ---------- HttpClient seam ----------

/// HTTP transport seam used by `AnthropicAdjudicator`.
///
/// Production code wires in `ReqwestClient` (rustls-backed reqwest blocking).
/// Tests substitute a mock implementation so we never hit the live Anthropic
/// API. This is documented in the spec as the testing seam.
pub trait HttpClient: Send + Sync {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Result<Value, AdjudicatorError>;
}

// ---------- AnthropicAdjudicator ----------

#[derive(Debug, Clone)]
pub struct AnthropicAdjudicator<C: HttpClient> {
    client: C,
    api_key: String,
    model: String,
    temperature: f64,
    max_tokens: u32,
    url: String,
}

impl<C: HttpClient> AnthropicAdjudicator<C> {
    /// Build an adjudicator with the supplied transport and API key.
    ///
    /// `api_key` is held in memory and forwarded to the HTTP layer via the
    /// `x-api-key` header. It is NEVER logged and never appears in error
    /// messages.
    pub fn new(client: C, api_key: String) -> Self {
        Self {
            client,
            api_key,
            model: DEFAULT_MODEL.to_string(),
            temperature: DEFAULT_TEMPERATURE,
            max_tokens: DEFAULT_MAX_TOKENS,
            url: ANTHROPIC_API_URL.to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Override the API URL. Used by integration tests to point at a mock
    /// HTTP server (mockito); production callers leave this at the default.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }
}

impl<C: HttpClient> Adjudicator for AnthropicAdjudicator<C> {
    fn adjudicate(&self, finding: &RankedFinding) -> Result<AdjudicationResult, DetectorError> {
        let prompt = build_prompt(finding, &HashMap::new());

        let body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "messages": [{"role": "user", "content": prompt}],
        });

        let headers = vec![
            ("x-api-key".to_string(), self.api_key.clone()),
            (
                "anthropic-version".to_string(),
                ANTHROPIC_VERSION.to_string(),
            ),
            ("content-type".to_string(), "application/json".to_string()),
        ];

        let raw = self
            .client
            .post_json(&self.url, &headers, &body)
            .map_err(DetectorError::from)?;

        let result = parse_response(&raw).map_err(DetectorError::from)?;
        Ok(result)
    }
}

// ---------- Prompt construction ----------

/// Public escape hatch for diagnostics / examples: returns the exact prompt
/// the adjudicator would send for a given `RankedFinding`. Stable as long as
/// the prompt template is.
#[doc(hidden)]
pub fn __sample_build_prompt(rf: &RankedFinding, lookup: &HashMap<&str, &str>) -> String {
    build_prompt(rf, lookup)
}

/// Build the adjudication prompt for a single ranked finding.
///
/// Pure, deterministic, allocation-only; no network or filesystem.
/// `citations_lookup` is reserved for a future enrichment that swaps citation
/// keys for human-readable titles inline. v0 leaves it unused but keeps the
/// parameter so the API does not need to break later.
pub(crate) fn build_prompt(rf: &RankedFinding, _citations_lookup: &HashMap<&str, &str>) -> String {
    let f = &rf.finding;

    let location = format!(
        "{}:{}",
        f.primary.file.to_string_lossy(),
        f.primary.start_line
    );

    let citations = if f.evidence.citation_keys.is_empty() {
        "(none)".to_string()
    } else {
        f.evidence.citation_keys.join(",")
    };

    let prior = match (rf.posterior_tp, rf.wilson_lower) {
        (Some(p), Some(w)) => format!("posterior_tp={:.4}, wilson_lower={:.4}", p, w),
        _ => "uncalibrated".to_string(),
    };

    let raw_pretty =
        serde_json::to_string_pretty(&f.evidence.raw).unwrap_or_else(|_| "{}".to_string());

    format!(
        "You are evaluating a static analysis finding from cntrdct. Decide whether it is a true bug or a false positive.\n\
         \n\
         DETECTOR: {detector}\n\
         MESSAGE: {message}\n\
         SEVERITY: {severity:?}\n\
         ANOMALY_CLASS: {anomaly:?}\n\
         LOCATION: {location}\n\
         CITATIONS: {citations}\n\
         STATISTICAL_PRIOR: {prior}\n\
         EVIDENCE_RAW:\n{raw_pretty}\n\
         \n\
         Respond with a single JSON object on one line, exactly this shape:\n\
         {{\"verdict\": \"LikelyTruePositive\"|\"LikelyFalsePositive\"|\"Uncertain\", \"confidence\": <0.0-1.0>, \"rationale\": \"<one to three sentences>\", \"calibration_tag\": \"T<scaling factor>\"}}\n",
        detector = f.detector_id,
        message = f.message,
        severity = f.raw_severity,
        anomaly = f.anomaly_class,
        location = location,
        citations = citations,
        prior = prior,
        raw_pretty = raw_pretty,
    )
}

// ---------- Response parsing ----------

/// Parse an Anthropic Messages API response into an `AdjudicationResult`.
///
/// Expected outer shape: `{"content": [{"type":"text","text":"<inner json>"}]}`.
/// Markdown code fences (```json ... ``` or ``` ... ```) on the inner string
/// are stripped before parsing.
///
/// Confidence values outside `[0.0, 1.0]` are silently clamped (documented in
/// the spec as the "be liberal in what you accept" policy — many models emit
/// `1.2` or `-0.0`; we prefer surface stability over hard rejection).
pub(crate) fn parse_response(raw: &Value) -> Result<AdjudicationResult, AdjudicatorError> {
    let text = raw
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("text"))
        .and_then(|t| t.as_str())
        .ok_or(AdjudicatorError::MissingContent)?;

    let stripped = strip_code_fence(text);

    let inner: Value =
        serde_json::from_str(stripped).map_err(|e| AdjudicatorError::InnerJson(e.to_string()))?;

    let verdict_str = inner
        .get("verdict")
        .and_then(|v| v.as_str())
        .ok_or(AdjudicatorError::MissingField("verdict"))?;
    let verdict = match verdict_str {
        "LikelyTruePositive" => AdjudicationVerdict::LikelyTruePositive,
        "LikelyFalsePositive" => AdjudicationVerdict::LikelyFalsePositive,
        "Uncertain" => AdjudicationVerdict::Uncertain,
        other => return Err(AdjudicatorError::InvalidVerdict(other.to_string())),
    };

    let confidence_raw = inner
        .get("confidence")
        .and_then(|v| v.as_f64())
        .ok_or(AdjudicatorError::MissingField("confidence"))?;
    let confidence = confidence_raw.clamp(0.0, 1.0);

    let rationale = inner
        .get("rationale")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let calibration_tag = inner
        .get("calibration_tag")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(AdjudicationResult {
        verdict,
        confidence,
        rationale,
        calibration_tag,
    })
}

fn strip_code_fence(s: &str) -> &str {
    let trimmed = s.trim();
    let trimmed = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    let trimmed = trimmed.trim_start_matches('\n');
    let trimmed = trimmed.strip_suffix("```").unwrap_or(trimmed);
    trimmed.trim()
}

// ---------- ReqwestClient ----------

/// Production HTTP client backed by `reqwest::blocking` with rustls.
///
/// Thin shim around `reqwest`; all decision logic (prompt assembly, response
/// parsing, error mapping) lives in `AnthropicAdjudicator` and the pure
/// helpers above so the only thing this struct contributes is wire transport.
#[derive(Debug)]
pub struct ReqwestClient {
    inner: reqwest::blocking::Client,
}

impl ReqwestClient {
    /// Build a new client with a 60-second total timeout. The Anthropic
    /// Messages endpoint can take ~30s under load; 60s gives one round of
    /// headroom without leaving the CLI hung indefinitely on a network blip.
    pub fn new() -> Result<Self, AdjudicatorError> {
        let inner = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| AdjudicatorError::Http(e.to_string()))?;
        Ok(Self { inner })
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new().expect("reqwest blocking client builds with default settings")
    }
}

impl HttpClient for ReqwestClient {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Result<Value, AdjudicatorError> {
        let mut req = self.inner.post(url).json(body);
        for (k, v) in headers {
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .map_err(|e| AdjudicatorError::Http(e.to_string()))?;
        let status = resp.status();
        let body: Value = resp
            .json()
            .map_err(|e| AdjudicatorError::Http(e.to_string()))?;
        if !status.is_success() {
            // Deliberately do NOT include the API key (or any header value) in
            // error messages. The body alone is sufficient to debug typical
            // 4xx/5xx failures.
            return Err(AdjudicatorError::Http(format!(
                "status {}: {}",
                status, body
            )));
        }
        Ok(body)
    }
}

// ---------- Test helpers ----------

/// Mock HTTP client used by unit tests in this crate and by integration tests
/// in the CLI crate. Exposed as `pub` so downstream test code can build
/// `AnthropicAdjudicator<MockClient>` directly without reinventing the seam.
#[derive(Debug, Clone, Serialize)]
pub struct MockResponse {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cntrdct_core::{AnomalyClass, Evidence, Finding, Location, RankedFinding, Severity};
    use std::path::PathBuf;
    use std::sync::Mutex;

    // ---- Mock client ----

    struct MockClient {
        response: Mutex<Result<Value, AdjudicatorError>>,
        last_url: Mutex<Option<String>>,
        last_headers: Mutex<Option<Vec<(String, String)>>>,
        last_body: Mutex<Option<Value>>,
    }

    impl MockClient {
        fn ok(v: Value) -> Self {
            Self {
                response: Mutex::new(Ok(v)),
                last_url: Mutex::new(None),
                last_headers: Mutex::new(None),
                last_body: Mutex::new(None),
            }
        }

        fn err(e: AdjudicatorError) -> Self {
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

    fn make_finding() -> Finding {
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
            },
        }
    }

    fn make_ranked(prior: Option<(f64, f64)>) -> RankedFinding {
        let (posterior_tp, wilson_lower) = match prior {
            Some((p, w)) => (Some(p), Some(w)),
            None => (None, None),
        };
        RankedFinding {
            finding: make_finding(),
            posterior_tp,
            wilson_lower,
            rank_score: 1.0,
            adjudication: None,
        }
    }

    fn anthropic_response(text: &str) -> Value {
        json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": DEFAULT_MODEL,
            "content": [{"type": "text", "text": text}],
            "stop_reason": "end_turn",
        })
    }

    // ---- build_prompt tests ----

    #[test]
    fn build_prompt_is_deterministic_for_identical_input() {
        let rf = make_ranked(Some((0.6, 0.4)));
        let lookup = HashMap::new();
        let a = build_prompt(&rf, &lookup);
        let b = build_prompt(&rf, &lookup);
        assert_eq!(a, b);
    }

    #[test]
    fn build_prompt_contains_all_fields() {
        let rf = make_ranked(Some((0.6, 0.4)));
        let p = build_prompt(&rf, &HashMap::new());
        assert!(p.contains("clone-drift"), "detector_id missing: {}", p);
        assert!(
            p.contains("function diverged from 3 similar siblings"),
            "message missing"
        );
        assert!(p.contains("src/foo.rs:42"), "location missing");
        assert!(p.contains("cordy-roy-icpc-2008"), "citation key missing");
        assert!(p.contains("krinke-icsm-2007"), "citation key missing");
        assert!(p.contains("Logic"), "anomaly_class missing");
        assert!(p.contains("Warning"), "severity missing");
        assert!(
            p.contains("posterior_tp=0.6000"),
            "posterior_tp must be in prior section"
        );
        assert!(
            p.contains("LikelyTruePositive"),
            "instruction schema missing"
        );
    }

    #[test]
    fn build_prompt_handles_uncalibrated_prior() {
        let rf = make_ranked(None);
        let p = build_prompt(&rf, &HashMap::new());
        assert!(
            p.contains("STATISTICAL_PRIOR: uncalibrated"),
            "expected uncalibrated marker, got: {}",
            p
        );
    }

    // ---- parse_response: each verdict variant ----

    #[test]
    fn parse_response_likely_true_positive() {
        let raw = anthropic_response(
            "{\"verdict\":\"LikelyTruePositive\",\"confidence\":0.9,\"rationale\":\"r\",\"calibration_tag\":\"T1.0\"}",
        );
        let res = parse_response(&raw).unwrap();
        matches!(res.verdict, AdjudicationVerdict::LikelyTruePositive);
        assert_eq!(res.confidence, 0.9);
        assert_eq!(res.calibration_tag.as_deref(), Some("T1.0"));
    }

    #[test]
    fn parse_response_likely_false_positive() {
        let raw = anthropic_response(
            "{\"verdict\":\"LikelyFalsePositive\",\"confidence\":0.7,\"rationale\":\"r\"}",
        );
        let res = parse_response(&raw).unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyFalsePositive
        ));
        assert_eq!(res.calibration_tag, None);
    }

    #[test]
    fn parse_response_uncertain() {
        let raw = anthropic_response(
            "{\"verdict\":\"Uncertain\",\"confidence\":0.5,\"rationale\":\"r\"}",
        );
        let res = parse_response(&raw).unwrap();
        assert!(matches!(res.verdict, AdjudicationVerdict::Uncertain));
    }

    // ---- parse_response: failure modes ----

    #[test]
    fn parse_response_malformed_inner_json_errs() {
        let raw = anthropic_response("not json at all");
        let err = parse_response(&raw).unwrap_err();
        assert!(matches!(err, AdjudicatorError::InnerJson(_)));
    }

    #[test]
    fn parse_response_missing_content_errs() {
        let raw = json!({"foo": "bar"});
        let err = parse_response(&raw).unwrap_err();
        assert!(matches!(err, AdjudicatorError::MissingContent));
    }

    #[test]
    fn parse_response_strips_markdown_fence() {
        let raw = anthropic_response(
            "```json\n{\"verdict\":\"LikelyTruePositive\",\"confidence\":0.8,\"rationale\":\"r\"}\n```",
        );
        let res = parse_response(&raw).unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyTruePositive
        ));
        assert_eq!(res.confidence, 0.8);
    }

    #[test]
    fn parse_response_strips_bare_fence() {
        let raw = anthropic_response(
            "```\n{\"verdict\":\"Uncertain\",\"confidence\":0.5,\"rationale\":\"r\"}\n```",
        );
        let res = parse_response(&raw).unwrap();
        assert!(matches!(res.verdict, AdjudicationVerdict::Uncertain));
    }

    #[test]
    fn parse_response_clamps_confidence_above_one() {
        let raw = anthropic_response(
            "{\"verdict\":\"LikelyTruePositive\",\"confidence\":1.2,\"rationale\":\"r\"}",
        );
        let res = parse_response(&raw).unwrap();
        assert_eq!(res.confidence, 1.0);
    }

    #[test]
    fn parse_response_clamps_confidence_below_zero() {
        let raw = anthropic_response(
            "{\"verdict\":\"Uncertain\",\"confidence\":-0.5,\"rationale\":\"r\"}",
        );
        let res = parse_response(&raw).unwrap();
        assert_eq!(res.confidence, 0.0);
    }

    #[test]
    fn parse_response_missing_confidence_errs() {
        let raw = anthropic_response("{\"verdict\":\"LikelyTruePositive\",\"rationale\":\"r\"}");
        let err = parse_response(&raw).unwrap_err();
        assert!(matches!(err, AdjudicatorError::MissingField("confidence")));
    }

    #[test]
    fn parse_response_invalid_verdict_errs() {
        let raw =
            anthropic_response("{\"verdict\":\"Maybe\",\"confidence\":0.5,\"rationale\":\"r\"}");
        let err = parse_response(&raw).unwrap_err();
        assert!(matches!(err, AdjudicatorError::InvalidVerdict(_)));
    }

    // ---- adjudicate end-to-end with mock client ----

    #[test]
    fn adjudicate_happy_path_returns_verdict() {
        let mock = MockClient::ok(anthropic_response(
            "{\"verdict\":\"LikelyTruePositive\",\"confidence\":0.85,\"rationale\":\"matches drift\",\"calibration_tag\":\"T1.5\"}",
        ));
        let adj = AnthropicAdjudicator::new(mock, "test-key".to_string());
        let res = adj.adjudicate(&make_ranked(Some((0.6, 0.4)))).unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyTruePositive
        ));
        assert_eq!(res.confidence, 0.85);
        assert_eq!(res.rationale, "matches drift");
        assert_eq!(res.calibration_tag.as_deref(), Some("T1.5"));
    }

    #[test]
    fn adjudicate_sends_correct_headers_and_body() {
        let mock = MockClient::ok(anthropic_response(
            "{\"verdict\":\"Uncertain\",\"confidence\":0.5,\"rationale\":\"r\"}",
        ));
        let adj = AnthropicAdjudicator::new(mock, "secret-key".to_string())
            .with_url("https://example.test/v1/messages");
        adj.adjudicate(&make_ranked(None)).unwrap();

        let url = adj.client.last_url.lock().unwrap().clone().unwrap();
        assert_eq!(url, "https://example.test/v1/messages");

        let headers = adj.client.last_headers.lock().unwrap().clone().unwrap();
        let pairs: HashMap<String, String> = headers.into_iter().collect();
        assert_eq!(
            pairs.get("x-api-key").map(String::as_str),
            Some("secret-key")
        );
        assert_eq!(
            pairs.get("anthropic-version").map(String::as_str),
            Some(ANTHROPIC_VERSION)
        );
        assert_eq!(
            pairs.get("content-type").map(String::as_str),
            Some("application/json")
        );

        let body = adj.client.last_body.lock().unwrap().clone().unwrap();
        assert_eq!(body["model"], json!(DEFAULT_MODEL));
        assert_eq!(body["temperature"], json!(DEFAULT_TEMPERATURE));
        assert!(body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("DETECTOR: clone-drift"));
    }

    #[test]
    fn adjudicate_http_error_propagates() {
        let mock = MockClient::err(AdjudicatorError::Http("503".to_string()));
        let adj = AnthropicAdjudicator::new(mock, "k".to_string());
        let err = adj.adjudicate(&make_ranked(None)).unwrap_err();
        match err {
            DetectorError::Config(msg) => assert!(msg.contains("http error")),
            other => panic!("expected Config error, got {:?}", other),
        }
    }

    #[test]
    fn adjudicate_inner_json_malformed_errors() {
        let mock = MockClient::ok(anthropic_response("garbage"));
        let adj = AnthropicAdjudicator::new(mock, "k".to_string());
        let err = adj.adjudicate(&make_ranked(None)).unwrap_err();
        match err {
            DetectorError::Config(msg) => assert!(msg.contains("inner json")),
            other => panic!("expected Config error, got {:?}", other),
        }
    }

    #[test]
    fn api_key_never_appears_in_error_messages() {
        let secret = "sk-ant-XXXX-do-not-leak-XXXX";
        let mock = MockClient::err(AdjudicatorError::Http("boom".to_string()));
        let adj = AnthropicAdjudicator::new(mock, secret.to_string());
        let err = adj.adjudicate(&make_ranked(None)).unwrap_err();
        let msg = format!("{}", err);
        assert!(!msg.contains(secret), "API key leaked in error: {}", msg);
    }

    #[test]
    fn adjudicator_citations_lists_spiess() {
        assert!(ADJUDICATOR_CITATIONS
            .iter()
            .any(|c| c.key == "spiess-icse-2025"));
    }
}
