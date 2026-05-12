# cntrdct external-source audit corpus

Q-14 deliverable from `ROADMAP.md`. Houses the corpus
`cntrdct calibrate --audit-recall` runs against to report
per-detector recall upper bounds. Spec:
[`docs/spec/recall-audit-v0.md`](../../docs/spec/recall-audit-v0.md).

Status: Phase B batch 2 (2026-05-12). Twelve expected entries
across two detectors and two external sources after the batch-2
addition. Batch 2 deepens the recall ceiling on
`unreachable-after-terminator` rather than broadening detector
coverage — the six new entries all sit on patterns cntrdct's
statement-level scan misses by construction (return-as-expression-
tail, return-as-call-argument, divergent-loop terminator), so the
new figures sharpen the honest gap rather than hide it. Subsequent
batches still owe coverage for the remaining four detectors
(clone-drift, comment-code, config-interaction, pr-miner) and for
the semgrep / codeql / clippy / paper-appendix source kinds. The
figures under "Latest audit run" refresh on each release tag.

## Why this corpus is separate from `wild-corpus/`

`benchmarks/wild-corpus/` is selected by crates.io
top-by-downloads with no reference to bug-tracker history; its
manifest captures cntrdct's own findings, triaged by a
maintainer. That makes it suitable for false-positive discovery
(P-1 / P-4) but unable to measure recall — every expected entry
is, by construction, something cntrdct already produced.

`benchmarks/audit-corpus/` is selected by *external sources*
(NVD / OSV.dev / Semgrep / CodeQL community / Clippy testsets,
plus paper-appendix anomaly sets). Its expected entries are
findings cntrdct *should* catch, regardless of whether cntrdct
currently does. Recall numbers therefore mean what they say:
out of the externally-flagged bugs, what fraction did cntrdct
detect?

The recall is reported as an upper bound — external sources
have their own recall failures, so the audit denominator is
itself a subset of the unobserved full ground truth.
Heckman & Williams IST 2011 is the canonical reference for the
selection-bias issue this corpus is built to counter.

## Layout

```
audit-corpus/
├── README.md                       (this file)
├── manifest.jsonl                  (Phase B batches 1-2)
└── files/
    ├── rustc_ui_unreachable_code_ret.rs    (batch 1)
    ├── rustc_ui_expr_block.rs              (batch 1)
    ├── rustc_ui_expr_if.rs                 (batch 1)
    ├── rustc_ui_expr_return.rs             (batch 2)
    ├── rustc_ui_expr_call.rs               (batch 2)
    ├── rustc_ui_expr_loop.rs               (batch 2)
    └── totalsegmentator_statistics.py      (batch 1)
```

`manifest.jsonl` follows the schema in
`docs/spec/recall-audit-v0.md` §F1-F3. Every `expected` entry
carries an `external_source` block with `kind`, `ref`, and a
stable `url`. The per-file `source` / `license` / `sha256`
triple is shared with `wild-corpus/`.

## Source list

Canonical `external_source.kind` values used by this corpus:

