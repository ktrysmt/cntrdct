# cntrdct external-source audit corpus

Q-14 deliverable from `ROADMAP.md`. Houses the corpus
`cntrdct calibrate --audit-recall` runs against to report
per-detector recall upper bounds. Spec:
[`docs/spec/recall-audit-v0.md`](../../docs/spec/recall-audit-v0.md).

Status: Phase B batch 14 (2026-05-15). Forty-one expected
entries across six detectors and six external source kinds
(`rustc-lint-testset`, `github-commit`, `paper-appendix`,
`clippy`, `codeql`, `semgrep`). Batch 14 shifts `comment-code`
audit coverage from Pattern C only (deprecated prose without
`#[deprecated]` attribute, batches 3 / 11 / 12 / 13) to
Pattern B + Pattern C by adding six TPs from a fifth
permissive-licensed Rust upstream (zarrs/zarrs
`zarrs_data_type/src/codec_traits/bitround.rs`,
MIT OR Apache-2.0). Each of `round_bytes_int16` /
`round_bytes_int32` / `round_bytes_int64` /
`round_bytes_float16` / `round_bytes_float32` /
`round_bytes_float64` carries a `///` doc block whose
`# Panics` section reads `Panics if \`bytes.len()\` is not a
multiple of N.` for N in {2, 4, 8}, but the body uses
`bytes.as_chunks_mut::<N>().0` which never panics on
non-multiple-of-N lengths — `slice::as_chunks_mut::<N>`
(stabilised in Rust 1.88.0, 2025-06-26) returns
`(&mut [[T; N]], &mut [T])` where the second tuple element is
the remainder with length strictly less than N; the upstream
code accesses only `.0` and silently discards the remainder,
so trailing bytes that don't form a complete N-byte chunk are
ignored rather than triggering the documented panic. The
textbook Tan SOSP 2007 §3.2 Pattern B ("bad comment": panic
claim without panicking constructs in the body) bug shape,
previously not exercised by the audit corpus. After batch 14
the `comment-code` detector's audit evidence spans two of the
three `docs/spec/comment-code-v0.md` patterns (Pattern A —
Result / Option claim without matching return type — still
owed) on five upstreams across five unrelated domains
(whisky-archive Cardano Plutus-data helpers 4 + tls-parser
TLS NextProtocol parsers 2 + glium OpenGL draw-parameter
check 1 + pkg-config-rs build-tool / system-package bindings 1
+ zarrs Zarr-format data-type bindings 6). The source-kind
footprint stays at six (`github-commit` absorbs the new
entries; no new kind invented). Batch 11 (the immediately
preceding batch 2026-05-13) added two `comment-code` TPs from
the second permissive-licensed Rust upstream
(rusticata/tls-parser `src/tls_handshake.rs`, MIT OR Apache-2.0):
both `pub fn parse_tls_handshake_next_protocol(...)` and
`pub fn parse_tls_handshake_msg_next_protocol(...)` carry a
`///` doc block ending in `Deprecated in favour of ALPN.`
without the `#[deprecated]` attribute. Batch 10
(2026-05-13) flipped the `pr-miner` detector from
`0 TP / 2 FN / 0.00` (batch 9) to `2 TP / 0 FN / 1.00` by
adding ten density-support files that lifted the corpus-wide
`{open} -> {close}` mined-rule confidence above the spec F3
`MIN_CONFIDENCE = 0.85` threshold. Each density-support file
ships with `expected: []` rather than a labelled finding because
the Semgrep `open-never-closed` rule produces no findings on
files where every top-level `def` that calls `open` also
explicitly calls `close` (or closes via try/finally). The ten
files contribute eighteen paired open+close top-level
transactions to pr-miner's spec F3 Apriori mining database
(MIT / BSD-3-Clause / Apache-2.0 permissive Python: carla
Util/Tools/Import.py, Telluric-Fitter setup.py, baidu/tera
tera_setup.py, nottheswimmer/pytago examples/fileloop.py,
dkruchinin/sanic-prometheus scripts/release.py, carljm/django-
secure setup.py and doc/conf.py, adnanademovic/rosrust
genaction.py, R-s0n/ars0n-framework fire-scanner.py, apache/ranger
tagsync setup.py). Math: before batch 10 the corpus had four
top-level Python defs containing `open` (`get_ver`,
`replace_ver`, `readfile`, `test_identity_source_write_read`)
with only `replace_ver` containing `close` too, giving
`{open} -> {close}` confidence of 1/4 = 0.25 — far below the
threshold. Batch 10 adds 18 paired transactions to both numerator
and denominator, pushing the confidence to (1+18)/(4+18) = 19/22
≈ 0.864 ≥ 0.85, after which spec F3 mines the rule, spec F4
scans the full transaction set for violations, and both batch-8
`get_ver` and batch-9 `readfile` flip from FN to TP without any
detector-side change. Batch 9 (the immediately preceding batch
2026-05-13) deepened the `pr-miner` denominator on the existing
`semgrep` source kind via a second permissive-licensed
`open-never-closed` instance: the same Semgrep registry rule
(`python.lang.best-practice.open-never-closed.open-never-closed`,
pinned at semgrep/semgrep-rules@9d73d08e) applied to
gregmuellegger/django-mobile@fafc3890 `setup.py` (BSD-3-Clause).
The labelled bug is `return open(filename, ...).read()` inside
top-level `def readfile(filename):` (upstream line 13 / corpus
line 17) — both branches of the `sys.version_info` check return
an `open(...).read()` chain without any matching `close()`,
`with`, or try-finally; the companion top-level `def get_author`
and `def get_version` delegate file reading to `readfile` and do
not call `open` directly, so the rule does not fire on them, and
the `UltraMagicString` class methods are excluded from pr-miner's
spec F2 extractor by design (only top-level
`function_definition` / `decorated_definition` are walked).
Mapping against cntrdct's pr-miner: F2 reaches `readfile`
cleanly and produces item set `{open, read}`, but spec F3
Apriori mining at `MIN_SUPPORT = 0.05` / `MIN_CONFIDENCE = 0.85`
still cannot synthesise the `{open} -> {close}` rule — with
batch 9's transaction added, three corpus-wide functions contain
`open` (`get_ver` and `readfile` open-only, `replace_ver`
open+close) and only one of those contains `close`, so the rule
confidence is 1/3 ≈ 33%, far below the 0.85 threshold. Selecting
django-mobile rather than semgrep-rules' own test fixtures keeps
batch 9 outside the Semgrep Rules License v1.0 carve-out the
same way batch 8 did with Apache-2.0 tugraph-db. Overall
`recall_upper_bound` jumps from 0.26 at batch 9 to 0.32 at batch
10 — driven entirely by `pr-miner` flipping from `0/2/0.00` to
`2/0/1.00` once Apriori mining crosses the 0.85 confidence gate;
the other five detectors are unchanged because their findings
depend only on per-file content (not on cross-file mining), and
the density-support files carry no expected entries. Batch 8 (2026-05-13) introduced
the `semgrep` source kind alongside the sixth and final cntrdct
detector, `pr-miner`, via the same Semgrep registry rule applied
to TuGraph-family/tugraph-db@672e4b19 `release/det_ver.py`
(Apache-2.0): `f = open('Options.cmake','r')` inside top-level
`def get_ver():` (upstream line 5 / corpus line 9) without any
matching `close()`, `with`, or try-finally; the companion
`def replace_ver(...)` in the same file opens AND closes, so the
rule does not fire on it. Batch 7 (2026-05-12): the `codeql` source kind via
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
`clippy`, `codeql`, `semgrep`). Batch 10 (2026-05-13) closed
pr-miner's mined-rule confidence gap by adding ten density-
support files containing eighteen paired open+close top-level
Python transactions, lifting the {open} -> {close} confidence
from 0.25 to 0.864 and flipping both labelled `pr-miner` FN
entries (batch-8 `get_ver`, batch-9 `readfile`) to TPs without
any detector-side change. Future Phase B batches deepen coverage
on the remaining 0.00 detectors via either more labelled
findings on the existing source kinds, or detector-side scope
lifts under separate preregistrations: broader
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
├── manifest.jsonl                  (Phase B batches 1-14)
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
    ├── tugraph_det_ver.py                             (batch 8)
    ├── django_mobile_setup.py                         (batch 9)
    ├── tera_docker_setup.py                           (batch 10, density)
    ├── pytago_fileloop.py                             (batch 10, density)
    ├── sanic_prometheus_release.py                    (batch 10, density)
    ├── django_secure_setup.py                         (batch 10, density)
    ├── django_secure_conf.py                          (batch 10, density)
    ├── rosrust_genaction.py                           (batch 10, density)
    ├── carla_import.py                                (batch 10, density)
    ├── telluric_fitter_setup.py                       (batch 10, density)
    ├── ars0n_fire_scanner.py                          (batch 10, density)
    ├── apache_ranger_tagsync_setup.py                 (batch 10, density)
    ├── rusticata_tls_handshake_next_protocol.rs      (batch 11)
    ├── glium_draw_parameters_validate.rs              (batch 12)
    ├── pkg_config_rs_find_library.rs                  (batch 13)
    └── zarrs_bitround_round_bytes.rs                  (batch 14)
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
      "tp": 14,
      "fn": 0,
      "recall_upper_bound": 1.0,
      "source_breakdown": { "github-commit": { "tp": 14, "fn": 0 } }
    }
  },
  "overall": { "tp": 20, "fn": 21, "recall_upper_bound": 0.49, "source_breakdown": { /* aggregated */ } },
  "corpus_size": 31,
  "expected_total": 41,
  "sources": { "clippy": 2, "codeql": 6, "github-commit": 15, "paper-appendix": 3, "rustc-lint-testset": 13, "semgrep": 2 }
}
```

## Latest audit run

Refreshed 2026-05-15 against `v0.2.0-rc.17` per the Q-14 Phase C
discipline. The figures match the batch-14 mid-cycle snapshot
bit-for-bit (no detector or audit-corpus change since the same
day), so this refresh is a no-op for the figures but keeps the
assertion live, the same way CI runs the rest of the suite on
every push.
Batch 14 shifts `comment-code` audit coverage from
single-pattern (Pattern C only, batches 3 / 11 / 12 / 13) to
two-pattern (Pattern B + Pattern C) by adding six TPs from a
fifth permissive-licensed Rust upstream — zarrs/zarrs@3b944c57a0b7af127ae73ea250d3ffce60e51f0b
`zarrs_data_type/src/codec_traits/bitround.rs` (MIT OR Apache-2.0).
Six top-level `pub fn` items —
`round_bytes_int16` (upstream line 58 / corpus line 42),
`round_bytes_int32` (upstream 70 / corpus 54),
`round_bytes_int64` (upstream 82 / corpus 66),
`round_bytes_float16` (upstream 94 / corpus 78),
`round_bytes_float32` (upstream 106 / corpus 90), and
`round_bytes_float64` (upstream 118 / corpus 102) — each carry
a `///` doc block with a `# Panics\nPanics if \`bytes.len()\` is
not a multiple of N.` section for N in {2, 4, 8}, but the body
uses `bytes.as_chunks_mut::<N>().0` which does NOT panic on
non-multiple-of-N lengths. `slice::as_chunks_mut::<N>`
(stabilised in Rust 1.88.0, 2025-06-26) returns
`(&mut [[T; N]], &mut [T])`; the upstream code accesses only
`.0` (the chunked arrays) and silently discards the remainder,
so trailing bytes that don't form a complete N-byte chunk are
ignored rather than triggering the documented panic — the
textbook Tan SOSP 2007 §3.2 Pattern B ("bad comment": panic
claim without panicking constructs in the body) bug shape,
previously not exercised by the audit corpus. cntrdct's spec F4
trigger (rendered doc string contains `panic` substring after
`to_lowercase`) fires on each function; the body substring
check finds none of `panic!`, `unwrap`, `expect(`,
`unreachable!`, `assert!`, `assert_eq!`, `assert_ne!`, `todo!`,
`unimplemented!`, `debug_assert` in the
`for chunk in bytes.as_chunks_mut::<N>().0 { ... }` body (the
`assert!(N != 0)` inside `slice::as_chunks_mut` lives in std,
not the call-site body, so the substring check sees no marker).
All six entries are TP, taking the detector's audit evidence
from four upstreams (batches 3 / 11 / 12 / 13, all Pattern C)
to five upstreams across five unrelated domains
(whisky-archive Cardano Plutus-data helpers 4 + tls-parser TLS
NextProtocol parsers 2 + glium OpenGL draw-parameter check 1
+ pkg-config-rs build-tool / system-package bindings 1 + zarrs
Zarr-format data-type bindings 6) AND from single-pattern
(Pattern C) to two-pattern (Pattern B + Pattern C) coverage —
a regression that broke Pattern B but preserved Pattern C
would now surface in the audit rather than going undetected.
Pattern A coverage (Result / Option claim without matching
return type) remains owed; v0 already detects Pattern A but
the audit corpus has not yet exercised it. The source-kind
footprint stays at six (`github-commit` absorbs the six new
entries; batch 14 does not introduce a new kind). Batch 13
(2026-05-14) added one `comment-code` TP from
rust-lang/pkg-config-rs@f36d32a09824a6b2c18475c8a4b7df1cb2c50c95
`src/lib.rs` (MIT OR Apache-2.0).
`pub fn find_library(name: &str) -> Result<Library, String>` at
upstream line 453 (corpus line 7) carries a single-line `///`
doc block reading `Deprecated in favor of the probe_library
function` and a `#[doc(hidden)]` attribute, but does not carry
the `#[deprecated]` runtime attribute the Rust deprecation
lints honour — Pattern C correctly distinguishes the
visibility-hiding `#[doc(hidden)]` (first identifier `doc`)
from the runtime-lint-enforcing `#[deprecated]` (the actual
fix the maintainers would need to apply for downstream
consumers to receive a compiler warning). Batch 12
(2026-05-13) added one `comment-code` TP from
glium/glium@8d6fd34d9171172928771657fc5c9103107ff978
`src/draw_parameters/mod.rs` (Apache-2.0):
`pub fn validate(...)` at upstream line 531 (corpus line 6)
carries a single-line `///` doc block reading `DEPRECATED.
Checks parameters and returns an error if something is wrong.`
without a matching `#[deprecated]` runtime attribute. Batch 11
(2026-05-13) added two `comment-code` TPs from
rusticata/tls-parser@6554155918278531370e7d0addbd5d759e3a4cc9
`src/tls_handshake.rs` (MIT OR Apache-2.0): both
`pub fn parse_tls_handshake_next_protocol(...)` (upstream line
850 / corpus line 10) and
`pub fn parse_tls_handshake_msg_next_protocol(...)` (upstream
line 865 / corpus line 25) carry a `///` doc block ending in
`Deprecated in favour of ALPN.` without a matching
`#[deprecated]` runtime attribute. Batch 10 (also 2026-05-13)
surfaced the first `pr-miner` true positives by adding ten
density-support files containing eighteen paired open+close
top-level Python defs. Overall `recall_upper_bound` moved from
0.26 at batch 9 to 0.32 at batch 10 (driven entirely by
`pr-miner` flipping from `0/2/0.00` to `2/0/1.00`), to 0.36 at
batch 11 (driven by `comment-code` adding two TPs on a new
upstream without retreating the existing detectors), to 0.38
at batch 12 (driven by `comment-code` adding one more TP on a
third upstream), to 0.40 at batch 13 (driven by `comment-code`
adding one more TP on a fourth upstream), and now to 0.49 at
batch 14 (driven by `comment-code` adding six TPs on a fifth
upstream that also extends pattern coverage from Pattern C
only to Pattern B + Pattern C). Batch 14's pr-miner mining
margin is preserved because the new file is Rust — Python
`{open} → {close}` confidence stays at 19/22 ≈ 0.864 ≥ 0.85,
and both pr-miner TPs (`get_ver`, `readfile`) remain TPs.

| detector                       | tp | fn | recall upper bound | dominant source                          |
| ------------------------------ | --:| --:| ------------------:| ---------------------------------------- |
| `arg-swap`                     |  0 |  4 |               0.00 | `paper-appendix` (3/4 entries)           |
| `clone-drift`                  |  0 |  2 |               0.00 | `clippy` (2/2 entries)                   |
| `comment-code`                 | 14 |  0 |               1.00 | `github-commit` (14/14 entries)          |
| `config-interaction`           |  0 |  2 |               0.00 | `rustc-lint-testset` (2/2)               |
| `pr-miner`                     |  2 |  0 |               1.00 | `semgrep` (2/2 entries)                  |
| `unreachable-after-terminator` |  4 | 13 |               0.24 | `rustc-lint-testset` (11/17 entries)     |
| **overall**                    | 20 | 21 |               0.49 |                                          |

Corpus size: 31 files (10 of which are batch-10 density-support
files with `expected: []` — they do not enter the recall
denominator). Expected entries: 41. Source mix:
`github-commit` (15 entries), `rustc-lint-testset` (13),
`codeql` (6), `paper-appendix` (3), `clippy` (2), `semgrep`
(2).

Reading the figures:

- The fourteen `comment-code` true positives split across five
  permissive-licensed upstreams and two of the three
  `docs/spec/comment-code-v0.md` patterns: eight Pattern C
  entries — four from `sidan-lab/whisky-archive@99243766`
  (Cardano Plutus-data helper, batch 3), two from
  `rusticata/tls-parser@65541559` (TLS-parser crate, batch 11),
  one from `glium/glium@8d6fd34d` (OpenGL bindings, batch 12),
  and one from `rust-lang/pkg-config-rs@f36d32a0` (build-tool /
  system-package bindings, batch 13) — and six Pattern B
  entries from `zarrs/zarrs@3b944c57` (Zarr-format data-type
  bindings, batch 14). The eight Pattern C entries share the
  textbook Tan SOSP 2007 §3.2 Pattern C ("bad comment") shape:
  a `///` doc block whose rendered prose contains `deprecated`
  (case-folded) without a matching `#[deprecated]` runtime
  attribute on the same top-level `function_item`. The
  whisky-archive entries are the
  `con_str` / `con_str0` / `con_str1` / `con_str2` family
  (constructors superseded by `constr` / `constr0` / etc.); the
  tls-parser entries are `parse_tls_handshake_next_protocol`
  and `parse_tls_handshake_msg_next_protocol` (NextProtocol
  parsers superseded by ALPN); the glium entry is `validate`
  (draw-parameter check superseded by glium's draw-time
  validation); the pkg-config-rs entry is `find_library`
  (string-error helper superseded by `probe_library`'s typed
  `Error`). The pkg-config-rs entry is also the first audit
  evidence that Pattern C's
  `preceding_siblings_have_deprecated` walker correctly skips
  the orthogonal `#[doc(hidden)]` attribute_item (first
  identifier `doc`) and continues looking for `#[deprecated]`
  — the two attributes serve different purposes (visibility
  hiding vs. runtime lint enforcement), and Pattern C
  distinguishes them by literal identifier match. The six
  Pattern B entries from zarrs are
  `round_bytes_int16` / `round_bytes_int32` /
  `round_bytes_int64` / `round_bytes_float16` /
  `round_bytes_float32` / `round_bytes_float64`, each carrying
  a `///` doc block whose `# Panics` section claims a panic on
  non-multiple-of-N length but whose body uses
  `bytes.as_chunks_mut::<N>().0` (which silently drops the
  trailing-remainder slice rather than panicking) and has no
  panicking constructs (no `panic!` / `unwrap` / `expect(` /
  `unreachable!` / `assert!` / `assert_eq!` / `assert_ne!` /
  `todo!` / `unimplemented!` / `debug_assert` in the body
  source text). The Pattern B / Pattern C split surfaces the
  first audit signal that breaking either pattern would not be
  masked by the other — a regression that loosens Pattern B
  while Pattern C still fires (or vice versa) would now reduce
  the corresponding TP count rather than going undetected.
  cntrdct's Pattern B (`docs/spec/comment-code-v0.md` F4) and
  Pattern C (F5) both fire on the syntactic surface, and the
  recall_upper_bound of 1.00 reflects that both bug-classes
  within cntrdct's detection scope (top-level `fn` items) are
  captured cleanly. The five-upstream / two-pattern coverage on
  five unrelated domains (Cardano Plutus-data helpers, TLS
  parser, OpenGL bindings, build-tool / system-package bindings,
  Zarr-format data-type bindings) reduces the single-upstream
  and single-pattern regression-detection risk that
  batch 11 / 12 / 13 / 14 already broke. Pattern A (Result /
  Option claim without matching return type) remains owed; v0
  already detects Pattern A but the audit corpus has not yet
  exercised it.
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

- The two `pr-miner` true positives are both labelled by the
  same Semgrep registry rule `open-never-closed` and partition
  by source file:
  - `tugraph_det_ver.py` (batch 8, `semgrep`) — `f = open('Options.cmake','r')`
    inside `def get_ver():` at corpus line 9. Semgrep fires
    because no `close()` / `with` / try-finally reaches the file
    handle before the function returns. The companion top-level
    `def replace_ver(...)` in the same file opens AND closes
    (corpus lines 22 and 33), so the rule does not fire on it.
  - `django_mobile_setup.py` (batch 9, `semgrep`) —
    `return open(filename, ...).read()` inside
    `def readfile(filename):` at corpus line 17. Both branches
    of `if sys.version_info[0] >= 3:` return the same
    `open(...).read()` chain without any matching close /
    with / try-finally, so the open file handle is dropped on
    return. The companion `def get_author` / `def get_version`
    delegate file reading to `readfile` and do not call `open`
    directly, so the rule does not fire on them; the
    `UltraMagicString` class methods are out of scope for
    pr-miner's spec F2 extractor (only top-level
    `function_definition` / `decorated_definition` are walked).
  cntrdct's pr-miner reaches both functions (spec F2 extracts
  top-level `function_definition` items) and at batch-10 corpus
  density mines the `{open} → {close}` rule under spec F3's
  Apriori pass: `MIN_SUPPORT = 0.05` is trivially satisfied
  (19 supporting transactions / 28+ total mining-DB
  transactions = 0.68); `MIN_CONFIDENCE = 0.85` is just crossed
  (19 / 22 = 0.864). With the rule mined, spec F4 scans the
  full transaction set and flags every top-level def that has
  `open` and lacks `close`. Both `get_ver` and `readfile` match,
  along with `test_identity_source_write_read` from
  `nbrmd_test_ipynb_to_R.py` — the third match is an
  unmatched-actual finding (not labelled in any expected[],
  hence not counted in recall; pr-miner cannot distinguish
  context-managed `with open` from plain open in v0, so this is
  a documented FP under the spec's no-`with`-recognition scope).
  Before batch 10 the corpus density was too sparse (1
  open+close transaction against 4 open-using transactions =
  0.25 confidence, far below 0.85), so the rule was never mined
  and F4 was never evaluated against either labelled FN. Batch
  10's ten density-support files (eighteen paired open+close
  top-level transactions across permissive MIT / BSD-3-Clause /
  Apache-2.0 Python files) lift confidence to 0.864 and surface
  both FNs as TPs without any detector-side change — exactly
  the "more paired open/close transactions" path the batch-8
  and batch-9 documentation telegraphed.

