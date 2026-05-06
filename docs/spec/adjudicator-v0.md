# cntrdct adjudicator v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

Layer 3 of the cntrdct stack. Reads `RankedFinding`s emitted by Layer 2 and
asks an LLM to label each one as a likely true positive, false positive, or
uncertain, returning a verbalised confidence and a per-model calibration tag.

The adjudicator is the FIRST and ONLY component in the workspace permitted to
invoke an LLM (design constraint P3). All detector and ranker code remains
deterministic and network-free.

## Scope

In scope:

- A new crate `crates/adjudicator-llm` exposing `AnthropicAdjudicator<C: HttpClient>`
- Anthropic Messages API integration via `reqwest::blocking` (rustls)
- A `--adjudicate` CLI flag on `cntrdct scan` plus `--adjudicate-top <N>`
- Extension of `RankedFinding` with `adjudication: Option<AdjudicationResult>`
- SARIF surfacing of adjudication under `result.properties.adjudication`
- Citation-consistency contract for Layer 3, mirroring the existing Layer 1
  contract

Out of scope (v0):

- Live integration tests against the real Anthropic endpoint
- Streaming responses
- Multi-model voting or self-consistency
- Cost / token accounting beyond the configurable `max_tokens` ceiling

## Functional requirements

### F1 — `RankedFinding.adjudication`

`cntrdct-core`'s `RankedFinding` gains:

```
pub adjudication: Option<AdjudicationResult>,
```

Serialised with `#[serde(skip_serializing_if = "Option::is_none")]` so the
field is omitted when adjudication did not run, keeping the JSON surface
backward-compatible for callers that pre-date Layer 3.

### F2 — `Adjudicator` trait (existing)

`cntrdct_core::Adjudicator` is unchanged. The new crate's
`AnthropicAdjudicator<C: HttpClient>` implements it.

### F3 — `HttpClient` seam

```
pub trait HttpClient: Send + Sync {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, AdjudicatorError>;
}
```

Production code wires in `ReqwestClient` (rustls-backed reqwest blocking).
Tests substitute their own implementation so the suite NEVER touches the
real Anthropic endpoint.

### F4 — `AnthropicAdjudicator`

```
pub struct AnthropicAdjudicator<C: HttpClient> {
    client: C,
    api_key: String,
    model: String,        // default DEFAULT_MODEL
    temperature: f64,     // default 0.0
    max_tokens: u32,      // default 1024
    url: String,          // default ANTHROPIC_API_URL
}
```

Constants:

| name | value |
|---|---|
| `DEFAULT_MODEL` | `claude-sonnet-4-6` |
| `DEFAULT_TEMPERATURE` | `0.0` |
| `DEFAULT_MAX_TOKENS` | `1024` |
| `ANTHROPIC_API_URL` | `https://api.anthropic.com/v1/messages` |
| `ANTHROPIC_VERSION` | `2023-06-01` |

Builder methods: `with_model`, `with_temperature`, `with_max_tokens`,
`with_url` (the last is the override seam used by integration tests so a
mockito server can stand in for the real endpoint).

### F5 — Prompt template

`build_prompt(rf: &RankedFinding, _: &HashMap<&str, &str>) -> String` is a
pure, deterministic helper. The output contains:

- A header instructing the model to evaluate a static analysis finding
- `DETECTOR:`, `MESSAGE:`, `SEVERITY:`, `ANOMALY_CLASS:`, `LOCATION:`,
  `CITATIONS:`, `STATISTICAL_PRIOR:` (formatted `posterior_tp=…, wilson_lower=…`
  when present, or the literal token `uncalibrated` otherwise),
  `EVIDENCE_RAW:` (pretty JSON of `finding.evidence.raw`)
- A trailer instructing the model to respond with a single-line JSON object
  matching:
  ```
  {"verdict": "LikelyTruePositive"|"LikelyFalsePositive"|"Uncertain",
   "confidence": <0.0-1.0>,
   "rationale": "<one to three sentences>",
   "calibration_tag": "T<scaling factor>"}
  ```

Determinism is guaranteed by allocation-only string construction with no
clock, RNG, or filesystem reads. Identical input → byte-identical output.

### F6 — Response parsing

`parse_response(raw: &Value) -> Result<AdjudicationResult, AdjudicatorError>`:

