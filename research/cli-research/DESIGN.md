# cntrdct-research design contract

This document specifies the design rules for `cntrdct-research`,
the research-side CLI that hosts the empirical-study tooling
(corpus fetch, aggregate, overlap, clippy harness, sample,
stratified-sample, rank, plus pending subcommands for the Rice
replication). Existing subcommands follow these rules; new
subcommands MUST follow them. The rules exist so the crate
remains testable, reproducible, and decoupled from the technical
workspace.

## 1. Scope and workspace boundary

`cntrdct-research` is a member of the research workspace
(`research/Cargo.toml`). Its dependencies are limited to:

- `cntrdct-corpus-fetch` (research-side, the only intra-workspace
  Rust crate it depends on),
- third-party crates (`serde`, `serde_json`, `thiserror`, `clap`,
  `rayon`, `tempfile`, `fastrand`).

It MUST NOT depend on `cntrdct-core`, on any detector crate, on
the ranker, the calibration crate, the SARIF emitter, or any
other technical-workspace member. The boundary is enforced by
discipline: the workspace split (CLAUDE.md root §1) forbids
cross-workspace path dependencies, and there is no structural CI
gate beyond the absence of such a path entry from
`research/cli-research/Cargo.toml`. If research code ever needs
a `cntrdct-core` type, the type is duplicated in
`cntrdct-research`, not pulled in via dependency.

The boundary's purpose is reversibility: a research subcommand
that becomes load-bearing for the product can be promoted under
`crates/*` without dragging research-only dependencies into the
technical build, and a research-side experiment that fails can
be deleted without touching the shipped CLI.

## 2. Subcommand addition pattern

Every subcommand has three layers, in this order:

1. A `run_<name>` function in `src/lib.rs` whose signature takes
   plain arguments (paths, integers, enums) and returns
   `Result<(), <Subject>Error>`. This is the public API and the
   single unit of testability. The function MUST be reachable
   without spawning a process.
2. A `<Subject>Error` enum in the same `src/lib.rs`, derived from
   `thiserror::Error`. Each variant carries enough context
   (paths, underlying-error sources) for an operator to diagnose
   the failure without re-running with `RUST_BACKTRACE=1`.
3. A `Commands::<Name>` arm in the `Commands` enum in
   `src/main.rs`, decorated with `clap::Subcommand`. The arm
   parses CLI flags, calls `run_<name>`, and maps the error to
   `ExitCode::FAILURE` with a printed `eprintln!` summary.

All three layers MUST land in the same commit. A subcommand that
exists in `lib.rs` but not in `main.rs` is a dead API; a `main.rs`
arm without a `lib.rs` `run_<name>` is untestable.

Naming:

- Function: `run_snake_case_name`. Multi-word names use
  underscore (e.g. `run_stratified_sample`, `run_clippy_harness`).
- Error type: `PascalCaseName + Error` (e.g. `OverlapError`,
  `AggregateError`, `FetchRunError`). Keep the suffix consistent
  to make `grep -E '^pub enum.*Error'` a complete inventory.
- `Commands` enum variant: PascalCase short form
  (e.g. `Overlap`, `StratifiedSample`).

## 3. JSON I/O conventions

Most subcommands consume or produce JSON. The convention is:

- Input format: prefer line-delimited JSON (JSONL) for streams of
  records, single JSON object for configuration. Existing inputs:
  `findings.json` (single document), labelled-findings
  (`benchmarks/labelled-findings.jsonl`, JSONL).
- Output format: choose to match downstream consumers, NOT to
  match input. The aggregator emits a single JSON object because
  the consumer is a Markdown render step; the sampler emits JSONL
  because the consumer is a labelling-CSV builder that processes
  rows.
- Encoding: `serde_json::to_writer_pretty` for diagnostic output
  (when a human will read it), `serde_json::to_writer` for
  machine-parsed output.
- Sort keys deterministically before emission. `serde_json` does
  not guarantee a stable key order across versions, but stability
  helps reviewers diff outputs across runs. When the output is a
  list, sort the list by a defined primary key (typically `id`,
  detector_id, or crate name) before serialising.
- Schema documentation: any new output schema gets a paragraph in
  the rustdoc of the producing function describing each field's
  semantics. The downstream consumer must be able to read the
  rustdoc and write a parser without running the producer.

Stdout vs file output:

- Subcommands that produce structured output MUST accept
  `--out <path>`. When `--out` is omitted, the convention varies
  by subcommand purpose:
  - Sampling / aggregation subcommands write to stdout (allows
    piping in shell loops).
  - Subcommands that produce a directory of files (e.g.
    `clippy-harness` writing per-crate JSON) MUST require
    `--out` (no implicit per-file stdout).

## 4. Determinism and reproducibility

Every subcommand MUST be deterministic given identical inputs
and arguments. This is a hard rule, not a guideline: research
results derived from a non-deterministic subcommand cannot be
audited.

Mechanisms:

- Sampling subcommands (`run_sample`, `run_stratified_sample`)
  take a `seed: u64` argument. The default is `0`; tests override.
  PRNG: `fastrand::Rng::with_seed(seed)`. Do NOT use thread-local
  randomness (`fastrand::shuffle`'s default uses a global RNG).
- Filesystem traversal MUST be canonicalised: walk in
  lexicographic order, not in `read_dir`'s native (filesystem-
  dependent) order. Use `WalkDir::sort_by_file_name()` or sort the
  collected entries before processing.
- Time-based fields (timestamps, durations) MUST NOT appear in
  any subcommand's output. If a producer needs to record "when
  this ran", that goes to stderr (a log channel), not to stdout
  (the data channel).
