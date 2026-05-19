# cntrdct eval harness v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Background

The α phase shipped four detectors (clone-drift, arg-swap, comment-code,
unreachable-after-terminator) with no precision / recall measurement.
P1 ("evidence-based") is partially honoured by per-detector citations; the
β phase tightens it by adding empirical precision/recall numbers from a
labelled corpus that reviewers can replicate from the repo.

The original design log named Defects4J and BigCloneBench as the
benchmark targets. Both are Java-centric and out of scope for a Rust-only
α/β. This spec defines a corpus-format-agnostic harness that:

- ingests a directory of Rust source files paired with a JSONL manifest of
  expected findings, and
- emits per-detector and aggregate precision/recall/F1.

A small handcrafted seed corpus (`benchmarks/corpus/`) accompanies the
crate and serves as both a smoke-test and a worked example for future
contributions.

## Scope

In scope (v0):

- A new crate `crates/eval` exposing pure-function evaluation primitives
- A free function `cntrdct_eval::evaluate(manifest, findings) -> EvalReport`
- A loader for the JSONL manifest format
- A CLI subcommand `cntrdct eval <CORPUS_DIR>` that runs scan and prints
  the report as JSON
- A seed corpus under `benchmarks/corpus/` with at least one positive case
  per registered detector

Out of scope (v0):

- Multi-language corpora (Defects4J etc.)
- Statistical significance testing across runs
- Comparison harnesses against rustc / clippy
- HTML / markdown report rendering — JSON only
- Automated corpus mining (e.g. SZZ over a git history)
- Cross-revision regression tracking

## Corpus layout

```
<corpus_dir>/
├── manifest.jsonl
└── files/
    ├── unreachable_001.rs
    ├── arg_swap_001.rs
    └── ...
```

`manifest.jsonl` is one JSON object per non-blank line. Each object has the
shape:

```
{
  "file": "files/unreachable_001.rs",
  "expected": [
    { "detector_id": "unreachable-after-terminator", "line": 3 },
    ...
  ]
}
```

- `file` is the path to the source file relative to `corpus_dir`. The file
  must exist and be readable; missing files are a hard error
  (`EvalError::MissingSource`).
- `expected` is an array (possibly empty). An empty array means "this file
  is a true negative — no findings expected".
- `line` is the 1-based start line of the expected finding (matches
  `Finding.primary.start_line`). `line` is mandatory; column is ignored in
  v0 because hand-labelling columns is brittle.
- Lines starting with `//` (after whitespace stripping) and blank lines are
  skipped by the loader to allow inline comments in the manifest. JSON
  parse failures report the 1-based line number.

## Functional requirements

### F1 — `cntrdct_eval::Manifest`

```
pub struct ExpectedFinding {
    pub detector_id: String,
    pub line: u32,
}

pub struct ManifestEntry {
    pub file: PathBuf,           // relative to corpus_dir
    pub expected: Vec<ExpectedFinding>,
}

pub struct Manifest {
    pub entries: Vec<ManifestEntry>,
}
```

### F2 — `load_manifest`

```
pub fn load_manifest(path: &Path) -> Result<Manifest, EvalError>;
```

- Reads the JSONL file at `path`.
- Skips blank and `//`-prefixed lines.
- Returns `EvalError::Parse { line, source }` (1-based) on any JSON failure.
- Returns `EvalError::Io { path, source }` on I/O failure.

### F3 — `evaluate`

```
pub fn evaluate(
    manifest: &Manifest,
    actual: &[Finding],
    corpus_dir: &Path,
) -> EvalReport;
```

A predicted `Finding` matches an `ExpectedFinding` iff:

- `actual.detector_id == expected.detector_id` AND
- `actual.primary.start_line == expected.line` AND
- the actual finding's file path, made relative to `corpus_dir`, equals
  the manifest entry's `file`.

Each expected finding can satisfy at most one actual finding and vice
versa (1:1 matching). Counts:

- TP: matched pairs
- FP: actual findings with no matching expected
- FN: expected findings with no matching actual

### F4 — `EvalReport`

```
pub struct DetectorMetrics {
    pub tp: u32,
    pub fp: u32,
    pub fn_: u32,
    pub precision: f64,   // 0.0 when tp+fp == 0
    pub recall: f64,      // 0.0 when tp+fn == 0
    pub f1: f64,          // 0.0 when precision+recall == 0
}

pub struct EvalReport {
    pub per_detector: BTreeMap<String, DetectorMetrics>,
    pub overall: DetectorMetrics,
    pub corpus_size: u32,        // number of source files in manifest
    pub expected_total: u32,
    pub actual_total: u32,
}
```

Field ordering is stable: `per_detector` is a BTreeMap so JSON keys come
out in lexicographic order.

Precision / recall / F1 use the standard definitions:

- `precision = tp / (tp + fp)`
- `recall = tp / (tp + fn)`
- `f1 = 2 * precision * recall / (precision + recall)`

