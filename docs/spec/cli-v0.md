# cntrdct CLI v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Scope

- Single subcommand: `cntrdct scan <PATH>`
- Hardcoded detector: clone-drift only
- Output: JSON pretty-print to stdout (SARIF arrives in next task)
- No config file, no flags beyond positional path

## Functional requirements

### F1 — Subcommand

`cntrdct scan <PATH>` where `PATH` is a file or directory.

### F2 — Library API

`cntrdct_cli::scan(path: &Path) -> Result<Vec<Finding>, ScanError>` is the testable
entry point. The binary is a thin wrapper that calls it and serializes the result.

### F3 — File collection

- If `PATH` is a file with extension `.rs`, process that single file.
- If `PATH` is a file without `.rs`, return empty Vec (skip silently).
- If `PATH` is a directory, walk recursively and collect all `*.rs` files.
- File ordering MUST be sorted lexicographically by path for determinism.

### F4 — File reading

Each `.rs` file is read as UTF-8. Files that fail to read (permission denied,
invalid UTF-8) are skipped without error in v0.

### F5 — Detector invocation

A `CloneDrift` is constructed and registered (P1 enforcement). All collected
ParsedFile entries are passed in a single `DetectContext`.

### F6 — Output format

Findings are always passed through the Layer 2 ranker before output. Output
shape depends on `--format`:

- `--format json` (default): pretty-print `Vec<RankedFinding>` (includes
  `rank_score`, plus `posterior_tp` / `wilson_lower` which are `null` until
  calibration ships).
- `--format sarif`: SARIF 2.1.0 emit. Order of `runs[0].results` is the rank
  order (descending `rank_score`). Ranker metadata is currently dropped from
  SARIF; will be re-added once a SARIF rank mapping is specified.

### F7 — Exit codes

- `0` on success regardless of finding count
- `1` on path-not-found, invalid arguments, or any `ScanError`

### F8 — Error reporting

Errors print to stderr in the format `error: <message>`. No panics on bad input.

## Non-functional requirements

### N1 — Determinism

Identical filesystem state produces identical JSON output, including ordering.

### N2 — No side effects beyond stdout/stderr

No network, no LLM, no writes to filesystem.

### N3 — P3 preserved

The CLI does not invoke an LLM. Adjudication crate (Layer 3) integration is out
of scope for v0.

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | tempdir with 4 base + 1 drifted .rs file | scan returns 1 Finding, primary ends with `e.rs` |
| T2 | empty tempdir | 0 Findings |
| T3 | single .rs file with one fn | 0 Findings (group size 1) |
| T4 | path that does not exist | `Err(ScanError::PathNotFound(_))` |
| T5 | T1 fixture run twice | identical JSON output |
| T6 | T1 fixture + non-rs files (readme.md, package.json) | 1 Finding (non-rs ignored) |

## Non-goals (v0)

- SARIF emission (next task)
- Multiple detectors, detector selection flag
- Configuration file
- Parallel file processing
- Progress UI / verbose mode
- LSP server mode

## Dependencies

- `clap` (subcommand parsing)
- `walkdir` (recursive traversal with deterministic sort)
- `tempfile` (test only)