- Extracts `content[0].text` from the Anthropic Messages API response shape
- Strips a leading ` ``` ` or ` ```json ` fence and trailing ` ``` ` fence,
  if present (some models prefer to wrap their JSON in markdown)
- Parses the inner JSON
- Maps the `verdict` string to `AdjudicationVerdict` (any other value →
  `AdjudicatorError::InvalidVerdict`)
- Reads `confidence` and clamps to `[0.0, 1.0]` (see policy below)
- Reads `rationale` (defaults to `""` if missing)
- Reads `calibration_tag` as `Option<String>`

#### Confidence-clamping policy

The parser silently clamps confidence values outside `[0.0, 1.0]` to the
nearest valid value (e.g., `1.2 → 1.0`, `-0.5 → 0.0`). Rationale: language
models routinely emit values like `1.2` for "very high confidence"; rejecting
the response would force the CLI to either retry (cost) or drop the verdict
(loss of useful information). Surface stability wins. The clamp is recorded
only via the resulting numeric value — no warning is logged because the
events are too frequent to be useful and noisy stderr would break the JSON
output contract on stdout.

A missing `confidence` field is a hard error (`AdjudicatorError::MissingField`)
because there is no defensible default.

### F7 — `adjudicate` end-to-end

`AnthropicAdjudicator::adjudicate`:

1. Calls `build_prompt`
2. Builds the Messages API request body
   ```
   {
     "model": self.model,
     "max_tokens": self.max_tokens,
     "temperature": self.temperature,
     "messages": [{"role": "user", "content": <prompt>}]
   }
   ```
3. Headers: `("x-api-key", api_key)`, `("anthropic-version", ANTHROPIC_VERSION)`,
   `("content-type", "application/json")`
4. Calls `self.client.post_json(&self.url, &headers, &body)`
5. Parses via `parse_response`
6. Maps any error into `DetectorError::Config` via the `From` impl in
   `AdjudicatorError`

### F8 — CLI integration

`cntrdct scan` gains:

- `--adjudicate` (bool, default false)
- `--adjudicate-top <N>` (default 5)

When `--adjudicate` is set:

- The CLI reads `ANTHROPIC_API_KEY` from the environment via
  `cntrdct::read_anthropic_api_key()`. The helper treats a present but
  empty value as absent (defends against `export ANTHROPIC_API_KEY=` in a
  shell profile).
- If absent: a single line is emitted on stderr —
  `note: --adjudicate requested but ANTHROPIC_API_KEY not set; skipping adjudication`
  — and the run continues with `adjudication = None` on every finding. This
  preserves the P3 contract: nothing breaks if the key is missing; the linter
  remains useful.
- If present: an `AnthropicAdjudicator<ReqwestClient>` is constructed and
  passed to `adjudicate_top_n(&mut ranked, &adj, top_n)`, which fills the
  field in place for the top-N entries.
- The integration tests inject `ANTHROPIC_API_URL_OVERRIDE` to redirect the
  HTTP traffic to a mockito server. Production callers leave the variable
  unset and hit the real endpoint.

### F9 — SARIF surfacing

`cntrdct_sarif::to_sarif_with_rules_pretty_ranked(&[RankedFinding], &[&dyn Detector])`
emits the same shape as `to_sarif_with_rules_pretty(...)`, with one addition:
when a `RankedFinding.adjudication` is `Some`, the result's `properties` map
gains:

```
"adjudication": {
    "verdict": "LikelyTruePositive" | "LikelyFalsePositive" | "Uncertain",
    "confidence": <f64>,
    "rationale": "<string>",
    "calibration_tag": "<string>"   // only when present
}
```

The legacy `to_sarif_with_rules_pretty` is preserved unchanged for callers
that pass plain `Finding`s.

### F10 — Citations contract

`cntrdct-adjudicator-llm` exposes a `pub static ADJUDICATOR_CITATIONS:
&[Citation]` mirroring how detectors expose `Detector::citations()`. The CLI's
`citations_consistency` test enforces that:

- every key in `ADJUDICATOR_CITATIONS` appears under `## Layer 3` in
  `CITATIONS.md`
- every bullet under `## Layer 3` in `CITATIONS.md` appears in
  `ADJUDICATOR_CITATIONS`

This keeps the markdown ↔ code contract intact for Layer 3, mirroring the
Layer 1 contract.

## Non-functional requirements

- N1. P3 preserved — only this crate calls out to an LLM. Verified by a
  workspace grep at end of phase: no `reqwest` / `anthropic` / `api_key`
  references in any detector or ranker crate.
- N2. The API key is held as a `String` field on `AnthropicAdjudicator` and
  is forwarded exclusively via the `x-api-key` header. It is never logged,
  never appears in any `Display`/`Debug` impl that the CLI prints, and never
  appears in error messages. The `api_key_never_appears_in_error_messages`
  unit test guards this contract.
