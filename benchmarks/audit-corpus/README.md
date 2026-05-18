# cntrdct external-source audit corpus

Q-14 deliverable from `ROADMAP.md`. Houses the corpus
`cntrdct calibrate --audit-recall` runs against to report
per-detector recall upper bounds. Spec:
[`docs/spec/recall-audit-v0.md`](../../docs/spec/recall-audit-v0.md).

Status: Phase B closed at batch 32 (2026-05-18, v0.3.0). Sixty-one
expected entries across forty-nine files, six detectors, and six
external source kinds (`rustc-lint-testset`, `github-commit`,
`paper-appendix`, `clippy`, `codeql`, `semgrep`). All six cntrdct
detectors are represented; `comment-code` saturates all three Tan
SOSP 2007 patterns (Pattern A on three upstreams, Pattern B on
three upstreams, Pattern C on seventeen upstreams across
twenty-three permissive-licensed domains); `pr-miner` reports
`2/0/1.00` with mined `{open} -> {close}` confidence above the 0.85
Apriori threshold. Overall `recall_upper_bound` = 0.66 raw 0.6557.
The four detectors still at 0.00 (`arg-swap`, `clone-drift`,
`config-interaction`, `unreachable-after-terminator`) surface v0
scope choices rather than detector regressions; closing those gaps
is detector-side engineering under separate preregistrations,
outside Phase B. The `nvd` and `osv` slots documented under "Source
list" remain unused — CVE-shaped bugs that map to a cntrdct
detector are rare in practice. "Latest audit run" below refreshes
on each release tag per the Refresh discipline section.

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
├── README.md       (this file)
├── manifest.jsonl  (canonical record of expected entries and batch provenance)
└── files/
    ├── anycode_decode_project_path.rs
    ├── apache_ranger_tagsync_setup.py
    ├── ars0n_fire_scanner.py
    ├── azalea_legacy_blocks_motion.rs
    ├── boundless_default_registry.rs
    ├── carla_import.py
    ├── clippy_ui_branches_sharing_code_shared_at_top.rs
    ├── clippy_ui_if_same_then_else.rs
    ├── codeql_python_unreachable_test.py
    ├── django_mobile_setup.py
    ├── django_secure_conf.py
    ├── django_secure_setup.py
    ├── glium_draw_parameters_validate.rs
    ├── lakefs_internal_delete_gc_rules.rs
    ├── lsvine_transform_readdir.rs
    ├── move_native_destroy_signer.rs
    ├── nbrmd_test_ipynb_to_R.py
    ├── nono_warn_for_deprecated_flags.rs
    ├── parking_lot_core_unpark.rs
    ├── pkg_config_rs_find_library.rs
    ├── pytago_fileloop.py
    ├── rarfile_set_attrs.py
    ├── readur_test_app_state_legacy.rs
    ├── reflex_find_ruby_gem_names.rs
    ├── rosrust_genaction.py
    ├── rust_s3_set_retries.rs
    ├── rustc_ui_both_true_false.rs
    ├── rustc_ui_expr_block.rs
    ├── rustc_ui_expr_call.rs
    ├── rustc_ui_expr_if.rs
    ├── rustc_ui_expr_loop.rs
    ├── rustc_ui_expr_return.rs
    ├── rustc_ui_unreachable_code_ret.rs
    ├── rusticata_tls_handshake_next_protocol.rs
    ├── sanic_prometheus_release.py
    ├── smolvm_export_layer.rs
    ├── sui_mysten_metrics_channel.rs
    ├── teensycore_write_byte.rs
    ├── telluric_fitter_setup.py
    ├── tera_docker_setup.py
    ├── totalsegmentator_statistics.py
    ├── tugraph_det_ver.py
    ├── unv_app_settings.py
    ├── vcpkg_rs_probe_package.rs
    ├── vortex_buffer_get_bit.rs
    ├── vst2_process_deprecated.rs
    ├── wasmtime_fuzz_roundtrip.rs
    ├── whisky_archive_constructors.rs
    └── zarrs_bitround_round_bytes.rs
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
      "tp": 34,
      "fn": 0,
      "recall_upper_bound": 1.0,
      "source_breakdown": { "github-commit": { "tp": 34, "fn": 0 } }
    }
  },
  "overall": { "tp": 40, "fn": 21, "recall_upper_bound": 0.6557, "source_breakdown": { /* aggregated */ } },
  "corpus_size": 49,
  "expected_total": 61,
  "sources": { "clippy": 2, "codeql": 6, "github-commit": 35, "paper-appendix": 3, "rustc-lint-testset": 13, "semgrep": 2 }
}
```

## Latest audit run

Refreshed 2026-05-18 against `v0.3.0` per the Refresh discipline
section below.

| detector                       | tp | fn | recall_upper_bound | dominant source       |
| ------------------------------ | -- | -- | ------------------ | --------------------- |
| `arg-swap`                     | 0  | 4  | 0.00               | `paper-appendix`      |
| `clone-drift`                  | 0  | 2  | 0.00               | `clippy`              |
| `comment-code`                 | 34 | 0  | 1.00               | `github-commit`       |
| `config-interaction`           | 0  | 2  | 0.00               | `rustc-lint-testset`  |
| `pr-miner`                     | 2  | 0  | 1.00               | `semgrep`             |
| `unreachable-after-terminator` | 4  | 13 | 0.24 (raw 0.2353)  | `rustc-lint-testset`  |
| overall                        | 40 | 21 | 0.66 (raw 0.6557)  | mixed                 |

Corpus: 49 files, 61 expected entries. Source mix:
`github-commit` 35, `rustc-lint-testset` 13, `codeql` 6,
`paper-appendix` 3, `clippy` 2, `semgrep` 2.

Reading the figures: the four detectors at 0.00 / 0.24 surface v0
scope choices rather than measurement bugs. `arg-swap` is
constrained by the narrow Rice-2017 scope; `clone-drift` by spec F2
top-level `fn` granularity; `config-interaction` by spec F5
requiring an explicit `not(...)` wrapper; `unreachable-after-
terminator` by spec F3 not lifting constant-condition / typed-
exception reasoning. Lifting any of these is detector-side
engineering under a separate preregistration, not a Phase B item.
`comment-code` reports `34/0/1.00` across all three Tan SOSP 2007
patterns (Pattern A on three upstreams, Pattern B on three
upstreams, Pattern C on seventeen upstreams). `pr-miner` reports
`2/0/1.00` once the corpus-wide `{open} -> {close}` mining-DB
confidence sits above the 0.85 Apriori threshold (10 paired
open+close density-support files keep the margin).

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
