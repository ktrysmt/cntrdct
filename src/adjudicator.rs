//! Layer 3 LLM adjudicator for cntrdct findings.
//!
//! Specs: `docs/spec/adjudicator-v0.md` (base), `docs/spec/llm-calibration-v0.md`
//! (Q-12 post-hoc Platt scaling), `docs/spec/cross-model-kappa-v0.md`
//! (Q-13 cross-model audit; CLI-shellout providers).
//!
//! Per design constraint P3 in `cntrdct-core`, this module is the SOLE
//! component permitted to invoke an LLM. Three concrete providers ship:
//! [`AnthropicAdjudicator`] (Anthropic Messages — the `--adjudicate`
//! default, HTTP via `reqwest`), [`ClaudeCliAdjudicator`] (Q-13 CLI
//! shellout to `claude --print`), and [`GeminiCliAdjudicator`] (Q-13
//! CLI shellout to `gemini -p`). The CLI providers do not introduce
//! a new HTTP path on the cntrdct side — auth is delegated to each
//! CLI's own login. Detector and Ranker modules remain network-free.
//!
//! References:
//! - `spiess-icse-2025` — Spiess et al., "Calibration and Correctness of
//!   Language Models for Code", ICSE 2025.
//! - `wataoka-2024` — K. Wataoka, T. Takahashi, R. Ri, "Self-Preference
//!   Bias in LLM-as-a-Judge", arXiv:2410.21819, 2024.
//! - `zheng-neurips-2023` — L. Zheng et al., "Judging LLM-as-a-Judge with
//!   MT-Bench and Chatbot Arena", NeurIPS 36, 46595–46623, 2023.

use std::collections::HashMap;
use std::time::Duration;

