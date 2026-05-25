# cntrdct rebuild plan

Last updated: 2026-05-25. Supersedes the retired `ROADMAP.md` and
the retired `REBUILD-handoff.md` (the latter's R-1 procedure and
verification rules are absorbed into this file below).

Current shipped version: v0.5.2 (Rust + Python, six detectors). This
file replaces the old Q-series / T-series / M-series roadmap with a
single forward-looking plan oriented around the v0.6.0 rebuild:
a language-agnostic IR layer plus a community-friendly contribution
path for new languages.

For historical detector / corpus / quality-audit milestones (P-1
through Q-16, T1-1 through T2-11, M-1 through M-6, T3-14 through
T3-16, T4-17 through T4-19), see `CHANGELOG.md` and the per-detector
specs under `docs/spec/`. They are not duplicated here.

## Status legend

- `[x]` completed
- `[~]` in progress
- `[ ]` pending
- `(carry-over)` — inherited from the retired `ROADMAP.md`,
  re-prioritised under the rebuild plan

## 0. Why rebuild

The retired `ROADMAP.md` accumulated three structural premises that
have since broken:

1. Q-15 "SOTA baseline comparators" assumed pinned Docker images of
   PyBugLab and SourcererCC could be wired against the corpus on
   release. The 2026-05-22 evidence audit (`gh api`, HuggingFace,
   DockerHub, fork survey) showed PyBugLab pre-trained weights are
   not distributed in any form, the upstream README explicitly says
   the infrastructure cannot be shared, and no academic alternative
   ships an installable arg-swap detector. The "external SOTA
   comparator" framing was structurally unrealisable.
2. The M-series serial language-extension plan scaled by editing
   every detector's `match Language::*` arms for each new language.
   The marginal cost per language was ~1500-1900 LOC of detector
   source plus per-(detector, language) literature surveys, which
   does not invite outside contribution at the rate the
   peer-reviewed-citation differentiator deserves.
3. The closed `Language` enum + compile-time tree-sitter linkage
   model assumed cntrdct was the only place new languages could
   land. There was no concept of a contribution path for a language
   the maintainers had not personally surveyed.

The rebuild keeps every shipping behaviour (P1-P5 constraints, the
four-layer architecture, the SARIF / eval / calibrate / scan /
cross-model-kappa CLI surface, the `network-isolation` CI gate) but
replaces (1)-(3) with a single architectural pivot: language-agnostic
IR + community-PR-driven plugin source under one repo.

## 1. Architecture goals

Goal G1. Language-agnostic IR

A new module `src/ir.rs` defines a normalised AST shape
(`IrCallSite`, `IrBlock`, `IrComment`, `IrExpr`, `IrPath`, etc.) that
every cross-cutting detector consumes. Per-language code lives only
in `src/parsers/<lang>.rs` which converts the language's tree-sitter
AST into IR. After landing, the marginal cost of adding a new
language drops from ~1500-1900 LOC across five files (one per
cross-cutting detector) to ~300-500 LOC in one file (the parser
converter).

Goal G2. Two-tier detector layout

```
src/detectors/
├── arg_swap.rs                  # cross-cutting, IR-based
├── clone_drift.rs
├── comment_code.rs
├── pr_miner/
├── unreachable_after_terminator.rs
└── lang/                        # language-specific
    ├── rust_config_interaction.rs
    ├── python_unreachable_except.rs
    └── ...
```

Cross-cutting detectors are written once on IR. Language-specific
detectors (Rust `#[cfg]` interaction, Python `except` reachability,
future Go build-tag interaction, etc.) live under `src/detectors/lang/`
and continue to read tree-sitter ASTs directly. The two-tier split
keeps the IR small (no concession to single-language AST quirks) and
keeps the language-specific detectors honest.

Goal G3. Single-repo contribution model

cntrdct stays a single crates.io artefact and a single GitHub repo.
There is no separate `cntrdct-plugins` repo, no out-of-process plugin
discovery, no WASM modules, no `.so` loading. Adding a new language
is a PR against this repo that adds:

- `src/parsers/<lang>.rs` (the IR converter)
- `Cargo.toml` dependency on `tree-sitter-<lang>`
- `benchmarks/wild-corpus-<lang>/` fixture set (≥ 8 positive / 3
  negative per cross-cutting detector that opts in)
