# cntrdct SOTA baseline-comparator harness v0 spec

Status: active 2026-05-19.

Q-15 deliverable from `ROADMAP.md`. Publishes side-by-side
precision / recall / F1 against state-of-the-art external comparators
on the same corpora cntrdct already ships. Q-14 closed the
recall-side gap with an external denominator; Q-15 closes the
comparator-side gap with pinned external tools so cntrdct's numbers
are placed against an independently engineered baseline rather than
evaluated in isolation.

## Design rationale

Two earlier framings were considered and dropped:

1. **Live SOTA comparator runs gated by network access.** Rejected
   for the same reason Q-14 dropped continuous monitoring: external
   tools version-bump silently and the snapshot would not be
   stationary. The shipped design pins each baseline as a Docker
   image by tag *and* by content digest, so re-running the
   comparison from a clean environment a year later produces the
   same numbers.

2. **Embed the baselines' output into cntrdct's JSON / SARIF
   directly.** Rejected because that would make cntrdct
   responsible for shipping the baselines' findings to end-users,
   which is a different product than "evidence-based linter with a
   comparison table on the README". The harness produces a separate
   comparison artefact and the README references it; the runtime
   `cntrdct scan` output is untouched.

The shipped design — `cntrdct eval --baseline <name>[,<name>...]`,
Docker-image-pinned comparators with content digests committed to
the repo, a JSONL adapter contract that normalises each baseline's
output into the same shape `cntrdct::eval` already consumes — is
the smallest implementation that lets the comparison numbers be
reproduced from the repo without either rerunning external tools
or coupling cntrdct's runtime to external dependencies.

## Scope

In scope (v0):

- A new module `src/baselines.rs` with the per-baseline adapter
  registry, the Docker invocation helpers, and the normalised
  finding type the existing `evaluate` function consumes.
- A new CLI flag `cntrdct eval --baseline <names>` that runs
  cntrdct's own scan plus the named baselines, then prints a
  comparison report (separate from the existing `EvalReport`).
- v0 pilot baselines:
  - `sourcerercc` — SourcererCC clone detector (Sajnani et al.
    ICSE 2016), comparator for `clone-drift`. Wrapper Dockerfile
    under `baselines/sourcerercc/Dockerfile`; upstream commit
    pinned in the same directory's `UPSTREAM.md`.
  - `pybuglab` — PyBugLab self-supervised bug detector
    (Allamanis et al. NeurIPS 2021), comparator for `arg-swap`.
    Lands once the wrapper image is pinned.
- A `baselines/` top-level directory carrying the wrapper
  Dockerfiles, their pinned upstream metadata, and the normalising
  shell entrypoint each image emits.
- `benchmarks/baselines/` directory carrying the per-tag JSONL
  artefacts the comparison run produced (one subdirectory per
  release tag), so the comparison is reproducible without
  re-running the Docker images.
- A CITATIONS.md entry for `sajnani-icse-2016` under Layer 2
  (evaluation methodology; the SourcererCC paper grounds the
  comparator role, not a detector role).
- Deterministic test fixtures under `tests/fixtures/baselines/`
  carrying canned JSONL outputs from each baseline shape, plus
  `tests/baselines.rs` exercising the adapter contract and the
  comparison-report shape against those canned outputs.

Out of scope (v0):

