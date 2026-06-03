# cntrdct recall-audit harness v0 spec

Status: active draft, approved for TDD implementation 2026-05-11.

Q-14 deliverable (originally tracked in the retired `ROADMAP.md`; the
forward plan now lives in `REBUILD.md`). Counters the labeller-bias loop in
which cntrdct's priors are fit on corpora cntrdct labelled itself
(P-1 → P-4 → embedded `priors-default.json`). When the only labels
come from triaging cntrdct's own findings, the resulting recall
estimate is undefined (every expected entry is a finding cntrdct
already produced). The audit-recall harness instead measures recall
against externally-sourced ground truth — CVEs, OSV.dev advisories,
and findings from independent SAST tools — yielding a recall upper
bound that the priors loop cannot self-confirm.

## Design rationale

Three earlier framings were considered and dropped:

1. Pull `expected: []` entries directly into the existing wild-corpus
   manifests. Rejected because wild-corpus is selected by
   top-by-downloads with no reference to the bug-tracker history;
   merging external bug locations into it would make the corpus
   simultaneously an FP-discovery harness (current role, P-1) and a
   recall harness (Q-14 role), with no way to keep the two metrics
   from drifting against each other.

2. Emit recall numbers from `cntrdct eval` itself with a `--audit`
   switch. Rejected because the eval CLI's positional arg is a
   corpus dir under the existing manifest schema. Adding an audit
   path would require eval to interpret manifest entries
   differently based on the switch, and would couple eval's
   precision/recall/F1 reporting to a metric (recall upper bound)
   that has different semantics — the audit denominator is "external
   labellers caught it", not "cntrdct's own triage caught it".

3. Compute recall by comparing cntrdct's findings to a snapshot of
   another SAST tool's findings on the same corpus. Rejected on the
   same grounds Q-13 dropped continuous monitoring: the comparator
   tools are version-bumped silently and the snapshot would not be
   stationary. The shipped design uses the comparator's *published
   testset findings* (rule IDs with stable URLs), not live
   comparator runs.

The shipped design — `cntrdct calibrate --audit-recall <CORPUS>`,
manifest entries that cite each expected finding's external source by
stable URL — is the smallest implementation that lets the recall
upper bound be reproduced from the repo without either rerunning
external tools or muddling the wild-corpus precision metric.

## Scope

In scope (v0):

- A new module `src/recall_audit.rs` with the audit-corpus
  manifest loader, the `audit_recall` pure function, and the
  `RecallAuditReport` shape.
- CLI: `cntrdct calibrate --audit-recall <CORPUS_DIR>
  [--manifest <PATH>] [--output <PATH>]`. The existing `corpus`
  positional arg is interpreted as a directory in this mode rather
  than a JSONL file; both `--fit-platt` and `--audit-recall` are
  treated as alternative modes of the `calibrate` subcommand and
  are mutually exclusive at runtime (clap's argument-conflict
  surface).
- `benchmarks/audit-corpus/` directory with a README documenting
  the source list and selection rules. v0 ships the README as a
  skeleton — the actual labelled entries land in Phase B (Q-14
  follow-up data collection).
- A CITATIONS.md entry for `heckman-williams-ist-2011` under
  Layer 2 (selection-bias warning is a property of the
  precision-fitting loop, not of any one detector).
- A deterministic synthetic-fixture test under
  `tests/recall_audit.rs` so PR CI exercises the loader / matcher /
  report shape on every push, independent of Phase B data
  availability.

Out of scope (v0):

- The audit-corpus *content* — actual labelled CVE / Semgrep /
  CodeQL / Clippy entries. Phase B work, tracked separately.
- Quarterly cron job that recomputes recall and updates the README.
  Q-13's design rationale point 3 applies: external sources update
  silently, and a time-series of recall figures captures upstream
  catalogue drift more than any cntrdct property. Phase B's manual
  refresh on each release tag is sufficient.
- Comparison against live SAST tool runs. The harness consumes
  *labelled audit-corpus entries*; live comparator runs are Q-15
  territory.
- A "missed-finding" remediation pipeline. Audit-recall reports the
  gap; closing it (detector improvement / new detector) is a
  separate engineering effort.
- `cntrdct.toml` per-language enable/disable interaction with the
  audit. v0 expects the corpus and the running scan to share the
  same `cntrdct.toml`; cross-config audits are out of scope.

## Audit corpus layout

```
benchmarks/audit-corpus/
├── README.md
├── manifest.jsonl
└── files/
    ├── arg_swap_001.rs
    ├── clone_drift_python_001.py
    └── ...
```