- `CITATIONS.md` entries with `Languages:` lines (P1 + per-language
  SHOULD per `docs/spec/citations-policy.md`)
- `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` per detector
  surveyed
- Optionally, `src/detectors/lang/<lang>_<id>.rs` for any
  language-specific detector the contributor wants to ship alongside

`CONTRIBUTING.md` documents this workflow as the canonical path.

Goal G4. P1-P5 preserved

The rebuild touches none of the design constraints:

- P1 (peer-reviewed citation per detector) — IR-based detectors keep
  their existing citation sets; new languages add `Languages:` lines
  and per-language surveys per existing policy.
- P3 (LLM only on Layer 3) — the IR layer is in Layer 1; no new
  socket-opening code paths. The `network-isolation` CI gate
  continues to cover `scan` / `calibrate` / `eval`.
- P4 (priors from labelled corpora) — `cntrdct calibrate` continues
  to feed `benchmarks/priors-default.json`. Per-language priors are
  re-fitted after detectors migrate to IR (a Layer 2 reconciliation
  task inside R-1).
- P5 (severities map to IEEE 1044-2009) — SARIF emission is
  unchanged. IR carries no severity; severity stays detector-defined.

## 2. Design constraints (carried from CLAUDE.md, unchanged)

- P1 / P3 / P4 / P5: as written in `CLAUDE.md` "Design constraints".
- Boundary contract between root `cntrdct` and `research/`: as
  written in `CLAUDE.md` "Boundary contract (do not violate)".
  Research promotions remain explicit `promote(<area>): ...` commits.
- Citation policy: as written in `docs/spec/citations-policy.md`.
  Per-language survey requirement applies to every new language
  added under G3.
- Release procedure: as written in `CLAUDE.md` "Release procedure".
  Annotated tag + version match + `tests/fixtures/baselines/baselines/v<release>/`
  rename discipline continues. The rename step disappears for v0.6.0
  if R-1 retires the baselines fixture tree (see R-1 scope).

## 3. Internal layout target (v0.6.0)

```
cntrdct/                                # single crate, single repo
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── cargo_subcommand.rs
│   ├── core.rs                         # Detector trait, Language enum, Finding, etc.
│   ├── ir.rs                           # NEW: language-agnostic AST shape
│   ├── parsers/                        # NEW: per-language tree-sitter → IR
│   │   ├── mod.rs                      # ParserProvider trait, parser_for()
│   │   ├── rust.rs
│   │   ├── python.rs
│   │   └── ...                         # one file per language
│   ├── detectors/
│   │   ├── arg_swap.rs                 # cross-cutting, IR-based
│   │   ├── clone_drift.rs
│   │   ├── comment_code.rs
│   │   ├── pr_miner/
│   │   ├── unreachable_after_terminator.rs
│   │   └── lang/                       # NEW: language-specific
│   │       └── rust_config_interaction.rs   # moved from src/detectors/config_interaction.rs
│   ├── eval.rs / sarif.rs / ranker.rs / adjudicator.rs / ...   # unchanged
│   └── ...
├── tests/                              # unchanged shape; tests gain IR-conversion coverage
├── benchmarks/
│   ├── audit-corpus/
│   ├── wild-corpus/                    # Rust (current)
│   ├── wild-corpus-python/             # current
│   └── wild-corpus-<lang>/             # added one per new language
├── docs/
│   ├── spec/                           # gains ir-v0.md alongside existing specs
│   └── surveys/                        # per-(detector, language) surveys
├── CITATIONS.md
├── CONTRIBUTING.md                     # documents the language-contribution workflow
├── CHANGELOG.md
└── REBUILD.md                          # this file
```

Boundary against `research/` is unchanged. Research members do not
gain IR access; promotion remains explicit re-implementation.

## 4. Phased execution sequence

The work is sequenced so each phase has a discrete review surface.

R-0. IR design spec — `[x]`

