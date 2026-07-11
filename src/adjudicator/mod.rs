//! Layer 3 LLM adjudicator for cntrdct findings.
//!
//! Specs: `docs/spec/adjudicator-v0.md` (base), `docs/spec/llm-calibration-v0.md`
//! (Q-12 post-hoc Platt scaling), `docs/spec/cross-model-kappa-v0.md`
//! (Q-13 cross-model audit; CLI-shellout providers).
//!
//! Per design constraint P3 in `cntrdct-core`, this module is the SOLE
//! component permitted to invoke an LLM. Three concrete providers ship,
//! one per submodule: [`AnthropicAdjudicator`] (Anthropic Messages — the
//! `--adjudicate` default, HTTP via `reqwest`; `anthropic.rs`),
//! [`ClaudeCliAdjudicator`] (CLI shellout to `claude --print`;
//! `claude_cli.rs`), and [`AgyCliAdjudicator`] (CLI shellout to `agy -p`,
//! Google Antigravity's multi-model CLI; `agy_cli.rs`). The CLI providers
//! do not introduce a new HTTP path on the cntrdct side — auth is
//! delegated to each CLI's own login. Detector and Ranker modules remain
//! network-free.
//!
//! This root module holds everything the providers share, so the policy
//! lives in exactly one place: the provider-agnostic [`PromptDispatch`] /
//! [`HttpClient`] seams, prompt construction ([`build_prompt`] and the
//! agy-specific compact variant), the response-envelope parsers, the
//! error type, the Layer 3 citations, and the usage-cap
//! [`FallbackAdjudicator`]. The submodules contribute transport only.
//!
//! Antigravity (`agy`) replaces the retired `gemini` CLI shellout: the
//! standalone Gemini CLI was folded into Antigravity upstream and is no
//! longer a distributed binary, so `gemini -p` no longer resolves on a
//! current install. `agy` is multi-model (Gemini / Claude / GPT-OSS), so
//! the self-preference guard (`candidate_llm::model_family`) classifies it
//! by the SELECTED MODEL string, not by the `agy-cli` provider id — the
//! shipped default forces a Gemini model so the provider stays
//! non-Anthropic (cross-family vs the `claude-cli` proposer / Anthropic
//! adjudicator). Unlike `claude`, `agy -p` has no `--output-format json`
//! or `--system-prompt` flag, so [`AgyCliAdjudicator`] parses the raw text
//! response directly and folds [`CLI_SYSTEM_PROMPT`] into the prompt body.
//!
//! All three CLI/HTTP providers implement [`PromptDispatch`]; the two CLI
//! providers additionally implement [`Adjudicator`] (via `build_prompt` +
//! `dispatch`) so `scan --adjudicate --adjudicate-via=claude-cli|agy-cli`
//! can run Layer 3 on subscription auth without an `ANTHROPIC_API_KEY`.
//!
//! References:
//! - `spiess-icse-2025` — Spiess et al., "Calibration and Correctness of
//!   Language Models for Code", ICSE 2025.
//! - `wataoka-2024` — K. Wataoka, T. Takahashi, R. Ri, "Self-Preference
//!   Bias in LLM-as-a-Judge", arXiv:2410.21819, 2024.
//! - `zheng-neurips-2023` — L. Zheng et al., "Judging LLM-as-a-Judge with
//!   MT-Bench and Chatbot Arena", NeurIPS 36, 46595–46623, 2023.

mod agy_cli;
mod anthropic;
mod claude_cli;
#[cfg(test)]
pub(crate) mod test_support;

pub use agy_cli::{
    AgyCliAdjudicator, AGY_CLI_MODEL, AGY_CLI_PROGRAM, AGY_CLI_PROVIDER_ID, AGY_SYSTEM_PROMPT,
};
pub use anthropic::{
    AnthropicAdjudicator, ReqwestClient, ANTHROPIC_API_URL, ANTHROPIC_PROVIDER_ID,
    ANTHROPIC_VERSION, DEFAULT_MAX_TOKENS, DEFAULT_MODEL, DEFAULT_TEMPERATURE,
};
pub use claude_cli::{
    ClaudeCliAdjudicator, CLAUDE_CLI_ADJUDICATE_MODEL, CLAUDE_CLI_MODEL, CLAUDE_CLI_PROGRAM,
    CLAUDE_CLI_PROVIDER_ID, CLI_SYSTEM_PROMPT,
};