Schema for `manifest.jsonl` extends the eval manifest with an
`external_source` field on every `expected` entry:

```jsonc
{
  "file": "files/arg_swap_001.rs",
  "expected": [
    {
      "detector_id": "arg-swap",
      "line": 42,
      "external_source": {
        "kind": "semgrep",
        "ref": "rust.lang.security.arg-swap.example",
        "url": "https://semgrep.dev/r/rust.lang.security.arg-swap.example"
      }
    }
  ],
  "source": "https://static.crates.io/...",
  "license": "MIT OR Apache-2.0",
  "sha256": "..."
}
```

- `external_source.kind` is a freeform string. Canonical values
  used by the v0 corpus: `"nvd"`, `"osv"`, `"semgrep"`,
  `"codeql"`, `"clippy"`, `"paper-appendix"`. Future sources are
  permitted without spec churn; the README's "Sources" section is
  the live registry.
- `external_source.ref` identifies the labelled finding within its
  source (CVE ID, Semgrep rule ID, CodeQL query path, Clippy lint
  name, etc.).
- `external_source.url` is a stable, deep-linkable URL. Required —
  the acceptance criterion is "the audit corpus README cites every
  CVE / external finding source with a stable URL", and the
  manifest is the authoritative store.
- The per-file `source` / `license` / `sha256` triple is the same
  shape as the wild-corpus manifests (eval-v0 §F1); audit-corpus
  reuses it so the existing wild-corpus tooling
  (`scripts/fetch_*_corpus.py` family) can be reused for the
  fetcher in Phase B.
- `//`-prefixed lines and blank lines are skipped by the loader.

## Functional requirements

### F1 — `recall_audit::ExternalSource`

```rust
pub struct ExternalSource {
    pub kind: String,
    #[serde(rename = "ref")]
    pub ref_: String,
    pub url: String,
}
```

`ref` is reserved in Rust so the field uses `ref_` with a
`#[serde(rename = "ref")]` to match the manifest JSON.

### F2 — `recall_audit::AuditExpectedFinding`

```rust
pub struct AuditExpectedFinding {
    pub detector_id: String,
    pub line: u32,
    pub external_source: ExternalSource,
}
```

`external_source` is required; v0 does not allow audit entries
without provenance. Loader fails on a missing field.

### F3 — `recall_audit::AuditManifestEntry`

```rust
pub struct AuditManifestEntry {
    pub file: PathBuf,
    pub expected: Vec<AuditExpectedFinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}
```

`source` / `license` / `sha256` are optional for symmetry with
eval-v0; the audit-corpus README documents that v0 fetcher output
should set all three.

### F4 — `recall_audit::load_audit_manifest`

```rust
pub fn load_audit_manifest(path: &Path) -> Result<AuditManifest, RecallAuditError>;
```

- JSONL parse, blank / `//` lines skipped.
- 1-based line numbers in parse errors.
- Entries returned in source order.

### F5 — `recall_audit::audit_recall`

```rust
pub fn audit_recall(
    manifest: &AuditManifest,
    actual: &[Finding],
    corpus_dir: &Path,
) -> RecallAuditReport;
```

Matching rule (mirrors eval-v0 §F3):

- predicted matches expected iff
  `actual.detector_id == expected.detector_id` AND
  `actual.primary.start_line == expected.line` AND
  `actual.primary.file` made relative to `corpus_dir` equals
  `entry.file`.
- 1-to-1 matching (each expected satisfies at most one actual).

Counts per detector:

- TP: matched pairs — cntrdct caught a finding the external source
  also flagged.
- FN: unmatched expected — cntrdct missed a finding the external
  source flagged.
- FP is intentionally not reported. Audit-corpus precision is
  meaningless here: the corpus is constructed *because* an external
  source flagged something, so a cntrdct finding outside that
  expected set is not necessarily wrong, just unreviewed against
  the audit denominator. Precision lives on `cntrdct eval`.

Per-detector recall upper bound:

```
recall_upper_bound = tp / (tp + fn)        (0.0 when tp + fn == 0)
```

The "upper bound" qualifier is mandatory in the spec text:
external sources have their own recall failures (they miss bugs
too), so the audit's denominator is itself a subset of the true
ground truth. cntrdct's measured recall is therefore a ceiling on
its real recall against the unobserved full ground truth.

### F6 — `recall_audit::RecallAuditReport`