- Authoritative design at `docs/spec/ir-v0.md`. Drafted 2026-05-24,
  post-R-0-review revision 2026-05-25 absorbing 11 blockers from a
  4-axis parallel review (IR design / detector coverage / spec
  consistency / risk completeness). Reviewer-absorbed decisions
  are settled — `Arc<str>` source sharing,
  `struct IrStmt { kind, attributes, location }`,
  `IrStmtKind::{With, HoistedItem}` variants,
  `Location.{start_byte, end_byte}` for raw-source-slice consumers,
  `DivergentKind` / `BranchMergeKind` / `ConstantBranchKind` enums
  replacing stringly-typed `kind`, `IrExpr::{Return, Raise}`
  Option-wrapping, `IrFile::resolve(NodeRef)` lookup,
  `IrConvertError` production-runtime contract (warn-log +
  per-file skip, no synthetic SARIF finding), LSP cache requirement
  for partial-parse non-regression. See ir-v0.md "Risks and open
  questions" R8-R11 for derived decisions; do not re-litigate.

R-1. IR implementation + Rust/Python migration — `[ ]`

Authoritative design: `docs/spec/ir-v0.md`. Sub-step ordering is
binding (ir-v0.md R11):

R-1.0. Capture T1 pinning fixtures (ir-v0.md F6 T1) at v0.5.2 HEAD
       before any rewrite begins. `cntrdct scan --json` against
       `benchmarks/audit-corpus` and `benchmarks/wild-corpus*` per
       detector, snapshot to `tests/fixtures/ir-pinning/<detector>/
       {audit,wild}.json`.
R-1.a. Create `src/ir.rs` with the IR types per ir-v0.md F1.
R-1.b. Promote `src/parsers.rs` to `src/parsers/{mod,rust,python}.rs`
       and implement `to_ir` per ParserProvider per ir-v0.md F2.
R-1.c. Rewrite the five cross-cutting detectors against IR.
       Suggested order: comment-code → arg-swap →
       unreachable-after-terminator → clone-drift → pr-miner.
       `cntrdct-lsp` LSP cache change (ir-v0.md F5 non-regression)
       lands in this commit.
R-1.c'. T7 measurement (ir-v0.md F6 T7): capture wall-clock +
       peak-RSS on `benchmarks/wild-corpus-python` (~600 files)
       via the still-present `tests/baselines.rs` harness. MUST
       run before R-1.e retires that harness.
R-1.d. `git mv src/detectors/config_interaction.rs
       src/detectors/lang/rust_config_interaction.rs` + module
       path updates + test path rewrite
       (`tests/detector_config_interaction.rs`).
R-1.e. Retire Q-15 baseline scaffolding:

       ```sh
       git rm src/baselines.rs
       git rm -r baselines/sourcerercc baselines/pybuglab
       git rm tests/baselines.rs
       git rm -r tests/fixtures/baselines/
       ```

       Remove `pub mod baselines` from `src/lib.rs`. Remove
       `--baseline` / `--baselines-out` / `--baselines-skip-run`
       flags from `cntrdct eval` (clap + match). Rewrite the
       README "Baseline comparison" section as "Self-replication
       ledger" documenting `benchmarks/self-replication/v<release>/`.
R-1.f. Introduce `benchmarks/self-replication/v0.6.0/cntrdct.jsonl`
       as the eval snapshot. Add an `assemble_report` helper
       computing the delta against the previous tag's snapshot
       (F1 / P / R deltas). Manual refresh per release; no CI gate.
R-1.g. Recalibrate priors:

       ```sh
       cargo run --release -- calibrate
       cargo run --release -- calibrate --audit-recall benchmarks/audit-corpus
       ```

       Commit the `benchmarks/priors-default.json` update.
R-1.h. `CHANGELOG.md` entry + `Cargo.toml` version → `0.6.0`.
       Conventional Commits prefix: `feat(ir)!`. Breaking-change
       note in the commit message body.
R-1.i. Release per CLAUDE.md "Release procedure":

       ```sh
       cargo update -p cntrdct
       git add Cargo.toml Cargo.lock
       git commit -m "chore(release): bump version to 0.6.0"
       git tag -a v0.6.0 -m "release v0.6.0"
       git push --follow-tags
       ```

R-1 also resolves the historical references to the retired
`ROADMAP.md` and the v0.5.x `ParsedFile` type across the repo
(see §9 "R-1 cross-cutting sweep" below) and updates the five
detector specs + `lsp-v0.md` per ir-v0.md F4. The
`tests/fixtures/baselines/baselines/v<release>/` rename step from
CLAUDE.md "Release procedure" is retired by R-1.e — does not
apply to v0.6.0.

