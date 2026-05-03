---
name: cntrdct
description: Evidence-based contradiction linter. Scans Rust code for drifted clones — code fragments that look identical but have diverged inconsistently, which often indicates a fix that was missed in some copies. Every finding cites the peer-reviewed paper that justifies the detection. Use when the user asks to scan/lint/check for inconsistencies, drifted clones, or contradictions.
allowed-tools: Bash, Read
---

# cntrdct skill

This skill is the entry point to the `cntrdct` Rust binary. The skill itself
performs no detection; it orchestrates the binary and presents results.

## Activation

Activate when the user invokes `/cntrdct` or asks to:

- "scan for contradictions / drifted clones / inconsistencies"
- "lint with cntrdct"
- "check for clone drift in this repo"

If the user provides a path argument, scan that path. Otherwise scan the current
working directory.

For the LLM-backed Layer 3 verdict on top findings, pass `--adjudicate` to the
binary; this requires `ANTHROPIC_API_KEY` in the environment. When the key is
absent the binary skips adjudication silently (with a stderr note) and the
scan still completes — never block on adjudication.

## Step 1 — Verify the binary

Run `command -v cntrdct` via Bash.

- If the command exits 0, the binary is on PATH. Continue to step 2.
- If exit non-zero, also check `~/.cargo/bin/cntrdct` via `test -x ~/.cargo/bin/cntrdct`.
- If still missing, tell the user:

  > `cntrdct` binary not found. Install with `cargo install --path crates/cli`
  > from the cntrdct repository, or `cargo install cntrdct` once a release ships.

  Stop. Do not attempt to run scans.

## Step 2 — Run the scan

Run via Bash:

```
cntrdct scan <path> --format json [--adjudicate]
```

`<path>` is the user-provided argument or `.` if none given. Pass
`--adjudicate` only when the user explicitly asks for an LLM verdict;
otherwise default to plain ranked output. The binary requires
`ANTHROPIC_API_KEY` to actually contact the model — without it the run
prints a one-line stderr note and continues without `adjudication` fields.

`--format json` returns a JSON array of `RankedFinding` objects sorted by
`rank_score` descending. Each entry has:

```
{
  "finding": {
    "detector_id": "clone-drift",
    "primary":  { "file": "...", "start_line": N, "start_col": N, "end_line": N, "end_col": N },
    "related":  [ { "file": "...", "start_line": N, ... }, ... ],
    "message":  "function diverged from N similar siblings",
    "raw_severity": "Warning",
    "evidence": {
      "citation_keys": ["cordy-roy-icpc-2008", "bettenburg-msr-2009"],
      "raw": { "group_size": N, "partition_sizes": [...], "similarity_threshold": 0.5 }
    }
  },
  "posterior_tp": null,
  "wilson_lower": null,
  "rank_score": N.0
}
```

`posterior_tp` and `wilson_lower` are `null` for findings whose detector has no
labelled corpus prior; otherwise they carry the calibrated posterior and Wilson
lower bound from `cntrdct calibrate`. `rank_score` orders the list (higher is
more salient).

When `--adjudicate` is passed, each top-N entry may also carry an
`adjudication` object:

```
"adjudication": {
  "verdict": "LikelyTruePositive" | "LikelyFalsePositive" | "Uncertain",
  "confidence": 0.0 - 1.0,
  "rationale": "<one to three sentences>",
  "calibration_tag": "T<scaling factor>"   // optional; per spiess-icse-2025
}
```

The field is omitted when adjudication did not run (no API key, finding outside
top-N, or transient error).

## Step 3 — Summarize for the user

Present the top 10 findings (or all if fewer) in this format:

```
N findings (showing top 10):

1. <relative path>:<start_line>  (rank_score: <N>)
   <message>
   Cited: <citation_key list>
   Similar in: <up to 3 related file:line entries>
   Verdict: <verdict> (confidence <0.0-1.0>) — <rationale>   # only when adjudication present

2. ...
```

The `Verdict:` line appears only when the finding has an `adjudication` object
(i.e., the user passed `--adjudicate` and the call succeeded). Render the
rationale verbatim — do not paraphrase or expand it.

Then add one paragraph summarizing:

- How many total findings
- The dominant detector (e.g., clone-drift only in v0)
- The single highest rank_score and what it represents
- A reminder that Layer 2 statistics (`posterior_tp`, `wilson_lower`) are
  uncalibrated in v0 — the ordering is by sibling count, not by true-positive
  probability.

## Step 4 — Citation reference (when asked)

When the user asks "why is this a finding?" or "what's the evidence?", cite
the specific paper from the finding's `citation_keys`. The mapping is:

clone-drift detector:

- `cordy-roy-icpc-2008` — Cordy & Roy, "The NiCad Clone Detector", ICPC 2008.
  Defines Type-3 near-miss clone detection by AST normalization.
- `bettenburg-msr-2009` — Bettenburg et al., "An Empirical Study on Inconsistent
  Changes to Code Clones at the Release Level", MSR 2009. Empirical evidence
  that inconsistent clone evolution is a real bug source.
- `krinke-icsm-2007` — Krinke, "A Study of Consistent and Inconsistent Changes
  to Code Clones", ICSM 2007. Quantifies how often clones evolve out of sync
  and the bug-risk window that drift opens.

arg-swap detector:

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically Extracting
  Implicit Programming Rules and Detecting Violations in Large Software Code",
  ESEC/FSE 2005. Foundational implicit-rule mining behind argument-order checks.
- `rice-icse-2017` — A. Rice et al., "Detecting Argument Selection Defects",
  ICSE 2017. Industrial evidence that argument-name swaps are a recurring
  defect class detectable by lightweight static analysis.

Layer 3 LLM adjudicator (only when `--adjudicate` is in use):

- `spiess-icse-2025` — C. Spiess et al., "Calibration and Correctness of
  Language Models for Code", ICSE 2025. Source of the verbalised confidence +
  per-model `calibration_tag` (e.g. "T1.5") schema returned by the
  adjudicator.

Show DOI / venue when relevant. Do not invent additional citations.

## Constraints

- Do NOT execute detection logic in this skill (P3: only the future Layer 3
  adjudicator may invoke an LLM, and it lives in the Rust binary, not here).
- Do NOT modify or summarize code that wasn't returned by the binary.
- If the binary returns 0 findings, say so plainly. Do not invent issues.
- Do NOT re-run the scan to "double check" unless the user asks.

## Output format examples

For 0 findings:

```
cntrdct scan complete: 0 findings.

Path scanned: <path>
```

For >0 findings: use the format from step 3.