```rust
pub struct DetectorRecall {
    pub tp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
    pub recall_upper_bound: f64,
    pub source_breakdown: BTreeMap<String, SourceTally>,
}

pub struct SourceTally {
    pub tp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
}

pub struct RecallAuditReport {
    pub per_detector: BTreeMap<String, DetectorRecall>,
    pub overall: DetectorRecall,
    pub corpus_size: u32,
    pub expected_total: u32,
    pub sources: BTreeMap<String, u32>,
}
```

- `per_detector` is a BTreeMap (lexicographic key order, byte-stable
  JSON).
- `source_breakdown` per detector aggregates by
  `external_source.kind`, surfacing whether a given detector's
  recall is dominated by one source (e.g. all clone-drift TPs come
  from CodeQL; no cross-source agreement) — Heckman & Williams's
  selection-bias warning applied at the audit level itself.
- `sources` (top level) maps each `kind` to the count of expected
  entries it contributed, so the README can cite the source mix
  without re-walking the manifest.

### F7 — CLI flag

```sh
cntrdct calibrate --audit-recall <CORPUS_DIR>
                  [--manifest <PATH>]
                  [--output <PATH>]
```

- `<CORPUS_DIR>` is the audit-corpus root (typically
  `benchmarks/audit-corpus`).
- `--manifest` overrides the default
  `<CORPUS_DIR>/manifest.jsonl`.
- `--output PATH` writes the report JSON to disk; default is
  stdout. Symmetric with `cntrdct cross-model-kappa`'s shape.
- `--fit-platt` and `--audit-recall` are mutually exclusive;
  passing both produces a clap conflict error.
- The CLI calls `cntrdct::run_recall_audit(corpus_dir, manifest_path)`
  which validates every manifest `file` exists under `corpus_dir`,
  then invokes `cntrdct::scan(corpus_dir)`, then
  `audit_recall(...)`.
- Exit code: `0` on a clean run regardless of recall values; `1`
  on any I/O / parse / scan failure.

### F8 — Determinism

`audit_recall` is a pure function of its inputs. `load_audit_manifest`
reads in line order. `RecallAuditReport` serialises through
BTreeMaps, so identical corpus + identical scan output produce
byte-identical JSON. The scan path itself is rayon-parallel but
its `Vec<Finding>` output is sorted before audit per the existing
`scan_full_with_config` contract (T2-10).

### F9 — `RecallAuditError`

```rust
pub enum RecallAuditError {
    Io { path: PathBuf, source: io::Error },
    Parse { line: u32, source: serde_json::Error },
    MissingSource(PathBuf),
}
```

`MissingSource` is raised by `run_recall_audit` (CLI side) when a
manifest entry references a file absent under the corpus dir.

## Non-functional requirements

- N1. P3 preserved. The audit harness performs no LLM calls and
  opens no sockets. It is gated by the `network-isolation` CI job
  alongside `scan` / `calibrate` / `eval`.
- N2. Layer separation. `recall_audit` depends on `core::Finding`
  and the file walker via `crate::scan`, not on
  `crate::adjudicator` or `crate::llm_calibration`.
- N3. Wild-corpus contract is unchanged. Audit-corpus does NOT
  feed `scripts/build_priors_corpus.py`; if it did, the priors
  loop would re-absorb external labels and the audit's purpose
  would be defeated. The audit is a read-only check on the priors
  loop's output, not an input to it.

## Test plan

| ID | File | Description |
|----|------|-------------|
| L1 | `tests/recall_audit.rs` | `load_audit_manifest` parses a valid synthetic JSONL with one detector and one external source |
| L2 | `tests/recall_audit.rs` | `load_audit_manifest` skips blank and `//` lines |
| L3 | `tests/recall_audit.rs` | `load_audit_manifest` reports 1-based line on parse failure |
| L4 | `tests/recall_audit.rs` | a manifest entry without `external_source` fails to load |
| A1 | `tests/recall_audit.rs` | `audit_recall` reports `tp = expected_total` when scan output covers every expected entry exactly |
| A2 | `tests/recall_audit.rs` | `audit_recall` reports `recall_upper_bound = 0.5` when scan covers half of expected entries |
| A3 | `tests/recall_audit.rs` | `source_breakdown` aggregates by `external_source.kind` |
| A4 | `tests/recall_audit.rs` | `RecallAuditReport` serialises byte-identically across two runs over the same corpus |
| C1 | `tests/recall_audit.rs` | CLI path `cntrdct calibrate --audit-recall <FIXTURE>` exits 0 and prints parseable JSON |
| C2 | `tests/recall_audit.rs` | passing both `--fit-platt` and `--audit-recall` exits non-zero (clap conflict) |