use crate::core::{
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

/// Provider id surfaced for the existing `scan --adjudicate` HTTP
/// path. Q-13's cross-model audit does not use this provider.
pub const ANTHROPIC_PROVIDER_ID: &str = "anthropic";

/// Q-13: default executable name for Claude Code's CLI.
pub const CLAUDE_CLI_PROGRAM: &str = "claude";
/// Q-13: default model passed to `claude --model`.
pub const CLAUDE_CLI_MODEL: &str = "claude-sonnet-4-6";
/// Q-13: provider id surfaced in cross-model audit logs.
pub const CLAUDE_CLI_PROVIDER_ID: &str = "claude-cli";

/// Q-13: default executable name for the Gemini CLI.
pub const GEMINI_CLI_PROGRAM: &str = "gemini";
/// Q-13: default model passed to `gemini -m`.
pub const GEMINI_CLI_MODEL: &str = "gemini-2.5-flash";
/// Q-13: provider id surfaced in cross-model audit logs.
pub const GEMINI_CLI_PROVIDER_ID: &str = "gemini-cli";

/// Q-13: minimal system prompt installed for both CLI providers. The
/// recipe assumes the CLI's default agentic persona is fully
/// overridden so the model receives essentially the user prompt only.
pub const CLI_SYSTEM_PROMPT: &str = "You are evaluating a static analysis finding from cntrdct. \
     Respond only with the requested JSON object on a single line. \
     Do not call tools, do not read files, do not produce additional \
     prose.";

// ---------- Citations (Layer 3) ----------

/// Citations for the Layer 3 adjudicator. Mirrors the `## Layer 3` section of
/// `CITATIONS.md`. Surfaced as a static so the citations-consistency test can
/// validate the contract between markdown and code for Layer 3, mirroring how
/// Layer 1 detectors expose `Detector::citations()`.
pub static ADJUDICATOR_CITATIONS: &[Citation] = &[
    Citation {
        key: "spiess-icse-2025",
        authors: "C. Spiess et al.",
        title: "Calibration and Correctness of Language Models for Code",
        venue: "ICSE",
        year: 2025,
        doi: None,
        url: None,
        // Layer 3 calibration is methodological — applies regardless of
        // the source language. Empty languages slice marks it as a general
        // / methodological reference per citations-policy.md.
        languages: &[],
    },
    Citation {
        key: "platt-1999",
        authors: "J. Platt",
        title: "Probabilistic Outputs for Support Vector Machines and Comparisons to Regularized Likelihood Methods",
        venue: "Advances in Large Margin Classifiers (MIT Press)",
        year: 1999,
        doi: None,
        url: None,
        languages: &[],
    },
    Citation {
        key: "spiess-koohestani-sergeyuk-2025",
        authors: "C. Spiess, P. Koohestani, A. Sergeyuk",
        title: "Verbalized Confidence in IDEs: A Large-Scale Empirical Study",
        venue: "arXiv:2510.22614",
        year: 2025,
        doi: None,
        url: None,
        languages: &[],
    },
    Citation {
        key: "wataoka-2024",
        authors: "K. Wataoka, T. Takahashi, R. Ri",
        title: "Self-Preference Bias in LLM-as-a-Judge",
        venue: "arXiv:2410.21819",
        year: 2024,
        doi: None,
        url: None,
        languages: &[],
    },
    Citation {
        key: "zheng-neurips-2023",
        authors: "L. Zheng et al.",
        title: "Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena",
        venue: "NeurIPS",
        year: 2023,
        doi: None,
        url: None,
        languages: &[],
    },
];

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

// ---------- PromptDispatch trait (Q-13) ----------

/// Lower-level dispatch surface used by the Q-13 cross-model audit. The
/// audit builds one prompt per finding and routes it to every configured
/// provider verbatim, so the `Adjudicator::adjudicate` entry point — which
/// internally constructs the prompt from a `RankedFinding` — is the wrong
/// shape. `PromptDispatch` exposes only the post-prompt half: send a
/// pre-built prompt, parse the provider-specific response envelope, and
/// hand back a uniform [`AdjudicationResult`].
///
/// Each shipped provider implements both `Adjudicator` and
/// `PromptDispatch`; the former calls into the latter so the wire-format
/// machinery lives in exactly one place.
///
/// Object-safe (`&self`, primitive arguments) so cross-model audits can
/// hold heterogeneous providers in a `Vec<Box<dyn PromptDispatch>>`.
pub trait PromptDispatch: Send + Sync {
    /// Stable id surfaced in cross-model audit logs (`"anthropic"`,
    /// `"openai"`, `"gemini"`). Used as the dictionary key in
    /// `AuditReport.providers` and as the prefix on pairwise pair labels.
    fn provider_id(&self) -> &'static str;
    /// Model name passed in the wire-format body. Mirrored into the
    /// audit log alongside `provider_id` so a regression caused by a
    /// model swap is recoverable from the log.
    fn model(&self) -> &str;
    /// Send `prompt` to the configured provider and return the
    /// adjudication result. Implementations are responsible for the
    /// provider-specific request body, headers, and response-envelope
    /// extraction; the inner JSON envelope is shared.
    fn dispatch(&self, prompt: &str) -> Result<AdjudicationResult, DetectorError>;
}

// ---------- ClaudeCliAdjudicator (Q-13) ----------

/// Q-13: shells out to Claude Code's `claude --print` with the
/// methodology-clean flag set documented in
/// `docs/spec/cross-model-kappa-v0.md` F2.
///
/// Auth is delegated to the user's existing `claude` login (OAuth /
/// subscription); the provider holds no API key. CLAUDE.md
/// auto-discovery is suppressed by spawning the subprocess with
/// `current_dir = <tempdir>`. The default flag set replaces Claude
/// Code's agentic persona with a minimal system prompt, disables
/// every built-in tool, and forces structured JSON output so the
/// inner verdict envelope is parseable byte-for-byte the same as
/// the HTTP path.
pub struct ClaudeCliAdjudicator {
    program: String,
    model: String,
    workdir: tempfile::TempDir,
}

impl ClaudeCliAdjudicator {
    /// Build a CLI adjudicator with default `program = "claude"` and
    /// `model = "claude-sonnet-4-6"`. Allocates a tempdir used as the
    /// subprocess `cwd` so CLAUDE.md auto-discovery picks up no
    /// project context.
    pub fn new() -> std::io::Result<Self> {
        let workdir = tempfile::tempdir()?;
        Ok(Self {
            program: CLAUDE_CLI_PROGRAM.to_string(),
            model: CLAUDE_CLI_MODEL.to_string(),
            workdir,
        })
    }

    /// Override the executable name / path. Used by tests to point at
    /// a stub script that emits a canned response envelope.
    pub fn with_program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    /// Override the model passed to `claude --model`.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl PromptDispatch for ClaudeCliAdjudicator {
    fn provider_id(&self) -> &'static str {
        CLAUDE_CLI_PROVIDER_ID
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dispatch(&self, prompt: &str) -> Result<AdjudicationResult, DetectorError> {
        let output = std::process::Command::new(&self.program)
            .current_dir(self.workdir.path())
            .arg("--print")
            .arg("--model")
            .arg(&self.model)
            .arg("--system-prompt")
            .arg(CLI_SYSTEM_PROMPT)
            .arg("--tools")
            .arg("")
            .arg("--strict-mcp-config")
            .arg("--disable-slash-commands")
            .arg("--no-session-persistence")
            .arg("--output-format")
            .arg("json")
            .arg(prompt)
            .output()
            .map_err(|e| DetectorError::Config(format!("claude CLI invoke failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectorError::Config(format!(
                "claude --print exited with {}: {}",
                output.status,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_claude_cli_envelope(stdout.as_ref()).map_err(DetectorError::from)
    }
}

// ---------- GeminiCliAdjudicator (Q-13) ----------

/// Q-13: shells out to the Gemini CLI's `gemini -p` with system-prompt
/// override via the `GEMINI_SYSTEM_MD` env var.
///
/// Auth is delegated to the user's existing `gemini` login (OAuth /
/// subscription); the provider holds no API key. GEMINI.md
/// auto-discovery is suppressed by spawning the subprocess with
/// `current_dir = <tempdir>`. The system prompt is written to a temp
/// file inside the same directory and exposed via `GEMINI_SYSTEM_MD`,
/// fully replacing the CLI's default agentic persona.
pub struct GeminiCliAdjudicator {
    program: String,
    model: String,
    workdir: tempfile::TempDir,
    system_prompt_path: std::path::PathBuf,
}

impl GeminiCliAdjudicator {
    /// Build a CLI adjudicator with default `program = "gemini"` and
    /// `model = "gemini-2.5-flash"`. Writes the minimal system prompt
    /// to a file in a fresh tempdir; the file is deleted along with
    /// the tempdir when the adjudicator is dropped.
    pub fn new() -> std::io::Result<Self> {
        let workdir = tempfile::tempdir()?;
        let system_prompt_path = workdir.path().join("system.md");
        std::fs::write(&system_prompt_path, CLI_SYSTEM_PROMPT)?;
        Ok(Self {
            program: GEMINI_CLI_PROGRAM.to_string(),
            model: GEMINI_CLI_MODEL.to_string(),
            workdir,
            system_prompt_path,
        })
    }

    /// Override the executable name / path. Used by tests to point at
    /// a stub script that emits a canned response envelope.
    pub fn with_program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    /// Override the model passed to `gemini -m`.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl PromptDispatch for GeminiCliAdjudicator {
    fn provider_id(&self) -> &'static str {
        GEMINI_CLI_PROVIDER_ID
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dispatch(&self, prompt: &str) -> Result<AdjudicationResult, DetectorError> {
        let output = std::process::Command::new(&self.program)
            .current_dir(self.workdir.path())
            .env("GEMINI_SYSTEM_MD", &self.system_prompt_path)
            .arg("-p")
            .arg(prompt)
            .arg("-m")
            .arg(&self.model)
            .arg("--output-format")
            .arg("json")
            .output()
            .map_err(|e| DetectorError::Config(format!("gemini CLI invoke failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectorError::Config(format!(
                "gemini -p exited with {}: {}",
                output.status,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_gemini_cli_envelope(stdout.as_ref()).map_err(DetectorError::from)
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
         {{\"verdict\": \"LikelyTruePositive\"|\"LikelyFalsePositive\"|\"Uncertain\", \"confidence\": <0.0-1.0>, \"rationale\": \"<one to three sentences>\"}}\n",
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
    parse_inner_text(text)
}

/// Q-13: parse the stdout envelope produced by `claude --output-format
/// json --print …`. Claude Code returns a single JSON object whose
/// `result` field carries the model's text response (which is itself
/// the verdict JSON envelope this module already understands).
pub(crate) fn parse_claude_cli_envelope(
    stdout: &str,
) -> Result<AdjudicationResult, AdjudicatorError> {
    let outer: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| AdjudicatorError::InnerJson(e.to_string()))?;
    let text = outer
        .get("result")
        .and_then(|t| t.as_str())
        .ok_or(AdjudicatorError::MissingContent)?;
    parse_inner_text(text)
}

/// Q-13: parse the stdout envelope produced by
/// `gemini -p … --output-format json`. The Gemini CLI returns a
/// single JSON object whose `response` field carries the model's text
/// response (which is itself the verdict JSON envelope).
pub(crate) fn parse_gemini_cli_envelope(
    stdout: &str,
) -> Result<AdjudicationResult, AdjudicatorError> {
    let outer: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| AdjudicatorError::InnerJson(e.to_string()))?;
    let text = outer
        .get("response")
        .and_then(|t| t.as_str())
        .ok_or(AdjudicatorError::MissingContent)?;
    parse_inner_text(text)
}

/// Shared inner-JSON parser. Every supported provider returns a text payload
/// that, after markdown-fence stripping, is the same JSON envelope:
/// `{"verdict": "...", "confidence": <0..1>, "rationale": "..."}`.
/// Centralising the parsing keeps the verdict / confidence policy
/// (clamp to `[0, 1]`, accept missing rationale, tolerate the legacy
/// pre-Q-12 `calibration_tag`) in exactly one place.
fn parse_inner_text(text: &str) -> Result<AdjudicationResult, AdjudicatorError> {
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
        calibrated_confidence: None,
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
    use crate::core::{
        AnomalyClass, Evidence, Finding, LanguageCitationStatus, Location, RankedFinding, Severity,
    };
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
                language_citation_status: LanguageCitationStatus::Confirmed,
            },
        }
    }

    fn make_ranked(prior: Option<(f64, f64)>) -> RankedFinding {
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
    fn build_prompt_no_longer_requests_calibration_tag() {
        // Q-12: the prompt schema dropped the verbalised calibration_tag.
        // Verbalised confidence does not improve ECE on average
        // (Spiess, Koohestani, Sergeyuk 2025); post-hoc Platt
        // scaling replaces this role.
        let rf = make_ranked(Some((0.6, 0.4)));
        let p = build_prompt(&rf, &HashMap::new());
        assert!(
            !p.contains("calibration_tag"),
            "Q-12: prompt must not request calibration_tag, got: {}",
            p
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

    #[test]
    fn adjudicator_citations_lists_q13_keys() {
        // Q-13: cross-model κ audit must cite Wataoka et al. 2024 and
        // Zheng et al. 2023, and the citations-consistency suite picks
        // these up via ADJUDICATOR_CITATIONS.
        assert!(ADJUDICATOR_CITATIONS
            .iter()
            .any(|c| c.key == "wataoka-2024"));
        assert!(ADJUDICATOR_CITATIONS
            .iter()
            .any(|c| c.key == "zheng-neurips-2023"));
    }

    // ---- CLI-envelope parsers (Q-13) ----

    #[test]
    fn parse_claude_cli_envelope_happy_path() {
        let stdout = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":42,"result":"{\"verdict\":\"LikelyTruePositive\",\"confidence\":0.7,\"rationale\":\"r\"}","session_id":"s"}"#;
        let res = parse_claude_cli_envelope(stdout).unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyTruePositive
        ));
        assert_eq!(res.confidence, 0.7);
    }

    #[test]
    fn parse_claude_cli_envelope_missing_result_errs() {
        let stdout = r#"{"type":"result","subtype":"error","is_error":true}"#;
        let err = parse_claude_cli_envelope(stdout).unwrap_err();
        assert!(matches!(err, AdjudicatorError::MissingContent));
    }

    #[test]
    fn parse_claude_cli_envelope_strips_markdown_fence() {
        let stdout = r#"{"type":"result","result":"```json\n{\"verdict\":\"Uncertain\",\"confidence\":0.4,\"rationale\":\"r\"}\n```"}"#;
        let res = parse_claude_cli_envelope(stdout).unwrap();
        assert!(matches!(res.verdict, AdjudicationVerdict::Uncertain));
    }

    #[test]
    fn parse_gemini_cli_envelope_happy_path() {
        let stdout = r#"{"response":"{\"verdict\":\"LikelyFalsePositive\",\"confidence\":0.9,\"rationale\":\"r\"}","stats":{}}"#;
        let res = parse_gemini_cli_envelope(stdout).unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyFalsePositive
        ));
        assert_eq!(res.confidence, 0.9);
    }

    #[test]
    fn parse_gemini_cli_envelope_missing_response_errs() {
        let stdout = r#"{"stats":{}}"#;
        let err = parse_gemini_cli_envelope(stdout).unwrap_err();
        assert!(matches!(err, AdjudicatorError::MissingContent));
    }

    #[test]
    fn parse_gemini_cli_envelope_strips_markdown_fence() {
        let stdout = r#"{"response":"```\n{\"verdict\":\"Uncertain\",\"confidence\":0.5,\"rationale\":\"r\"}\n```"}"#;
        let res = parse_gemini_cli_envelope(stdout).unwrap();
        assert!(matches!(res.verdict, AdjudicationVerdict::Uncertain));
    }

    // ---- ClaudeCliAdjudicator / GeminiCliAdjudicator dispatch via stub
    // bash scripts. The stub captures argv into a sidecar file the
    // assertions read so we can pin the methodology-clean flag set
    // structurally. ----

    fn write_stub_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).expect("write stub");
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        path
    }

    #[test]
    fn claude_cli_dispatch_passes_methodology_clean_flags() {
        let tmp = tempfile::tempdir().unwrap();
        let argv_log = tmp.path().join("argv.log");
        let stub_body = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\ncat <<'EOF'\n{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"duration_ms\":1,\"result\":\"{{\\\"verdict\\\":\\\"Uncertain\\\",\\\"confidence\\\":0.5,\\\"rationale\\\":\\\"r\\\"}}\",\"session_id\":\"s\"}}\nEOF\n",
            argv_log.display()
        );
        let stub = write_stub_script(tmp.path(), "claude-stub", &stub_body);

        let adj = ClaudeCliAdjudicator::new()
            .unwrap()
            .with_program(stub.to_string_lossy().into_owned());
        let res = adj.dispatch("PROMPT-BODY").unwrap();
        assert!(matches!(res.verdict, AdjudicationVerdict::Uncertain));

        let argv = std::fs::read_to_string(&argv_log).unwrap();
        // Pin every methodology-clean flag from the spec (F2).
        for flag in [
            "--print",
            "--model",
            CLAUDE_CLI_MODEL,
            "--system-prompt",
            CLI_SYSTEM_PROMPT,
            "--tools",
            "--strict-mcp-config",
            "--disable-slash-commands",
            "--no-session-persistence",
            "--output-format",
            "json",
            "PROMPT-BODY",
        ] {
            assert!(
                argv.lines().any(|l| l == flag),
                "claude argv missing {}: got\n{}",
                flag,
                argv
            );
        }
    }

    #[test]
    fn gemini_cli_dispatch_uses_system_md_env_and_flags() {
        let tmp = tempfile::tempdir().unwrap();
        let argv_log = tmp.path().join("argv.log");
        let env_log = tmp.path().join("env.log");
        let stub_body = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\nprintf '%s\\n' \"$GEMINI_SYSTEM_MD\" > {}\ncat <<'EOF'\n{{\"response\":\"{{\\\"verdict\\\":\\\"LikelyTruePositive\\\",\\\"confidence\\\":0.8,\\\"rationale\\\":\\\"r\\\"}}\",\"stats\":{{}}}}\nEOF\n",
            argv_log.display(),
            env_log.display()
        );
        let stub = write_stub_script(tmp.path(), "gemini-stub", &stub_body);

        let adj = GeminiCliAdjudicator::new()
            .unwrap()
            .with_program(stub.to_string_lossy().into_owned());
        let res = adj.dispatch("PROMPT-BODY").unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyTruePositive
        ));

        let argv = std::fs::read_to_string(&argv_log).unwrap();
        for flag in [
            "-p",
            "PROMPT-BODY",
            "-m",
            GEMINI_CLI_MODEL,
            "--output-format",
            "json",
        ] {
            assert!(
                argv.lines().any(|l| l == flag),
                "gemini argv missing {}: got\n{}",
                flag,
                argv
            );
        }

        let env_value = std::fs::read_to_string(&env_log).unwrap();
        let env_path = env_value.trim();
        assert!(
            !env_path.is_empty(),
            "GEMINI_SYSTEM_MD must be set on the spawned process"
        );
        let body = std::fs::read_to_string(env_path).expect("system prompt file readable");
        assert_eq!(body, CLI_SYSTEM_PROMPT);
    }

    #[test]
    fn claude_cli_provider_id_is_claude_cli() {
        let adj = ClaudeCliAdjudicator::new().unwrap();
        assert_eq!(
            <ClaudeCliAdjudicator as PromptDispatch>::provider_id(&adj),
            "claude-cli"
        );
        assert_eq!(adj.model(), CLAUDE_CLI_MODEL);
    }

    #[test]
    fn gemini_cli_provider_id_is_gemini_cli() {
        let adj = GeminiCliAdjudicator::new().unwrap();
        assert_eq!(
            <GeminiCliAdjudicator as PromptDispatch>::provider_id(&adj),
            "gemini-cli"
        );
        assert_eq!(adj.model(), GEMINI_CLI_MODEL);
    }

    #[test]
    fn cli_dispatch_surfaces_nonzero_exit_as_config_error() {
        let tmp = tempfile::tempdir().unwrap();
        let stub = write_stub_script(
            tmp.path(),
            "fail-stub",
            "#!/bin/sh\necho 'auth required' >&2\nexit 1\n",
        );
        let adj = ClaudeCliAdjudicator::new()
            .unwrap()
            .with_program(stub.to_string_lossy().into_owned());
        let err = adj.dispatch("p").unwrap_err();
        match err {
            DetectorError::Config(msg) => {
                assert!(msg.contains("claude --print exited"), "got: {}", msg);
                assert!(msg.contains("auth required"), "stderr propagated: {}", msg);
            }
            other => panic!("expected Config error, got {:?}", other),
        }
    }

    // ---- PromptDispatch object-safety (Q-13) ----

    #[test]
    fn prompt_dispatch_is_object_safe_across_providers() {
        // Cross-model audits hold Box<dyn PromptDispatch> with concrete
        // providers behind the trait object. Pin object-safety on the
        // shipped pair: AnthropicAdjudicator (HTTP, used by
        // scan --adjudicate) and one CLI provider.
        let m_ant = MockClient::ok(anthropic_response(
            "{\"verdict\":\"Uncertain\",\"confidence\":0.5,\"rationale\":\"r\"}",
        ));
        let claude = ClaudeCliAdjudicator::new().unwrap();
        let providers: Vec<Box<dyn PromptDispatch>> = vec![
            Box::new(AnthropicAdjudicator::new(m_ant, "k".to_string())),
            Box::new(claude),
        ];
        let ids: Vec<&'static str> = providers.iter().map(|p| p.provider_id()).collect();
        assert_eq!(ids, vec!["anthropic", "claude-cli"]);
    }
}