Cuts v0.6.0.

R-2. TypeScript pilot — `[ ]`

- Add `src/parsers/typescript.rs`, `tree-sitter-typescript` Cargo
  dep, `benchmarks/wild-corpus-typescript/`.
- Run the per-(detector, TypeScript) literature surveys under
  `docs/surveys/<detector>-typescript-2026-MM.md`. Citations land
  under `CITATIONS.md` with `Languages: TypeScript` lines (or the
  `unconfirmed:` form per `docs/spec/citations-policy.md`).
- Re-calibrate priors with the TypeScript corpus included.
- Cuts v0.7.0. This is the proof point that the G1+G3 promises hold:
  the PR diff should be parser + corpus + surveys, not detector
  edits.

R-3. Go pilot — `[ ]`

- Same shape as R-2 for Go. Validates that R-2 was not
  TypeScript-specific.
- Cuts v0.7.x.

R-4. P3 revisit for Layer 0 LLM (carry-over from Q-17) — `[ ]`

- Architectural amendment to P3 permitting a Layer 0 candidate
  generator that runs an LLM against IR call-site predicates
  before Layer 1 sees them. Out-of-scope for v0.6.0 but spec
  drafting starts after R-1 ships so the IR predicates are
  available to anchor the amendment.
- Required reading: `docs/spec/arg-swap-v0.md` "Name-correlation
  upper bound", `docs/spec/cross-model-kappa-v0.md` (Q-13 reuse
  precedent).
- Spec: `docs/spec/p3-amendment-v0.md` (new).

R-5. Python `except` handler reachability — `[ ]`

- Implement F4f (raise-set extraction + class-hierarchy check)
  under `src/detectors/lang/python_unreachable_except.rs`. This is
  the first language-specific detector under the new layout and
  serves as the canonical example for contributors.
- Builtin contract table at `data/python-builtin-exceptions.json`
  with per-entry CPython doc URLs (provenance audit-able for
  future Python version drift).
- Spec: extend `docs/spec/unreachable-after-terminator-v0.md` with
  the F4f section.

R-6. VS Code extension (carry-over from T3-12 Phase 2) — `[ ]`

- `vscode-cntrdct` extension scaffolding in a separate
  `ktrysmt/vscode-cntrdct` repo, bundling the LSP binary
  auto-downloaded from GitHub Releases. Phase 1 LSP already shipped
  in v0.5.2; Phase 2 is the extension itself.
- Independent of R-1 through R-5; can start any time after R-1
  ships v0.6.0 (the LSP binary tracks the same crate version).

R-7. Community foundation (carry-over from T4-20 / T4-21) — `[ ]`

- Code of Conduct: short pointer to Contributor Covenant URL at
  `CODE_OF_CONDUCT.md`. Adopt when external contributor activity
  warrants the triage path, but no later than R-2 cuts (because
  R-2 onwards explicitly invites outside language contributions).
- GitHub Discussions enabled; roadmap discussion pinned with a
  link to this file.

R-8. mdBook Pages re-enable (carry-over from T3-13 Phase 2) — `[ ]`

- Blocked on `docs/site/essays/` external-blog migration. No
  action needed at the v0.6.0 cut. Stays on the list as a
  visible reminder.

## 5. Execution order

1. R-0 — IR spec. (done 2026-05-25; see §4 R-0)
2. R-1 — IR implementation, Rust + Python migration, Q-15 scaffolding retirement. Cuts v0.6.0.
3. R-7 — Code of Conduct + Discussions before R-2 invites outside contributions.
4. R-2 — TypeScript pilot. Cuts v0.7.0.
5. R-3 — Go pilot.
6. R-4 — P3 amendment spec (R-1 IR enables predicate-level framing).
7. R-5 — Python F4f detector.
8. R-6 — VS Code extension (independent of the rest after R-1).
9. R-8 — mdBook Pages (blocked, fires when its prerequisite resolves).

The order assumes R-1 is the bottleneck. R-2 / R-3 / R-5 / R-6 /
R-8 can parallelise once R-1 lands.

## 6. Out of scope

Explicitly not part of this rebuild:

- External SOTA comparator framework (the retired Q-15 vision).
  The "compare against PyBugLab / SourcererCC head-to-head" idea
  was structurally unrealisable; the self-replication ledger
  under R-1 is the deliberate replacement.