The overall recall_upper_bound moved from 0.26 (batch 9) to
0.32 (batch 10), then to 0.36 (batch 11), then to 0.38 (batch
12), and now to 0.40 (batch 13). The batch-10 move was upward
because `pr-miner` flipped from `0/2/0.00` to `2/0/1.00` once
batch 10's eighteen paired open+close transactions lifted the
mined-rule confidence over the 0.85 threshold; none of the
other five detectors changed because their findings depend
only on per-file content (not on cross-file mining), and the
ten density-support files added by batch 10 all ship with
`expected: []` (the Semgrep labeller produces no findings on
them, so they contribute to corpus density without inflating
the recall denominator). The batch-11 move was upward because
`comment-code` added two TPs from a second permissive-licensed
Rust upstream (rusticata/tls-parser, MIT OR Apache-2.0)
covering the same Tan SOSP 2007 Pattern C bug shape on an
unrelated domain — the diversification broke the
single-upstream dependence the batch-3 `whisky-archive`
entries had, so a regression triggered by a
whisky-archive-specific quirk would no longer go undetected.
The batch-12 move was upward for the same reason on a third
independent upstream — `comment-code` adds one TP from
glium/glium@8d6fd34d (Apache-2.0), an OpenGL bindings crate
unrelated to either Cardano Plutus-data helpers or TLS
parsing. The batch-13 move is upward for the same reason on a
fourth independent upstream — `comment-code` adds one TP from
rust-lang/pkg-config-rs@f36d32a0 (MIT OR Apache-2.0), a
build-tool / system-package wrapper unrelated to any of the
prior three domains. Batch 13 also preserves the pr-miner
mining margin because the new file is Rust (Python
`{open} → {close}` confidence stays at batch-10's 19/22 ≈
0.864 ≥ 0.85, and both pr-miner TPs remain TPs). Closing the 1.00 gap on pr-miner
was a corpus-density problem (more paired open/close
transactions) rather than an extractor-widening problem;
documenting that the gap is density-bound and not scope-bound
was exactly the kind of signal Heckman & Williams IST 2011's
selection-bias warning motivates the audit harness to surface,
and batch 10 closed that gap on a labeller-bias-safe substrate
(every paired transaction is sourced from a different upstream
than the labelled bugs, so the audit-corpus does not
self-confirm). The earlier 0.30 → 0.28 drop at batch 7 came
from the `codeql` source kind contributing five FN entries on
bug shapes outside cntrdct's spec F3 terminator set
(constant-condition branches and typed-exception unreachability)
plus one TP on the Python `return` → following-statement pattern
F3 already catches. Closing the 0.76 gap on
`unreachable-after-terminator` requires F3 widening to
constant-condition / branch / exception-typed reasoning
(separate engineering with its own preregistration), not
audit-harness work.