| kind                  | source                                                                                |
| --------------------- | ------------------------------------------------------------------------------------- |
| `nvd`                 | NVD CVE entries (https://nvd.nist.gov/vuln)                                           |
| `osv`                 | OSV.dev advisory database (https://osv.dev/)                                          |
| `semgrep`             | Semgrep registry rules (https://semgrep.dev/r)                                        |
| `codeql`              | CodeQL community pack queries (https://github.com/github/codeql)                      |
| `clippy`              | rust-lang/rust-clippy lint testset                                                    |
| `rustc-lint-testset`  | rust-lang/rust built-in lint testset (`tests/ui/...`)                                 |
| `github-commit`       | upstream bug-fix commit on GitHub; `ref` carries `<owner>/<repo> <commit-or-PR>:<file>:<line>` |
| `paper-appendix`      | published bug catalogues from peer-reviewed work (e.g. PyPIBugs, PR-Miner anomalies)  |

Adding a new source kind is a pure README + manifest change; the
loader treats `kind` as a freeform string. The cap is editorial,
not structural.

## Per-detector seed targets (Phase B)

cntrdct's six detectors map to external sources unevenly. The
seed targets below are the working list for Phase B; revisit on
each release tag.

- `arg-swap` (Interface)
  - PyBugLab / PyPIBugs swapped-arguments partition (Allamanis
    NeurIPS 2021).
  - Semgrep `swapped-arguments` rule family.
- `clone-drift` (Logic)
  - Inconsistent-clone evolution findings from Assi TOSEM 2025
    on Python deep-learning frameworks.
  - SourcererCC / NiCad published clone benchmarks where
    upstream patches landed.
- `comment-code` (Documentation)
  - Tan SOSP 2007 (`/*iComment`) and PLDI 2011 (`aComment`)
    published bug pairs.
- `config-interaction` (Logic)
  - Nadi ICSE 2014 contradictory cfg constraints from
    Linux KConfig.
- `pr-miner` (Logic)
  - Li-Zhou FSE 2005 PR-Miner published anomalies (where
    sources are still accessible).
- `unreachable-after-terminator` (Logic)
  - Hovemeyer-Pugh OOPSLA 2004 FindBugs UR pattern testset.
  - rustc UCDR examples; `rust-clippy` `unreachable_code`
    fixture set.

Each Phase B addition lands with:

1. Source files committed under `files/` with the standard
   3-line provenance header (Source / License / Note). The header
   counts toward line numbering, so manifest `expected.line`
   values are upstream-line + header offset.
2. A `manifest.jsonl` entry with `expected[]` populated and
   each entry's `external_source` set. The `external_source.url`
   MUST be a stable, deep-linkable URL — fragile listing pages
   are explicitly out of scope. Commit-SHA-pinned blob URLs from
   GitHub are the canonical form for `rustc-lint-testset` and
   `github-commit` sources.
3. README's "Latest audit run" section updated with the new
   recall figures (re-run `cntrdct calibrate --audit-recall
   benchmarks/audit-corpus`).
4. `sha256` field on the manifest entry: for verbatim copies, the
   SHA-256 of the upstream file (anyone can re-fetch the source URL
   and verify). For minimal extracts (when the upstream file is too
   large or carries unrelated dependencies), the SHA-256 is of the
   audit-corpus file as committed; the Note line of the provenance
   header declares which mode applies.

Operational caveat: cntrdct's
`unreachable-after-terminator` detector treats any in-source
attribute containing the substring `unreachable_code` as a
suppression (see `SUPPRESSION_TOKEN` in
`src/detectors/unreachable_after_terminator.rs`). Audit entries
sourced from the rustc `unreachable_code` lint testset therefore
strip the file-level `#![deny(unreachable_code)]` attribute on
extraction; the Note line in each affected file documents the
stripping. Per-line `//~ ERROR ...` annotations are preserved
verbatim because cntrdct ignores comment bodies.

## Running the audit

```sh
cntrdct calibrate --audit-recall benchmarks/audit-corpus
```

Default output is pretty JSON to stdout. Pipe to a file
(`> audit.json`) or pass `--output PATH` to write to disk.

JSON shape (selected fields):

```jsonc
{
  "per_detector": {
    "arg-swap": {
      "tp": 0,
      "fn": 1,
      "recall_upper_bound": 0.0,
      "source_breakdown": { "github-commit": { "tp": 0, "fn": 1 } }
    }
  },
  "overall": { "tp": 3, "fn": 9, "recall_upper_bound": 0.25, "source_breakdown": { /* aggregated */ } },
  "corpus_size": 7,
  "expected_total": 12,
  "sources": { "github-commit": 1, "rustc-lint-testset": 11 }
}
```

## Latest audit run

Refreshed 2026-05-12 against the v0.2.0-rc.7 binary on the Phase
C release-tag cadence. No-op refresh from the rc.6 + Phase B
batch-2 numbers (same detector logic, same corpus); the re-run
itself is the Q-14 Phase C discipline.

| detector                       | tp | fn | recall upper bound | dominant source                  |
| ------------------------------ | --:| --:| ------------------:| -------------------------------- |
| `arg-swap`                     |  0 |  1 |               0.00 | `github-commit` (1/1 entries)    |
| `unreachable-after-terminator` |  3 |  8 |               0.27 | `rustc-lint-testset` (11/11)     |
| **overall**                    |  3 |  9 |               0.25 |                                  |

Corpus size: 7 files. Expected entries: 12. Source mix:
`rustc-lint-testset` (11 entries), `github-commit` (1).

Reading the figures:

- The three `unreachable-after-terminator` true positives all
  come from rust-lang/rust ui-tests where the rustc lint and
  cntrdct's tree-sitter scan converge on the same pattern
  (statement-level `return;` followed by a normal statement).
- The eight false negatives partition by detector limitation:
  - Two from `rustc_ui_expr_if.rs` (batch 1): the divergent
    control flow lives inside an `if` / `if-else` expression
    and cntrdct's detector does not look through expression
    boundaries by design.
  - One from `rustc_ui_expr_return.rs` (batch 2): the `return`
    sits inside a tail expression `let x: () = {return ...};`
    rather than at statement position; cntrdct's terminator
    classifier requires an `expression_statement` with a
    direct `return_expression` child.
  - Two from `rustc_ui_expr_call.rs` (batch 2):
    `foo(return, 22)` and `bar(return)` place the diverging
    `return` as a call argument. cntrdct does not descend into
    `call_expression` arguments to surface their type-`!`
    consequences.
  - Three from `rustc_ui_expr_loop.rs` (batch 2): the
    terminator is an unconditional `loop { return; }` or a
    nested non-breaking loop construct. The rustc lint reasons
    about the loop's never-type return; cntrdct does not
    recognise `loop_expression` itself as a terminator.
- The single `arg-swap` false negative is from a real bug-fix
  PR (wasserth/TotalSegmentator #556) where two args to
  `get_radiomics_features` were swapped. cntrdct's `arg-swap`
  detector fires only when the call-site identifier multiset is
  a reverse permutation of the parameter-name multiset; in this
  bug the call identifiers (`ct_file`, `mask`) do not match the
  parameter names (`seg_file`, `img_file`), so cntrdct cannot
  catch the swap semantically. This is a documented detector
  limitation, not a defect.

The shape of batch 2's contribution is intentional: every new
entry pins a known cntrdct-side limitation against rustc's
ground truth, lowering the recall ceiling from 0.50 to 0.25 on
the same external lint. The "upper bound" qualifier keeps the
figures honest — closing these gaps is detector-improvement work
(separate engineering, separate preregistrations), not audit-
harness work.

Future batches will broaden source coverage (semgrep, codeql,
clippy, paper-appendix) and add the four remaining detectors
(clone-drift, comment-code, config-interaction, pr-miner).

## Refresh discipline (Phase C)

Q-14 Phase C deliverable: the audit-run figures in this README
are refreshed manually on every `vX.Y.Z` release tag, not on a
cron. Q-13's design rationale carries over — commercial LLMs and
external SAST catalogues version-bump silently, so a time-series
of recall figures would capture upstream catalogue drift rather
than any cntrdct-side property. The release-tag cadence is the
narrowest window that still keeps the published figures honest.

Procedure on each release tag (the release-procedure non-
negotiable lives in `CLAUDE.md`; this section is the source-of-
truth for the audit-specific steps):

1. Sync `master` to the to-be-tagged commit.
2. Run the audit against the same binary the tag will produce:

   ```sh
   cargo run --release --bin cntrdct -- \
     calibrate --audit-recall benchmarks/audit-corpus
   ```

   Default output is pretty JSON to stdout. Capture the
   `per_detector` / `overall` / `corpus_size` / `expected_total`
   / `sources` blocks for the README.
3. Update this README's "Latest audit run" section:
   - Refresh date stamp to the release-tag day, and the binary
     reference to `v0.2.0-rc.N` / `v0.2.0` / `vX.Y.Z`.
   - Replace every per-detector row with the new `tp` / `fn` /
     `recall_upper_bound` / dominant-source column. Round
     `recall_upper_bound` to two decimal places to match the
     table format; the raw float lives in the JSON output for
     anyone re-running.
   - Update the overall row and the corpus / expected /
     source-mix one-liner directly below the table.
   - Update the "JSON shape (selected fields)" example numbers
     above the table so the README is internally consistent.
4. If a detector's `recall_upper_bound` moved by ≥ 0.05 since
   the previous tag, add a one-paragraph note under "Reading
   the figures" explaining the cause (new audit entries vs.
   detector-side change). Movements below 0.05 within a single
   release are noise; no note required.
5. Stage the README change in the same commit as the
   `chore(release): bump version to X.Y.Z` commit so the audit
   numbers and the published binary co-arrive. Do NOT split the
   audit refresh into a follow-up commit — the tag's
   reproducibility story is "this README claim was produced by
   that binary", and a follow-up commit breaks that link.
6. If no `expected[]` entries changed since the previous tag
   and the binary did not touch any of the six Layer 1
   detectors, the audit numbers are bit-identical and step 5
   becomes a no-op for the README (the figures pass through
   unchanged). The intent is still to re-run the audit on every
   tag for the same reason CI runs the rest of the suite on
   every push — to keep the assertion live, not to wait for a
   regression to surface it.

The refresh is not enforced by CI. The rationale is that
audit-recall depends on the embedded `priors-default.json` and
on the shipped binary's detector logic, both of which CI already
gates. A README-update gate on top would catch the same drift
twice. The release-procedure non-negotiable is the discipline
that closes the gap.

## Out of scope

- A quarterly cron job that recomputes recall and updates the
  README. External sources update silently and a time-series of
  recall figures captures upstream catalogue drift more than any
  cntrdct property; manual refresh on each release tag is
  sufficient.
- Live SAST tool runs as comparators. The harness consumes
  *labelled* audit-corpus entries, not live comparator findings.
  Comparator runs are Q-15 territory.
- A "missed-finding" remediation pipeline. The audit reports the
  gap; closing it (detector improvement / new detector) is
  separate engineering work.

## License notes

Every file under `files/` redistributes its upstream source under
the upstream license, recorded both in the file's provenance
header and the `license` field of its manifest entry. Permissive
licenses only; GPL / LGPL / MPL / proprietary excluded for the
same reason wild-corpus excludes them.
