//! Anthropic Messages HTTP provider (`scan --adjudicate-via=anthropic`).
//!
//! The crate's sole HTTP path: per design constraint P3, `reqwest` is
//! reachable only from [`ReqwestClient`] in this module, behind the
//! explicit `--adjudicate-via=anthropic` opt-in (needs
//! `ANTHROPIC_API_KEY`). The CLI-shellout providers (`claude-cli` /
//! `agy-cli`) never touch this module — they delegate auth and HTTP to
//! their respective CLIs.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};

use crate::core::{AdjudicationResult, Adjudicator, DetectorError, RankedFinding};

use super::{build_prompt, parse_response, AdjudicatorError, HttpClient, PromptDispatch};

// ---------- Constants ----------

pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
pub const DEFAULT_TEMPERATURE: f64 = 0.0;
pub const DEFAULT_MAX_TOKENS: u32 = 1024;
pub const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Provider id surfaced for the existing `scan --adjudicate` HTTP
/// path. Q-13's cross-model audit does not use this provider.
pub const ANTHROPIC_PROVIDER_ID: &str = "anthropic";

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

impl<C: HttpClient> PromptDispatch for AnthropicAdjudicator<C> {
    fn provider_id(&self) -> &'static str {
        ANTHROPIC_PROVIDER_ID
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dispatch(&self, prompt: &str) -> Result<AdjudicationResult, DetectorError> {
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

impl<C: HttpClient> Adjudicator for AnthropicAdjudicator<C> {
    fn adjudicate(&self, finding: &RankedFinding) -> Result<AdjudicationResult, DetectorError> {
        let prompt = build_prompt(finding, &HashMap::new());
        self.dispatch(&prompt)
    }
}

// ---------- ReqwestClient ----------

/// Production HTTP client backed by `reqwest::blocking` with rustls.
///
/// Thin shim around `reqwest`; all decision logic (prompt assembly, response
/// parsing, error mapping) lives in `AnthropicAdjudicator` and the pure
/// helpers in `mod.rs` so the only thing this struct contributes is wire
/// transport.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adjudicator::test_support::{anthropic_response, make_ranked, MockClient};
    use crate::core::AdjudicationVerdict;

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
}