- Parallelism (rayon) MUST preserve output ordering: collect
  results into a `Vec`, then sort post-hoc. Subcommands that
  process per-crate work with `par_iter()` MUST sort the result
  vector by crate name before emission.

Tests for new subcommands MUST include a "deterministic with
same seed" case (cf. `stratified_sample.rs::deterministic_with_same_seed`).

## 5. Error handling

Each subcommand's `run_<name>` returns
`Result<(), <Subject>Error>`. Rules:

- Errors are domain-specific, not `anyhow`. Each error variant
  describes a specific failure mode the operator can act on.
  Adding `#[from] std::io::Error` is permitted but the variant
  name MUST distinguish I/O on different paths (e.g.
  `ReadList { path }` vs `Io { path }` in `FetchRunError`).
- Source chains use `#[source]` so `eprintln!("{err:#?}")`
  surfaces the underlying error without manual unwrapping.
- Errors from `cntrdct-corpus-fetch` are wrapped with
  `#[from] cntrdct_corpus_fetch::FetchError`. They do NOT cross
  the workspace boundary in either direction; the technical
  workspace's `cntrdct` CLI does not see them.
- The `main.rs` arm prints `eprintln!("{err}")` (single-line
  Display) and exits with `ExitCode::FAILURE`. It does NOT print
  the source chain by default; operators who want the chain run
  with `RUST_BACKTRACE=1`.

## 6. Test plan structure

Tests live under `tests/<name>.rs` (one file per subcommand,
mirroring the bin layout). The convention is:

- Each test file is self-contained for fixture purposes UNTIL
  three test files share fixture helpers, at which point the
  helpers move into `tests/common/mod.rs` consumed via
  `mod common;`. This is the same threshold documented in
  CLAUDE.md PITFALL-6 for `corpus-fetch` integration tests, and
  is enforced by discipline rather than by tooling. Two consumers
  is the local optimum; three triggers the refactor.
- Tests exercise `run_<name>` directly (not via subprocess) so
  failures land at the function boundary, not at process exit.
  If a test needs the binary surface (argv parsing, exit code),
  use `assert_cmd` and place it under
  `tests/<name>_cli.rs`; this convention is currently unused but
  reserved.
- Determinism tests are mandatory for sampling subcommands
  (cf. §4). Schema tests verify that the emitted JSON keys match
  the documented schema.
- Tests SHOULD use `tempfile::tempdir()` for any filesystem
  fixture; the temp dir's path string MUST NOT leak into
  assertions because macOS canonicalises `/var/folders/...` to
  `/private/var/folders/...` (CLAUDE.md PITFALL-2).

`cargo test --workspace` from the `research/` directory runs all
research-side tests including this crate's. Gating in CI is via
the `research-clippy-test` job (mirrors the technical job at
`working-directory: research`).

## 7. Relationship to the technical-side cntrdct CLI

The split is exposed at the binary level:

- `cntrdct` (technical, `crates/cli/`): exposes only `scan`,
  `calibrate`, `eval`. These are the shippable surface — the
  detector-driven scan, the prior calibration, and the
  eval-against-manifest precision/recall computation.
- `cargo-cntrdct` (technical): a shim so `cargo cntrdct ...` is
  accepted by `cargo`. Same code path as `cntrdct`.
- `cntrdct-research` (research, this crate): exposes `fetch`,
  `aggregate`, `overlap`, `clippy`, `sample`, `stratified-sample`,
  `rank`, plus pending subcommands.

The CLAUDE.md root §1 contract forbids reintroducing any
research subcommand on `cntrdct`. Migration from research to
technical is explicit: the technical-side reimplementation lives
under `crates/*` with a `promote(<area>): ...` commit prefix. A
research subcommand MUST NOT be moved by `git mv` into the
technical workspace; promotion is a deliberate rewrite that
respects the workspace's dependency rules and preregistration
discipline.

## 8. Pending subcommands

The following subcommands are specified but not yet implemented;
their addition follows §2 above. Each has a research-side spec
that fixes its contract.

- `rice-types`: spawns rust-analyzer over LSP, queries parameter
  types for arg-swap candidates emitted by the
  `RICE_TRACE` trace path, writes a side-car JSON joined by
  finding id. Spec:
  `research/projects/B_rice_replication/replication-spec-v1.1-rust-analyzer.md`.
  Blocked on USR-3 (Rice paper read + v1 promote).
- `rice-aggregate`: joins the trace JSONL output and the
  rust-analyzer types side-car; emits per-KLOC, per-bucket, and
  type-distinct-fraction tables (the latter with Wilson 95% CI).
  Spec:
  `research/projects/B_rice_replication/replication-spec-v0.md`
  §9 plus the v1.1 addendum §3.
  Blocked on TECH-3 (RICE_TRACE detector path) and on `rice-types`
  shipping first.

Both pending subcommands are the trigger for the `tests/common/mod.rs`
refactor in this crate (cf. §6): if either lands and shares a
fixture pattern with two existing test files, the third-file
threshold fires.

## 9. References

- `Cargo.toml` (this crate) — current dependency surface.
- `src/lib.rs` — existing `run_*` functions and error types.
- `src/main.rs` — `Commands` enum mapping flags to library calls.
- `tests/*.rs` — per-subcommand integration tests.
- `CLAUDE.md` (repo root) — workspace contract, PITFALLs.
- `research/projects/B_rice_replication/replication-spec-v1.1-rust-analyzer.md`
  — pending `rice-types` spec.
