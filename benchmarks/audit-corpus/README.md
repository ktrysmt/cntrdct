# cntrdct external-source audit corpus

Q-14 deliverable from `ROADMAP.md`. Houses the corpus
`cntrdct calibrate --audit-recall` runs against to report
per-detector recall upper bounds. Spec:
[`docs/spec/recall-audit-v0.md`](../../docs/spec/recall-audit-v0.md).

Status: Phase B batch 8 (2026-05-13). Thirty expected
entries across six detectors and six external source kinds
(`rustc-lint-testset`, `github-commit`, `paper-appendix`,
`clippy`, `codeql`, `semgrep`). Batch 8 introduces the `semgrep`
source kind alongside the sixth and final cntrdct detector,
`pr-miner`, via the Semgrep registry rule `open-never-closed`
(`python.lang.best-practice.open-never-closed.open-never-closed`,
pinned at semgrep/semgrep-rules@9d73d08e) applied to
TuGraph-family/tugraph-db@672e4b19 `release/det_ver.py`
(Apache-2.0). The labelled bug is `f = open('Options.cmake','r')`
inside top-level `def get_ver():` (upstream line 5 / corpus line
9) without any matching `close()`, `with`, or try-finally; the
companion `def replace_ver(...)` in the same file opens AND
closes, so the rule does not fire on it. Mapping against
cntrdct's pr-miner: spec F2 reaches `get_ver` (top-level), but
spec F3 Apriori mining at `MIN_SUPPORT = 0.05` /
`MIN_CONFIDENCE = 0.85` cannot synthesise the `{open} → {close}`
rule from the single corpus-wide transaction that contains both
items (only `replace_ver`), so F4 violation detection never runs
against `get_ver` — FN by mining sparsity, NOT by extractor
scope. Selecting tugraph-db rather than semgrep-rules' own test
fixtures sidesteps the Semgrep Rules License v1.0 license
carve-out the README's earlier batches deferred on. Overall
`recall_upper_bound` settles at 0.27 (down from 0.28 at batch 7)
— a one-step drop driven entirely by `pr-miner` entering the
denominator with 0 TPs by detector design (mining sparsity at v0
corpus density), not by any of the other five detectors
regressing. Batch 7 (2026-05-12): the `codeql` source kind via
the CodeQL Python `UnreachableCode` query test fixture
(`github/codeql@592c7c04
python/ql/test/query-tests/Statements/unreachable/test.py`, MIT).
Six expected unreachable-statement entries land per CodeQL's
matching `UnreachableCode.expected`; one is TP (`for x in
first_unreachable_stmt():` directly following `return 5`), the
other five are FN by spec F3 (constant-condition branches and
typed-exception reasoning are outside cntrdct's terminator set). Batch 6 (2026-05-12): two
`rustc-lint-testset` config-interaction FNs from
`rust-lang/rust@29b75901 tests/ui/cfg/both-true-false.rs`; both
FN by `docs/spec/config-interaction-v0.md` F5 (atomic `true` /
`false` lack the `not(...)` wrapper). Batch 5 (2026-05-12): two
`clippy` clone-drift FNs from rust-clippy UI tests
(`if_same_then_else`, `branches_sharing_code/shared_at_top`)
pinned at master commit `c4b8c6d4`; both FN against cntrdct's
clone-drift detector by `docs/spec/clone-drift-v0.md` F2 (top-level
`fn` granularity only). Batch 4 (2026-05-12): three
`paper-appendix` arg-swap FNs from PyPIBugs (Allamanis NeurIPS
2021) on `c137digital/unv_app` MIT, `mwouts/nbrmd` MIT, and
`markokr/rarfile` ISC; all three are FNs against the narrow
Rice-2017 arg-swap detector. Batch 3 (2026-05-12): four
`comment-code` TPs on `sidan-lab/whisky-archive` `con_str*`
family — Tan SOSP 2007 §3.2 "bad comment". All six cntrdct
detectors are now represented in the corpus (arg-swap,
clone-drift, comment-code, config-interaction, pr-miner,
unreachable-after-terminator), and the six external source
kinds documented under "Sources" are all live
(`rustc-lint-testset`, `github-commit`, `paper-appendix`,
`clippy`, `codeql`, `semgrep`). Future Phase B batches deepen
coverage rather than introduce a new detector or kind: more
`open-never-closed` instances across permissive-licensed Python
files to push pr-miner's mined-rule confidence above the 0.85
threshold and surface the first pr-miner TP; broader
`unreachable-after-terminator` coverage if F3 widens to
constant-condition or exception-typed reasoning; arg-swap and
clone-drift TPs once the detectors' v0 scope is lifted. The
`nvd` and `osv` kinds documented in "Source list" remain unused
in v0; CVE-shaped bugs that map to any cntrdct detector are rare
in practice (security CVEs typically target injection,
deserialisation, or memory-safety patterns outside cntrdct's
contradiction-linter scope), so the slots stay open without a
pinned commitment. The figures under "Latest audit run" refresh
on each release tag.

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
├── manifest.jsonl                  (Phase B batches 1-8)
└── files/
    ├── rustc_ui_unreachable_code_ret.rs               (batch 1)
    ├── rustc_ui_expr_block.rs                         (batch 1)
    ├── rustc_ui_expr_if.rs                            (batch 1)
    ├── rustc_ui_expr_return.rs                        (batch 2)
    ├── rustc_ui_expr_call.rs                          (batch 2)
    ├── rustc_ui_expr_loop.rs                          (batch 2)
    ├── totalsegmentator_statistics.py                 (batch 1)
    ├── whisky_archive_constructors.rs                 (batch 3)
    ├── unv_app_settings.py                            (batch 4)
    ├── nbrmd_test_ipynb_to_R.py                       (batch 4)
    ├── rarfile_set_attrs.py                           (batch 4)
    ├── clippy_ui_if_same_then_else.rs                 (batch 5)
    ├── clippy_ui_branches_sharing_code_shared_at_top.rs (batch 5)
    ├── rustc_ui_both_true_false.rs                    (batch 6)
    ├── codeql_python_unreachable_test.py              (batch 7)
    └── tugraph_det_ver.py                             (batch 8)
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
    "comment-code": {
      "tp": 4,
      "fn": 0,
      "recall_upper_bound": 1.0,
      "source_breakdown": { "github-commit": { "tp": 4, "fn": 0 } }
    }
  },
  "overall": { "tp": 8, "fn": 22, "recall_upper_bound": 0.267, "source_breakdown": { /* aggregated */ } },
  "corpus_size": 16,
  "expected_total": 30,
  "sources": { "clippy": 2, "codeql": 6, "github-commit": 5, "paper-appendix": 3, "rustc-lint-testset": 13, "semgrep": 1 }
}
```

## Latest audit run

Refreshed 2026-05-13 against the master tip on top of batch 8.
The next release tag will re-run this against the to-be-tagged
binary per the Q-14 Phase C discipline; batch 8 lands mid-cycle,
so the figures here are the pre-tag snapshot. Batch 8 introduces
the `semgrep` source kind and the sixth detector, `pr-miner`, in
the same commit via the Semgrep registry rule `open-never-closed`
(`python.lang.best-practice.open-never-closed.open-never-closed`,
pinned at semgrep/semgrep-rules@9d73d08e) applied to
TuGraph-family/tugraph-db@672e4b19 `release/det_ver.py`
(Apache-2.0). cntrdct's pr-miner reaches `def get_ver():` (spec
F2, top-level function) but the spec F3 Apriori mining cannot
synthesise the `{open} → {close}` rule from the single
corpus-wide transaction (`replace_ver`) that contains both items,
so spec F4 violation detection never runs against `get_ver` —
FN by mining sparsity, NOT by extractor scope. Overall
`recall_upper_bound` settles at 0.27 (down from 0.28 at batch 7)
— the move is driven entirely by pr-miner entering the
denominator with 0 TPs by detector design at v0 corpus density,
not by any of the other five detectors regressing.

| detector                       | tp | fn | recall upper bound | dominant source                          |
| ------------------------------ | --:| --:| ------------------:| ---------------------------------------- |
| `arg-swap`                     |  0 |  4 |               0.00 | `paper-appendix` (3/4 entries)           |
| `clone-drift`                  |  0 |  2 |               0.00 | `clippy` (2/2 entries)                   |
| `comment-code`                 |  4 |  0 |               1.00 | `github-commit` (4/4 entries)            |
| `config-interaction`           |  0 |  2 |               0.00 | `rustc-lint-testset` (2/2)               |
| `pr-miner`                     |  0 |  1 |               0.00 | `semgrep` (1/1)                          |
| `unreachable-after-terminator` |  4 | 13 |               0.24 | `rustc-lint-testset` (11/17 entries)     |
| **overall**                    |  8 | 22 |               0.27 |                                          |

Corpus size: 16 files. Expected entries: 30. Source mix:
`rustc-lint-testset` (13 entries), `codeql` (6), `github-commit`
(5), `paper-appendix` (3), `clippy` (2), `semgrep` (1).

Reading the figures:

- The four `comment-code` true positives all come from the
  `con_str` / `con_str0` / `con_str1` / `con_str2` family in
  `sidan-lab/whisky-archive@99243766` (a Cardano Plutus-data
  helper crate). Each function carries a `/// Deprecated: Use
  ...` doc comment but ships without the `#[deprecated]` runtime
  attribute, so downstream consumers receive no compiler
  warning. cntrdct's Pattern C
  (`docs/spec/comment-code-v0.md` F5) flags exactly this
  prose / attribute disagreement. The recall_upper_bound of 1.00
  reflects that a Pattern C bug-class within cntrdct's detection
  scope (top-level `fn` items) is captured cleanly when the
  upstream surface matches the spec assumptions.