The Phase A fixture lives under
`tests/fixtures/recall-audit/` and contains 2 source files with 3
synthetic expected entries across 2 sources. It does NOT pretend to
be a real audit-corpus; the README under `benchmarks/audit-corpus/`
is the (Phase B) live data home.

## Phase B (Q-14 follow-up)

Out of scope for Phase A but referenced here so the spec is
forward-compatible:

- `scripts/fetch_audit_corpus.py` (stdlib-only, mirrors
  `scripts/fetch_rust_corpus.py` shape) pulls source files from
  the same upstream (crates.io / PyPI) as wild-corpus, but pinned
  by CVE / advisory version rather than top-by-downloads.
- Per-detector seed lists in `benchmarks/audit-corpus/README.md`:
  external sources known to label findings cntrdct's six detectors
  could plausibly catch. Initial candidates (subject to revision):
  - arg-swap: Allamanis NeurIPS 2021 PyPIBugs swapped-args partition;
    Semgrep `swapped-arguments` rule family.
  - clone-drift: Assi TOSEM 2025 inconsistent-clone evolution
    findings.
  - comment-code: Tan SOSP 2007 / PLDI 2011 published bug pairs.
  - config-interaction: Nadi ICSE 2014 contradictory cfg
    constraints in Linux KConfig.
  - pr-miner: Li-Zhou FSE 2005 published PR-Miner anomalies (where
    source remains accessible).
  - unreachable-after-terminator: Hovemeyer-Pugh OOPSLA 2004
    FindBugs UR pattern testset; rustc UCDR examples.
- README publishes the recall figures under a "Latest audit run"
  section, refreshed manually on each release tag.

## arg-swap / clone-drift FN triage (recorded 2026-06-03)

The two lowest per-detector recall figures —
`arg-swap = 0.25` (1tp/3fn) and `clone-drift = 0.5` (1tp/1fn) — were
triaged against the audit corpus to separate detector-logic limits
from labelling artefacts. Method: `audit_recall` (F5) matches on the
exact `(detector_id, file, start_line)` triple, so each expected
entry with no actual finding at that line is an FN; each FN file was
read and its root cause classified against the detector specs and
minimal repro experiments. Result: all four FNs are genuine anomalies
(verified by upstream fix commits / PyPIBugs labels / clippy lint
triggers) — zero labelling errors — and split into three structural,
documented bounds:

- arg-swap bound A (same-file resolution, F4): `unv_app_settings.py:41`,
  `nbrmd_test_ipynb_to_R.py:26` — callee imported from a module absent
  from the (single-file) corpus entry, so F4 finds no definition. See
  `docs/spec/arg-swap-v0.md` "Known recall upper bounds → Bound A".
- arg-swap bound B (name-correlation ceiling, F5):
  `totalsegmentator_statistics.py:10` — definition is same-file and
  resolves, but the argument identifiers share no lexical signal with
  the parameter names, so F5 (and the SwapD SOTA) emit nothing. Target
  of REBUILD.md R-4. See arg-swap-v0.md "Bound B".
- clone-drift bound C (granularity, F2/F2b):
  `clippy_ui_branches_sharing_code_shared_at_top.rs:15` — the clippy
  `branches_sharing_code` class (branches sharing a common prefix but
  diverging) is out of scope; F2b flags only fully byte-identical
  consequence/alternative blocks. See
  `docs/spec/clone-drift-v0.md` "Known recall bound — branches_sharing_code".

The triage also surfaced an arg-swap call-enumeration regression from
the R-1.c'' IR migration (calls nested in `IrExpr::Other` shapes were
unvisited) — invisible to the T1 gate because the one such audit call
is itself bound B. Fixed 2026-06-03 (arg-swap-v0.md §F3); it does not
move this corpus's recall figure but recovers real-world recall. These
bounds are recall *upper bounds*, not regressions: a figure at or above
the floor with T1 green is expected, not a defect.

## References

- `heckman-williams-ist-2011` — S. Heckman, L. Williams, "A
  systematic literature review of actionable alert identification
  techniques for automated static code analysis", Information and
  Software Technology 53(4), 363-387, 2011. Selection-bias warning
  for actionable-alert pipelines that calibrate priors against
  their own filtered output; methodological grounding for Q-14's
  external-denominator design.
- `kremenek-engler-sas-2003` — already cited under Layer 2; Z-Ranking
  assumes the TP/FP labels feed into the prior, but does not address
  the case where the labels themselves come from the ranked output.
  Q-14 is the explicit external check on that loop.
- `bettenburg-msr-2009` — already cited; "what counts as a true
  positive" framing applies symmetrically to recall denominators.