- Out-of-process plugin model (WASM, dlopen, JSON-line protocol,
  separate `cntrdct-plugin-<lang>` crates on crates.io). Considered
  and dropped; the single-repo contribution model is the
  intentional design.
- Cargo features for opt-in language compilation. Reconsider only
  if binary size becomes a measurable complaint after R-2 or R-3.
- AUR distribution. Dropped from T3-14 scope; reopen as a new
  R-series item if external demand surfaces.
- Apriori → FP-growth migration in pr-miner. Internal quality
  improvement, tracked in `docs/spec/pr-miner-v0.md` future work,
  not in REBUILD scope.
- Layer 3 ML-detector ensemble (PyBugLab / GraphCodeBERT alongside
  the LLM judge). Crosses the P3 boundary like R-4; subsumed by
  the R-4 amendment when that lands.
- Translation of cntrdct output, docs, or surveys. English-only,
  as before.

## 7. Glossary

- IR — Intermediate Representation. The language-agnostic AST shape
  defined in `src/ir.rs` that cross-cutting detectors consume.
  Specified in `docs/spec/ir-v0.md` (R-0).
- Cross-cutting detector — a detector whose concept transfers
  across languages (arg-swap, clone-drift, comment-code, pr-miner,
  unreachable-after-terminator). Written once on IR after R-1.
- Language-specific detector — a detector whose concept is bound to
  one language's syntax (Rust `#[cfg]` interaction, Python `except`
  reachability). Lives under `src/detectors/lang/<lang>_<id>.rs`,
  reads tree-sitter ASTs directly.
- Self-replication ledger — `benchmarks/self-replication/v<release>/`,
  the per-release snapshot of cntrdct's own eval against its
  corpora. Replaces the retired Q-15 "external SOTA comparator"
  framing.
- Layer 0 — proposed candidate-origination layer that runs an LLM
  against IR predicates before Layer 1 sees them. Requires the R-4
  P3 amendment before any implementation.
- R-series — the rebuild item numbering used in this file. Replaces
  the retired Q-series / T-series / M-series numbering.

## 8. Release sequence preview

- v0.6.0 — R-0 + R-1. Breaking change (the `Detector` trait
  migrates from `ParsedFile` to `IrFile`; the `cntrdct` crate
  bumps from v0.5.x to v0.6.0). Conventional Commits prefix
  `feat(ir)!`. Self-replication ledger replaces baseline scaffolding.
- v0.6.x — fast-follows: R-5 (Python F4f), R-7 (CoC + Discussions).
- v0.7.0 — R-2 (TypeScript pilot). First language added under the
  community-PR model.
- v0.7.x — R-3 (Go pilot), R-6 (VS Code extension), R-8 if
  unblocked.

Version bumps follow `CLAUDE.md` "Release procedure" verbatim; the
`tests/fixtures/baselines/baselines/v<release>/` rename step is
retired by R-1 along with the baselines scaffolding.

## 9. R-1 release gate (verification rules)

Before tagging v0.6.0:

- `cargo test --all-targets` green.
- `cargo clippy --all-targets -- -D warnings` green.
- `cargo fmt --all -- --check` green.
- `cargo run --release -- calibrate --audit-recall benchmarks/audit-corpus`
  reports `overall_recall_upper_bound` ≥ 0.92 (v0.5.2 baseline floor;
  ir-v0.md F6 T6 codifies the threshold). A regression below 0.92
  with T1 green indicates corpus / labelling drift; below 0.92 with
  T1 red indicates converter drift — go back to R-0 spec and revise
  IR representation.
- `cntrdct ALL_DETECTOR_IDS` includes `config-interaction` (verifies
  R-1.d module relocation did not drop the registration).
- `tests/citations_consistency.rs` green (citation key set unchanged
  by R-1).
- `network-isolation` CI job green: `sudo unshare --net cntrdct scan ...`
  must not emit `ENETUNREACH` / `EAI_*` (CLAUDE.md P3 gate). The
  Q-13 `cross-model-kappa` subcommand stays excluded from this
  gate by design; it does not go through IR.
- T1 pinning fixtures (ir-v0.md F6 T1) byte-identical against the
  v0.5.2 capture per detector.