Conventions for divide-by-zero:

| condition | precision | recall | f1 |
|---|---|---|---|
| tp + fp == 0 | 0.0 | (uses recall rule) | 0.0 |
| tp + fn == 0 | (uses precision rule) | 0.0 | 0.0 |
| precision + recall == 0 | (per above) | (per above) | 0.0 |

Returning `0.0` (not `NaN`) keeps JSON serialisation total and downstream
sorts well-defined.

### F5 — Determinism

`evaluate` is a pure function of its inputs. `load_manifest` reads the
file in line order and never inspects timestamps or environment. Identical
inputs produce identical reports.

### F6 — `EvalError`

```
pub enum EvalError {
    Io        { path: PathBuf, source: io::Error },
    Parse     { line: u32, source: serde_json::Error },
    MissingSource(PathBuf),
}
```

`MissingSource` is raised by the CLI when a manifest entry references a
file that does not exist under `corpus_dir`. The pure `evaluate` function
itself does not touch the filesystem.

### F7 — CLI subcommand

```
cntrdct eval <CORPUS_DIR> [--manifest <PATH>]
```

- `<CORPUS_DIR>` is the corpus root.
- `--manifest <PATH>` overrides the default `<CORPUS_DIR>/manifest.jsonl`.
- The CLI:
  1. Loads the manifest.
  2. Verifies every `file` exists under `corpus_dir` (else `MissingSource`).
  3. Calls `cntrdct::scan(corpus_dir)`.
  4. Calls `evaluate(...)`.
  5. Pretty-prints the `EvalReport` as JSON to stdout.
- Exit code: `0` on a clean run regardless of metric values; `1` on any
  `EvalError` or `ScanError`.

Layer 2 ranking is intentionally NOT applied — eval cares about raw
detector output, not triage order. Rankers are a separate evaluation
concern (γ phase).

### F8 — JSON output stability

The CLI prints `serde_json::to_string_pretty(&report)`. The report's field
order follows struct declaration order; `per_detector` is sorted by the
BTreeMap. Two runs over the same corpus produce byte-identical output.

## Non-functional requirements

- N1. P3 preserved: the eval harness performs no LLM calls. It runs
  detectors only.
- N2. The eval crate has no detector dependencies of its own. It only
  depends on `cntrdct-core` for `Finding`/`Location` types.
- N3. The CLI integration that wires `scan + evaluate` lives in
  `cntrdct` (the CLI crate); the eval crate stays decoupled from `walkdir` /
  `tree-sitter` entirely.

## Test plan

| ID | Crate / file | Description |
|----|---|---|
| L1 | eval/tests/integration.rs | load_manifest parses valid JSONL |
| L2 | eval/tests/integration.rs | load_manifest skips blank and `//` lines |
| L3 | eval/tests/integration.rs | load_manifest reports 1-based line on parse failure |
| L4 | eval/tests/integration.rs | load_manifest on empty file returns Manifest with 0 entries |
| E1 | eval/tests/integration.rs | evaluate counts TP/FP/FN correctly on a hand-crafted case |
| E2 | eval/tests/integration.rs | evaluate breaks down by detector_id |
| E3 | eval/tests/integration.rs | evaluate honours 1:1 matching (no double-count) |
| E4 | eval/tests/integration.rs | evaluate matches by (file relative to corpus_dir, line, detector_id) |
| E5 | eval/tests/integration.rs | precision == 0.0 when tp+fp == 0 |
| E6 | eval/tests/integration.rs | recall == 0.0 when tp+fn == 0 |
| E7 | eval/tests/integration.rs | f1 == 0.0 when precision+recall == 0 |
| E8 | eval/tests/integration.rs | f1 with precision=recall=0.5 equals 0.5 |
| E9 | eval/tests/integration.rs | overall aggregates TP/FP/FN across detectors |
| E10 | eval/tests/integration.rs | identical inputs produce identical reports |
| K1 | cli/tests/eval.rs | `cntrdct eval` against the seed corpus exits 0 and prints JSON parseable as EvalReport |
| K2 | cli/tests/eval.rs | the seed corpus yields non-zero overall recall and non-zero overall precision |
| K3 | cli/tests/eval.rs | `cntrdct eval` on a missing manifest exits non-zero |

## Non-goals (v0)

- Bayesian credible intervals on F1
- Bootstrap resampling
- Per-revision drift detection
- Side-by-side comparison with other linters (clippy, rust-analyzer)
- Markdown / HTML report rendering
- Reading manifest entries lazily (corpus expected to fit in memory)

## References

- `kremenek-engler-sas-2003` — already cited; Z-Ranking implicitly assumes
  TP/FP labels of the kind this harness produces.
- `bettenburg-msr-2009` — already cited; framing of "what counts as a
  true positive" for clone evolution mirrors this harness's matching rule.