- The four `unreachable-after-terminator` true positives split
  by source: three come from rust-lang/rust ui-tests where the
  rustc lint and cntrdct's tree-sitter scan converge on the same
  pattern (statement-level `return;` followed by a normal
  statement), and the fourth (batch 7, `codeql`) comes from the
  CodeQL Python `UnreachableCode` test fixture at corpus line 25
  (`for x in first_unreachable_stmt():` directly following
  `return 5`) — the F3 terminator + follower pattern reproduced
  in Python via cntrdct's `analyze_python_block`.
- The thirteen `unreachable-after-terminator` false negatives
  partition by detector limitation:
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
  - Five from `codeql_python_unreachable_test.py` (batch 7,
    `codeql`). Two flag bodies of `while 0:` / `while False:`
    (constant-false loop bodies, corpus lines 12 and 14); two
    flag bodies of `if False:` and the `else:` of `if True:`
    (constant-condition branches, corpus lines 20 and 32); one
    flags an `except NameError:` handler that can never fire
    because `str` is always defined in Python 3 (corpus line
    88). cntrdct's spec F3 terminator set covers neither
    constant-condition branches (Non-goal "Branch-merging
    analysis") nor typed-exception reachability, so all five are
    structural FNs surfacing the same scope choice the Rust-side
    `if`-expression FNs do.
- The four `arg-swap` false negatives partition by external
  source and FN class:
  - One from `totalsegmentator_statistics.py` (batch 1,
    `github-commit`): the call identifiers (`ct_file`, `mask`)
    do not match the parameter names (`seg_file`, `img_file`),
    so the reverse-permutation check in
    `docs/spec/arg-swap-v0.md` F5 returns no match.
  - One from `unv_app_settings.py` (batch 4, `paper-appendix`,
    PyPIBugs MIT label on `c137digital/unv_app@d217fa0d`): the
    callee `update_dict_recur` is imported from
    `unv.utils.collections`, so spec F4 (same-file resolution)
    cannot resolve the definition and the call is skipped.
  - One from `nbrmd_test_ipynb_to_R.py` (batch 4,
    `paper-appendix`, PyPIBugs MIT label on
    `mwouts/nbrmd@dfa96996`): the callee `compare_notebooks` is
    imported from `jupytext.compare` (test code); same F4 miss
    as unv_app.
  - One from `rarfile_set_attrs.py` (batch 4, `paper-appendix`,
    PyPIBugs ISC label on `markokr/rarfile@7fd6b2ca`): the
    buggy call is `self._set_attrs(dst, inf)`, a method call on
    `self`. Spec F3 drops qualified-path / method-call call
    sites entirely; even with a hypothetical method-resolving
    extension the call args (`dst`, `inf`) are not a reverse
    permutation of the definition's parameter names
    (`info`, `dstfn`), so F5 would also miss.

- The two `clone-drift` false negatives both come from
  rust-clippy UI tests pinned at master commit `c4b8c6d4`:
  - `clippy_ui_if_same_then_else.rs` (batch 5, `clippy`):
    `clippy::if_same_then_else` fires on a statement-block
    clone pair (`if true { ... } else { ... }`) inside `fn
    if_same_then_else()`. cntrdct clone-drift v0 operates at
    top-level `fn` granularity only per
    `docs/spec/clone-drift-v0.md` F2, so the clone pair never
    enters the candidate set.
  - `clippy_ui_branches_sharing_code_shared_at_top.rs` (batch 5,
    `clippy`): `clippy::branches_sharing_code` fires when
    if/else branches share a prefix of statement-block code.
    Same F2 miss: cntrdct's clustering operates on whole
    top-level fns, not statement prefixes.
- The two `config-interaction` false negatives both come from
  the rustc UI test `tests/ui/cfg/both-true-false.rs` pinned at
  main commit `29b75901`. Each `fn foo()` carries a syntactically
  contradictory `#[cfg(...)]` pair (`cfg(false)` + `cfg(true)`
  and `cfg(true)` + `cfg(false)`), so both are disabled under
  every configuration; the rustc Reference labels this as the
  `cfg.attr.duplicates` documented behaviour. cntrdct
  config-interaction v0 F5 requires one predicate to be
  structurally `not(X)` and the other to be structurally equal
  to `X`; atomic `true` / `false` lack the `not(...)` wrapper, so
  the detector skips the pair by design.

- The single `pr-miner` false negative (batch 8, `semgrep`) is
  the labelled `f = open('Options.cmake','r')` inside
  `def get_ver():` at corpus line 9 of
  `tugraph_det_ver.py`. Semgrep's `open-never-closed` rule fires
  because no `close()` / `with` / try-finally reaches the file
  handle before the function returns. cntrdct's pr-miner
  reaches the function (spec F2 extracts top-level
  `function_definition` items) and would emit a violation
  finding under spec F4 only if the mined rule
  `{open} → {close}` was first produced by spec F3's Apriori
  pass. With `MIN_SUPPORT = 0.05` and `MIN_CONFIDENCE = 0.85`
  and exactly one corpus-wide transaction (`replace_ver` in the
  same file at corpus line 21) that contains both items, v0's
  mining cannot synthesise the rule, so F4 is never evaluated
  against `get_ver`. The miss is a corpus-density property of
  the audit-corpus, not an extractor-scope property of pr-miner
  — pushing additional permissive-licensed Python files that
  pair `open` with `close` through later Phase B batches lifts
  the mined-rule confidence above 0.85 and is expected to
  surface the first pr-miner TP without any detector-side
  change.

The overall recall_upper_bound dropped from 0.28 (batch 7) to
0.27 (batch 8). The move is downward because `pr-miner` entered
the denominator with one FN by detector design at v0 corpus
density (Apriori mining sparsity, not extractor scope); none of
the existing five detectors regressed. Closing the 1.00 gap on
pr-miner is a corpus-density problem (more paired open/close
transactions) rather than an extractor-widening problem;
documenting that the gap is density-bound and not scope-bound
is exactly the kind of signal Heckman & Williams IST 2011's
selection-bias warning motivates the audit harness to surface.
The earlier 0.30 → 0.28 drop at batch 7 came from the `codeql`
source kind contributing five FN entries on bug shapes outside
cntrdct's spec F3 terminator set (constant-condition branches
and typed-exception unreachability) plus one TP on the Python
`return` → following-statement pattern F3 already catches.
Closing the 0.76 gap on `unreachable-after-terminator` requires
F3 widening to constant-condition / branch / exception-typed
reasoning (separate engineering with its own preregistration),
not audit-harness work.

Future batches deepen coverage on the existing six detectors
rather than introducing a seventh detector or a seventh source
kind. The most tractable next step is additional
`open-never-closed` instances on permissive-licensed Python
files to push pr-miner's mined-rule confidence above the 0.85
threshold and surface the first pr-miner TP.

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