- Live Docker invocation in CI. The pinned images are large
  (PyBugLab's pre-trained weights are ~1 GB) and pulling them on
  every push would multiply CI cost. v0 commits the per-tag JSONL
  under `benchmarks/baselines/<tag>/`; CI exercises the adapter and
  report shape against canned fixtures only. The Docker images run
  manually on the maintainer's workstation as part of the release
  pipeline.
- Comparison runs against more than the v0 baselines. Future
  comparators (CodeQL, Semgrep run as baselines rather than as
  Q-14 sources, GraphCodeBERT, etc.) land as additional registry
  entries; the scope is bounded by maintainer effort, not by
  policy.
- Comparison runs against cntrdct's own historical versions. The
  comparison is between cntrdct-current and the pinned baselines,
  not between cntrdct-current and cntrdct-past.
- Statistical significance testing across runs. The published
  cells are descriptive figures with Wilson / Jeffreys lower
  bounds (Q-11 §F4 shape); no hypothesis testing across cells.
- HTML / Markdown rendering of the comparison report beyond the
  README's "Baseline comparison" section. JSON is the
  authoritative format; the README text is a hand-edited summary.

## Baseline registry

Each registered baseline carries the following metadata:

```rust
pub struct BaselineSpec {
    /// CLI name, matched by `--baseline <name>`.
    pub name: &'static str,
    /// Human-readable description used in the comparison report header.
    pub description: &'static str,
    /// Detector this baseline maps to. v0 baselines map 1:1 to a
    /// single cntrdct detector; future baselines may map to more.
    pub detector_id: &'static str,
    /// Languages supported by this baseline. A baseline that does
    /// not support a corpus's language reports 0 findings and is
    /// surfaced as `unsupported_language` in the comparison report
    /// rather than silently dropped.
    pub supported_languages: &'static [Language],
    /// Pinned Docker image, fully qualified (registry/repo:tag).
    pub image_ref: &'static str,
    /// Pinned content digest. Verified by `docker inspect` before
    /// the run.
    pub image_digest: &'static str,
    /// CITATIONS.md key documenting the baseline's source.
    pub citation_key: &'static str,
}
```

v0 registry:

| name | detector_id | languages | image_ref (placeholder) |
|---|---|---|---|
| `sourcerercc` | `clone-drift` | `Rust`, `Python` | `ghcr.io/ktrysmt/cntrdct-baselines/sourcerercc:v1.0` |

The PyBugLab registry entry (`pybuglab` → `arg-swap`, Python only)
lands once its wrapper image is built and pinned. The concrete
image tags and digests for any baseline are committed to
`baselines/<name>/UPSTREAM.md` at the time the wrapper image is
built. The spec does not freeze the digest itself — that lives in
the release artefact — but it freezes the format: tag plus
sha256 digest, both required, both verified before the comparison
run starts.

## Adapter contract

Each baseline's Docker entrypoint reads cntrdct's corpus directory
mounted at `/corpus` and writes a JSONL stream of normalised
findings to `/out/findings.jsonl`. The schema is the smallest shape
the existing `cntrdct::eval::evaluate` matching rule (eval-v0 §F3)
can consume:

```jsonc
// One JSON object per line.
{
  "tool": "sourcerercc",
  "tool_version": "1.0-cntrdct-v0.4.0",
  "file": "files/clone_drift_005.rs",
  "line": 42,
  "detector_id": "clone-drift",
  "raw": { ... }  // verbatim from the upstream tool, opaque to the harness
}
```

- `tool` MUST equal the registry entry's `name`.
- `tool_version` is the upstream tool version with a `-cntrdct-vX.Y.Z`
  suffix identifying the cntrdct release the adapter shipped with.
  Used for the comparison-report header.
- `file` is relative to the corpus root, matching the audit-corpus /
  wild-corpus manifest convention (eval-v0 §F1, recall-audit-v0 §F3).
- `line` is 1-based, matching the eval-v0 §F3 matching rule.
- `detector_id` MUST equal the registry entry's `detector_id`. A row
  whose `detector_id` does not match is an adapter bug and the run
  aborts.
- `raw` carries the upstream tool's original output for that finding.
  It is not interpreted by the harness; it is committed alongside
  the normalised JSONL so a future reviewer can re-derive the
  normalised row if a schema change is needed.

The image entrypoint is responsible for:

1. Walking `/corpus` for files matching the baseline's
   `supported_languages` extensions.
2. Running the upstream tool against each file (or in batch, if the
   upstream supports it).
3. Filtering / mapping the upstream tool's output into the schema
   above. The mapping rules live in
   `baselines/<name>/entrypoint.sh` and are deliberately simple:
   one Python or shell file per baseline, no embedded
   transformations.
4. Writing the JSONL to `/out/findings.jsonl` atomically (write to
   a tempfile, then `mv`) so a partial run does not produce a
   partial artefact.
5. Exiting 0 on success, non-zero on any failure. The harness
   captures stderr verbatim and surfaces it on non-zero exit.

The host side runs the image with `--network=none` so the comparator
run is gated by the same P3 discipline as cntrdct's own scan path.
The image is required to have all of its dependencies (weights,
indices, etc.) baked in at build time.

## CLI flag

```sh
cntrdct eval <CORPUS_DIR> [--manifest <PATH>]
                          [--baseline <name>[,<name>...]]
                          [--baselines-out <PATH>]
                          [--baselines-skip-run]
```

- `<CORPUS_DIR>` and `--manifest` are unchanged from eval-v0 §F7.
- `--baseline <names>` is a comma-separated list of registered
  baseline names. When omitted, the run produces the existing
  `EvalReport` (eval-v0 §F4) unchanged. When set, the run produces
  the new `BaselineComparisonReport` (§F5 below) in addition to
  the existing report.
- `--baselines-out <PATH>` writes the comparison-report JSON to
  disk. When omitted, the JSON is printed to stdout after the
  existing `EvalReport`. The two reports are separable so a
  consumer that wants only the existing shape is unaffected.
- `--baselines-skip-run` reads cached per-baseline JSONL from
  `benchmarks/baselines/<release-tag>/<name>.jsonl` instead of
  invoking Docker. This is the path the maintainer uses to refresh
  the README table without re-running the images, and the path CI
  uses to exercise the adapter against canned fixtures.

The CLI invokes (per baseline) in order:

1. Verify the Docker image is present locally and its digest
   matches `BaselineSpec::image_digest`. On mismatch, the run
   aborts with `BaselineError::DigestMismatch`. This is what
   guarantees the published cells are reproducible from the spec.
2. Run the image with `--network=none --rm --read-only
   --mount type=bind,src=<corpus>,dst=/corpus,readonly
   --mount type=bind,src=<scratch>,dst=/out`.
3. Read `/out/findings.jsonl` and validate it against the adapter
   contract (§"Adapter contract" above). Schema violations abort.
4. Pass the parsed rows through the existing `cntrdct::eval::evaluate`
   matching rule (eval-v0 §F3), producing per-tool TP/FP/FN.

Step 1 is skipped under `--baselines-skip-run`; step 2 is replaced
by a read from the cached JSONL.

Exit code: `0` on a clean run regardless of comparison outcome; `1`
on any I/O / parse / scan / Docker failure or digest mismatch.

## Functional requirements

### F1 — `BaselineSpec` and registry

Documented in §"Baseline registry" above. The registry is a static
`&[BaselineSpec]` in `src/baselines.rs`. Adding or removing
entries is a deliberate code change; the registry is not loaded
from disk.

### F2 — `NormalisedFinding`

```rust
pub struct NormalisedFinding {
    pub tool: String,
    pub tool_version: String,
    pub file: PathBuf,
    pub line: u32,
    pub detector_id: String,
    pub raw: serde_json::Value,
}
```

Mirrors the JSONL schema. `raw` is held opaquely; nothing in the
harness inspects it.

### F3 — `load_baseline_jsonl`

```rust
pub fn load_baseline_jsonl(
    path: &Path,
    expected_tool: &str,
    expected_detector_id: &str,
) -> Result<Vec<NormalisedFinding>, BaselineError>;
```

JSONL parse, blank / `//` lines skipped, 1-based line numbers on
parse failure (same shape as eval-v0 §F2). A row whose `tool`
does not match `expected_tool` is `BaselineError::ToolMismatch`; a
row whose `detector_id` does not match `expected_detector_id` is
`BaselineError::DetectorIdMismatch`. Both are adapter bugs and
the comparison run aborts.

### F4 — `run_baseline_docker`

```rust
pub fn run_baseline_docker(
    spec: &BaselineSpec,
    corpus_dir: &Path,
    out_jsonl_path: &Path,
) -> Result<(), BaselineError>;
```

- Verifies the image digest matches `spec.image_digest` via
  `docker inspect --format '{{index .RepoDigests 0}}'`.
- Spawns `docker run --network=none --rm --read-only ...` with the
  bind mounts from §"CLI flag".
- On exit code 0, returns Ok. On non-zero, returns
  `BaselineError::ExitCode { code, stderr }`.

The host-side dependency on the `docker` binary is documented in
the spec but not enforced at build time; the CLI surfaces a
helpful error if `docker` is not on PATH.

### F5 — `BaselineComparisonReport`

```rust
pub struct ToolMetrics {
    pub tool: String,
    pub tool_version: String,
    pub tp: u32,
    pub fp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub wilson_lower_precision: f64,
    pub wilson_lower_recall: f64,
    pub interval_method: String, // "wilson" or "jeffreys" per Q-11
}

pub struct DetectorComparison {
    pub detector_id: String,
    pub corpus_name: String,
    pub cntrdct: ToolMetrics,
    pub baseline: ToolMetrics,
    pub concordance_tp_tp: u32,
    pub concordance_fp_fp: u32,
    pub expected_total: u32,
    pub corpus_size: u32,
}

pub struct BaselineComparisonReport {
    pub release_tag: String,
    pub priors_default_sha256: String,
    pub comparisons: Vec<DetectorComparison>,
}
```

- `release_tag` is the cntrdct version the comparison was run for
  (read from `CARGO_PKG_VERSION`).
- `priors_default_sha256` is the SHA-256 of the embedded
  `benchmarks/priors-default.json`. Surfaces alongside the
  comparison so a release-time consumer can verify the shipped
  priors against a known hash.
- `comparisons` is sorted by `(detector_id, corpus_name)` so the
  serialised JSON is byte-stable across runs over the same
  inputs.
- `interval_method` is per-cell per Q-11: `"wilson"` for cells
  with `tp + fp + fn >= 30`, `"jeffreys"` otherwise. The
  comparison-table footnote names this so a reader can tell at a
  glance whether each cell is Wilson- or Jeffreys-bounded.

### F6 — Determinism

`run_baseline_docker` is deterministic up to the Docker image's
own determinism contract:

- SourcererCC's tokeniser and clone-pair output are deterministic
  given the same input bytes; no seed required.
- PyBugLab is a learned model; the upstream provides a
  `--seed <int>` flag pinned in the wrapper Dockerfile. The
  resulting JSONL is recorded under `benchmarks/baselines/<tag>/`;
  re-running the image with the same seed against the same corpus
  produces byte-identical JSONL.

The harness side (`load_baseline_jsonl`, `evaluate`, report
assembly) is pure. The `BaselineComparisonReport` serialises
through BTreeMaps and sorted Vecs; identical JSONL inputs produce
byte-identical JSON output.

### F7 — `BaselineError`

```rust
pub enum BaselineError {
    Io { path: PathBuf, source: io::Error },
    Parse { line: u32, source: serde_json::Error },
    ToolMismatch { expected: String, got: String, line: u32 },
    DetectorIdMismatch { expected: String, got: String, line: u32 },
    DigestMismatch { expected: String, got: String, image: String },
    ExitCode { code: i32, stderr: String },
    DockerNotFound,
}
```

Each variant carries enough context to diagnose the failure
without re-running. `DigestMismatch` includes both digests so the
user can see at a glance whether the image was rebuilt or the
pin needs updating.

### F8 — Comparison-table format (README)

The README's "Baseline comparison" section is hand-edited from
the JSON report. The format is a Markdown table grouped by
`(detector_id, corpus_name)`:

```markdown
### `clone-drift` on `benchmarks/audit-corpus/`

| tool | TP | FP | FN | P | R | F1 | P-lower (95%) | R-lower (95%) | interval |
|---|---|---|---|---|---|---|---|---|---|
| cntrdct v0.4.0 | ... | ... | ... | ... | ... | ... | ... | ... | jeffreys |
| sourcerercc 1.0 | ... | ... | ... | ... | ... | ... | ... | ... | jeffreys |

Concordance TP-TP: ... · Concordance FP-FP: ...
```

The README table is regenerated by hand at release time; the JSON
report is the authoritative source for review.

## Non-functional requirements

- N1. P3 preserved. The harness opens no sockets on the cntrdct
  side. The Docker images run with `--network=none`. The
  `network-isolation` CI job already gates cntrdct; the spec
  documents the per-baseline network posture symmetrically.
- N2. Layer separation. `src/baselines.rs` depends on
  `core::Finding` (via `crate::eval`) and on `std::process` for
  Docker invocation. It does not depend on `crate::adjudicator`
  or `crate::llm_calibration`.

## Test plan

| ID | File | Description |
|----|------|-------------|
| L1 | `tests/baselines.rs` | `load_baseline_jsonl` parses a valid synthetic JSONL for `sourcerercc` |
| L2 | `tests/baselines.rs` | `load_baseline_jsonl` parses a valid synthetic JSONL for `pybuglab` (exercised via unit test once the registry entry lands) |
| L3 | `tests/baselines.rs` | `load_baseline_jsonl` returns `ToolMismatch` on a row whose `tool` does not match |
| L4 | `tests/baselines.rs` | `load_baseline_jsonl` returns `DetectorIdMismatch` on a row whose `detector_id` does not match |
| L5 | `tests/baselines.rs` | `load_baseline_jsonl` reports 1-based line on parse failure |
| L6 | `tests/baselines.rs` | `load_baseline_jsonl` skips blank and `//` lines |
| C1 | `tests/baselines.rs` | `compare_one` against a fixture produces the expected `DetectorComparison` (TP/FP/FN counts, precision/recall/F1, concordance cells) |
| C2 | `tests/baselines.rs` | `compare_one` produces `interval_method = "jeffreys"` when `tp + fp + fn < 30` and `"wilson"` otherwise |
| C3 | `tests/baselines.rs` | `BaselineComparisonReport` serialises byte-identically across two runs over the same inputs |
| C4 | `tests/baselines.rs` | `BaselineComparisonReport.priors_default_sha256` equals the SHA-256 of the embedded `benchmarks/priors-default.json` |
| R1 | `tests/baselines.rs` | CLI path `cntrdct eval --baseline sourcerercc --baselines-skip-run` against a fixture exits 0 and prints both `EvalReport` and `BaselineComparisonReport` JSON |
| R2 | `tests/baselines.rs` | `--baseline <unknown>` exits non-zero with a registry-miss error |
| R3 | `tests/baselines.rs` | `--baselines-skip-run` without `--baseline` is a no-op (the existing eval path runs unchanged) |

Live Docker runs are exercised manually on the maintainer's
workstation; the CI gate is the canned-fixture suite above.
Adding a live-Docker CI job is a future-work item and requires a
registry-cost decision.

## References

- `docs/spec/eval-v0.md` — operational definition of the matching
  rule (§F3) and the divide-by-zero conventions (§F4) reused
  unchanged for the baseline-comparator cells.
- `docs/spec/recall-audit-v0.md` — Q-14 spec; defines the
  audit-corpus used as the recall denominator.
- `docs/spec/ranker-v1.md` — Q-11 small-N switching; defines the
  Wilson / Jeffreys threshold per cell.
- `sajnani-icse-2016` — H. Sajnani, V. Saini, J. Svajlenko, C.K. Roy,
  C.V. Lopes, "SourcererCC: Scaling Code Clone Detection to
  Big-Code", ICSE 2016. Comparator for `clone-drift`.
- `allamanis-neurips-2021` — already cited under Layer 1 for
  `arg-swap`. Comparator for `arg-swap` in the planned PyBugLab
  cells; documented here rather than re-listed under Layer 1.