- T7 wall-clock + peak-RSS regression < 25 % vs v0.5.2 (ir-v0.md R1).
- T2 / T3 / T4 / T5 per ir-v0.md F6 (IrConvertError variants,
  parse_recovered carry-through, IR golden fixtures, Location
  equality).

## 10. R-1 cross-cutting sweep

The R-1 PR resolves historical cross-references that survive in
docs / book / benchmarks / research. Three-way choice per occurrence:
preserve as history, swap to `REBUILD.md` (or `ir-v0.md`), or delete
entirely. Decision logged in the R-1 PR description.

ROADMAP.md references (14 locations, none in code):

- `docs/spec/multilang-v0.md` (3)
- `docs/spec/pr-miner-v0.md` (3)
- `docs/spec/recall-audit-v0.md` (1)
- `docs/spec/citations-policy.md` (1)
- `docs/spec/cross-model-kappa-v0.md` (2)
- `docs/spec/llm-calibration-v0.md` (1)
- `docs/spec/sota-baselines-v0.md` (1) — file retired entirely by R-1
- `docs/spec/arg-swap-v0.md` (1)
- `docs/spec/lsp-v0.md` (1)
- `book/src/{introduction,releases}.md` (2)
- `docs/site/index.md` (1)
- `benchmarks/{audit-corpus,retraction-watch,wild-corpus-python}/README.md`
  (3)
- `research/projects/PLAN.md` (4)

ParsedFile references (per ir-v0.md F4 override):

- `docs/spec/{arg-swap-v0.md, clone-drift-v0.md, comment-code-v0.md,
  unreachable-after-terminator-v0.md, pr-miner-v0.md}` F1 input
  sections.
- `docs/spec/lsp-v0.md` `scan_buffer` signature and any other
  `ParsedFile` mention.

Verify after the sweep:

```sh
rg -l 'ROADMAP.md' -- docs/ book/ benchmarks/ research/
rg -l 'ParsedFile' -- docs/ src/ tests/
```

The first should return empty (or only `REBUILD.md` itself if its
historical commentary still references the retirement). The second
should return empty under `src/` and `tests/` (R-1 deletes the
type) and `docs/` (R-1 sweep).

## 11. R-1 reading list

A fresh session picking up R-1 needs:

Authoritative (read in full before R-1.a):

- This file (REBUILD.md), §3 layout target, §4 R-1, §9 verification,
  §10 sweep.
- `docs/spec/ir-v0.md` — IR types, conversion contract, test plan,
  risks. Settled design; do not re-litigate per §4 R-0.
- `CLAUDE.md` — design constraints (P1 / P3 / P4 / P5), boundary
  contract with `research/`, release procedure, repo layout rules.
- `docs/spec/multilang-v0.md` — the `ParserProvider` seam ir-v0.md
  extends with `to_ir`.
- `docs/spec/citations-policy.md` — unchanged by R-1 but governs
  the per-language survey requirement R-2 onwards.
- `src/parsers.rs` (149 LOC), `src/core.rs` (623 LOC) — the
  primary implementation seams R-1 modifies.

Context (skim before R-1.c):

- `src/detectors/{arg_swap, clone_drift, comment_code,
  unreachable_after_terminator}.rs` — the cross-cutting detectors
  R-1.c rewrites. Per-language `scan_rust` / `scan_python` helpers
  fold into IR converter responsibilities.
- `src/detectors/pr_miner/{mod, apriori, extract_rust,
  extract_python}.rs` — already split per language; IR converter
  replaces the `extract_*` files.
- `src/detectors/config_interaction.rs` — R-1.d target for
  relocation into `src/detectors/lang/`. Stays on raw tree-sitter
  (Pattern B, ir-v0.md F5 escape hatch).
- `src/baselines.rs` (931 LOC), `tests/baselines.rs` (364 LOC),
  `baselines/{sourcerercc,pybuglab}/`, `tests/fixtures/baselines/`
  — R-1.e retirement targets.
- `tests/citations_consistency.rs`, `tests/corpus_shape.rs`,
  `tests/wiring_consistency.rs` — touched by R-1 but shape
  preserved; only fixture / dispatch updates.
- `src/lsp.rs` — `scan_buffer` migrates to IrFile; the F5
  partial-parse cache change lands in R-1.c.