use std::collections::HashMap;

use crate::core::{
    AdjudicationResult, AdjudicationVerdict, Adjudicator, Citation, DetectorError, RankedFinding,
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

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

// ---------- FallbackAdjudicator ----------

/// True when `e` looks like the Claude subscription usage cap being hit
/// (the "$200" Max-plan limit, a 5-hour / weekly limit, or a rate limit).
/// Used by [`FallbackAdjudicator`] to decide when to switch from the
/// primary `claude -p` adjudicator to the `agy` fallback. Matched on
/// case-insensitive substrings of the surfaced error text — a deliberate
/// heuristic, since `claude --print`'s exact wording is not a stable API.
/// Other errors (malformed JSON, transient network) are NOT limit errors
/// and propagate normally.
pub fn is_usage_limit_error(e: &DetectorError) -> bool {
    let msg = e.to_string().to_ascii_lowercase();
    const NEEDLES: &[&str] = &[
        "usage limit",
        "limit reached",
        "reached your",
        "rate limit",
        "rate_limit",
        "too many requests",
        "out of credits",
        "insufficient credit",
        "quota",
        "429",
    ];
    NEEDLES.iter().any(|n| msg.contains(n))
}

/// Adjudicator that tries a `primary` and, ONLY when the primary fails with
/// a usage-limit error ([`is_usage_limit_error`]), retries on a `fallback`.
///
/// The shipped default chains `claude -p` (Haiku) → `agy` (Gemini): when the
/// Claude subscription hits its `$200` cap, adjudication transparently
/// continues on Antigravity instead of failing. A non-limit primary error
/// (malformed response, etc.) propagates without invoking the fallback, so
/// the fallback is reserved for the cap case the user asked for. Note the
/// fallback (`agy`, google) is a DIFFERENT model family than the
/// claude primary, so when it engages the verdict is cross-family.
pub struct FallbackAdjudicator {
    primary: Box<dyn Adjudicator>,
    fallback: Box<dyn Adjudicator>,
}

impl FallbackAdjudicator {
    pub fn new(primary: Box<dyn Adjudicator>, fallback: Box<dyn Adjudicator>) -> Self {
        Self { primary, fallback }
    }
}

impl Adjudicator for FallbackAdjudicator {
    fn adjudicate(&self, finding: &RankedFinding) -> Result<AdjudicationResult, DetectorError> {
        match self.primary.adjudicate(finding) {
            Ok(result) => Ok(result),
            Err(e) if is_usage_limit_error(&e) => {
                eprintln!(
                    "note: primary adjudicator hit a usage limit ({}); falling back to the secondary adjudicator",
                    e
                );
                self.fallback.adjudicate(finding)
            }
            Err(e) => Err(e),
        }
    }
}

// ---------- Prompt construction ----------

/// Compact, single-line-friendly adjudication prompt for the `agy`
/// provider. Mirrors the decision the verbose [`build_prompt`] asks for but
/// strips the labelled multi-field layout and the `EVIDENCE_RAW` JSON
/// block that trip agy's agentic persona. Evidence is rendered as flat
/// plain-text `k=v` pairs ([`render_evidence_plain`]) — NO nested `{...}`
/// JSON object, which (even compact / flattened) reliably makes `agy -p`
/// hang or return empty. Verified against `Gemini 3.5 Flash (Low)`.
pub(crate) fn build_compact_prompt(rf: &RankedFinding) -> String {
    let f = &rf.finding;
    let location = format!(
        "{}:{}",
        f.primary.file.to_string_lossy(),
        f.primary.start_line
    );
    let evidence = render_evidence_plain(&f.evidence.raw);
    format!(
        "Classify this static-analysis finding as a real bug or a false positive. \
         detector={detector}; message={message}; location={location}; evidence: {evidence}. \
         Respond with exactly one JSON object and nothing else: \
         {{\"verdict\":\"LikelyTruePositive\"|\"LikelyFalsePositive\"|\"Uncertain\",\"confidence\":<0.0-1.0>,\"rationale\":\"<one short sentence>\"}} \
         where LikelyTruePositive means it is a real bug.",
        detector = f.detector_id,
        message = f.message,
        location = location,
        evidence = evidence,
    )
}

/// Flatten an evidence-`raw` JSON object into plain-text `k=v` pairs for
/// the `agy` compact prompt: no nested `{...}` braces (which trip agy's
/// agentic persona), and dropping the proposer's own verdict fields
/// (`llm_rationale` / `llm_confidence` / `origin`) — those would both bloat
/// the prompt and bias the judge toward the proposer's conclusion (a mini
/// self-preference; the cross-family judge must re-decide from facts).
/// Object-valued and over-long fields are skipped.
fn render_evidence_plain(raw: &Value) -> String {
    let Some(obj) = raw.as_object() else {
        return String::new();
    };
    let mut parts = Vec::new();
    for (k, v) in obj {
        if matches!(k.as_str(), "llm_rationale" | "llm_confidence" | "origin") {
            continue;
        }
        let rendered = match v {
            Value::String(s) if s.len() <= 120 => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .filter_map(|x| match x {
                        Value::String(s) => Some(s.clone()),
                        Value::Number(n) => Some(n.to_string()),
                        Value::Bool(b) => Some(b.to_string()),
                        _ => None,
                    })
                    .collect();
                format!("[{}]", items.join(", "))
            }
            // Skip nested objects and over-long strings.
            _ => continue,
        };
        parts.push(format!("{k}={rendered}"));
    }
    parts.join(", ")
}

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