Future batches deepen coverage on the existing six detectors
rather than introducing a seventh detector or a seventh source
kind. pr-miner now reports `2/0/1.00` on the corpus and the
numerator-construction phase is closed; the next pr-miner move
shifts to broadening the labeller side — additional
`open-never-closed` instances (or other pr-miner-mappable
patterns) on permissive Python files would now contribute as
TPs (since the rule is mined) rather than FNs-by-sparsity,
provided each new instance comes with sufficient paired open+
close density-support to keep the mined-rule confidence above
the 0.85 threshold (every open-only addition to the Python
mining DB lowers the ratio, so a labelled FN must be
accompanied by ≥ 4 paired density-support transactions to
maintain the margin). The remaining 0.00 detectors (`arg-swap`,
`clone-drift`, `config-interaction`) still need detector-side
scope lifts under separate preregistrations, and
`unreachable-after-terminator` needs F3 widening to constant-
condition / exception-typed reasoning to close its 0.76 gap.
`comment-code` is now at 1.00 on five upstreams across two of
the three `docs/spec/comment-code-v0.md` patterns (Pattern B
from zarrs batch 14, Pattern C from batches 3 / 11 / 12 / 13),
so further batches on this detector would either add a sixth
upstream (diminishing-returns regression-detection insurance
for the existing two patterns) or shift to Pattern A coverage
(Result / Option claim without matching return type) which v0
already detects but the audit corpus has not yet exercised.

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