- N3. Network calls fire only when both `--adjudicate` is set AND
  `ANTHROPIC_API_KEY` resolves to a non-empty string. Either condition false →
  zero outbound traffic.
- N4. Tests MUST NOT contact the real Anthropic API. Unit tests use a
  hand-rolled mock client; the CLI integration test uses `mockito`.
- N5. Anomaly class — the adjudicator does not classify findings; it consumes
  `finding.anomaly_class` as input. No additions to `AnomalyClass`.

## HttpClient trait seam — testing rationale

The seam exists so the entire decision pipeline (prompt assembly, header
construction, response parsing, error mapping) is testable without a real
HTTP stack. `ReqwestClient` is therefore a thin shim — small enough that
"no integration test" is acceptable: there is no logic to regression-test
beyond what the trait already validates.

The seam is `pub trait HttpClient` (not a private implementation detail) so
downstream crates can plug in alternate transports (e.g., `ureq`, `hyper`)
without re-implementing the adjudicator.

## API key handling

- Source: process environment variable `ANTHROPIC_API_KEY`
- Lookup helper: `cntrdct::read_anthropic_api_key() -> Option<String>`
  (returns `None` for unset OR empty)
- Lifetime: held only in `AnthropicAdjudicator.api_key` for the duration of
  the scan; never written to disk, never logged
- Failure mode (key missing): silent skip with a single stderr note; exit code
  unchanged from a non-adjudicated scan

## Test plan

| ID  | Crate / file | Description |
|-----|---|---|
| A1  | core/src/lib.rs | RankedFinding omits adjudication when None |
| A2  | core/src/lib.rs | RankedFinding emits full adjudication object when Some |
| P1  | adjudicator-llm/src/lib.rs | build_prompt is deterministic for identical input |
| P2  | adjudicator-llm/src/lib.rs | build_prompt contains all required fields |
| P3  | adjudicator-llm/src/lib.rs | build_prompt formats uncalibrated prior |
| R1  | adjudicator-llm/src/lib.rs | parse_response handles each verdict variant |
| R2  | adjudicator-llm/src/lib.rs | parse_response strips ```json fence |
| R3  | adjudicator-llm/src/lib.rs | parse_response strips bare ``` fence |
| R4  | adjudicator-llm/src/lib.rs | parse_response clamps confidence above 1 |
| R5  | adjudicator-llm/src/lib.rs | parse_response clamps confidence below 0 |
| R6  | adjudicator-llm/src/lib.rs | parse_response missing confidence → Err |
| R7  | adjudicator-llm/src/lib.rs | parse_response invalid verdict → Err |
| R8  | adjudicator-llm/src/lib.rs | parse_response missing content[0] → Err |
| E1  | adjudicator-llm/src/lib.rs | adjudicate happy path returns expected verdict |
| E2  | adjudicator-llm/src/lib.rs | adjudicate sends correct headers / URL / body |
| E3  | adjudicator-llm/src/lib.rs | adjudicate maps HTTP error to DetectorError |
| E4  | adjudicator-llm/src/lib.rs | adjudicate maps inner JSON malformed to DetectorError |
| S1  | adjudicator-llm/src/lib.rs | API key never appears in error messages |
| S2  | sarif/tests/integration.rs | ranked without adjudication omits property |
| S3  | sarif/tests/integration.rs | ranked with adjudication surfaces in properties |
| C1  | cli/tests/citations_consistency.rs | adjudicator citations match Layer 3 section |
| C2  | cli/tests/citations_consistency.rs | adjudicator has at least one citation |
| I1  | cli/tests/adjudicate.rs | adjudicate_top_n populates only top-N findings |
| I2  | cli/tests/adjudicate.rs | CLI without API key skips with stderr note |
| I3  | cli/tests/adjudicate.rs | CLI with mock server populates top-N adjudications |

## Anomaly class subsection

N/A. The adjudicator consumes `finding.anomaly_class` as part of the prompt
context but does not classify or modify it.

## References

- `spiess-icse-2025` — C. Spiess et al., "Calibration and Correctness of
  Language Models for Code", ICSE 2025. Source of the verbalised confidence +
  per-model `calibration_tag` schema.
- `kremenek-engler-sas-2003` and `jung-kim-shin-yi-sas-2005` — supply the
  `posterior_tp` / `wilson_lower` features the adjudicator embeds in the
  STATISTICAL_PRIOR section of the prompt (Layer 2).