/// Parse the stdout produced by `agy --print …`. Unlike `claude` /
/// `gemini`, Antigravity's `agy -p` has no `--output-format json` flag —
/// it prints the model's text response directly with no outer envelope.
/// That text IS the verdict JSON envelope (after markdown-fence
/// stripping), so this parser hands the raw stdout straight to the shared
/// inner-text parser.
pub(crate) fn parse_agy_cli_envelope(stdout: &str) -> Result<AdjudicationResult, AdjudicatorError> {
    parse_inner_text(stdout)
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
    use super::test_support::{anthropic_response, make_ranked, write_stub_script, MockClient};
    use super::*;
    use serde_json::json;

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

    // ---- citations ----

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
    fn parse_agy_cli_envelope_happy_path() {
        // agy prints the model's raw text response with no outer envelope.
        let stdout = "{\"verdict\":\"LikelyFalsePositive\",\"confidence\":0.9,\"rationale\":\"r\"}";
        let res = parse_agy_cli_envelope(stdout).unwrap();
        assert!(matches!(
            res.verdict,
            AdjudicationVerdict::LikelyFalsePositive
        ));
        assert_eq!(res.confidence, 0.9);
    }

    #[test]
    fn parse_agy_cli_envelope_non_json_errs() {
        let stdout = "I am currently using Gemini 3.5 Flash.";
        let err = parse_agy_cli_envelope(stdout).unwrap_err();
        assert!(matches!(err, AdjudicatorError::InnerJson(_)));
    }

    #[test]
    fn parse_agy_cli_envelope_strips_markdown_fence() {
        let stdout =
            "```json\n{\"verdict\":\"Uncertain\",\"confidence\":0.5,\"rationale\":\"r\"}\n```";
        let res = parse_agy_cli_envelope(stdout).unwrap();
        assert!(matches!(res.verdict, AdjudicationVerdict::Uncertain));
    }

    // ---- cross-provider integration ----

    #[test]
    fn cli_providers_implement_adjudicator_via_stub() {
        // Task 1: ClaudeCliAdjudicator / AgyCliAdjudicator are usable as
        // the Layer 3 `Adjudicator` (scan --adjudicate-via). Route a
        // RankedFinding through each via a stub that echoes a verdict.
        let tmp = tempfile::tempdir().unwrap();
        // claude returns the JSON envelope inside `result`.
        let claude_stub = write_stub_script(
            tmp.path(),
            "claude-stub",
            "#!/bin/sh\ncat <<'EOF'\n{\"type\":\"result\",\"result\":\"{\\\"verdict\\\":\\\"LikelyTruePositive\\\",\\\"confidence\\\":0.7,\\\"rationale\\\":\\\"r\\\"}\"}\nEOF\n",
        );
        // agy returns the raw text envelope.
        let agy_stub = write_stub_script(
            tmp.path(),
            "agy-stub2",
            "#!/bin/sh\ncat <<'EOF'\n{\"verdict\":\"LikelyFalsePositive\",\"confidence\":0.6,\"rationale\":\"r\"}\nEOF\n",
        );

        let claude: Box<dyn Adjudicator> = Box::new(
            ClaudeCliAdjudicator::new()
                .unwrap()
                .with_program(claude_stub.to_string_lossy().into_owned()),
        );
        let agy: Box<dyn Adjudicator> = Box::new(
            AgyCliAdjudicator::new()
                .unwrap()
                .with_program(agy_stub.to_string_lossy().into_owned()),
        );

        let rf = make_ranked(None);
        assert!(matches!(
            claude.adjudicate(&rf).unwrap().verdict,
            AdjudicationVerdict::LikelyTruePositive
        ));
        assert!(matches!(
            agy.adjudicate(&rf).unwrap().verdict,
            AdjudicationVerdict::LikelyFalsePositive
        ));
    }

    // ---- FallbackAdjudicator (claude -p usage cap → agy) ----

    /// Canned `Adjudicator` returning a fixed verdict or error, freshly
    /// constructed per call (so `DetectorError`'s non-`Clone` is a non-issue).
    enum CannedAdj {
        Ok(AdjudicationVerdict),
        Err(&'static str),
    }
    impl Adjudicator for CannedAdj {
        fn adjudicate(&self, _f: &RankedFinding) -> Result<AdjudicationResult, DetectorError> {
            match self {
                CannedAdj::Ok(v) => Ok(AdjudicationResult {
                    verdict: *v,
                    confidence: 0.9,
                    rationale: "canned".to_string(),
                    calibration_tag: None,
                    calibrated_confidence: None,
                }),
                CannedAdj::Err(m) => Err(DetectorError::Config(m.to_string())),
            }
        }
    }

    #[test]
    fn is_usage_limit_error_matches_cap_messages() {
        for msg in [
            "claude --print exited with 1: Claude usage limit reached",
            "adjudicator: status 429: too many requests",
            "rate limit exceeded",
            "you have reached your weekly limit",
        ] {
            assert!(
                is_usage_limit_error(&DetectorError::Config(msg.to_string())),
                "should classify as usage-limit: {msg}"
            );
        }
        // Non-limit errors must NOT trigger the fallback.
        for msg in [
            "adjudicator: inner json parse error: expected value at line 1 column 1",
            "claude CLI invoke failed: No such file or directory",
        ] {
            assert!(
                !is_usage_limit_error(&DetectorError::Config(msg.to_string())),
                "should NOT classify as usage-limit: {msg}"
            );
        }
    }

    #[test]
    fn fallback_engages_only_on_usage_limit() {
        let rf = make_ranked(None);

        // Primary hits the cap → fallback's verdict is returned.
        let fb = FallbackAdjudicator::new(
            Box::new(CannedAdj::Err("usage limit reached")),
            Box::new(CannedAdj::Ok(AdjudicationVerdict::LikelyFalsePositive)),
        );
        assert!(matches!(
            fb.adjudicate(&rf).unwrap().verdict,
            AdjudicationVerdict::LikelyFalsePositive
        ));

        // Primary succeeds → fallback is never consulted (primary verdict).
        let fb = FallbackAdjudicator::new(
            Box::new(CannedAdj::Ok(AdjudicationVerdict::LikelyTruePositive)),
            Box::new(CannedAdj::Ok(AdjudicationVerdict::LikelyFalsePositive)),
        );
        assert!(matches!(
            fb.adjudicate(&rf).unwrap().verdict,
            AdjudicationVerdict::LikelyTruePositive
        ));

        // Primary fails with a NON-limit error → the error propagates and the
        // fallback is NOT used.
        let fb = FallbackAdjudicator::new(
            Box::new(CannedAdj::Err("inner json parse error")),
            Box::new(CannedAdj::Ok(AdjudicationVerdict::LikelyFalsePositive)),
        );
        assert!(fb.adjudicate(&rf).is_err());
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
