# cntrdct implementation roadmap

Last updated: 2026-05-17 (Q-14 recall-audit Phase B batch 24
landed on top of batch 23, diversifying `comment-code` Pattern
C audit coverage from eight upstreams (whisky-archive +
tls-parser + glium + pkg-config-rs + sui mysten-metrics +
vcpkg-rs + rust-vst2 + nono, batches 3 / 11 / 12 / 13 / 20 /
21 / 22 / 23) to nine upstreams by adding one `comment-code`
Pattern C TP from a fifteenth permissive-licensed Rust
upstream smol-machines/smolvm@019654bd61d9051fc42df7bbcdee446cc3e31ae1
`crates/smolvm-agent/src/storage.rs` (Apache-2.0). One
top-level `pub fn` item — `export_layer` (upstream line 1418 /
corpus line 10) — carries a four-line `///` doc block whose
later half (after a blank `///` separator) reads `DEPRECATED:
Prefer streaming export via `find_layer_path()` + piped tar.
This function creates a temp tar file that can fill the
storage disk for large layers. Kept for backward compatibility.`
but does not carry the `#[deprecated]` runtime attribute the
Rust deprecation lints honour, so downstream consumers receive
no compiler warning — the textbook Tan SOSP 2007 §3.2 Pattern
C bug shape. cntrdct's spec F5 Pattern C trigger fires on the
case-folded `deprecated` substring; the preceding-siblings
walker finds zero attribute items adjacent to the function so
the `#[deprecated]` lint is not honoured. The function body
calls `find_layer_path(image_digest, layer_index)?` to locate
the layer source directory, builds a temporary tar path under
`STORAGE_ROOT/tmp/`, spawns `tar -cf <tar_path> -C <layer_dir>
.` via `Command::new("tar")`, and returns the resulting tar
`PathBuf` — the in-tree-body sub-shape, the same body-shape
category as batch-11 tls-parser, batch-12 glium, and batch-20
sui mysten-metrics (in-tree implementation rather than
delegation to the replacement) on a fourth unrelated upstream.
This contrasts prior body shapes that delegate to the
replacement (batch-3 whisky-archive con_str → constr, batch-13
pkg-config-rs find_library → probe_library, batch-21 vcpkg-rs
probe_package → find_package), the batch-22 rust-vst2 stub-body
shape (empty `{ }` ABI placeholder), and the batch-23 nono
meta-deprecation-warning-emitter sub-shape. cntrdct's spec F5
Pattern C check ignores the body — only the doc and adjacent
attributes are inspected — so the new in-tree case fires
identically to the prior in-tree, delegate-body, stub-body,
and meta-warning-emitter cases, confirming the syntactic-only
design. The body-shape footprint within Pattern C therefore
stays at the four shapes saturated by batches 22 and 23
(delegate-body, in-tree-body, stub-body,
meta-deprecation-warning-emitter); batch 24 broadens
in-tree-body audit coverage rather than introducing a new
sub-shape. smolvm is the portable, lightweight,
self-contained VM image storage domain (the agent crate's
storage module manages OCI-style image layers on disk for the
VM runtime), unrelated to the prior eight Pattern C domains.
The function signature `pub fn export_layer(image_digest:
&str, layer_index: usize) -> Result<PathBuf>` returns
`Result<PathBuf>` so spec F3 Pattern A's return-type negation
does not pass, but the doc contains no Pattern A trigger
phrase (none of `returns err` / `returns result` / `may fail`
/ `fallible` / `returns option` / `may return none`) so
Pattern A does not fire either way; the doc contains no
`panic` substring so spec F4 Pattern B does not fire — only
Pattern C fires. The entry is TP. After batch 24 the
detector's audit evidence spans fifteen upstreams across
fifteen unrelated domains (Cardano Plutus-data helpers 4
Pattern C + TLS parser 2 Pattern C + OpenGL bindings 1
Pattern C + Unix pkg-config bindings 1 Pattern C + Zarr-format
data-type bindings 6 Pattern B + zkVM executor registry 1
Pattern A + parking_lot_core synchronization primitives 2
Pattern B + cranelift-assembler-x64 fuzzer infrastructure 1
Pattern A + S3-client configuration setter 1 Pattern A +
vortex-buffer bit-packed bitmap helpers 1 Pattern B + sui
mysten-metrics async-channel metrics wrapper 2 Pattern C +
Windows vcpkg bindings 1 Pattern C + VST 2.4 audio plugin host
1 Pattern C + capability-based sandbox CLI 1 Pattern C +
portable lightweight VM image layer storage 1 Pattern C), with
Pattern A and Pattern B saturated at three sub-shapes on three
upstreams each (batches 18 and 19) and Pattern C now exercised
on nine unrelated upstreams (lifted from eight at batch 23).
The source-kind footprint stays at six (`github-commit`
absorbs the new entry). Updated corpus 41 files / 53 expected
entries / overall recall_upper_bound 0.60 (32 TP / 21 FN, raw
0.6038 vs. 0.5962 at batch 23 — both round to 0.60 to two
decimal places); `comment-code` moves to tp=26 / fn=0 / 1.00
(was 25/0/1.00, source_breakdown github-commit 26/26); the
other five detectors are unchanged. Batch 24 preserves the
pr-miner mining margin because the new file is Rust — Python
`{open} → {close}` mining-DB confidence stays at batch-10's
19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs (`get_ver`,
`readfile`) remain TPs. Earlier 2026-05-16: Q-14 recall-audit
Phase B batch 23 landed on top of batch 22, diversifying
`comment-code` Pattern C audit coverage from seven upstreams
(whisky-archive + tls-parser + glium + pkg-config-rs + sui
mysten-metrics + vcpkg-rs + rust-vst2, batches 3 / 11 / 12 /
13 / 20 / 21 / 22) to eight upstreams by adding one
`comment-code` Pattern C TP from a fourteenth
permissive-licensed Rust upstream
always-further/nono@2e5504f2b0c3e7803e7e3e3971154dc370a69be2
`crates/nono-cli/src/deprecated_schema.rs` (Apache-2.0); one
top-level `pub fn` item — `warn_for_deprecated_flags` (upstream
line 196 / corpus line 10) — carries a four-line `///` doc
block whose later half (after a blank `///` separator) reads
`DEPRECATED: delete when all long-flag aliases in this module
are removed / in v1.0.0. See module-level removal steps.` but
does not carry the `#[deprecated]` runtime attribute the Rust
deprecation lints honour, so spec F5 Pattern C fires; the
function body iterates `detect_deprecated_flags(args)`,
matches each canonical legacy spelling against its replacement
(`--override-deny` → `--bypass-protection`), and calls
`emit_deprecation_warning(...)` — the meta-deprecation-warning-
emitter sub-shape within Pattern C; nono is the
capability-based sandbox CLI domain, unrelated to the prior
seven Pattern C domains. Earlier 2026-05-16: Q-14 recall-audit
Phase B batch 22 landed on top of batch 21, diversifying
`comment-code` Pattern C audit coverage from six upstreams
(whisky-archive + tls-parser + glium + pkg-config-rs + sui
mysten-metrics + vcpkg-rs, batches 3 / 11 / 12 / 13 / 20 / 21)
to seven upstreams by adding one `comment-code` Pattern C TP
from a thirteenth permissive-licensed Rust upstream
overdrivenpotato/rust-vst2@244e14bd28caff3b21aa27f26a57bf01f01b7780
`src/interfaces.rs` (MIT). One top-level `pub fn` item —
`process_deprecated` (upstream line 17 / corpus line 6) —
carries a single-line `///` doc block reading `Deprecated
process function.` but does not carry the `#[deprecated]`
runtime attribute the Rust deprecation lints honour, so
downstream consumers receive no compiler warning — the
textbook Tan SOSP 2007 §3.2 Pattern C bug shape. The function
body is the empty block `{ }` retained as an ABI / API
placeholder for callers that link against the old symbol while
migrating to the replacement `process_replacing` /
`process_replacing_f64` family — a new body-shape variant
within Pattern C, contrasting prior body shapes that delegate
to the replacement (batch-3 whisky-archive con_str → constr,
batch-13 pkg-config-rs find_library → probe_library, batch-21
vcpkg-rs probe_package → find_package) and prior body shapes
that retain the original implementation in-tree (batch-11
tls-parser parse_tls_handshake_*next_protocol family, batch-12
glium validate, batch-20 sui mysten-metrics channel /
channel_with_total). rust-vst2 is the audio / DSP plugin host
domain (VST 2.4 API implementation in Rust for creating audio
plugins and hosts), unrelated to the prior six Pattern C
domains. `comment-code` moved to tp=24 / fn=0 / 1.00 and
overall recall_upper_bound to 0.59 (30 TP / 21 FN / 51
expected). Earlier 2026-05-16:
Q-14 recall-audit Phase B batch 21 landed on top of batch 20,
diversifying `comment-code` Pattern C audit coverage from five
upstreams (whisky-archive + tls-parser + glium + pkg-config-rs
+ sui mysten-metrics, batches 3 / 11 / 12 / 13 / 20) to six
upstreams by adding one `comment-code` Pattern C TP from a
twelfth permissive-licensed Rust upstream
mcgoo/vcpkg-rs@fa42994af157924e44223f2559a95c906e00c1fa
`src/lib.rs` (MIT OR Apache-2.0). One top-level `pub fn` item —
`probe_package` (upstream line 293 / corpus line 7) — carries
a single-line `///` doc block reading `Deprecated in favor of
the find_package function` and a `#[doc(hidden)]` non-
suppressive attribute, but no `#[deprecated]` runtime
attribute the Rust deprecation lints honour. cntrdct's spec F5
Pattern C trigger fires on the `deprecated` substring; the
walker checks the literal first identifier of the attribute
path (`doc` vs. `deprecated`) so `#[doc(hidden)]` does NOT
suppress the firing — the textbook Tan SOSP 2007 §3.2 Pattern
C bug shape. The function signature
`pub fn probe_package(name: &str) -> Result<Library, Error>`
has Result return so spec F3 Pattern A's return-type negation
fails (Pattern A does not fire); the body `Config::new()
.probe(name)` delegates to the replacement function and
contains no `panic!` / `unwrap` / `expect(` / `unreachable!` /
`assert!` / `assert_eq!` / `assert_ne!` / `todo!` /
`unimplemented!` / `debug_assert` substrings, but the doc
contains no `panic` substring either, so spec F4 Pattern B
does not fire — only Pattern C fires. Structurally pairs with
batch-13 pkg-config-rs as the Windows side of the build-script
system-package binding family: pkg-config-rs covers Unix
pkg-config tool bindings; vcpkg-rs covers Microsoft Windows
vcpkg C/C++ package manager bindings. Both upstreams
independently exhibit the same Pattern C bug shape
(`/// Deprecated in favor of X` single-line doc +
`#[doc(hidden)]` non-suppressive attribute + body delegating
to the replacement function) on functionally-paired
native-library-discovery crates targeting different platform
ecosystems — exercising cntrdct's Pattern C check on
cross-platform-paired upstreams that independently
rediscovered the same `Deprecated`-prose-without-
`#[deprecated]`-attribute oversight. The entry is TP. After
batch 21 the detector's audit evidence spans twelve upstreams
across twelve unrelated domains (Cardano Plutus-data helpers 4
Pattern C + TLS parser 2 Pattern C + OpenGL bindings 1 Pattern
C + Unix pkg-config bindings 1 Pattern C + Zarr-format
data-type bindings 6 Pattern B + zkVM executor registry 1
Pattern A + parking_lot_core synchronization primitives 2
Pattern B + cranelift-assembler-x64 fuzzer infrastructure 1
Pattern A + S3-client configuration setter 1 Pattern A +
vortex-buffer bit-packed bitmap helpers 1 Pattern B + sui
mysten-metrics async-channel metrics wrapper 2 Pattern C +
Windows vcpkg bindings 1 Pattern C), with Pattern A and
Pattern B saturated at three sub-shapes on three upstreams
each (batches 18 and 19) and Pattern C now exercised on six
unrelated upstreams (lifted from five at batch 20). The
source-kind footprint stays at six (`github-commit` absorbs
the new entry). Updated corpus 38 files / 50 expected entries
/ overall recall_upper_bound 0.58 (29 TP / 21 FN, up from 0.57
at batch 20); `comment-code` moves to tp=23 / fn=0 / 1.00 (was
22/0/1.00, source_breakdown github-commit 23/23); the other
five detectors are unchanged. Batch 21 preserves the pr-miner
mining margin because the new file is Rust — Python `{open}
→ {close}` mining-DB confidence stays at batch-10's 19/22 ≈
0.864 ≥ 0.85, and both pr-miner TPs (`get_ver`, `readfile`)
remain TPs. Earlier 2026-05-15: Q-14 recall-audit Phase B
batch 20 (comment-code Pattern C diversification via sui
mysten-metrics eleventh upstream) shifted Pattern C audit
coverage from four upstreams (whisky-archive + tls-parser +
glium + pkg-config-rs, batches 3 / 11 / 12 / 13) to five
upstreams by adding two `comment-code` Pattern C TPs from an
eleventh permissive-licensed Rust upstream
MystenLabs/sui@add9d4722104173186e78df5aac4b07b6e019b40
`crates/mysten-metrics/src/metered_channel.rs` (Apache-2.0).
Two top-level `pub fn` items — `channel` (upstream line 321 /
corpus line 8) and `channel_with_total` (upstream line 339 /
corpus line 26) — each carry a `///` doc block containing the
substring `Deprecated: use `monitored_mpsc::channel`
instead.`; cntrdct's spec F5 Pattern C trigger fires
(`doc_lc.contains("deprecated")` matches both). Both
functions carry the `#[track_caller]` attribute (which
propagates the caller location for panic reporting but does
NOT trigger the Rust deprecation lints) and neither carries
the `#[deprecated]` runtime attribute the Rust deprecation
lints honour, so downstream consumers receive no compiler
warning — the same Tan SOSP 2007 §3.2 Pattern C ("bad
comment": deprecation prose without `#[deprecated]`
attribute) bug shape batches 3 / 11 / 12 / 13 flag on four
prior unrelated domains. The per-fn loop walks each
`function_item` separately checking the attribute path's
literal first identifier (`track_caller` vs. `deprecated`)
without interpreting the attribute's behavioural semantics,
so the non-suppressive `#[track_caller]` attribute does NOT
prevent Pattern C from firing — structurally analogous to the
`#[doc(hidden)]` attribute on batch-13 pkg-config-rs
`find_library` in that BOTH are non-suppressive attributes
present alongside the deprecation prose without triggering
the `#[deprecated]` lint. The doc contains no Pattern A
trigger phrases (`returns err` / `returns result` / `may
fail` / `fallible` / `returns option` / `may return none`),
so Pattern A does NOT fire on either function. The doc
contains no `panic` substring, so spec F4 Pattern B does NOT
fire — only Pattern C fires on both functions. Both entries
are TP. After batch 20 the detector's audit evidence spans
eleven upstreams across eleven unrelated domains (Cardano
Plutus-data helpers 4 Pattern C + TLS parser 2 Pattern C +
OpenGL bindings 1 Pattern C + build-tool / system-package
bindings 1 Pattern C + Zarr-format data-type bindings 6
Pattern B + zkVM executor registry 1 Pattern A +
parking_lot_core synchronization primitives 2 Pattern B +
cranelift-assembler-x64 fuzzer infrastructure 1 Pattern A +
S3-client configuration setter 1 Pattern A + vortex-buffer
bit-packed bitmap helpers 1 Pattern B + sui mysten-metrics
async-channel metrics wrapper 2 Pattern C), with Pattern A
and Pattern B exercised across three sub-shapes on three
upstreams each (saturated at batches 18 and 19) and Pattern C
now exercised on five unrelated upstreams (lifted from four
at batch 13). The source-kind footprint stays at six
(`github-commit` absorbs the two new entries). Updated corpus
37 files / 49 expected entries / overall recall_upper_bound
0.57 (28 TP / 21 FN, up from 0.55 at batch 19); `comment-code`
moves to tp=22 / fn=0 / 1.00 (was 20/0/1.00, source_breakdown
github-commit 22/22); the other five detectors are unchanged.
Batch 20 preserves the pr-miner mining margin because the new
file is Rust — Python `{open} → {close}` mining-DB confidence
stays at batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner
TPs (`get_ver`, `readfile`) remain TPs. Earlier 2026-05-15:
Q-14 recall-audit Phase B batch 19 (comment-code Pattern B
diversification via vortex-buffer tenth upstream) shifted
Pattern B audit coverage from two upstreams (zarrs +
parking_lot_core, batches 14 and 16) to three upstreams by
adding one `comment-code` Pattern B TP from a tenth
permissive-licensed Rust upstream
vortex-data/vortex@4c1ae92d20856fbea29ed92d8554a3832b5a540c
`vortex-buffer/src/bit/mod.rs` (v0.70.0 release tag,
Apache-2.0). `pub fn get_bit(buf: &[u8], index: usize) ->
bool` (upstream line 33 / corpus line 11) carries a `///`
doc block whose `# Panics` section reads `Panics if `index`
is not between 0 and length of `buf * 8`.`; the body
`buf[index / 8] & (1 << (index % 8)) != 0` contains NONE of
cntrdct's Pattern B body markers, so spec F4 fires — the
implicit-indexing-panic sub-shape, contrasting batch-14
zarrs silent-drop-on-mismatch and batch-16
parking_lot_core callback-contract sub-shapes. Pattern B
saturated at three sub-shapes across three upstreams at
batch 19. Earlier 2026-05-15: Q-14
recall-audit Phase B batch 18 (comment-code Pattern A
diversification via rust-s3 ninth upstream) shifted Pattern A
audit coverage from two upstreams (boundless + wasmtime,
batches 15 and 17) to three upstreams by adding one
`comment-code` Pattern A TP from
durch/rust-s3@771be1650c23bf9aa482907c2d88dcf55d2e3ebc
`s3/src/lib.rs` (MIT). `pub fn set_retries(retries: u8)` at
upstream line 54 (corpus line 23) carries a `///` doc block
whose first line reads `Sets the number of retries for
operations that may fail and need to be retried.` —
configuration-doc-references-external-fallibility sub-shape
of Tan SOSP 2007 §3.1 Pattern A. Earlier 2026-05-15: Q-14
recall-audit Phase B batch 17 (comment-code Pattern A
diversification via wasmtime eighth upstream) shifted Pattern A audit coverage from
a single upstream (boundless only, batch 15) to two upstreams
by adding one `comment-code` Pattern A TP from
bytecodealliance/wasmtime@63330f11f50f6721a0f4b41ae366bba00a58536d
`cranelift/assembler-x64/src/fuzz.rs` (Apache-2.0 WITH
LLVM-exception). One top-level `pub fn` item — `roundtrip`
(upstream line 26 / corpus line 13) — carries a `///` doc
block whose `# Panics` section reads `This function panics to
express failure as expected by the \`arbitrary\` fuzzer
infrastructure. It may fail during assembly, disassembly, or
when comparing the disassembled strings.`; spec F4 Pattern B's
body-marker negation suppresses Pattern B even though the doc
contains the substring `panic` — only Pattern A fires.
Earlier 2026-05-15: Q-14 recall-audit Phase B batch 16
(comment-code Pattern B diversification via
parking_lot_core seventh upstream) shifted Pattern B audit
coverage from a single upstream (zarrs only, batch 14) to two
upstreams by adding two `comment-code` Pattern B TPs from
Amanieu/parking_lot@d7828fff7b5d6327ae608e82db45f888b344449a
`core/src/parking_lot.rs` (parking_lot_core-v0.9.12 release
tag, MIT OR Apache-2.0). Two top-level `pub unsafe fn` items —
`unpark_one` (upstream line 732 / corpus line 29) and
`unpark_requeue` (upstream line 888 / corpus line 122) — each
carry a `///` doc block ending in `must not panic or call into
any function in parking_lot.` cntrdct's spec F4 trigger fires
(`doc_lc.contains("panic")` matches), and the body substring
check finds none of the Pattern B markers in either body —
the same Tan SOSP 2007 §3.2 Pattern B bug shape the batch-14
zarrs `round_bytes_*` family flags. Earlier 2026-05-15: Q-14
recall-audit Phase B batch 15 (comment-code Pattern A coverage
via boundless sixth upstream) shifted comment-code audit
coverage
from two-pattern (Pattern B + Pattern C) to the complete
three-pattern (Pattern A + Pattern B + Pattern C) triple by
adding one TP from boundless-xyz/boundless@1a2770b2
`crates/executor/src/backends/mod.rs` (Apache-2.0):
`pub fn default_registry() -> Registry` at upstream line 44
carries a `///` doc block whose prose contains the Pattern A
trigger phrase `may fail`, but the function signature
`() -> Registry` lacks the `Result` / `Option` substring
required by F3's return-type negation, and the body absorbs
constructor failures via `tracing::warn!` rather than
propagating an error type. Earlier 2026-05-15: Q-14
recall-audit Phase B batch 14 landed on top of batch 13,
shifting `comment-code` audit coverage from Pattern C only —
deprecated prose without `#[deprecated]` attribute, batches
3 / 11 / 12 / 13 — to Pattern B + Pattern C by adding six
`comment-code` TPs from a fifth permissive-licensed Rust
upstream
zarrs/zarrs@3b944c57a0b7af127ae73ea250d3ffce60e51f0b
`zarrs_data_type/src/codec_traits/bitround.rs`
(MIT OR Apache-2.0). Six top-level `pub fn` items —
`round_bytes_int16` (upstream line 58 / corpus line 42),
`round_bytes_int32` (upstream 70 / corpus 54),
`round_bytes_int64` (upstream 82 / corpus 66),
`round_bytes_float16` (upstream 94 / corpus 78),
`round_bytes_float32` (upstream 106 / corpus 90), and
`round_bytes_float64` (upstream 118 / corpus 102) — each carry
a `///` doc block with a `# Panics\nPanics if \`bytes.len()\`
is not a multiple of N.` section for N in {2, 4, 8}, but the
body uses `bytes.as_chunks_mut::<N>().0` which never panics on
non-multiple-of-N lengths. `slice::as_chunks_mut::<N>`
(stabilised in Rust 1.88.0, 2025-06-26) returns
`(&mut [[T; N]], &mut [T])` where the second tuple element is
the remainder with length strictly less than N; the upstream
code accesses only `.0` (the chunked arrays) and silently
discards the remainder, so trailing bytes that don't form a
complete N-byte chunk are ignored rather than triggering the
documented panic — the textbook Tan SOSP 2007 §3.2 Pattern B
("bad comment": panic claim without panicking constructs in
the body) bug shape, previously not exercised by the audit
corpus. cntrdct's spec F4 trigger fires (rendered doc contains
`panic` substring after `to_lowercase`) and the body substring
check finds none of `panic!` / `unwrap` / `expect(` /
`unreachable!` / `assert!` / `assert_eq!` / `assert_ne!` /
`todo!` / `unimplemented!` / `debug_assert` in the body source
text (the `assert!(N != 0)` inside `slice::as_chunks_mut` is
in std, not the call-site body, so substring check sees no
marker). All six entries are TP. After batch 14 the detector's
audit evidence spans five upstreams across five unrelated
domains (Cardano Plutus-data helpers 4 + TLS parser 2 + OpenGL
bindings 1 + build-tool / system-package bindings 1 +
Zarr-format data-type bindings 6) AND two of the three
`docs/spec/comment-code-v0.md` patterns (Pattern B + Pattern
C; Pattern A — Result / Option claim without matching return
type — still owed), so a regression that broke one pattern
while preserving the other would now surface in the audit.
The source-kind footprint stays at six (`github-commit`
absorbs the six new entries; no new kind invented). Updated
corpus 31 files / 41 expected entries / overall
recall_upper_bound 0.49 (20 TP / 21 FN, up from 0.40 at batch
13); `comment-code` moves to tp=14 / fn=0 / 1.00 (was
8/0/1.00, source_breakdown github-commit 14/14); the other
five detectors are unchanged. Batch 14 preserves the pr-miner
mining margin because the new file is Rust — Python
`{open} → {close}` mining-DB confidence stays at batch-10's
19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs (`get_ver`,
`readfile`) remain TPs. Earlier 2026-05-14: Q-14 recall-audit
Phase B batch 13 landed on top of batch 12, adding one
`comment-code` TP from a fourth permissive-licensed Rust
upstream
rust-lang/pkg-config-rs@f36d32a09824a6b2c18475c8a4b7df1cb2c50c95
`src/lib.rs` (MIT OR Apache-2.0).
`pub fn find_library(name: &str) -> Result<Library, String>`
at upstream line 453 (corpus line 7) carries a single-line
`///` doc block reading `Deprecated in favor of the
probe_library function` and a `#[doc(hidden)]` attribute, but
does not carry the `#[deprecated]` runtime attribute the Rust
deprecation lints honour — the same Tan SOSP 2007 §3.2
Pattern C ("bad comment") bug shape the batch-3
`sidan-lab/whisky-archive` con_str* family, batch-11
`rusticata/tls-parser`
`parse_tls_handshake_*next_protocol` family, and batch-12
`glium/glium` `validate` flag. Earlier 2026-05-13: Q-14 recall-audit
Phase B batch 12 landed on top of batch 11, adding one
`comment-code` TP from a third permissive-licensed Rust
upstream glium/glium@8d6fd34d9171172928771657fc5c9103107ff978
`src/draw_parameters/mod.rs` (Apache-2.0). `pub fn validate(...)`
at upstream line 531 (corpus line 6) carries a single-line
`///` doc block reading `DEPRECATED. Checks parameters and
returns an error if something is wrong.` but does not carry
the `#[deprecated]` runtime attribute the Rust deprecation
lints honour — the same Tan SOSP 2007 §3.2 Pattern C ("bad
comment") bug shape the batch-3 `sidan-lab/whisky-archive`
con_str* family and batch-11 `rusticata/tls-parser`
`parse_tls_handshake_*next_protocol` family flag. After batch
12 the detector's audit evidence spans three upstreams on
three unrelated domains (Cardano Plutus-data helpers 4 + TLS
parser 2 + OpenGL bindings 1), further reducing the
regression-detection risk of single-source dominance that
batch 11's second upstream already broke. Earlier 2026-05-13:
Q-14 recall-audit Phase B batch 11 added two `comment-code`
TPs from a second permissive-licensed Rust upstream
rusticata/tls-parser@6554155918278531370e7d0addbd5d759e3a4cc9
`src/tls_handshake.rs` (MIT OR Apache-2.0): both
`pub fn parse_tls_handshake_next_protocol(...)` (upstream line
850 / corpus line 10) and
`pub fn parse_tls_handshake_msg_next_protocol(...)` (upstream
line 865 / corpus line 25) carry a `///` doc block ending in
`Deprecated in favour of ALPN.` without `#[deprecated]`; corpus
moved to 28 files / 33 expected entries / overall
recall_upper_bound 0.36 (12 TP / 21 FN, up from 0.32 at
batch 10); `comment-code` to tp=6 / fn=0 / 1.00 (was 4/0/1.00,
source_breakdown github-commit 6/6); the other five detectors
unchanged; pr-miner mining margin preserved because the file is
Rust. Earlier 2026-05-13: Q-14 recall-audit Phase B batch 10
landed on top of batch 9, surfacing the first pr-miner true
positives by adding ten density-support files containing
eighteen paired open+close top-level Python transactions. The
files (MIT / BSD-3-Clause / Apache-2.0): carla
`Util/Tools/Import.py` (minimal extract, 3 paired), Telluric-
Fitter `setup.py` (minimal extract, 2 paired), baidu/tera
`tera_setup.py` (verbatim, 1 paired), nottheswimmer/pytago
`examples/fileloop.py` (verbatim, 1 paired), dkruchinin/sanic-
prometheus `scripts/release.py` (verbatim, 2 paired),
carljm/django-secure `setup.py` (verbatim, 1 paired) and
`doc/conf.py` (minimal extract, 1 paired), adnanademovic/rosrust
`genaction.py` (verbatim, 2 paired; BSD-3-Clause file header
inside an MIT-overall repo), R-s0n/ars0n-framework
`fire-scanner.py` (minimal extract, 2 paired), apache/ranger
`tagsync/scripts/setup.py` (minimal extract, 3 paired). Each
file ships with `expected: []` because the Semgrep
`open-never-closed` labeller produces no findings on files where
every top-level `def` calling `open` also explicitly closes —
the negative labeller result is faithfully captured by an empty
expected array. Math: before batch 10 the corpus had 4 top-level
Python defs containing `open` (`get_ver`, `replace_ver`,
`readfile`, `test_identity_source_write_read`) with only
`replace_ver` containing `close`, giving `{open} -> {close}`
confidence of 1/4 = 0.25 < `MIN_CONFIDENCE = 0.85`. Batch 10's
18 paired transactions push confidence to (1+18)/(4+18) = 19/22
≈ 0.864 ≥ 0.85, so spec F3 Apriori mines the rule, spec F4
scans the full transaction set for violations, and both batch-8
`tugraph_det_ver.py::get_ver` (corpus line 9) and batch-9
`django_mobile_setup.py::readfile` (corpus line 17) flip from
FN to TP — without any detector-side change. `pr-miner` moves
from 0/2/0.00 to 2/0/1.00; overall recall_upper_bound jumps from
0.26 to 0.32 (10 TP / 21 FN over 31 expected entries, corpus 27
files); the other five detectors are unchanged (their findings
depend only on per-file content and the density-support files
carry no expected entries). The "numerator construction"
direction telegraphed in batches 8 and 9 closes cleanly here.
Earlier 2026-05-13: Q-14 recall-audit Phase B batch 9 landed,
deepening the pr-miner FN denominator on the existing `semgrep`
source kind with a second permissive-licensed
`open-never-closed` instance. The labeller
is the same Semgrep registry rule
(`python.lang.best-practice.open-never-closed.open-never-closed`,
pinned at semgrep/semgrep-rules@9d73d08e70fee9fc1fd940d1378ca6c601312883
python/lang/best-practice/open-never-closed.yaml) applied to a
different permissive upstream:
gregmuellegger/django-mobile@fafc389057d9dfab5f3c69f7e054dbee8b546f44
`setup.py` (BSD-3-Clause). The single expected entry flags
`return open(filename, ...).read()` inside top-level
`def readfile(filename):` at upstream line 13 (corpus line 17)
— both branches of the `sys.version_info` check return the same
`open(...).read()` chain without any matching `close()` /
`with` / try-finally, so the file handle is dropped on return.
The companion top-level `def get_author` and `def get_version`
delegate to `readfile` rather than calling `open` directly, so
the rule does not fire on them; the `UltraMagicString` class
methods are out of scope for pr-miner's spec F2 extractor (only
top-level `function_definition` / `decorated_definition` are
walked). cntrdct's pr-miner reaches `readfile` (spec F2,
top-level function) and produces item set `{open, read}`, but
spec F3 Apriori mining at `MIN_SUPPORT = 0.05` /
`MIN_CONFIDENCE = 0.85` cannot synthesise the `{open} → {close}`
rule: across the corpus three top-level Python defs contain
`open` (`get_ver` and `readfile` open-only, batch 8's
`replace_ver` open+close), only one of which contains `close`,
so the rule's confidence is 1/3 ≈ 33%, far below the 0.85
threshold. Spec F4 violation detection therefore never runs
against `readfile` — FN by mining sparsity (denominator weight
deepened without crossing confidence), NOT by extractor scope.
Updated corpus 17 files / 31 expected entries / overall
recall_upper_bound 0.26 (down from 0.27 at batch 8); pr-miner
moves to tp=0 / fn=2 / 0.00 dominated by the existing `semgrep`
source kind (2/2 entries); all five other detectors
(`arg-swap` 0/4/0.00, `clone-drift` 0/2/0.00, `comment-code`
4/0/1.00, `config-interaction` 0/2/0.00,
`unreachable-after-terminator` 4/13/0.24) are unchanged. With
the pr-miner FN denominator now broadened twice on the same FN
class (open-only top-level Python def under permissive
license), the next pr-miner move shifts from denominator
widening to numerator construction: paired open+close
transactions on permissive Python upstreams to lift
`{open} → {close}` mined-rule confidence above 0.85, after
which the existing FN entries become TPs without any
detector-side change. Earlier 2026-05-13: Q-14 recall-audit
Phase B batch 8 landed, introducing the `semgrep` source kind
and the sixth and final cntrdct detector — `pr-miner` — to the
audit corpus in the same commit via the same Semgrep registry
rule applied to
TuGraph-family/tugraph-db@672e4b1998b78e5dbd45ae44950e86b48c841437
`release/det_ver.py` (Apache-2.0), which sidesteps the Semgrep
Rules License v1.0 carve-out entirely: only the rule's stable
identity is cited, and the labelled code is Apache-2.0
tugraph-db, not semgrep-rules itself. The single batch-8
expected entry flags `f = open('Options.cmake','r')` inside
top-level `def get_ver():` at upstream line 5 (corpus line 9)
with no matching `close()` / `with` / try-finally; the
companion `def replace_ver(...)` opens AND closes, so the rule
does not fire on it. With all six detectors and six external
source kinds (`rustc-lint-testset`, `github-commit`,
`paper-appendix`, `clippy`, `codeql`, `semgrep`) now live, the
originally-named Phase B blockers — pr-miner detector coverage
and semgrep source kind — were both closed at batch 8; batch 9
and further Phase B batches deepen existing detector coverage
rather than introduce a new detector or kind. 2026-05-12: Q-14 recall-audit Phase B batch 7
landed on top of batch 6, introducing the `codeql` source kind
via the CodeQL Python `UnreachableCode` query test fixture
(`github/codeql@592c7c043734f6bb48768a56261d711446cde25f
python/ql/test/query-tests/Statements/unreachable/test.py`,
MIT). Six expected unreachable-statement entries land per
CodeQL's matching `UnreachableCode.expected`: corpus line 25
(`for x in first_unreachable_stmt():` directly following
`return 5`) is TP against cntrdct's Python
unreachable-after-terminator detector — the F3 terminator +
follower pattern reproduced in Python via
`analyze_python_block`. The other five (corpus lines 12, 14,
20, 32, 88) are FN by spec F3: cntrdct's terminator set covers
neither constant-condition branches (`while False:` /
`if False:` / `else` of `if True:`) nor typed-exception
reachability (`except NameError:` for a name that is always
defined). Earlier 2026-05-12: Q-14
recall-audit Phase B batch 6 landed same day on top of batch 5,
introducing the `config-interaction` detector to the corpus via
the rustc UI test for the `cfg.attr.duplicates` Rust Reference
behaviour (`rust-lang/rust@29b7590130c83542a095cdf1323ed0f78eec2bb8
tests/ui/cfg/both-true-false.rs`, MIT OR Apache-2.0); both
entries FN by `docs/spec/config-interaction-v0.md` F5 (atomic
`true` / `false` predicates lack the `not(...)` wrapper).
Earlier 2026-05-12: Q-14
recall-audit Phase B batch 5 introduces the `clippy` source
kind via two rust-clippy UI tests (MIT OR Apache-2.0) pinned at
master commit `c4b8c6d454c648ef2d7cb86ca1bc698da829e4bc`:
`tests/ui/if_same_then_else.rs` (statement-block clone pair
flagged by `clippy::if_same_then_else`) and
`tests/ui/branches_sharing_code/shared_at_top.rs` (shared
statement-block prefix in if/else branches flagged by
`clippy::branches_sharing_code`). Both are FN against cntrdct's
clone-drift detector by `docs/spec/clone-drift-v0.md` F2 — cntrdct
clone-drift v0 operates at top-level `fn` granularity only and
requires `MIN_FN_TOKENS >= 22` + `MIN_GROUP_SIZE >= 3`. Earlier
2026-05-12: Q-14
recall-audit Phase B batch 4 introduces the `paper-appendix`
source kind via three PyPIBugs (Allamanis NeurIPS 2021) ArgSwap
entries on permissive-licensed Python repositories:
`c137digital/unv_app@d217fa0d` MIT, `mwouts/nbrmd@dfa96996` MIT,
`markokr/rarfile@7fd6b2ca` ISC; all three are FN against
cntrdct's narrow Rice-2017 arg-swap detector. Q-14
recall-audit Phase B batch 3 broadens the corpus to a third
detector, `comment-code`, via the textbook Pattern C bug
(Tan SOSP 2007 §3.2 "bad comment": `/// Deprecated` prose
without the runtime `#[deprecated]` attribute). Seeded from
Apache-2.0 `sidan-lab/whisky-archive`
`packages/whisky-common/src/data/primitives/constructors.rs`
pinned at commit `99243766` — the `con_str` / `con_str0` /
`con_str1` / `con_str2` family contributes four expected TP
entries, so comment-code enters the corpus with single-source
recall_upper_bound 1.0. Clone-drift was investigated as the
original batch-3 target but punted to a later batch: the
published peer-reviewed clone-drift bug catalogues (Bettenburg
MSR 2009, Krinke ICSM 2007) target C/Java rather than
Rust/Python, and the Assi TOSEM 2025 deep-learning-framework
genealogies (mia1q/code-clone-DL-frameworks replication CSVs)
expose size-2 clone pairs that fall under cntrdct's
`MIN_GROUP_SIZE = 3` floor by construction. Further Phase B
batches still owe coverage for clone-drift, config-interaction,
and pr-miner, plus the semgrep / codeql / clippy source kinds. 2026-05-12 earlier: Q-14 recall-audit
Phase B batch 2 and Phase C release-tag refresh discipline both
landed. Phase B batch 2: six additional expected entries on
`unreachable-after-terminator` deepening the recall ceiling on
the existing detector via three rustc UI testset files
(`expr_return.rs`, `expr_call.rs`, `expr_loop.rs`) that probe
control-flow shapes cntrdct's statement-level scan misses by
construction; corpus before batch 3 was 7 files / 12 expected
entries / overall recall_upper_bound 0.25. The drop from 0.50
to 0.25 is intentional honest signal — closing the gaps is
detector-improvement work, not audit-harness work. Phase C:
`benchmarks/audit-corpus/README.md` adds a "Refresh discipline
(Phase C)" section enumerating the on-tag procedure;
`CLAUDE.md` "Release procedure" steps now include the audit
re-run between lockfile sync and commit, and the non-negotiables
pin the same-commit rule and a no-op refresh policy (figures
unchanged = no-op is fine; the discipline is the re-run, not the
delta). No CI enforcement on top — audit-recall is already gated
indirectly via the embedded priors and detector logic CI already
checks.
2026-05-11: Q-14 recall-audit Phase B first batch landed — six
expected entries / two detectors / two sources / overall
recall_upper_bound 0.50; Q-16 cargo-mutants nightly landed same day
— `.cargo/mutants.toml` scopes mutation testing to
`src/detectors/**/*.rs` via `examine_globs`,
`.github/workflows/mutants.yml` runs `cargo mutants --no-shuffle -j 2`
on a 06:00 UTC cron + `workflow_dispatch`, the post-run step tallies
`mutants.out/{caught,missed,unviable,timeout}.txt` and fails the job
when `caught / (caught + missed) < 0.80`, the missed-mutant list lands
in `$GITHUB_STEP_SUMMARY` and `mutants.out/` is archived as a 30-day
artifact for off-runner inspection; first nightly run on master is the
real signal for whether the codebase already meets the 80% gate since
local validation is multi-hour. Q-14 recall-audit harness Phase A
scaffolding landed same day — `cntrdct calibrate --audit-recall
<CORPUS_DIR>` flag wired with clap conflict against `--fit-platt`,
new `src/recall_audit.rs` module with `audit_recall(...) ->
RecallAuditReport` pure function and `external_source: {kind, ref,
url}` provenance per labelled finding, `cntrdct::run_recall_audit`
orchestrator, 12 tests (4 unit + 8 integration) pin loader / matcher
/ byte-stable JSON / CLI conflict, `benchmarks/audit-corpus/`
skeleton with per-detector seed targets documented, CITATIONS.md
adds `heckman-williams-ist-2011` under Layer 2; Q-14 Phase B first
batch landed same day — six expected entries across two detectors
and two sources (`rustc-lint-testset` ×5 from rust-lang/rust@4b0c9d76
`tests/ui/reachable/{unreachable-code-ret,expr_block,expr_if}.rs`,
`github-commit` ×1 from `wasserth/TotalSegmentator` PR #556
`statistics.py:58`), audit numbers
`unreachable-after-terminator` recall_upper_bound 0.60 (3 TP / 2 FN),
`arg-swap` 0.00 (0 TP / 1 FN), overall 0.50; the rustc-testset
copies strip the file-level `#![deny(unreachable_code)]` attribute
because cntrdct's SUPPRESSION_TOKEN scan would otherwise honour it.
README "Latest audit run" populated, ROADMAP Phase B entry
expanded, further batches broadening detector and source coverage
plus Phase C release-tag refresh discipline still pending. Spec:
`docs/spec/recall-audit-v0.md`. Q-13 cross-model κ
audit redesigned to
CLI-only same day — `PromptDispatch` trait + `ClaudeCliAdjudicator` +
`GeminiCliAdjudicator` ship alongside the existing
`AnthropicAdjudicator`; OpenAI / Google API-key paths and the
nightly workflow were dropped because (a) Codex CLI's system prompt
could not be cleanly replaced, (b) continuous monitoring was
unsupported by measurement stationarity given silent model drift +
sampler stochasticity. Audit is now on-demand: `cntrdct
cross-model-kappa <CORPUS>` shells out to `claude --print` and
`gemini -p` (auth via each CLI's OAuth, no API keys read by
cntrdct), prints to stdout by default. `src/cross_model_kappa.rs`
carries the pure κ math + per-`(detector_id, anomaly_class)`
aggregation; PR CI exercises `tests/cross_model_kappa.rs` with
`CannedDispatch` mocks plus stub-script tests for the CLI flag
sets; citations `wataoka-2024` and `zheng-neurips-2023` added to
Layer 3;
Q-11 small-N Wilson → Jeffreys interval
switching landed 2026-05-10; Q-12 LLM-calibration post-hoc Platt fit
landed 2026-05-10 — adjudicator prompt drops the verbalised
`calibration_tag`, post-hoc Platt scaling fit per `(detector_id,
anomaly_class)` cell takes its place, embedded
`benchmarks/llm-calibration/platt-default.json` ships empty in v0,
`AdjudicationResult.calibrated_confidence` + SARIF `properties.
adjudication.calibrated_confidence` plumbed end-to-end,
`tests/calibration_ece.rs` pins ≥ 0.05 holdout-ECE drop on a
constructed over-confidence fixture; Phase G RC1-blocker Q-series
Q-1..Q-5 landed; Phase H governance items Q-6..Q-10 all landed
2026-05-07, closing Phase H; P-7 clone-drift residual FP cleanup
landed 2026-05-07, taking wild β clone-drift FPs to 0 in both Rust
and Python; T3-15 git-cliff release-notes pipeline and T3-16
telemetry-free assurance both landed 2026-05-08; T3-14
cargo-binstall metadata and Homebrew tap
(`ktrysmt/homebrew-cntrdct`) landed 2026-05-08, with the AUR
sub-target dropped from scope; T3-12 LSP server Phase 1 scaffolding
(`cntrdct-lsp` binary behind the `lsp` Cargo feature) landed
2026-05-08, with Phase 1.b document events + Finding -> Diagnostic
mapping (`textDocument/{didOpen,didChange,didSave,didClose}` →
`publishDiagnostics`, sharing the Layer 1 detector battery with the
disk-walking scan path via a new `scan_buffer` + `run_detectors_on`
seam) landing the same day on top, Phase 1.c per-URI didChange
debouncing (250 ms quiet window backed by an
`Arc<tokio::sync::Mutex<HashMap<Url, JoinHandle>>>`, with
`didSave` / `didClose` draining the pending map for their URI before
acting) landing 2026-05-09, and Phase 1.c+ per-URI monotonic
generation counter (extends the per-URI map to a `UriState` carrying
`{handle, latest_generation}`, every event bumps and captures, every
publish is gated on `captured == latest`) closing the
`spawn_blocking`-cannot-be-aborted race the same day; v0.2.0-rc.1
tag cut 2026-05-08 — first end-to-end run of the git-cliff + Homebrew
bump pipelines, both green on first try; T4-17, T4-18, T4-19 landed
earlier; T4-20 / T4-21 deferred per maintainer decision; community
scaffolding minus the formal CoC, which is deferred until external
contributor activity warrants it)

Engineering roadmap for shipping cntrdct as a usable open-source Rust
tool.

## Status legend

- `[x]` completed
- `[~]` in progress
- `[ ]` pending

## Practical track

Items that are not strictly OSS-readiness work but are critical
engineering deliverables that shape the public-facing v1. Done in
parallel with Tier 1 OSS readiness.

P-1. β corpus collection (real-world Rust crates)

- Status: `[x]`
- Summary: `scripts/fetch_rust_corpus.py` (stdlib only) pins
  `(crate, version, file_path)` triples, pulls `.crate` tarballs
  from `static.crates.io`, verifies SHA-256 against the sparse-index
  `cksum`, and rejects `@generated` sources. Result:
  `benchmarks/wild-corpus/` with 36 crates / 270 files / ~13 MB
  under permissive licenses; 124 hand-labelled findings (all v0
  FPs). Per-detector precision = 0 here is the intentional
  non-trivial signal feeding P-4. Limitations enumerated in
  `benchmarks/wild-corpus/README.md`.

P-2. pr-miner detector (multi-language)

- Status: `[x]`
- Summary: sixth Layer 1 detector mining implicit programming
  rules via Apriori (`MAX_ITEMSET_SIZE = 2`) over per-function
  call-site transactions. Spec at `docs/spec/pr-miner-v0.md`;
  module ships under `src/detectors/pr_miner/` (`apriori.rs`,
  `extract_rust.rs`, `extract_python.rs`, `mod.rs`). Citation:
  `li-zhou-fse-2005` (Confirmed for Rust, grandfather clause);
  Python is `LanguageCitationStatus::Unconfirmed` per
  `docs/surveys/pr-miner-python-2026-05.md`. Eight positives + three
  negatives per language;
  `tests/corpus_shape.rs::pr_miner_corpus_meets_per_language_positives`
  enforces the per-language commitment.
- Followup: Q-1 wires pr-miner into the SARIF detectors array;
  Future Q-series candidates note the Apriori → FP-growth lift.

P-3. SARIF output validation in CI

- Status: `[x]`
- Summary: `.github/workflows/ci.yml:86-106` runs
  `Sarif.Multitool validate` against the OASIS 2.1.0 schema on
  every CI run.

P-4. Layer 2 ranker recalibration on the β corpus

- Status: `[x]`
- Summary: `scripts/build_priors_corpus.py` derives a labelled
  JSONL from `(benchmarks/corpus, benchmarks/wild-corpus-python)`
  (87 rows: 69 TP / 18 FP) at `benchmarks/labelled-findings.jsonl`.
  `cntrdct calibrate` writes `benchmarks/priors-default.json`,
  embedded into the binary via `include_str!`. Fallback chain:
  explicit `--priors` → per-user cache → embedded → uncalibrated.
  Reorder sensitivity covered by
  `calibrated_ranker_reorders_when_wilson_disagrees_with_related_count`.

P-5. β release tagging and crates.io publish

- Status: `[x]` 2026-05-06
- Summary: `v0.2.0-beta.1` shipped — GitHub Release
  <https://github.com/ktrysmt/cntrdct/releases/tag/v0.2.0-beta.1>
  (linux x86_64/aarch64, darwin aarch64, windows x86_64 with
  matching `.sha256`); crates.io
  <https://crates.io/crates/cntrdct/0.2.0-beta.1>. Install:
  `cargo install cntrdct --version 0.2.0-beta.1 --locked`
  (explicit version qualifier required for pre-releases per SemVer).
  Workspace consolidated from 15 crates into one package; P3 LLM
  gating preserved by module boundary (only `src/adjudicator.rs`
  references reqwest).
- Successor: `v0.2.0-rc.1` cut 2026-05-08 — GitHub Release
  <https://github.com/ktrysmt/cntrdct/releases/tag/v0.2.0-rc.1>,
  crates.io <https://crates.io/crates/cntrdct/0.2.0-rc.1>. Install:
  `cargo install cntrdct --version 0.2.0-rc.1 --locked` (still pre-
  release per SemVer). Bundles the Phase G/H Q-series, P-7
  clone-drift residual cleanup, T3-12 LSP scaffolding, T3-14
  Homebrew + cargo-binstall, T3-15 git-cliff release-notes pipeline,
  and T3-16 netns telemetry-free assurance. First end-to-end run of
  the git-cliff release-body pipeline and the Homebrew tap auto-
  bump workflow; both green on first execution.
- Followup landed 2026-05-10: `cntrdct --version` / `-V` now wired.
  `Cli` derive at `src/main.rs:13` carries `#[command(version)]`,
  which clap fills from `CARGO_PKG_VERSION`. Both flags emit
  `cntrdct <version>` and exit 0 before subcommand resolution, so
  the prior `error: unexpected argument` clap message is gone. The
  `cargo cntrdct --version` shim path inherits the behaviour for
  free since `src/cargo_subcommand.rs` is a verbatim arg forwarder.
  Two new integration tests
  (`cli_version_long_flag_prints_cargo_pkg_version`,
  `cli_version_short_flag_prints_cargo_pkg_version` in
  `tests/integration.rs`) pin the contract structurally.

P-6. v0 → v0.1 detector quality fixes (wild β FP reduction pass)

- Status: `[x]` 2026-05-07
- Summary: wild β FPs cut from Rust 124 → 24 (80.6 % reduction)
  and Python 19 → 4 (78.9 %) via structural fixes — not corpus-
  specific tweaks. `unreachable-after-terminator` F4b/F4c
  (cfg-gated terminator suppression + hoisted item filtering),
  `comment-code` F5b/F5c (Python factory-shape + parameter-level
  `.. deprecated::` peeking), `clone-drift` F5b/F5c (scope-bounded
  clustering + strict-majority + Jaccard ≥ 0.7 gate). Embedded
  priors recomputed (clone-drift Wilson 0.073→0.355,
  unreachable-after-terminator 0.407→0.796, comment-code
  0.298→0.657). `cntrdct calibrate` made byte-stable via sorted
  `BTreeMap`. New prereg `prereg/2026-05-07-osf-prereg.md`
  supersedes the 2026-05-06 file.

P-7. clone-drift within-scope residual cleanup

- Status: `[x]` 2026-05-07
- Summary: F5d sibling-family discriminator (3 sub-gates) closes
  all 5 P-6 residuals. F5d-i suppresses clusters carrying ≥ 2
  size-1 partitions (the Python `charset_normalizer.utils`
  `is_<script>` family at `:70` and `:194`); F5d-ii suppresses
  high-Jaccard / high-length-imbalance singletons when the
  dominant partition holds only 2 functions (uuid `encode_*` at
  `uuid__fmt.rs:280`, tracing-subscriber `*_is_none` twins at
  `tracing_subscriber__layer_mod.rs:1547`); F5d-iii suppresses
  3-fn clusters whose dominant exemplar normalises to within 2
  tokens of `MIN_FN_TOKENS` (syn parse-API family at
  `syn__lib.rs:961`). The dominant-floor conditioner on F5d-ii
  (`LENGTH_IMBALANCE_DOMINANT_FLOOR = 3`) is what keeps the seed-
  corpus `clone_drift_005` TP at length imbalance 0.258 from
  being suppressed alongside the wild β residuals at 0.186 and
  0.242 — empirically the FP / TP bands overlap on length
  imbalance alone and are distinguished only by dominant size.
  Wild β clone-drift FP count → 0 in both Rust and Python.
  `tests/detector_clone_drift.rs` t29 / t30 / t30b / t31 pin the
  new gates structurally; t1–t28 all pass. Spec:
  `docs/spec/clone-drift-v0.md` F5d. Embedded priors recompute:
  clone-drift Wilson lower 0.355 → 0.676 (8 TP / 0 FP).

## Tier 1 — usable OSS (blocking for first announcement)

T1-1. GitHub Actions CI

- Status: `[x]`
- Summary: `.github/workflows/ci.yml` runs `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo fmt --all -- --check` on a Linux + macOS matrix.

T1-2. crates.io metadata for every crate

- Status: `[x]`
- Summary: root `Cargo.toml` carries `description`, `repository`,
  `keywords`, `categories`, `readme`, `license`;
  `cargo publish --dry-run` is green.

T1-3. README polish for OSS audience

- Status: `[x]`
- Summary: `README.md` opens with a one-paragraph pitch, badges
  (CI / crates.io / docs.rs / license), copy-pasteable quickstart,
  and explicit P1-P5 one-liners.

T1-4. examples directory

- Status: `[x]`
- Summary: ≥ 3 self-contained examples under `examples/` (scan,
  calibrate, adjudicate-with-mock-API); each is invoked by a CI
  smoke-test step.

T1-5. rustdoc on cntrdct-core public surface

- Status: `[x]`
- Summary: every public item carries a doc comment;
  `#![deny(missing_docs)]` is enforced and
  `cargo doc -p cntrdct --no-deps` is clean.

T1-6. LICENSE coverage review

- Status: `[x]`
- Summary: workspace is MIT-only; `cargo deny check licenses`
  passes. NOTICE / `LICENSES/` to be added if non-MIT deps land.

T1-7. GitHub Pages essay site

- Status: `[ ]` (retired 2026-05-12 without ever serving)
- Summary: Jekyll site under `docs/site/` (`_config.yml`, `index.md`,
  `essays/`) plus a `.github/workflows/pages.yml` deploy step were
  staged on 2026-05-03, but GitHub Pages was never enabled on the
  repository (`gh api repos/.../pages` returns 404 and
  `https://ktrysmt.github.io/cntrdct/` returns HTTP 404). The
  workflow's path filter kept it dormant until 2026-05-12, when an
  unrelated bump to the workflow file itself triggered the first
  run — which failed at `actions/configure-pages@v5` with
  `Get Pages site failed ... verify that the repository has Pages
  enabled`. `pages.yml` retired the same day; `docs/site/` source
  survives pending the external-blog migration scheduled under
  T3-13, and the broken README link to
  `https://ktrysmt.github.io/cntrdct/essays/citation-as-api/` was
  removed.

## Tier 2 — adoption-grade (drives external usage)

T2-7. Suppression mechanism

- Status: `[x]`
- Summary: in-source `#[cntrdct::allow(<detector_id>)]` plus
  project-wide `cntrdct.toml` for severity remapping, threshold
  overrides, and per-path allow/deny rules. Integration tests
  cover all three suppression paths.
- Followup landed in Q-9 (2026-05-07): the Python whole-file skip
  was replaced with a tree-sitter-python suppression scanner that
  recognises `# cntrdct: allow(<id>, ...)` line comments.

T2-8. Pre-built release binaries

- Status: `[x]`
- Summary: tag-driven release workflow uploads binaries for
  Linux x86_64 / aarch64, macOS aarch64, Windows x86_64;
  `curl | sh` install path works end-to-end.

T2-9. GitHub Action wrapper

- Status: `[x]`
- Summary: action consumes the pre-built binary and surfaces
  findings as PR comments matching GitHub Annotations
  conventions; sample workflow demonstrates inline findings.

T2-10. Parallel detection via rayon

- Status: `[x]`
- Summary: per-file detector runs execute in parallel;
  `tests/parallel_scan.rs:50-72` asserts byte-identical
  `Vec<Finding>` between serial and parallel runs.

T2-11. cargo cntrdct subcommand

- Status: `[x]`
- Summary: `cargo install cntrdct` ships a `cargo-cntrdct` shim so
  `cargo cntrdct scan` works alongside `cntrdct scan`.

## Tier 3 — polish (post-launch)

T3-12. LSP server

- Status: `[~]` (Phase 1 scaffolding landed 2026-05-08; Phase 1.b
  document events + Finding -> Diagnostic mapping landed 2026-05-08;
  Phase 1.c didChange debouncing landed 2026-05-09; Phase 1.c+
  per-URI generation counter landed 2026-05-09; Phases 2 / 3
  still pending)
- Goal: a `cntrdct-lsp` crate that exposes findings to IDEs
  (VS Code, Helix, Neovim) via the Language Server Protocol.
- Acceptance: a `vscode-cntrdct` extension or comparable
  client surfaces findings inline.
- Effort: 4-6 weeks (across all phases).
- Phase 1 — server scaffolding (done 2026-05-08): `cntrdct-lsp`
  binary added under the optional `lsp` Cargo feature
  (`tower-lsp` 0.20 + tokio multi-thread runtime). Implements the
  LSP lifecycle methods (`initialize` returning
  `text_document_sync = Full`, `initialized` logging via
  `window/logMessage`, `shutdown`). Spec: `docs/spec/lsp-v0.md`.
  CI gains a `clippy (lsp feature)` step so a future cntrdct API
  change that breaks `src/lsp.rs` fails CI rather than rotting
  silently. Default `cargo install cntrdct` is unchanged; LSP build
  is opt-in via `cargo install cntrdct --features lsp`.
- Phase 1.b — document events + Finding -> Diagnostic mapping
  (done 2026-05-08): `textDocument/{didOpen,didChange,didSave,didClose}`
  wired through to `textDocument/publishDiagnostics`. Buffer scan
  goes through a new `crate::scan_buffer` entry point that shares
  the Layer 1 detector battery with `scan_full_with_config` via
  the extracted `run_detectors_on` helper (so registration ordering
  lives in exactly one place). Severity, code, source, message,
  range (1-based → 0-based), `relatedInformation` (one entry per
  citation key, with the citation URL resolved through a static
  detector-citation registry when available, falling back to the
  buffer URI when the key is unknown), and `data` (verbatim
  `evidence.raw`) all follow the lsp-v0.md mapping table. Scans
  run on `tokio::task::spawn_blocking` so the event loop is not
  blocked while a multi-thousand-LOC buffer parses. CI exercises
  the new surface through `cargo test --features lsp --test
  lsp_smoke` (a subprocess JSON-RPC round-trip) and
  `cargo test --features lsp --lib lsp::tests` (seven unit tests
  pinning the Finding -> Diagnostic mapping); the `clippy (lsp
  feature)` step now passes `--all-targets` so the new test files
  are checked too.
- Phase 1.c — debouncing on didChange (done 2026-05-09): per-URI
  250 ms quiet window in `src/lsp.rs`. `did_change` now spawns a
  debounced task (`tokio::spawn` + `tokio::time::sleep`) instead of
  scanning inline; a successor `did_change` for the same URI calls
  `JoinHandle::abort()` on the prior handle and replaces it.
  `did_save` and `did_close` drain the per-URI pending map before
  acting so an explicit user action is not shadowed by a stale
  follow-up publish. `Cargo.toml` adds `time` + `sync` to the
  optional `tokio` features. New smoke test
  `did_change_debounces_rapid_bursts_to_one_publish` in
  `tests/lsp_smoke.rs` fires three notifications inside the window
  and asserts exactly one `publishDiagnostics` survives, carrying
  the most recent buffer state.
- Phase 1.c+ — per-URI generation counter (done 2026-05-09): the
  per-URI map evolves from `HashMap<Url, JoinHandle>` to
  `HashMap<Url, UriState>` where `UriState { handle: Option<JoinHandle>,
  latest_generation: u64 }`. Every event that produces a new scan
  (`did_open` / `did_change` / `did_save`) or invalidates pending
  work (`did_close`) bumps `latest_generation` atomically with the
  abort + spawn it performs; each scheduled scan captures the value
  at scheduling time and re-checks it after `spawn_blocking` returns,
  dropping its `publish_diagnostics` if a fresher event has overtaken
  it. This closes the documented race in which `JoinHandle::abort()`
  cannot interrupt a blocking-pool thread that is already inside the
  detector pass. Four new unit tests (`bump_generation_*`,
  `is_current_*`) in `src/lsp.rs::tests` pin the counter primitives;
  the existing `did_change_debounces_rapid_bursts_to_one_publish`
  smoke test continues to pass unchanged. The error-log path
  (`window/logMessage`) is intentionally left ungated — a stale scan
  that errored out describes a real failure the user wants to see.
- Phase 2 — `vscode-cntrdct` extension scaffolding (TypeScript /
  pnpm), bundling the LSP binary auto-downloaded from GitHub
  Releases. Separate repository under `ktrysmt/vscode-cntrdct`.
- Phase 3 — VS Code Marketplace listing + announcement.

T3-13. mdBook user guide

- Status: `[ ]`
- Goal: a `book/` directory hosting user-facing documentation
  (concepts, detector reference, configuration, FAQ) built with
  mdBook and published to GitHub Pages.
- Acceptance: `book/book.toml` builds without errors, all
  pages have at least placeholder content, and the published
  URL is linked from the README.
- Effort: 2-3 weeks.
- Note: the existing Jekyll essays under `docs/site/essays/` are
  scheduled to migrate to a separate external blog rather than be
  absorbed into mdBook. `pages.yml` was retired on 2026-05-12 (see
  T1-7) because GitHub Pages was never enabled on the repository
  and the workflow had failed on its first ever invocation; the
  `docs/site/` source itself stays in place until the external
  blog has absorbed the content, after which `docs/site/` will
  also be retired and the (re-enabled) GitHub Pages URL will serve
  the mdBook user guide alone.

T3-14. Distribution channels beyond crates.io

- Status: `[~]` (cargo-binstall and Homebrew tap landed 2026-05-08;
  AUR package deferred per maintainer decision)
- Goal: Homebrew tap, `cargo-binstall` metadata. AUR is no longer in
  scope.
- Effort: 1-2 days each.
- Depends on: T2-8.
- Summary: cargo-binstall metadata block in root `Cargo.toml` maps
  the existing release-archive layout
  (`cntrdct-v{version}-{target}/{cntrdct,cargo-cntrdct}{,.exe}`,
  tar.gz on Linux/macOS and zip on Windows); users can now run
  `cargo binstall cntrdct` to fetch the pre-built archive instead of
  compiling. The Homebrew tap lives at
  `ktrysmt/homebrew-cntrdct`, with `Formula/cntrdct.rb` covering
  macOS aarch64 and Linux x86_64/aarch64. The bump path is
  `.github/workflows/homebrew.yml` in this repo: it triggers on
  every `v*` tag push, polls for the release artifacts to be
  uploaded by `release.yml`, then rewrites and pushes
  `Formula/cntrdct.rb` in the tap repo. The `v0.2.0-rc.1` tag
  (2026-05-08) was the first end-to-end run; the workflow bumped
  the Formula from the seeded `0.2.0-beta.1` to `0.2.0-rc.1` with
  refreshed SHA256s on first try. README Quickstart documents both
  `brew tap ktrysmt/cntrdct` and `cargo binstall cntrdct`.
- Operational note: the bump workflow consumes the
  `HOMEBREW_TAP_TOKEN` repo secret on `ktrysmt/cntrdct` (a
  fine-grained PAT scoped to `ktrysmt/homebrew-cntrdct` with
  Contents: Read and write). The secret is registered as of
  2026-05-08; if it is ever rotated, the workflow exits with a
  pointer back to this entry on the next tag push.
- AUR (out of scope): originally listed as a distribution target,
  now dropped — the operational cost of maintaining an AUR account
  + submission flow is not justified by current Arch demand. If
  external demand materialises, reopen as a new T-series item.

T3-15. Auto-generated changelog

- Status: `[x]` 2026-05-08
- Summary: `cliff.toml` at repo root configures `git-cliff` against
  cntrdct's Conventional Commits prefixes (`feat` / `fix` / `perf` /
  `refactor` / `promote` / `docs` / `test` / `ci` / `chore`;
  `chore(release)` / `chore(changelog)` / `Merge` are skipped). The
  release workflow's `release` job now checks out with
  `fetch-depth: 0` and runs `orhun/git-cliff-action@v4` with
  `--latest --strip header`, then feeds the output into
  `softprops/action-gh-release` via `body_path: RELEASE_NOTES.md`,
  replacing the prior `generate_release_notes: true` path. CI side
  is fully self-contained: no local `git-cliff` install is required.
  `CONTRIBUTING.md` "Pull request review" updated so the squash-on-
  merge guidance points at the new pipeline; CLAUDE.md "Release
  procedure" non-negotiables documents the parser's drop list.
- Followup: a checked-in `CHANGELOG.md` and an auto-commit-back step
  on tag push were deferred until a future tag confirmed the
  release-body path is healthy in production. `v0.2.0-rc.1`
  (2026-05-08) is that confirmation — git-cliff produced the
  expected grouped output (Bug Fixes / CI / Chores / Documentation /
  Features) on first run with commit-link backrefs and a
  `compare/v0.2.0-beta.1..v0.2.0-rc.1` URL. The followup is now
  unblocked for whoever picks it up next; it is no longer a
  prerequisite for any other roadmap item, just an OSS-hygiene
  improvement.

T3-16. Telemetry-free assurance

- Status: `[x]` 2026-05-08
- Summary: a new `network-isolation` job in
  `.github/workflows/ci.yml` runs `cntrdct scan` inside a fresh
  Linux network namespace (`sudo unshare --net`) on every push and
  pull request. The namespace ships with no outbound routes; any
  unexpected network call from the scan path fails `ENETUNREACH` /
  `EAI_*` and the job goes red. The job exercises walker →
  parsers → Layer 1 detectors → Layer 2 ranker → Layer 4 SARIF
  emitter — i.e. everything that runs by default for end-users —
  and asserts a non-empty, well-formed SARIF document on stdout.
  The reqwest dependency stays constrained to `src/adjudicator.rs`
  (gated by the explicit `--adjudicate` flag) and `src/lib.rs`'s
  `wire_adjudicator` constructor. README.md carries a new
  "Network access" section documenting both the design property
  and the CI enforcement; the assurance has no opt-out path.
- Implementation note: the first attempt used the unprivileged
  `unshare -r --net` form, but Ubuntu 24.04's AppArmor
  `unprivileged_userns` profile blocks `/proc/self/uid_map` writes
  from non-root processes on GitHub-hosted runners
  (`unshare: write failed /proc/self/uid_map: Operation not
  permitted`). The fix was to drop the user-ns mapping entirely
  and run `sudo unshare --net` instead — passwordless sudo is
  available on GHA runners, and `--no-calibration` keeps the
  process from needing `$HOME` access since the priors are
  embedded into the binary via `include_str!`. Carried as a future
  signal: if GHA's runner image ever loosens the AppArmor profile,
  the unprivileged form is preferable for the smaller blast
  radius.

## Multi-language track (M-series)

Promotes cntrdct from a Rust-only linter to a multi-language one.
Strategic rationale: the differentiator (peer-reviewed citations on
every finding) is language-agnostic, and the commercial market for a
single-language linter is bounded. Pilot language is Python; the
architecture is built so subsequent languages (TypeScript, Go, Java)
plug in without rework.

This track interrupts Phase D — `P-2 pr-miner-rust detector` and
`P-4 ranker recalibration` are deferred until M-series completes so
new detectors are designed multi-language from day one rather than
retrofitted.

Constraint extension: P1 still binds. Each new language added to a
detector requires at least one peer-reviewed citation grounded in
empirical work on that target language; the existing Rust citation
does not transfer automatically. See `docs/spec/citations-policy.md`.

M-1. Language abstraction foundation

- Status: `[x]`
- Summary: `cntrdct::parsers` owns the `Language` enum and
  extension mapping. `ParsedFile.language` migrated from
  `String` to enum. The walker discovers all supported
  languages, not just `.rs`.

M-2. Pilot Python detector

- Status: `[x]`
- Summary: `unreachable-after-terminator` extended to Python
  (`raise`, `sys.exit()`, `os._exit()`, `assert False`,
  trailing-`return`). ≥ 5 positive + ≥ 3 negative Python fixtures
  shipped.
- Citation: `LanguageCitationStatus::Unconfirmed` per
  `docs/surveys/unreachable-after-terminator-python-2026-05.md`;
  P1 remains satisfied by two grandfathered Rust citations.

M-3. Cross-cutting detectors to Python

- Status: `[x]`
- Summary: `clone-drift` / `arg-swap` / `comment-code` extended
  to Python via internal `Language` dispatch (parameterised, not
  duplicated). Citation status:
  - `comment-code`: Unconfirmed
    (`docs/surveys/comment-code-python-2026-05.md`).
  - `arg-swap`: Confirmed via Allamanis, Jackson-Flux, Brockschmidt
    NeurIPS 2021 (PyBugLab / PyPIBugs).
  - `clone-drift`: Confirmed via Assi, Hassan, Zou TOSEM 2025
    (NiCad / SourcererCC on nine Python DL frameworks),
    DOI 10.1145/3721125; `MIN_FN_TOKENS = 22` size guard added.

M-4. Python β corpus

- Status: `[x]`
- Summary: `scripts/fetch_python_corpus.py` (stdlib-only) pins
  `(package, version, file_path)` triples from PyPI with SHA-256
  verification. v0 corpus = 11 files / 5 packages (six, attrs,
  click, idna, charset-normalizer) under
  `benchmarks/wild-corpus-python/`. `cntrdct eval` reports
  precision = 0.05, recall = 1.00, F1 = 0.10 — non-trivial
  precision is the M-4 acceptance signal. `ManifestEntry` extended
  with optional `source` / `license` / `sha256`
  (`#[serde(default)]`).

M-5. Surface multi-language across tooling

- Status: `[x]`
- Summary: `cntrdct.toml` `[languages.<canonical>]` section
  (`enabled`, `suppress`); GitHub Action `paths:` accepts
  `<path>:<lang_csv>` per-line entries via
  `prepare_config.py` / `merge_json.py` / `merge_sarif.py`. Mixed
  Rust+Python SARIF path verified by
  `sarif_emitter_handles_mixed_rust_and_python_unchanged`
  (`tests/multilang_config.rs`). Sample workflow:
  `examples/github-action-usage.yml`.

M-6. Citation policy for multi-language detectors

- Status: `[x]`
- Summary: `docs/spec/citations-policy.md` codifies P1 for the
  multi-language case (each supported language must carry at least
  one citation grounded in empirical work on that language).
  `tests/citations_consistency.rs` enforces the rule structurally;
  fails on a deliberately under-cited fixture detector.

## Tier 4 — community (opens contribution funnel)

T4-17. Issue templates

- Status: `[x]`
- Summary: `.github/ISSUE_TEMPLATE/{bug_report,feature_request,detector_proposal}.md`.
  `detector_proposal.md` requires citation key, citations-policy
  clause, IEEE 1044-2009 anomaly class, and the ≥ 8 positives-per-
  language commitment upfront.

T4-18. PR template

- Status: `[x]`
- Summary: `.github/PULL_REQUEST_TEMPLATE.md` covers Conventional
  Commit prefix, DCO sign-off, detector / corpus checklist, and
  gate boxes (`cargo test` / clippy / fmt).

T4-19. CONTRIBUTING.md

- Status: `[x]`
- Summary: `CONTRIBUTING.md` documents the two workspaces, the
  `promote(<area>)` rule, the detector authoring flow
  (proposal → spec → CITATIONS.md → implementation → corpus),
  Conventional Commits, DCO via `git commit -s` (no CLA), and PR
  review expectations. Carries an interim conduct paragraph until
  T4-20 lands.

T4-20. Code of Conduct

- Status: `[ ]`
- Goal: `CODE_OF_CONDUCT.md` based on Contributor Covenant 2.1.
- Effort: 15 minutes.
- Note: deferred until external contributor activity or GitHub
  Discussions warrants the operational overhead (running an
  enforcement contact and triage path). At adoption time the file
  will be a short pointer to the canonical Contributor Covenant URL
  rather than an inline copy. `CONTRIBUTING.md` carries an interim
  conduct paragraph until then.

T4-21. Roadmap discussion pinned

- Status: `[ ]`
- Goal: a GitHub Discussion thread that surfaces this roadmap
  and invites community input on prioritisation.
- Effort: 15 minutes.

## Quality-audit track (Q-series)

Beta-stage wiring fixes, governance hardenings, and methodology
lifts identified during the post-beta.1 quality audit. Q-1 through
Q-5 are RC1 blockers (release-tag prerequisites for v0.2.0-beta.2 /
v0.2.0-rc.1); Q-6 through Q-10 are RC1 governance / hygiene must-
haves; Q-11 through Q-16 target RC2 / v0.2.0 stable.

Q-1. SARIF detectors array missing pr-miner

- Status: `[x]` 2026-05-07
- Summary: `PrMinerDetector` re-added to the SARIF detectors vec
  in `src/main.rs` so `runs[0].tool.driver.rules[]` carries a
  `pr-miner` entry alongside the five Layer 1 peers. The
  `sarif_emitter_handles_mixed_rust_and_python_unchanged` test in
  `tests/multilang_config.rs` now asserts the full
  `cntrdct::ALL_DETECTOR_IDS` set is present in
  `tool.driver.rules` so the regression is caught at CI rather than
  after release.

Q-2. SARIF informationUri placeholder

- Status: `[x]` 2026-05-07
- Summary: `INFORMATION_URI` at `src/sarif.rs:15` is now
  `https://github.com/ktrysmt/cntrdct`, matching `Cargo.toml`'s
  `repository`. `docs/spec/sarif-v0.md` F3 updated to reflect the
  canonical URL. A `grep -RE 'TBD' src/` gate added to the
  `rustfmt` job in `.github/workflows/ci.yml` fails CI on a
  deliberately reintroduced placeholder.

Q-3. clone-drift doc-comment / value drift

- Status: `[x]` 2026-05-07
- Summary: doc comment on `NEAR_DUPLICATE_THRESHOLD`
  (`src/detectors/clone_drift.rs:38-50`) rewritten to describe
  the 0.7 threshold and its effective drift band, and to point at
  `docs/spec/clone-drift-v0.md` F5c-ii (which already documents
  the same value). The previous "0.85" mention left over from a
  draft before P-6's strict-majority + Jaccard gate landed is
  gone.

Q-4. Wiring consistency test

- Status: `[x]` 2026-05-07
- Summary: `cntrdct::ALL_DETECTOR_IDS` introduced as the single
  source of truth for the Layer 1 detector set.
  `tests/wiring_consistency.rs` asserts that (a) detector
  constructions matching `src/lib.rs::scan_full_with_config` and
  (b) the SARIF rules taxonomy emitted by the `cntrdct` binary
  (`src/main.rs`) both equal that constant.
  `tests/prereg_consistency.rs::registered_detectors` now
  includes `PrMinerDetector` and a new
  `registered_detectors_match_canonical_id_set` test pins it
  against `ALL_DETECTOR_IDS`. Removing any detector from any one
  of the three sites fails the suite.

Q-5. SARIF Severity::Info mapping rationale

- Status: `[x]` 2026-05-07
- Summary: `docs/spec/sarif-v0.md` F5 carries a decision-log
  entry that retains `Severity::Info → SARIF "none"`. Rationale:
  no shipped detector emits `Info` by construction (the variant
  enters only via user-authored `cntrdct.toml` severity
  overrides), so a user explicitly downgrading a finding to
  `Info` is signalling "less visible than `Note`" — which is
  exactly the GitHub Code Scanning behaviour for `none`-level
  findings. The original `raw_severity` is recoverable from
  `result.properties.raw` for SARIF consumers that need the full
  four-valued vocabulary.

Q-6. Citation retraction monitor

- Status: `[x]` 2026-05-07
- Summary: `scripts/check_retractions.py` extracts every DOI from
  `CITATIONS.md` and the `doi: Some("...")` slots of every
  `Citation` static array under `src/`, then cross-references them
  against (a) the cached Retraction Watch snapshot at
  `benchmarks/retraction-watch/cache.csv` (SHA-256-pinned by
  `cache.sha256`; mismatch fails CI) and (b) Crossref Works'
  `update-to` field with `type: "retraction"` (skipped under
  `--no-network`). `.github/workflows/citations.yml` runs the
  monitor on every push / PR and a Mondays-06:00-UTC cron refreshes
  the cache via the Crossref Labs Retraction Watch endpoint, opening
  a `chore(citations): refresh Retraction Watch cache` PR when the
  snapshot changes (gated on the `RETRACTION_WATCH_EMAIL` repo
  secret). The fixture under
  `tests/fixtures/retraction-watch/{citations.md,cache.csv,cache.sha256}`
  plants a synthetic-DOI retraction (`10.99999/cntrdct-q6-...`); the
  workflow's smoke step asserts the script exits 1 on it, so a future
  loosening of the matcher fails CI rather than silently re-opening
  the path. Evidence: Fong & Wilhite (2017) PLOS ONE 12(12),
  e0187394; COPE (2019) discussion document on citation
  manipulation.

Q-7. Venue tier whitelist

- Status: `[x]` 2026-05-07
- Summary: `docs/spec/citations-policy.md` carries a "Venue tier
  whitelist" section enumerating Tier-A (ICSE / FSE / OOPSLA /
  PLDI / POPL / ASE / ISSTA / EMSE / TOSEM / IEEE TSE plus
  adjacent SOSP / OSDI / EuroSys / NeurIPS / ICML / USENIX
  Security / S&P / CCS) and Tier-B (ICPC / ICSM / ICSME / MSR /
  SANER / WCRE / SCAM / ICST / ISSRE / JSS / IST). Tier-C is
  documented but starts empty; entries emit CI warnings rather
  than failures so grandfather clauses stay workable.
  `tests/citations_consistency.rs` adds
  `every_shipped_detector_citation_has_known_tier`,
  `fabricated_fixture_venue_is_rejected`, and
  `venue_tier_examples_classify_as_documented`. The fixture's
  fabricated venue (`"Fixture"`) is asserted to be unrecognised so
  the rejection path is pinned structurally; all six shipped
  detectors classify into Tier-A or Tier-B.

Q-8. Preregistration deviation log

- Status: `[x]` 2026-05-07
- Summary: `prereg/deviations/<date>-<topic>.md` is the new
  audit-trail surface for any preregistration revision carrying
  a `Supersedes:` header. Three back-filled entries land the
  retroactive 2026-05-03 → 2026-05-05 → 2026-05-06 → 2026-05-07
  supersession chain:
  `prereg/deviations/2026-05-05-multilang-rollup.md`,
  `prereg/deviations/2026-05-06-clone-drift-python.md`,
  `prereg/deviations/2026-05-07-wild-beta-fp-reduction.md`.
  `tests/prereg_consistency.rs` adds three new tests:
  `every_supersession_has_a_matching_deviation_log`,
  `deviation_logs_carry_required_headers`, and
  `deviation_log_supersedes_resolves_to_a_real_prereg_file`. A
  future revision with a `Supersedes:` line but no matching
  `prereg/deviations/<date>-*.md` fails the suite. Ungrounded
  per-deviation rationale is the documented Q-8 failure mode (van
  den Akker et al. 2024, doi:10.1037/met0000687); the three
  required headers (`Prereg:` / `Supersedes:` / `Author:` /
  `Date:`) keep the audit trail machine-checkable.

Q-9. Python attribute-style suppression

- Status: `[x]` 2026-05-07
- Summary: the wholesale Python skip in
  `collect_attribute_suppressions` (formerly an early
  `if file.language != Language::Rust { return vec![]; }`) is
  replaced by a per-language dispatch
  (`collect_rust_suppressions` / `collect_python_suppressions`).
  The Python path drives tree-sitter-python via
  `crate::parsers::parser_for(Language::Python)` (Q-10 seam) and
  recognises two forms of `# cntrdct: allow(<id>, ...)`:
  - Trailing comment on a code line — suppression range is the
    single comment line.
  - Standalone whole-line comment — suppression range covers the
    next non-comment named sibling (function / class / statement),
    mirroring the Rust attribute-precedes-item shape.
  `# cntrdct: allow()` is the catch-all (matches the Rust empty
  argument list). New unit tests in `src/config.rs` cover trailing
  / standalone / catch-all / wrong-id paths; integration tests in
  `tests/multilang_config.rs` (`python_attribute_allow_*`) drive the
  full scan + apply pipeline through both forms over the existing
  `PYTHON_ARG_SWAP` corpus and confirm that Rust findings on the
  same scan stay intact. The Q-10 parser seam was extended to
  cover `src/config.rs` at the same time, so adding M-7+ languages
  is still a single-module change in `src/parsers.rs`.

Q-10. ParserProvider seam tightening

- Status: `[x]` 2026-05-07
- Summary: every detector now reaches tree-sitter through
  `crate::parsers::parser_for(Language::*).ts_language()`. Eleven
  direct call sites across `arg_swap`, `clone_drift`,
  `comment_code`, `config_interaction`,
  `unreachable_after_terminator`, `pr_miner::extract_rust`, and
  `pr_miner::extract_python` were rewritten. A new
  `parser seam` step in `.github/workflows/ci.yml` greps
  `src/detectors/` for `tree_sitter_*::language()` and fails CI on
  any reintroduction, so a future M-7+ language addition is a
  single-module change in `src/parsers.rs`.

Q-11. Small-N statistical interval switching

- Status: `[x]` 2026-05-10
- Summary: `compute_priors` now switches between Wilson and a
  Beta(1, 1) Bayes-Laplace 95% lower bound based on cell size.
  The switch lives in `compute_lower_bound(tp, fp)` in
  `src/calibration.rs`: at `tp + fp >= SMALL_SAMPLE_THRESHOLD`
  (n = 30) it returns Wilson; below that, the Beta(1, 1) lower
  2.5% quantile, with the BCD 2001 §4 boundary modification at
  `tp = 0` (return 0 to align with Wilson at the same cell). The
  new `PriorMethod` enum is stored on `DetectorPrior` and propagated
  through `RankedFinding.prior_method` and SARIF
  `result.properties.priorMethod`. Field name `wilson_lower_95`
  is preserved on `DetectorPrior` for back-compat with pre-Q-11
  per-user cache files; `serde(default)` keeps old JSON loadable.
  `tests/ranker_small_sample.rs` (8 tests) gates the switching
  threshold, the boundary modification, distinguishability of the
  two methods at intermediate `(tp, fp)`, both methods staying
  near nominal one-sided lower coverage at `n >= 30`, and
  end-to-end `prior_method` propagation through the calibrated
  ranker, the uncalibrated fallback, and the SARIF emitter.
  Embedded `benchmarks/priors-default.json` regenerated: five of
  six shipped detectors now carry `prior_method: "jeffreys"`
  (only `pr-miner` at n=38 stays on Wilson). Spec
  `docs/spec/ranker-v1.md` adds the Q-11 section. CITATIONS.md
  adds `brown-cai-dasgupta-stat-sci-2001` and `thulin-ejs-2014`.
- Honesty note: the original "Jeffreys is closer to nominal than
  Wilson at n < 30" framing in the acceptance criterion does not
  hold robustly under one-sided lower coverage averaged over `p`
  (debug numbers: at small `n`, Wilson's mean coverage error sits
  marginally below Jeffreys' on a uniform-`p` grid, regardless of
  the boundary modification). The realised acceptance test gates
  the structurally provable properties Q-11 actually depends on
  rather than the brittle coverage-superiority claim. The
  calibrator still picks Jeffreys at `n < 30` for the
  methodological reason captured in `docs/spec/ranker-v1.md`
  ("Q-11 design notes"): `posterior_tp` is already a Beta(1, 1)
  Bayesian update, so a Beta(1, 1) credible-interval lower bound
  is the regime-coherent companion at small `n`.
- Evidence: Brown, Cai, DasGupta (2001) Statistical Science
  16(2), 101-133, doi:10.1214/ss/1009213286 (boundary modification
  + small-N regime); Thulin (2014) Electronic Journal of
  Statistics 8(1), 817-840, doi:10.1214/14-EJS909 (independent
  argument for Beta-prior credible bounds at small N).

Q-12. LLM calibration post-hoc Platt fit

- Status: `[x]` 2026-05-10
- Summary: the adjudicator prompt no longer requests a verbalised
  `calibration_tag`; the response parser still reads the field
  (`Option<String>`) so adjudication records collected before Q-12
  round-trip cleanly. Post-hoc Platt scaling fit per
  `(detector_id, anomaly_class)` cell replaces the verbalised tag.
  `cntrdct calibrate --fit-platt <CORPUS>` (extension of the
  existing `calibrate` subcommand) reads a JSONL of
  `LabelledLlmConfidence` rows and writes the per-cell `(a, b)`
  registry to
  `benchmarks/llm-calibration/platt-default.json` (or `--output
  <PATH>`); the file is `include_str!`-embedded into the binary so
  a fresh `cargo install cntrdct` ships with calibration ready.
  v0 ships an empty `{}` registry so `apply_llm_calibration` is a
  no-op until a real labelled adjudication corpus is fit. Wiring:
  `AdjudicationResult.calibrated_confidence: Option<f64>`,
  `cntrdct::apply_llm_calibration`, SARIF
  `result.properties.adjudication.calibrated_confidence` (omitted
  when `None`). `tests/calibration_ece.rs` runs end-to-end on a
  constructed-pathology fixture (over-confidence at 0.95/0.85/0.75
  raw with empirical accuracy ≈ 0.5) and asserts holdout ECE drops
  by ≥ 0.05 after Platt; on the shipped fixture raw ECE 0.256 →
  calibrated ECE near 0.001. Spec
  `docs/spec/llm-calibration-v0.md`. Citations: `platt-1999` and
  `spiess-koohestani-sergeyuk-2025` added under Layer 3
  (CITATIONS.md + `ADJUDICATOR_CITATIONS`).
- Evidence: Spiess, Koohestani, Sergeyuk (2025) arXiv:2510.22614;
  Spiess et al. (2025) ICSE 2025; J. Platt (1999), "Probabilistic
  Outputs for Support Vector Machines and Comparisons to
  Regularized Likelihood Methods", Advances in Large Margin
  Classifiers (MIT Press).

Q-13. Cross-model κ audit

- Status: `[x]` 2026-05-11 (CLI-shellout redesign 2026-05-11)
- Summary: on-demand 2-family κ audit between Claude Code's
  `claude --print` and the Gemini CLI's `gemini -p`. Three providers
  ship behind the new `PromptDispatch` trait:
  `AnthropicAdjudicator` (HTTP via `reqwest`, retained for
  `scan --adjudicate`), `ClaudeCliAdjudicator` (CLI shellout with
  `--system-prompt` / `--tools ""` / `--strict-mcp-config` /
  `--no-session-persistence` / `--output-format json` so Claude
  Code's agentic persona and tool surface are fully stripped), and
  `GeminiCliAdjudicator` (CLI shellout with `GEMINI_SYSTEM_MD` env
  override pointing at a temp file, `--output-format json`). Both
  CLI providers spawn the subprocess with `current_dir = <tempdir>`
  to suppress CLAUDE.md / GEMINI.md auto-discovery. Auth is
  delegated to each CLI's own login (no API keys read by cntrdct).
  Module `src/cross_model_kappa.rs` carries the pure `cohen_kappa`
  helper, per-`(detector_id, anomaly_class)` aggregation, the
  audit-report serde shapes, and stdlib-only date helpers. CLI:
  `cntrdct cross-model-kappa <CORPUS>` accepts JSONL or JSON-array
  ranked-finding corpora; default output is pretty JSON to stdout,
  `--output PATH` writes to disk. PR CI exercises κ aggregation via
  `tests/cross_model_kappa.rs` with `CannedDispatch` and pins the
  CLI flag set via stub-script tests in `src/adjudicator.rs::tests`.
- Design pivots (documented in
  `docs/spec/cross-model-kappa-v0.md` "Design rationale"):
  - Codex CLI dropped because `codex exec` cannot replace the
    system prompt (only `developer_instructions` additive), so
    Codex's residual persona would have confounded the κ signal.
  - OpenAI / Google API-key paths replaced by CLI shellout — users
    authenticate via subscription, not API keys.
  - Nightly CI workflow dropped. Continuous monitoring was unsupported
    by measurement stationarity: commercial LLMs version-bump
    silently, sampler stochasticity at temperature 0 still produces
    variance, and the time-series κ would have captured noise more
    than any cntrdct-side property. The audit ships as an
    on-demand snapshot only.
- Evidence: Wataoka, Takahashi, Ri (2024) arXiv:2410.21819;
  Zheng et al. (2023) NeurIPS 36, 46595-46623; Cohen (1960) and
  Landis & Koch (1977) for the κ statistic and substantial-agreement
  threshold.
- Spec: `docs/spec/cross-model-kappa-v0.md`.

Q-14. Recall-audit harness

- Status: `[~]` (Phase A scaffolding + Phase B first batch
  landed 2026-05-11; Phase B batches 2-7 landed 2026-05-12 (see
  per-batch summaries below); Phase B batches 8 and 9 landed
  2026-05-13, closing the originally-named Phase B blockers and
  deepening the pr-miner FN denominator on the `semgrep` source
  kind; Phase B batch 10 landed 2026-05-13 on top of batch 9,
  surfacing the first pr-miner TPs by adding ten density-support
  files containing 18 paired open+close top-level Python
  transactions — confidence (1+18)/(4+18) = 0.864 ≥ 0.85 crosses
  the F3 mining gate and both batch-8/9 FN entries flip to TPs
  without any detector-side change. pr-miner moves from
  0/2/0.00 to 2/0/1.00; overall recall_upper_bound jumps from
  0.26 to 0.32. Phase B batch 11 landed 2026-05-13 on top of
  batch 10, adding two `comment-code` TPs from a second
  permissive-licensed Rust upstream (rusticata/tls-parser
  `src/tls_handshake.rs`, MIT OR Apache-2.0) carrying the same
  Tan SOSP 2007 §3.2 Pattern C bug shape (`/// Deprecated`
  prose without `#[deprecated]` attribute) the batch-3
  `sidan-lab/whisky-archive` con_str* family flags;
  `comment-code` moves to 6/0/1.00 and overall
  recall_upper_bound to 0.36 (12 TP / 21 FN / 33 expected).
  Phase B batch 12 landed 2026-05-13 on top of batch 11, adding
  one `comment-code` TP from a third permissive-licensed Rust
  upstream (glium/glium `src/draw_parameters/mod.rs`,
  Apache-2.0) carrying the same Pattern C bug shape on a third
  unrelated domain (OpenGL bindings); `comment-code` moves to
  7/0/1.00 and overall recall_upper_bound to 0.38 (13 TP /
  21 FN / 34 expected). Phase B batch 13 landed 2026-05-14 on
  top of batch 12, adding one `comment-code` TP from a fourth
  permissive-licensed Rust upstream (rust-lang/pkg-config-rs
  `src/lib.rs`, MIT OR Apache-2.0) carrying the same Pattern C
  bug shape on a fourth unrelated domain (build-tool /
  system-package bindings); the `#[doc(hidden)]` attribute the
  function carries does NOT suppress Pattern C because the
  walker checks the literal first identifier (`doc` vs.
  `deprecated`), confirming Pattern C distinguishes
  visibility-hiding from runtime-lint enforcement;
  `comment-code` moves to 8/0/1.00 and overall
  recall_upper_bound to 0.40 (14 TP / 21 FN / 35 expected).
  Phase B batch 23 landed 2026-05-16 on top of batch 22,
  diversifying `comment-code` Pattern C audit coverage from
  seven upstreams (whisky-archive Cardano Plutus-data helpers
  4 + tls-parser TLS NextProtocol parsers 2 + glium OpenGL
  draw-parameter check 1 + pkg-config-rs Unix pkg-config
  bindings 1 + sui mysten-metrics async-channel metrics
  wrapper 2 + vcpkg-rs Windows vcpkg bindings 1 + rust-vst2
  VST 2.4 audio plugin host 1, batches 3 / 11 / 12 / 13 / 20 /
  21 / 22) to eight upstreams by adding one TP from a
  fourteenth permissive-licensed Rust upstream
  (always-further/nono@2e5504f2
  `crates/nono-cli/src/deprecated_schema.rs`, Apache-2.0).
  `pub fn warn_for_deprecated_flags(args:
  &[std::ffi::OsString])` at upstream line 196 (corpus line
  10) carries a four-line `///` doc block whose later half
  (after a blank `///` separator) reads `DEPRECATED: delete
  when all long-flag aliases in this module are removed / in
  v1.0.0. See module-level removal steps.` but does not carry
  the `#[deprecated]` runtime attribute the Rust deprecation
  lints honour, so downstream consumers receive no compiler
  warning — the textbook Tan SOSP 2007 §3.2 Pattern C bug
  shape. Introduces the meta-deprecation-warning-emitter sub-
  shape within Pattern C: the function body iterates
  `detect_deprecated_flags(args)`, matches each canonical
  legacy spelling against its replacement (`--override-deny` →
  `--bypass-protection`), and calls `emit_deprecation_warning(
  ...)` — its purpose is to emit per-call-site diagnostics for
  OTHER deprecated long-flag aliases routed through clap,
  while itself being marked deprecated in the module-level
  removal checklist; the function and the flags it warns about
  are co-scheduled for removal in v1.0.0, so the doc claim is
  forward-looking rather than legacy. This contrasts prior
  body shapes that delegate to the replacement (batch-3
  whisky-archive con_str → constr, batch-13 pkg-config-rs
  find_library → probe_library, batch-21 vcpkg-rs
  probe_package → find_package), prior body shapes that retain
  the original implementation in-tree (batch-11 tls-parser
  parse_tls_handshake_*next_protocol family, batch-12 glium
  validate, batch-20 sui mysten-metrics channel /
  channel_with_total), and the batch-22 rust-vst2 stub-body
  shape (empty `{ }` ABI placeholder). cntrdct's spec F5
  Pattern C check ignores the body — only the doc and adjacent
  attributes are inspected — so the meta-warning-emitter case
  fires identically to delegate-body, in-tree-body, and
  stub-body cases, confirming the syntactic-only design. nono
  is the capability-based sandbox CLI domain (fine-grained
  policy brokering for agent operating contexts, zero setup
  and zero latency), unrelated to the prior seven Pattern C
  domains. `comment-code` moves to 25/0/1.00 and overall
  recall_upper_bound to 0.60 (31 TP / 21 FN / 52 expected, up
  from 0.59 at batch 22); Pattern C is now exercised on eight
  unrelated upstreams (Cardano Plutus-data + TLS NextProtocol
  + OpenGL draw-parameters + Unix pkg-config + async-channel
  metrics + Windows vcpkg + VST 2.4 audio plugin host +
  capability-based sandbox CLI), lifted from seven at batch
  22. Pattern A and Pattern B remain saturated at three
  sub-shapes across three upstreams each (batches 18 and 19).
  Phase B batch 22 landed 2026-05-16 on top of batch 21,
  diversifying `comment-code` Pattern C audit coverage from
  six upstreams (whisky-archive Cardano Plutus-data helpers 4
  + tls-parser TLS NextProtocol parsers 2 + glium OpenGL
  draw-parameter check 1 + pkg-config-rs Unix pkg-config
  bindings 1 + sui mysten-metrics async-channel metrics
  wrapper 2 + vcpkg-rs Windows vcpkg bindings 1, batches 3 /
  11 / 12 / 13 / 20 / 21) to seven upstreams by adding one TP
  from a thirteenth permissive-licensed Rust upstream
  (overdrivenpotato/rust-vst2@244e14bd `src/interfaces.rs`,
  MIT). `pub fn process_deprecated(_effect: *mut AEffect,
  _inputs_raw: *mut *mut f32, _outputs_raw: *mut *mut f32,
  _samples: i32) { }` at upstream line 17 (corpus line 6)
  carries a single-line `///` doc block reading `Deprecated
  process function.` but does not carry the `#[deprecated]`
  runtime attribute the Rust deprecation lints honour, so
  downstream consumers receive no compiler warning — the
  textbook Tan SOSP 2007 §3.2 Pattern C bug shape. Introduces
  a new body-shape variant within Pattern C: the function
  body is the empty block `{ }` retained as an ABI / API
  placeholder for callers that link against the old symbol
  while migrating to the replacement `process_replacing` /
  `process_replacing_f64` family, contrasting prior body
  shapes that delegate to the replacement (batch-3
  whisky-archive con_str → constr, batch-13 pkg-config-rs
  find_library → probe_library, batch-21 vcpkg-rs
  probe_package → find_package) and prior body shapes that
  retain the original implementation in-tree (batch-11
  tls-parser parse_tls_handshake_*next_protocol family,
  batch-12 glium validate, batch-20 sui mysten-metrics
  channel / channel_with_total). cntrdct's spec F5 Pattern C
  check ignores the body — only the doc and adjacent
  attributes are inspected — so the stub-body case fires
  identically to delegate-body and in-tree-body cases,
  confirming the syntactic-only design. rust-vst2 is the
  audio / DSP plugin host domain (VST 2.4 API implementation
  in Rust for creating audio plugins and hosts), unrelated to
  the prior six Pattern C domains. `comment-code` moves to
  24/0/1.00 and overall recall_upper_bound to 0.59 (30 TP /
  21 FN / 51 expected, up from 0.58 at batch 21); Pattern C
  is now exercised on seven unrelated upstreams (Cardano
  Plutus-data + TLS NextProtocol + OpenGL draw-parameters +
  Unix pkg-config + async-channel metrics + Windows vcpkg +
  VST 2.4 audio plugin host), lifted from six at batch 21.
  Pattern A and Pattern B remain saturated at three
  sub-shapes across three upstreams each (batches 18 and 19).
  Phase B batch 21 landed 2026-05-16 on top of batch 20,
  diversifying `comment-code` Pattern C audit coverage from
  five upstreams (whisky-archive Cardano Plutus-data helpers 4
  + tls-parser TLS NextProtocol parsers 2 + glium OpenGL
  draw-parameter check 1 + pkg-config-rs build-tool /
  system-package bindings 1 + sui mysten-metrics async-channel
  metrics wrapper 2, batches 3 / 11 / 12 / 13 / 20) to six
  upstreams by adding one TP from a twelfth permissive-
  licensed Rust upstream (mcgoo/vcpkg-rs@fa42994a `src/lib.rs`,
  MIT OR Apache-2.0). `pub fn probe_package(name: &str) ->
  Result<Library, Error>` at upstream line 293 (corpus line 7)
  carries a single-line `///` doc block reading `Deprecated in
  favor of the find_package function` and a `#[doc(hidden)]`
  attribute, but no `#[deprecated]` runtime attribute the Rust
  deprecation lints honour, so spec F5 Pattern C fires — the
  textbook Tan SOSP 2007 §3.2 Pattern C bug shape. Structurally
  pairs with batch-13 pkg-config-rs as the Windows side of the
  build-script system-package binding family: pkg-config-rs
  covers Unix pkg-config tool bindings; vcpkg-rs covers
  Microsoft Windows vcpkg C/C++ package manager bindings. Both
  upstreams independently exhibit the same Pattern C bug shape
  (`/// Deprecated in favor of X` single-line doc +
  `#[doc(hidden)]` non-suppressive attribute + body delegating
  to the replacement function) on functionally-paired
  native-library-discovery crates targeting different platform
  ecosystems — exercising cntrdct's Pattern C check on cross-
  platform-paired upstreams that independently rediscovered the
  same `Deprecated`-prose-without-`#[deprecated]`-attribute
  oversight. The `#[doc(hidden)]` attribute does NOT suppress
  Pattern C because the walker checks the literal first
  identifier (`doc` vs. `deprecated`), confirming again that
  Pattern C distinguishes visibility-hiding from runtime-lint
  enforcement. `comment-code` moves to 23/0/1.00 and overall
  recall_upper_bound to 0.58 (29 TP / 21 FN / 50 expected, up
  from 0.57 at batch 20); Pattern C is now exercised on six
  unrelated upstreams (Cardano Plutus-data + TLS NextProtocol
  + OpenGL draw-parameters + Unix pkg-config + async-channel
  metrics + Windows vcpkg), lifted from five at batch 20.
  Pattern A and Pattern B remain saturated at three sub-shapes
  across three upstreams each (batches 18 and 19).
  Phase B batch 20 landed 2026-05-15 on top of batch 19,
  diversifying `comment-code` Pattern C audit coverage from
  four upstreams (whisky-archive Cardano Plutus-data helpers
  4 + tls-parser TLS NextProtocol parsers 2 + glium OpenGL
  draw-parameter check 1 + pkg-config-rs build-tool /
  system-package bindings 1, batches 3 / 11 / 12 / 13) to
  five upstreams by adding two TPs from an eleventh
  permissive-licensed Rust upstream (MystenLabs/sui@add9d472
  `crates/mysten-metrics/src/metered_channel.rs`,
  Apache-2.0). `pub fn channel<T>(size: usize, gauge:
  &IntGauge) -> (Sender<T>, Receiver<T>)` at upstream line
  321 (corpus line 8) carries a two-line `///` doc block
  whose second line reads `Deprecated: use
  `monitored_mpsc::channel` instead.`; `pub fn
  channel_with_total<T>` at upstream line 339 (corpus line
  26) carries a single-line `///` doc block reading the same.
  Both functions carry the `#[track_caller]` attribute (which
  propagates the caller location for panic reporting but does
  NOT trigger the Rust deprecation lints) and neither carries
  the `#[deprecated]` runtime attribute the Rust deprecation
  lints honour, so spec F5 Pattern C fires on both — the same
  Tan SOSP 2007 §3.2 Pattern C bug shape batches 3 / 11 / 12 /
  13 flag, on a fifth unrelated domain (async-channel metrics
  wrapper in Sui's blockchain runtime observability layer).
  The `#[track_caller]` attribute is structurally analogous to
  batch-13 pkg-config-rs's `#[doc(hidden)]` non-suppressive
  attribute: both are attributes present alongside the
  deprecation prose without triggering the `#[deprecated]`
  lint, confirming again that Pattern C distinguishes the
  literal first identifier of the attribute path
  (`track_caller` / `doc` vs. `deprecated`) rather than
  interpreting the attribute's behavioural semantics.
  `comment-code` moves to 22/0/1.00 and overall
  recall_upper_bound to 0.57 (28 TP / 21 FN / 49 expected, up
  from 0.55 at batch 19); Pattern C is now exercised on five
  unrelated upstreams (Cardano Plutus-data + TLS NextProtocol
  + OpenGL draw-parameters + build-tool / system-package +
  async-channel metrics), lifted from four at batch 13.
  Pattern A and Pattern B remain saturated at three sub-shapes
  across three upstreams each (batches 18 and 19).
  Phase B batch 19 landed 2026-05-15 on top of batch 18,
  diversifying `comment-code` Pattern B audit coverage from
  two upstreams (zarrs silent-drop-on-mismatch +
  parking_lot_core callback-contract) to three upstreams by
  adding one TP from a tenth permissive-licensed Rust upstream
  (vortex-data/vortex@4c1ae92d `vortex-buffer/src/bit/mod.rs`,
  v0.70.0 release tag, Apache-2.0). `pub fn get_bit(buf: &[u8],
  index: usize) -> bool` at upstream line 33 (corpus line 11)
  carries a `///` doc block whose `# Panics` section reads
  `Panics if `index` is not between 0 and length of `buf * 8`.`
  cntrdct's spec F4 Pattern B trigger fires on the case-folded
  `panic` substring; the body `buf[index / 8] & (1 << (index %
  8)) != 0` contains NONE of the Pattern B body markers
  (`panic!` / `unwrap` / `expect(` / `unreachable!` /
  `assert!` / `assert_eq!` / `assert_ne!` / `todo!` /
  `unimplemented!` / `debug_assert`), so spec F4's body-marker
  negation passes and Pattern B fires. The body DOES panic on
  the documented out-of-bounds condition (slice bracket
  indexing on `&[u8]` panics through the `core::ops::Index`
  impl for `[T]` when `index / 8 >= buf.len()`, equivalent to
  `index >= buf.len() * 8` modulo the upstream `buf * 8` typo
  for `buf.len() * 8`), but the panic is implicit in the
  slice `Index` impl rather than expressed via any of
  cntrdct's syntactic body-marker substrings, so cntrdct's
  spec F4 fires on syntactic substring rule rather than
  semantic correctness. Introduces the third Pattern B
  sub-shape: implicit-indexing-panic (body uses bracket
  indexing that panics on OOB through the slice `Index` impl
  without any of cntrdct's body-marker substrings),
  contrasting batch 14's zarrs silent-drop-on-mismatch sub-
  shape (body uses `as_chunks_mut().0` which silently drops
  the trailing remainder rather than panicking) and batch
  16's parking_lot_core callback-contract sub-shape (doc says
  "must not panic" as a contract on the supplied callback,
  function body has no panic-producing constructs).
  `comment-code` moves to 20/0/1.00 and overall
  recall_upper_bound to 0.55 (26 TP / 21 FN / 47 expected, up
  from 0.54 at batch 18); Pattern B is now exercised across
  all three syntactic-Pattern-B sub-shapes on three unrelated
  upstreams (Zarr-format codec helpers + parking_lot queue
  primitives + vortex-buffer bit-packed bitmap helpers), and
  every `docs/spec/comment-code-v0.md` pattern has at least
  two upstreams of regression-detection insurance after batch
  19 — with both Pattern A and Pattern B exercised across all
  three sub-shapes on three upstreams each. Phase B batch 18
  landed 2026-05-15 on top of batch 17,
  diversifying `comment-code` Pattern A audit coverage from
  two upstreams (boundless silent-absorb-and-log + wasmtime
  documented-panic-on-failure) to three upstreams by adding
  one TP from a ninth permissive-licensed Rust upstream
  (durch/rust-s3@771be165 `s3/src/lib.rs`, MIT). `pub fn
  set_retries(retries: u8)` at upstream line 54 (corpus line
  23) carries a `///` doc block whose first line reads `Sets
  the number of retries for operations that may fail and need
  to be retried.`; the substring `may fail` fires the same
  Pattern A check, the function signature `pub fn set_retries(
  retries: u8)` has unit return so F3's return-type negation
  passes, and the body
  `RETRIES.store(retries, std::sync::atomic::Ordering::SeqCst);`
  is unconditionally infallible so neither spec F4 Pattern B
  nor spec F5 Pattern C fires (the doc contains no `panic` or
  `deprecated` substrings). Introduces the third Pattern A
  sub-shape: configuration-doc-references-external-fallibility
  (the doc's `may fail` substring describes downstream S3
  request operations the configured retries protect against,
  not the function body itself, while the function body is
  unconditionally infallible), contrasting batch 15's boundless
  silent-absorb-and-log sub-shape (body uses `tracing::warn!`
  and returns a partial `Registry`) and batch 17's wasmtime
  documented-panic-on-failure sub-shape (body uses `unwrap` /
  `assert_eq!` to surface failure intentionally). `comment-code`
  moves to 19/0/1.00 and overall recall_upper_bound to 0.54 (25
  TP / 21 FN / 46 expected, up from 0.53 at batch 17); Pattern
  A is now exercised across all three syntactic-Pattern-A
  sub-shapes on three unrelated upstreams (zkVM executor
  registry + assembler-fuzzer infrastructure + S3-client
  configuration setter), and every
  `docs/spec/comment-code-v0.md` pattern has at least two
  upstreams of regression-detection insurance after batch 18.
  Phase B batch 17 landed 2026-05-15 on top of batch 16,
  diversifying `comment-code` Pattern A audit coverage from
  a single upstream (boundless only, batch 15) to two
  upstreams by adding one TP from an eighth permissive-
  licensed Rust upstream (bytecodealliance/wasmtime@63330f11
  `cranelift/assembler-x64/src/fuzz.rs`, Apache-2.0 WITH
  LLVM-exception). `pub fn roundtrip` at upstream line 26
  (corpus line 13) carries a `///` doc block whose `# Panics`
  section contains the Pattern A trigger phrase `may fail`,
  but the function signature `pub fn roundtrip(inst:
  &Inst<FuzzRegs>)` has unit return so F3's return-type
  negation passes and Pattern A fires; the body uses
  `unwrap` and `assert_eq!` (the fuzzer intentionally panics
  to express failure to the `arbitrary` harness), so spec
  F4 Pattern B's body-marker negation suppresses Pattern B
  even though the doc contains the substring `panic` — only
  Pattern A fires. Introduces the second Pattern A
  sub-shape: documented-panic-on-failure (body panics
  intentionally to surface failure), contrasting batch 15's
  boundless silent-absorb-and-log sub-shape (body uses
  `tracing::warn!` and returns a partial `Registry`).
  `comment-code` moves to 18/0/1.00 and overall
  recall_upper_bound to 0.53 (24 TP / 21 FN / 45 expected,
  up from 0.52 at batch 16); Pattern A's single-upstream
  dominance is now broken — every
  `docs/spec/comment-code-v0.md` pattern has at least two
  upstreams of regression-detection insurance after
  batch 17.
  Phase B batch 16 landed 2026-05-15 on top of batch 15,
  diversifying `comment-code` Pattern B audit coverage from
  a single upstream (zarrs only, batch 14) to two upstreams
  by adding two TPs from a seventh permissive-licensed Rust
  upstream (Amanieu/parking_lot@d7828fff
  `core/src/parking_lot.rs`, parking_lot_core-v0.9.12
  release tag, MIT OR Apache-2.0). `pub unsafe fn unpark_one`
  at upstream line 732 (corpus line 29) and
  `pub unsafe fn unpark_requeue` at upstream line 888 (corpus
  line 122) each carry a `///` doc block ending in `must not
  panic or call into any function in parking_lot.` spec F4
  trigger fires (`doc_lc.contains("panic")`) and the body
  substring check finds none of the Pattern B markers
  (`panic!` / `unwrap` / `expect(` / `unreachable!` /
  `assert!` / `assert_eq!` / `assert_ne!` / `todo!` /
  `unimplemented!` / `debug_assert`) in either body, so
  Pattern B fires — the same Tan SOSP 2007 §3.2 bug shape
  the batch-14 zarrs `round_bytes_*` family flags. The doc
  here is a contract on callbacks rather than an
  implementation claim, but cntrdct's syntactic substring
  rule does not distinguish 'must not panic' (callback
  contract) from 'panics if' (implementation claim) — both
  are F4 hits on the same trigger, exercising both phrasings
  on two unrelated upstreams. Both entries are TP;
  `comment-code` moves to 17/0/1.00 and overall
  recall_upper_bound to 0.52 (23 TP / 21 FN / 44 expected,
  up from 0.50 at batch 15). Phase B batch 15 landed
  2026-05-15 on top of batch 14, shifting `comment-code`
  audit coverage from two-pattern (Pattern B + Pattern C) to
  the complete three-pattern (Pattern A + Pattern B +
  Pattern C) triple by adding one `comment-code` TP from a
  sixth permissive-licensed Rust upstream
  (boundless-xyz/boundless@1a2770b2
  `crates/executor/src/backends/mod.rs`, Apache-2.0).
  `pub fn default_registry() -> Registry` at upstream line 44
  (corpus line 10) carries a `///` doc block whose prose
  contains the Pattern A trigger phrase `may fail`, but the
  function signature `() -> Registry` lacks the `Result` /
  `Option` substring required by F3's return-type negation,
  and the body absorbs constructor failures via
  `tracing::warn!` rather than propagating an error type;
  `comment-code` moves to 15/0/1.00 and overall
  recall_upper_bound to 0.50 (21 TP / 21 FN / 42 expected).
  Phase B batch 14 landed 2026-05-15 on top of batch 13,
  adding six `comment-code` TPs from a fifth permissive-
  licensed Rust upstream (zarrs/zarrs
  `zarrs_data_type/src/codec_traits/bitround.rs`,
  MIT OR Apache-2.0) — six top-level `pub fn` items each
  carrying a `# Panics` doc claim that contradicts the
  `as_chunks_mut::<N>().0` body (which silently drops the
  trailing-remainder slice rather than panicking). The
  textbook Tan SOSP 2007 §3.2 Pattern B ("bad comment": panic
  claim without panicking constructs in the body) bug shape,
  previously not exercised by the audit corpus; `comment-code`
  moves to 14/0/1.00 and overall recall_upper_bound to 0.49
  (20 TP / 21 FN / 41 expected). All six cntrdct detectors
  and six external source kinds are live in the corpus;
  comment-code now spans nine independent upstreams across
  all three `docs/spec/comment-code-v0.md` patterns
  (Pattern A from boundless batch 15, wasmtime batch 17, and
  rust-s3 batch 18 — all three syntactic-Pattern-A sub-shapes
  (silent-absorb-and-log / documented-panic-on-failure /
  configuration-doc-references-external-fallibility) covered;
  Pattern B from zarrs batch 14 and parking_lot_core batch 16;
  Pattern C from batches 3 / 11 / 12 / 13), with both
  Pattern A's and Pattern B's single-upstream dominance
  broken at batches 17 and 16 respectively and Pattern A's
  sub-shape coverage completed at batch 18 — every pattern
  now has at least two upstreams of regression-detection
  insurance and Pattern A covers all three syntactic
  sub-shapes, further reducing the regression-detection risk
  of single-source and single-pattern dominance that batches
  11 / 12 / 13 / 14 / 15 / 16 / 17 / 18 progressively broke;
  pr-miner's numerator-construction phase remains closed and
  the next moves shift to detector-side scope lifts on the
  remaining 0.00 detectors (arg-swap, clone-drift,
  config-interaction) and F3 widening on
  unreachable-after-terminator.

  Per-batch history below for the record; Phase B batch 2
  (deepening `unreachable-after-terminator` measurement) landed
  2026-05-12;
  Phase B batch 3 (broadening to a third detector,
  `comment-code`, via the Tan SOSP 2007 Pattern C bug from
  `sidan-lab/whisky-archive`) landed 2026-05-12;
  Phase C release-tag refresh discipline landed 2026-05-12;
  Phase B batch 4 (introducing the `paper-appendix` source kind
  via three PyPIBugs ArgSwap entries on permissive-licensed
  Python repositories — `c137digital/unv_app` MIT,
  `mwouts/nbrmd` MIT, `markokr/rarfile` ISC; all three are FN
  against cntrdct's narrow Rice-2017 arg-swap detector,
  documenting the gap between PyPIBugs labels and the detector's
  same-file + bare-identifier scope) landed 2026-05-12;
  Phase B batch 5 (introducing the `clippy` source kind via two
  rust-clippy UI tests pinned at master commit `c4b8c6d4` —
  `tests/ui/if_same_then_else.rs` and
  `tests/ui/branches_sharing_code/shared_at_top.rs`; both are
  FN against cntrdct's clone-drift detector by
  `docs/spec/clone-drift-v0.md` F2, introducing clone-drift to
  the corpus with single-source recall_upper_bound 0.00 and
  surfacing the v0 fn-level scope choice) landed 2026-05-12;
  Phase B batch 6 (introducing the `config-interaction` detector
  to the corpus via the rustc UI test for the
  `cfg.attr.duplicates` Rust Reference behaviour —
  `rust-lang/rust@29b75901 tests/ui/cfg/both-true-false.rs`
  MIT OR Apache-2.0; both `fn foo()` items in the file are FN
  against cntrdct's config-interaction detector by
  `docs/spec/config-interaction-v0.md` F5, introducing
  config-interaction at single-source recall_upper_bound 0.00
  and surfacing the v0 require-`not(...)`-wrapper scope choice)
  landed 2026-05-12;
  Phase B batch 7 (introducing the `codeql` source kind via the
  CodeQL Python `UnreachableCode` query test fixture —
  `github/codeql@592c7c04
  python/ql/test/query-tests/Statements/unreachable/test.py`
  MIT; one TP at corpus line 25 (`for ...` after `return 5`,
  F3 terminator + follower match in Python) and five FN at
  corpus lines 12, 14, 20, 32, 88 — bug shapes outside the F3
  terminator set (constant-condition branches and typed-
  exception reachability)) landed 2026-05-12;
  Phase B batch 8 (introducing the `semgrep` source kind AND
  the sixth and final cntrdct detector `pr-miner` to the corpus
  in the same commit via the Semgrep registry rule
  `open-never-closed` (`python.lang.best-practice.open-never-closed.open-never-closed`,
  pinned at semgrep/semgrep-rules@9d73d08e
  python/lang/best-practice/open-never-closed.yaml) applied to
  TuGraph-family/tugraph-db@672e4b19 `release/det_ver.py`
  (Apache-2.0). The target file is from a DIFFERENT upstream
  than semgrep-rules, which sidesteps the Semgrep Rules License
  v1.0 carve-out entirely: we cite the rule's stable identity
  and redistribute permissive Apache-2.0 code the rule fires
  on, not semgrep-rules' own test fixtures. Single expected
  entry: `f = open('Options.cmake','r')` inside top-level
  `def get_ver():` (upstream line 5; corpus line 9) with no
  matching `close()` / `with` / try-finally. cntrdct's pr-miner
  spec F2 reaches `get_ver` (top-level fn), but spec F3 Apriori
  mining at `MIN_SUPPORT = 0.05` / `MIN_CONFIDENCE = 0.85`
  cannot synthesise the `{open} → {close}` rule from the
  single corpus-wide transaction (`replace_ver`) that contains
  both items, so spec F4 violation detection never runs against
  `get_ver` — FN by mining sparsity, NOT by extractor scope. The
  original blocker framing ("extractor walks only top-level
  `fn` / `def`, which collides with modern Rust RAII and Python
  `with` idioms — paired-API patterns survive almost exclusively
  in class methods the extractor drops") was a false-positive
  reading of the blocker: the bug here lives in a bare
  top-level `def`, the extractor reaches it cleanly, and the
  detector's inability to flag it is downstream of corpus
  density, not extractor scope) landed 2026-05-13;
  further Phase B batches deepen coverage on the existing six
  detectors and six source kinds rather than introduce a new
  detector or kind — the most tractable next step is additional
  `open-never-closed` instances on permissive Python files to
  push pr-miner's mined-rule confidence above the 0.85
  threshold and surface the first pr-miner TP)
- Goal: counter the labeller-bias loop where cntrdct's priors are
  derived from corpora it labelled itself, biasing toward
  precision and silently sacrificing recall. Build
  `benchmarks/audit-corpus/` from the union of NVD / OSV.dev
  CVEs and findings from independent SAST tools (Semgrep,
  CodeQL community, Clippy), then add `cntrdct calibrate
  --audit-recall` to report per-detector recall upper bounds
  quarterly.
- Acceptance: the audit corpus README cites every CVE / external
  finding source with a stable URL; `cntrdct calibrate
  --audit-recall` produces non-trivial recall numbers for all
  six detectors; the README publishes the latest figures.
- Effort: 4-6 weeks.
- Depends on: P-1 (corpus tooling).
- Evidence: Heckman & Williams (2011) Information and Software
  Technology 53(4), 363-387 (selection-bias warning for
  actionable-alert pipelines).
- Phase A — harness scaffolding (done 2026-05-11): the `cntrdct
  calibrate --audit-recall <CORPUS_DIR> [--manifest <PATH>]
  [--output <PATH>]` flag set ships behind `clap`'s
  `conflicts_with` against `--fit-platt`, polymorphically
  reinterpreting the `corpus` positional as a directory in this
  mode. New module `src/recall_audit.rs` carries the audit-corpus
  manifest schema (`AuditExpectedFinding` requiring an
  `external_source: {kind, ref, url}` block per labelled
  finding), the `audit_recall(...) -> RecallAuditReport` pure
  function, and a `source_breakdown` per-detector tally so a
  detector whose recall is dominated by one source surfaces
  visibly. The orchestrator `cntrdct::run_recall_audit` mirrors
  `run_eval` (manifest validation → scan → audit). Twelve tests
  (4 unit + 8 integration) pin the loader edge cases, the
  matching arithmetic, byte-stable JSON, and the CLI conflict
  surface; `tests/fixtures/recall-audit/` carries a 2-detector /
  2-source synthetic fixture so PR CI exercises the path
  independently of Phase B data. `benchmarks/audit-corpus/`
  ships as a skeleton with the source list and per-detector
  seed targets documented; `manifest.jsonl` is empty pending
  Phase B. CITATIONS.md adds `heckman-williams-ist-2011` under
  Layer 2. Spec: `docs/spec/recall-audit-v0.md`.
- Phase B — audit-corpus data collection: per-detector seed
  lists land in `benchmarks/audit-corpus/files/` + the manifest,
  with each `expected[].external_source.url` resolving to a
  stable external page. Per-detector recall figures populate the
  README's "Latest audit run" section.
  - First batch (done 2026-05-11): six expected entries across
    two detectors and two sources. Three rust-lang/rust ui-test
    files for the rustc `unreachable_code` lint
    (`tests/ui/reachable/{unreachable-code-ret,expr_block,expr_if}.rs`
    at commit `4b0c9d76`, MIT OR Apache-2.0, stripped of the
    file-level `#![deny(unreachable_code)]` because cntrdct's
    SUPPRESSION_TOKEN scan would otherwise honour it) contribute
    five entries; one minimal extract of `totalsegmentator/statistics.py`
    at the pre-fix parent of PR #556 contributes one arg-swap
    entry. Audit numbers: `unreachable-after-terminator`
    tp=3 / fn=2 / recall_upper_bound=0.60; `arg-swap`
    tp=0 / fn=1 / recall_upper_bound=0.00; overall recall_upper_bound
    0.50. The 0.60 figure is the honest signal — cntrdct's
    statement-level terminator scan misses cases where the
    terminator lives inside an `if` / `if-else` expression. The
    arg-swap miss is also expected: the reverse-permutation
    heuristic does not catch swaps where the call-site
    identifiers do not share names with the parameter list.
    Source breakdown is preserved per detector so a future
    detector dominated by a single source surfaces visibly.
  - Second batch (done 2026-05-12): three additional rust-lang/rust
    ui-test files at the same pinned commit `4b0c9d76` —
    `tests/ui/reachable/{expr_return,expr_call,expr_loop}.rs`,
    each stripped of `#![deny(unreachable_code)]` on the same
    grounds as batch 1, contributing six additional
    `unreachable-after-terminator` entries. The batch intentionally
    deepens rather than broadens — every new entry pins a control-
    flow shape cntrdct's `rust_terminator_kind` classifier ignores
    by design: `expr_return.rs` puts `return` inside a tail
    expression (`let x: () = {return ...};`), `expr_call.rs`
    places `return` as a call argument (`foo(return, 22)`,
    `bar(return)`), `expr_loop.rs` uses divergent
    `loop { return; }` as the terminator (which the rustc lint
    recognises as never-typed but cntrdct treats as just another
    `loop_expression`). All six new entries are FN. Updated audit
    numbers: `unreachable-after-terminator` tp=3 / fn=8 /
    recall_upper_bound=0.27 (3/11); `arg-swap` unchanged at
    tp=0 / fn=1; overall recall_upper_bound 0.25 (3/12). The
    drop from 0.50 to 0.25 is the audit doing its job — the
    rustc lint's denominator captures shapes cntrdct's
    statement-level scan does not look through, and closing the
    gap is detector-improvement work (separate engineering,
    separate preregistration), not audit-harness work.
  - Third batch (done 2026-05-12): four expected entries adding
    a third detector, `comment-code`, to the corpus. Source:
    `sidan-lab/whisky-archive` `packages/whisky-common/src/data/
    primitives/constructors.rs` at commit `99243766`,
    Apache-2.0 licensed, verbatim copy. The `con_str` /
    `con_str0` / `con_str1` / `con_str2` family (upstream lines
    64 / 69 / 74 / 79; audit-corpus lines 68 / 73 / 78 / 83
    after the 3-line provenance header + 1 blank-line offset)
    each carries `/// Deprecated: Use ... instead.` prose but
    ships without the `#[deprecated]` runtime attribute — the
    textbook Pattern C bug Tan SOSP 2007 §3.2 ("bad comment")
    describes. All four are TP, so comment-code enters the
    corpus with single-source recall_upper_bound 1.00. Updated
    audit numbers: `comment-code` tp=4 / fn=0 / 1.00;
    `unreachable-after-terminator` tp=3 / fn=8 / 0.27 unchanged;
    `arg-swap` tp=0 / fn=1 / 0.00 unchanged; overall
    recall_upper_bound 0.4375 (7/16, up from 0.25). The lift is
    upward for the right reason: a new detector entered with
    high single-source recall, not because the existing
    detectors improved. Clone-drift was the original batch-3
    target but punted to a later batch: the published
    peer-reviewed clone-drift bug catalogues (Bettenburg MSR
    2009, Krinke ICSM 2007) target C/Java rather than
    Rust/Python, and the Assi TOSEM 2025 deep-learning-framework
    genealogies (mia1q/code-clone-DL-frameworks replication
    CSVs) expose size-2 clone pairs that fall under cntrdct's
    `MIN_GROUP_SIZE = 3` floor by construction.
  - Fourth batch (done 2026-05-12): three expected entries
    introducing the `paper-appendix` source kind via three
    PyPIBugs (Allamanis NeurIPS 2021) ArgSwap labels on
    permissive-licensed Python repositories —
    `c137digital/unv_app@d217fa0d` MIT (cross-file imported
    `update_dict_recur` call), `mwouts/nbrmd@dfa96996` MIT
    (cross-file imported `compare_notebooks` call in a pytest
    test), `markokr/rarfile@7fd6b2ca` ISC
    (`self._set_attrs(dst, inf)` method call). All three are FN
    against cntrdct's narrow Rice-2017 arg-swap detector by
    `docs/spec/arg-swap-v0.md` F3 / F4 / F5. Updated audit
    numbers: `arg-swap` tp=0 / fn=4 / 0.00 (up from 0/1/0.00);
    `comment-code` and `unreachable-after-terminator` unchanged;
    overall recall_upper_bound 0.37 (7/19, down from 0.44). The
    drop is downward for the right reason: PyPIBugs labels
    arg-swap bugs cntrdct's detector cannot catch by design.
  - Fifth batch (done 2026-05-12): two expected entries
    introducing the `clippy` source kind via two rust-clippy UI
    tests (MIT OR Apache-2.0, pinned at master commit
    `c4b8c6d454c648ef2d7cb86ca1bc698da829e4bc`):
    `tests/ui/if_same_then_else.rs:25` (the first
    `clippy::if_same_then_else` trigger, audit-corpus line 29)
    and `tests/ui/branches_sharing_code/shared_at_top.rs:11`
    (the first `clippy::branches_sharing_code` trigger,
    audit-corpus line 15). Both are FN against cntrdct's
    clone-drift detector by `docs/spec/clone-drift-v0.md` F2:
    cntrdct clone-drift v0 operates at top-level `fn`
    granularity only and requires `MIN_FN_TOKENS >= 22` +
    `MIN_GROUP_SIZE >= 3`, so the statement-block clone
    patterns clippy's lints target are out of scope. The batch
    therefore introduces clone-drift to the audit corpus with
    single-source recall_upper_bound 0.00. Updated audit
    numbers: `clone-drift` tp=0 / fn=2 / 0.00 (new);
    `arg-swap`, `comment-code`, `unreachable-after-terminator`
    unchanged; overall recall_upper_bound 0.33 (7/21, down from
    0.37). Downward for the right reason: a new detector entered
    the denominator with 0 TPs by detector design. Closing the
    0.67 gap on clone-drift requires lifting F2 to cover
    statement blocks and `impl` / `trait` methods — separate
    engineering with its own preregistration.
  - Sixth batch (done 2026-05-12): two expected entries
    introducing the `config-interaction` detector to the corpus
    via a single rustc UI test for the `cfg.attr.duplicates`
    Rust Reference behaviour
    (`rust-lang/rust@29b75901 tests/ui/cfg/both-true-false.rs`,
    MIT OR Apache-2.0). Each of the two `fn foo()` items
    (upstream lines 7 and 11; audit-corpus lines 11 and 15 after
    the 3-line header + 1 blank) carries a syntactically
    contradictory `#[cfg(...)]` pair (`cfg(false)` + `cfg(true)`
    and `cfg(true)` + `cfg(false)`), so both items are disabled
    under every configuration. Both entries are FN against
    cntrdct's config-interaction detector by
    `docs/spec/config-interaction-v0.md` F5: the detector
    recognises a contradiction only when one predicate is
    structurally `not(X)` and the other is structurally equal to
    `X`, while `true` and `false` are atomic primitives without
    the `not(...)` wrapper. The batch therefore introduces
    config-interaction to the corpus with single-source
    recall_upper_bound 0.00. Updated audit numbers:
    `config-interaction` tp=0 / fn=2 / 0.00 (new); `arg-swap`,
    `clone-drift`, `comment-code`,
    `unreachable-after-terminator` unchanged; overall
    recall_upper_bound 0.30 (7/23, down from 0.33). Downward for
    the right reason: a new detector entered the denominator
    with 0 TPs by detector design. Closing the 1.00 gap on
    config-interaction requires both broader external sources
    (Tartler EuroSys 2011 / Nadi ICSE 2014 catalogues target
    C/Linux KConfig rather than Rust `#[cfg]` attributes, so a
    Rust-side semgrep / codeql sweep is still owed) and
    detector-side F5 widening to recognise primitive `true` /
    `false` pairs and `not(...)` reductions — separate
    engineering with its own preregistration.
  - Seventh batch (done 2026-05-12): six expected entries
    introducing the `codeql` source kind via the CodeQL Python
    `UnreachableCode` query test fixture
    (`github/codeql@592c7c043734f6bb48768a56261d711446cde25f
    python/ql/test/query-tests/Statements/unreachable/test.py`,
    MIT). CodeQL's matching `UnreachableCode.expected` flags six
    unreachable statements at upstream lines 8, 10, 16, 21, 28,
    84 (audit-corpus lines 12, 14, 20, 25, 32, 88 after the
    3-line header + 1 blank). Mapping against cntrdct's Python
    unreachable-after-terminator implementation: corpus line 25
    is TP — the `for x in first_unreachable_stmt():` statement
    directly follows `return 5` in the same function-body block,
    matching spec F3 (return-statement terminator + follower)
    and the Python codepath in
    `src/detectors/unreachable_after_terminator.rs::analyze_python_block`.
    Corpus lines 12, 14, 20, 32 are FN: bodies of `while 0:` /
    `while False:` (constant-false loop bodies) and `if False:` /
    `else` of `if True:` (constant-condition branches) are
    unreachable for reasons outside cntrdct's F3 terminator set —
    the detector recognises only return / raise / break /
    continue / `assert False` / exit-call terminators inside a
    block, not constant-condition branches (Non-goal
    "Branch-merging analysis"). Corpus line 88 is FN: the
    `except NameError:` handler is unreachable because the bound
    name `str` is always defined in Python 3, a typed-exception
    reasoning step cntrdct's AST-only detector cannot perform.
    Updated audit numbers: `unreachable-after-terminator`
    tp=4 / fn=13 / 0.235 (was 3/8/0.27 — +1 TP and +5 FN from
    codeql); `arg-swap`, `clone-drift`, `comment-code`,
    `config-interaction` unchanged; overall recall_upper_bound
    0.276 (8/29, down from 0.30). Downward for the right reason:
    CodeQL's denominator captures bug shapes outside cntrdct's
    statement-level F3 scope. Closing the 0.76 gap on
    `unreachable-after-terminator` requires F3 widening to
    constant-condition / branch / exception-typed reasoning —
    separate engineering with its own preregistration.
  - Eighth batch (done 2026-05-13): one expected entry
    introducing the `semgrep` source kind AND the sixth detector
    `pr-miner` to the corpus in the same commit, closing the
    last two Phase B blockers (pr-miner detector coverage +
    semgrep source kind) named at the end of batch 7. The
    labeller is the Semgrep registry rule `open-never-closed`
    (`python.lang.best-practice.open-never-closed.open-never-closed`,
    pinned at semgrep/semgrep-rules@9d73d08e
    python/lang/best-practice/open-never-closed.yaml). The
    redistributed target file is from a DIFFERENT upstream than
    semgrep-rules: TuGraph-family/tugraph-db@672e4b19
    `release/det_ver.py` (Apache-2.0). Selecting permissive
    tugraph-db code that the rule fires on — rather than
    semgrep-rules' own `python/lang/best-practice/open-never-closed.py`
    test fixture — sidesteps the Semgrep Rules License v1.0
    carve-out entirely: only the rule's stable identity is
    cited via its registry id + GitHub blob URL, and the
    audit-corpus only ships permissive-licensed source.
    Single expected entry maps to cntrdct's `pr-miner`:
    `f = open('Options.cmake','r')` inside top-level
    `def get_ver():` at upstream line 5 (corpus line 9) is
    followed by `f.readlines()` and a `return` without any
    matching `f.close()`, `with`, or try-finally; the companion
    `def replace_ver(...)` at upstream line 17 (corpus 21)
    opens AND closes, so the rule does not fire on it.
    cntrdct's pr-miner spec F2 `function_definition` extractor
    reaches `get_ver` (top-level def) and yields the item set
    `{open, readlines, find, split}`, but spec F3 Apriori
    mining at `MIN_SUPPORT = 0.05` / `MIN_CONFIDENCE = 0.85`
    cannot synthesise the `{open} → {close}` rule from the
    single corpus-wide transaction (`replace_ver`) that
    contains both items, so spec F4 violation detection never
    runs against `get_ver` — FN by mining sparsity, NOT by
    extractor scope. The original batch-3 framing ("paired-API
    patterns survive almost exclusively in class methods the
    extractor drops") was a false-positive reading of the
    blocker for this specific shape: bare top-level `def` with
    `open()` and no `close()` is reachable cleanly. Updated
    audit numbers: `pr-miner` tp=0 / fn=1 / 0.00 (new);
    `arg-swap`, `clone-drift`, `comment-code`,
    `config-interaction`, `unreachable-after-terminator`
    unchanged; overall recall_upper_bound 0.27 (8/30, down from
    0.28). Downward for the right reason: a new detector
    entered the denominator with 0 TPs by detector design at
    v0 corpus density. Closing the 1.00 gap on pr-miner is a
    corpus-density problem (more paired open/close
    transactions in the audit-corpus pulls the mined-rule
    confidence above 0.85, after which v0 begins to flag
    `get_ver` without any detector-side change), not an
    extractor-widening problem. With all six cntrdct detectors
    and six external source kinds now represented in the
    corpus, future Phase B batches deepen existing coverage
    rather than introduce a new detector or kind — the most
    tractable next step is additional `open-never-closed`
    instances on permissive Python files to surface the first
    pr-miner TP.
  - Ninth batch (done 2026-05-13): one expected entry deepening
    the `pr-miner` denominator on the existing `semgrep` source
    kind via a second permissive-licensed `open-never-closed`
    instance. The labeller is the same Semgrep registry rule
    (`python.lang.best-practice.open-never-closed.open-never-closed`,
    pinned at semgrep/semgrep-rules@9d73d08e) applied to a
    different permissive upstream:
    gregmuellegger/django-mobile@fafc3890 `setup.py`
    (BSD-3-Clause). Selecting django-mobile rather than
    semgrep-rules' own test fixtures keeps batch 9 outside the
    Semgrep Rules License v1.0 carve-out the same way batch 8
    did with Apache-2.0 tugraph-db: only the rule's stable
    identity is cited, and the labelled code is permissive.
    Single expected entry maps to cntrdct's `pr-miner`:
    `return open(filename, ...).read()` inside top-level
    `def readfile(filename):` at upstream line 13 (corpus
    line 17) — both branches of the `sys.version_info` check
    return the same `open(...).read()` chain without any
    matching `close()` / `with` / try-finally, so the open file
    handle is dropped on return. The companion top-level
    `def get_author` (upstream line 20) and `def get_version`
    (upstream line 29) delegate file reading to `readfile` and
    do not call `open` directly, so the rule does not fire on
    them; the `UltraMagicString` class methods at upstream
    lines 37-55 are excluded from pr-miner's spec F2 extractor
    by design (only top-level `function_definition` /
    `decorated_definition` are walked, class bodies are out of
    scope). cntrdct's pr-miner reaches `readfile` cleanly and
    produces item set `{open, read}`. Spec F3 Apriori mining at
    `MIN_SUPPORT = 0.05` / `MIN_CONFIDENCE = 0.85` still cannot
    synthesise the `{open} → {close}` rule: across the corpus
    three top-level Python defs contain `open` (batch 8's
    `get_ver` open-only, batch 8's `replace_ver` open+close,
    batch 9's `readfile` open-only) and only one (`replace_ver`)
    also contains `close`, so the rule's confidence is 1/3 ≈
    33%, far below the 0.85 threshold. Spec F4 violation
    detection therefore never runs against `readfile` — FN by
    mining sparsity (denominator weight deepened without
    crossing confidence), NOT by extractor scope. Updated audit
    numbers: `pr-miner` tp=0 / fn=2 / 0.00 (up from 0/1/0.00,
    `semgrep` 2/2); `arg-swap`, `clone-drift`, `comment-code`,
    `config-interaction`, `unreachable-after-terminator`
    unchanged; overall recall_upper_bound 0.26 (8/31, down from
    0.27 at batch 8). Downward for the right reason: another
    `pr-miner` FN-by-sparsity entry on the existing FN class,
    not any of the five other detectors regressing. With the
    pr-miner FN denominator now broadened twice on the same FN
    class, the next pr-miner move shifts from denominator
    widening to numerator construction: paired open+close
    transactions on permissive Python upstreams to lift
    `{open} → {close}` mined-rule confidence above 0.85, after
    which the existing FN entries become TPs without any
    detector-side change.
  - Tenth batch (done 2026-05-13): ten density-support files
    added in a single batch to lift pr-miner's mined
    `{open} → {close}` confidence above the F3 0.85 threshold
    and surface the first pr-miner true positives. Each file
    ships with `expected: []` rather than a labelled finding
    because the Semgrep `open-never-closed` rule produces no
    findings on files where every top-level `def` calling
    `open` also explicitly calls `close` (or closes via try/
    finally); the labeller's negative outcome is captured by an
    empty expected array. Files and per-file paired-transaction
    counts (all MIT / BSD-3-Clause / Apache-2.0, permissive
    Python; ten files contribute 18 paired transactions total):
    carla-simulator/carla `Util/Tools/Import.py` (3 paired:
    generate_json_package, generate_decals_file,
    generate_import_setting_file — minimal extract),
    kgullikson88/Telluric-Fitter `setup.py` (2 paired:
    gfortran_mode, MakeTAPE3 — minimal extract dropping the
    file's open-only download_file and GetCompilerString defs
    which would have lowered confidence), baidu/tera
    `example/docker/tera_setup.py` (1 paired: write_config —
    verbatim), nottheswimmer/pytago `examples/fileloop.py`
    (1 paired: main — verbatim, where the explicit `fh.close()`
    after `fh = open(...)` is followed by two `with open(...)`
    blocks that Semgrep correctly treats as resource-managed),
    dkruchinin/sanic-prometheus `scripts/release.py` (2 paired:
    get_version, update_changelog — both use try/finally with
    explicit close — verbatim), carljm/django-secure `setup.py`
    (1 paired: get_version — verbatim, with the module-level
    `long_description = (open(...).read() + ...)` chain
    correctly outside pr-miner's spec F2 scope), carljm/django-
    secure `doc/conf.py` (1 paired: get_version — minimal
    extract from a 225-line Sphinx config), adnanademovic/
    rosrust `msg_examples/actionlib_msgs/scripts/genaction.py`
    (2 paired: write_file, main — verbatim; the file carries
    its own BSD-3-Clause Willow Garage 2009 header inside an
    MIT-overall repository), R-s0n/ars0n-framework
    `toolkit/toolkit/fire-scanner.py` (2 paired:
    write_urls_file, build_slack_message — minimal extract
    dropping the file's open-only process_results), apache/
    ranger `tagsync/scripts/setup.py` (3 paired:
    convertInstallPropsToXML, write_env_files, main — minimal
    extract dropping three open-only defs). Math: before batch
    10 the corpus had 4 top-level Python defs containing `open`
    and 1 of those also containing `close`, so `{open} →
    {close}` confidence was 1/4 = 0.25 < `MIN_CONFIDENCE = 0.85`.
    Batch 10 adds 18 paired transactions to both numerator and
    denominator, pushing confidence to (1+18)/(4+18) = 19/22 ≈
    0.864 ≥ 0.85. Spec F3 Apriori mines the rule, spec F4 scans
    the full transaction set, and both batch-8
    `tugraph_det_ver.py::get_ver` (corpus line 9) and batch-9
    `django_mobile_setup.py::readfile` (corpus line 17) flip
    from FN to TP without any detector-side change. A third
    function, `nbrmd_test_ipynb_to_R.py::test_identity_source_write_read`,
    also matches the F4 violation pattern (uses `with open(...)`
    which Semgrep correctly treats as resource-managed but
    pr-miner's v0 syntax-level extractor cannot distinguish from
    plain `open` without close) — this is an unmatched-actual
    finding (no expected entry, no effect on recall; the audit
    matches expected against actual and ignores unmatched
    actuals). The `with` vs explicit close asymmetry between
    Semgrep and pr-miner is the documented v0 detector-scope
    choice (extending pr-miner to recognise context-managed
    paired patterns is separate engineering with its own
    preregistration). Updated audit numbers: `pr-miner` tp=2 /
    fn=0 / 1.00 (up from 0/2/0.00, `semgrep` 2/2 → 2/0 paired);
    `arg-swap`, `clone-drift`, `comment-code`,
    `config-interaction`, `unreachable-after-terminator`
    unchanged; overall recall_upper_bound 0.32 (10 TP / 21 FN /
    31 expected, up from 0.26 at batch 9). Corpus size 27 files
    (17 labelled + 10 density-support). Upward for the right
    reason: the labelled FN entries flipped to TPs at a corpus
    density that was telegraphed in batch 8 and batch 9 as the
    next pr-miner move; the audit denominator stayed at 31
    expected entries because the density-support files carry
    `expected: []`. The MIN_DATABASE_SIZE = 20 spec F3 gate is
    also satisfied because batch 10's 18 paired transactions
    plus the ~10+ existing multi-item top-level Python defs in
    the corpus push the mining-DB count past 20. SHA-256 policy:
    verbatim copies (5 files) carry the upstream file's
    SHA-256; minimal extracts (5 files) carry the SHA-256 of
    the audit-corpus file as committed.
  - Eleventh batch (done 2026-05-13): two expected entries
    deepening the `comment-code` detector with a second
    permissive-licensed Rust upstream — rusticata/tls-parser
    at commit `6554155918278531370e7d0addbd5d759e3a4cc9`
    `src/tls_handshake.rs` (MIT OR Apache-2.0). Both
    `pub fn parse_tls_handshake_next_protocol(...)` at
    upstream line 850 (corpus line 10) and
    `pub fn parse_tls_handshake_msg_next_protocol(...)` at
    upstream line 865 (corpus line 25) carry a `///` doc
    block ending in `Deprecated in favour of ALPN.` but
    neither carries the `#[deprecated]` runtime attribute the
    Rust deprecation lints honour — the same Tan SOSP 2007
    §3.2 Pattern C ("bad comment") bug shape the batch-3
    `sidan-lab/whisky-archive` con_str* family flags.
    Mapping against cntrdct's comment-code detector: spec F5
    trigger (rendered doc string contains `deprecated`
    substring after lowercasing) fires on both functions
    because `collect_preceding_doc` walks every `///`-
    prefixed line back to the first non-`///` ancestor (the
    file-level `// Source: ...` header breaks the walk
    cleanly because `//` does not start with `///`), and
    `preceding_siblings_have_deprecated` finds no
    `#[deprecated]` attribute between the doc block and the
    `function_item`. Both entries are TP. The batch's value-
    add is dual: (a) `comment-code` source-of-evidence
    diversifies from a single upstream (`sidan-lab/whisky-
    archive`, 4/4 in batch 3) to two upstreams on unrelated
    domains (whisky-archive Plutus-data helpers 4 + tls-
    parser TLS-handshake parsers 2), so a regression
    triggered by a whisky-archive-specific quirk would
    surface in the audit rather than going undetected;
    (b) the source-kind footprint stays at the existing six
    external kinds (this batch contributes under
    `github-commit`, already populated by batches 1 / 3 / 4),
    so no new kind is invented and the source-list registry
    under README "Source list" remains unchanged. Selection
    rationale: rusticata/tls-parser is a long-lived TLS-
    parser crate whose deprecation prose dates to the v0
    NextProtocolNegotiation / ALPN transition and has
    remained without `#[deprecated]` across many releases —
    a stable upstream commit and a stable bug shape, the
    same stability profile the batch-3 whisky-archive
    entries have. Updated audit numbers: `comment-code`
    tp=6 / fn=0 / 1.00 (up from 4/0/1.00, source_breakdown
    `github-commit` 6/6); `arg-swap`, `clone-drift`,
    `config-interaction`, `pr-miner`,
    `unreachable-after-terminator` unchanged; overall
    recall_upper_bound 0.36 (12 TP / 21 FN / 33 expected, up
    from 0.32 at batch 10). Corpus size 28 files (18
    labelled + 10 density-support). Upward for the right
    reason: a new TP class entered the corpus on an
    independent upstream, broadening the detector's audit
    evidence base without retreating any existing detector.
    Batch 11 also preserves the pr-miner mining margin
    because the new file is Rust — Python `{open} →
    {close}` mining-DB confidence stays at batch-10's
    19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs (`get_ver`,
    `readfile`) remain TPs (a labelled `open-never-closed`
    instance on a NEW Python file would have lowered the
    ratio because each open-only addition to the mining DB
    requires ≥ 4 paired density-support transactions to
    maintain the margin; staying on the Rust side bypasses
    the constraint entirely). SHA-256 is of the audit-
    corpus file as committed (minimal extract, per
    `benchmarks/audit-corpus/README.md` `Per-detector seed
    targets` item 4).
  - Twelfth batch (done 2026-05-13): one expected entry
    deepening the `comment-code` detector with a third
    permissive-licensed Rust upstream — glium/glium at commit
    `8d6fd34d9171172928771657fc5c9103107ff978`
    `src/draw_parameters/mod.rs` (Apache-2.0). `pub fn
    validate(...)` at upstream line 531 (corpus line 6) carries
    a single-line `///` doc block reading `DEPRECATED. Checks
    parameters and returns an error if something is wrong.` but
    does not carry the `#[deprecated]` runtime attribute the
    Rust deprecation lints honour — the same Tan SOSP 2007 §3.2
    Pattern C ("bad comment") bug shape the batch-3
    `sidan-lab/whisky-archive` con_str* family and batch-11
    `rusticata/tls-parser`
    `parse_tls_handshake_*next_protocol` family flag. Mapping
    against cntrdct's comment-code detector: spec F5 trigger
    (rendered doc string contains `deprecated` substring after
    lowercasing) fires because `DEPRECATED.` lowercases to
    `deprecated.` which contains the `deprecated` substring;
    `collect_preceding_doc` walks the single `///` line back
    to the `}` of the previous item (non-`///` ancestor)
    cleanly; `preceding_siblings_have_deprecated` finds no
    `#[deprecated]` attribute item between the doc block and
    the `function_item`. The entry is TP. The batch's value-
    add is dual: (a) `comment-code` source-of-evidence
    diversifies from two upstreams (whisky-archive 4 +
    tls-parser 2) to three upstreams on three unrelated
    domains (whisky-archive Cardano Plutus-data helpers 4 +
    tls-parser TLS NextProtocol parsers 2 + glium OpenGL
    draw-parameter check 1), further reducing the
    regression-detection risk of single-source dominance the
    batch-11 second upstream already broke — a regression
    triggered by an upstream-specific quirk on any one of them
    still surfaces in the audit via the other two; (b) the
    source-kind footprint stays at the existing six external
    kinds (this batch contributes under `github-commit`,
    already populated by batches 1 / 3 / 4 / 11), so no new
    kind is invented and the source-list registry under README
    "Source list" remains unchanged. Selection rationale: glium
    is a long-lived (2014-) OpenGL wrapper crate with 3.6k
    stars on GitHub; the `pub fn validate` deprecation prose
    marks an API the maintainers stopped recommending without
    removing, a stable upstream commit and a stable bug shape
    — the same stability profile the batch-3 / 11 entries have.
    Updated audit numbers: `comment-code` tp=7 / fn=0 / 1.00
    (up from 6/0/1.00, source_breakdown `github-commit` 7/7);
    `arg-swap`, `clone-drift`, `config-interaction`,
    `pr-miner`, `unreachable-after-terminator` unchanged;
    overall recall_upper_bound 0.38 (13 TP / 21 FN / 34
    expected, up from 0.36 at batch 11). Corpus size 29 files
    (19 labelled + 10 density-support). Upward for the right
    reason: a new TP entry lands cleanly within the existing
    1.00 detector; the audit's denominator widens by 1
    expected entry (from 33 to 34), so the percentage figure
    reflects the new labelled sample size rather than detector
    improvement. Batch 12 also preserves the pr-miner mining
    margin because the new file is Rust — Python
    `{open} → {close}` mining-DB confidence stays at
    batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs
    (`get_ver`, `readfile`) remain TPs. SHA-256 is of the
    audit-corpus file as committed (minimal extract, per
    `benchmarks/audit-corpus/README.md` `Per-detector seed
    targets` item 4).
  - Thirteenth batch (done 2026-05-14): one expected entry
    deepening the `comment-code` detector with a fourth
    permissive-licensed Rust upstream — rust-lang/pkg-config-rs
    at commit `f36d32a09824a6b2c18475c8a4b7df1cb2c50c95`
    `src/lib.rs` (MIT OR Apache-2.0).
    `pub fn find_library(name: &str) -> Result<Library, String>`
    at upstream line 453 (corpus line 7) carries a single-line
    `///` doc block reading `Deprecated in favor of the
    probe_library function` and a `#[doc(hidden)]` attribute,
    but does not carry the `#[deprecated]` runtime attribute
    the Rust deprecation lints honour — the same Tan SOSP 2007
    §3.2 Pattern C ("bad comment") bug shape the batch-3
    `sidan-lab/whisky-archive` con_str* family, batch-11
    `rusticata/tls-parser`
    `parse_tls_handshake_*next_protocol` family, and batch-12
    `glium/glium` `validate` flag. Mapping against cntrdct's
    comment-code detector: spec F5 trigger (rendered doc string
    contains `deprecated` substring after lowercasing) fires
    because `Deprecated in favor of the probe_library function`
    lowercases to `deprecated in favor of the probe_library
    function` which contains the `deprecated` substring;
    `collect_preceding_doc` walks the single `///` line back
    to the `}` of the previous fn cleanly (the
    `#[doc(hidden)]` attribute_item sitting between the doc
    block and the `function_item` is skipped per the comment-
    code detector's attribute_item passthrough);
    `preceding_siblings_have_deprecated` checks each
    `#[...]` attribute_item's first identifier against the
    literal string `deprecated`, so `#[doc(hidden)]` (first
    identifier `doc`) does NOT suppress the trigger, and the
    walker reaches the previous fn's closing `}` without
    finding a `#[deprecated]` attribute. The entry is TP. The
    batch's value-add is dual: (a) `comment-code`
    source-of-evidence diversifies from three upstreams
    (whisky-archive 4 + tls-parser 2 + glium 1) to four
    upstreams on four unrelated domains (whisky-archive
    Cardano Plutus-data helpers 4 + tls-parser TLS
    NextProtocol parsers 2 + glium OpenGL draw-parameter
    check 1 + pkg-config-rs build-tool / system-package
    bindings 1), further reducing the regression-detection
    risk of single-source dominance the batch-11 / 12 second /
    third upstreams already broke — a regression triggered by
    an upstream-specific quirk on any one of them still
    surfaces in the audit via the other three; (b) the
    source-kind footprint stays at the existing six external
    kinds (this batch contributes under `github-commit`,
    already populated by batches 1 / 3 / 4 / 11 / 12), so no
    new kind is invented and the source-list registry under
    README "Source list" remains unchanged. Selection
    rationale: pkg-config-rs is a long-lived (2014-)
    build-script helper maintained under the rust-lang GitHub
    organisation, the canonical wrapper for invoking
    `pkg-config` from Cargo build scripts; the
    `pub fn find_library` deprecation prose marks an API the
    maintainers stopped recommending without removing (the
    `#[doc(hidden)]` attribute IS attached but is orthogonal
    to the runtime deprecation lint), a stable upstream commit
    and a stable bug shape — the same stability profile the
    batch-3 / 11 / 12 entries have. The `#[doc(hidden)]`
    attribute is the most interesting wrinkle in this batch:
    cntrdct's Pattern C `preceding_siblings_have_deprecated`
    walker checks the first identifier inside each `#[...]`
    attribute_item against the literal string `deprecated`,
    so `#[doc(hidden)]` (first identifier `doc`) does NOT
    suppress the trigger — confirming Pattern C correctly
    distinguishes the visibility-hiding `#[doc(hidden)]` from
    the runtime-lint-enforcing `#[deprecated]` (the actual fix
    the maintainers would need to apply for downstream
    consumers to receive a compiler warning). Updated audit
    numbers: `comment-code` tp=8 / fn=0 / 1.00 (up from
    7/0/1.00, source_breakdown `github-commit` 8/8);
    `arg-swap`, `clone-drift`, `config-interaction`,
    `pr-miner`, `unreachable-after-terminator` unchanged;
    overall recall_upper_bound 0.40 (14 TP / 21 FN / 35
    expected, up from 0.38 at batch 12). Corpus size 30 files
    (20 labelled + 10 density-support). Upward for the right
    reason: a new TP entry lands cleanly within the existing
    1.00 detector; the audit's denominator widens by 1
    expected entry (from 34 to 35), so the percentage figure
    reflects the new labelled sample size rather than detector
    improvement. Batch 13 also preserves the pr-miner mining
    margin because the new file is Rust — Python
    `{open} → {close}` mining-DB confidence stays at
    batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs
    (`get_ver`, `readfile`) remain TPs. SHA-256 is of the
    audit-corpus file as committed (minimal extract, per
    `benchmarks/audit-corpus/README.md` `Per-detector seed
    targets` item 4).
  - Fourteenth batch (done 2026-05-15): six expected entries
    shifting `comment-code` audit coverage from single-pattern
    (Pattern C only, batches 3 / 11 / 12 / 13) to two-pattern
    (Pattern B + Pattern C) by adding six TPs from a fifth
    permissive-licensed Rust upstream — zarrs/zarrs at commit
    `3b944c57a0b7af127ae73ea250d3ffce60e51f0b`
    `zarrs_data_type/src/codec_traits/bitround.rs`
    (MIT OR Apache-2.0). Six top-level `pub fn` items
    (`round_bytes_int16` upstream line 58 / corpus 42,
    `round_bytes_int32` upstream 70 / corpus 54,
    `round_bytes_int64` upstream 82 / corpus 66,
    `round_bytes_float16` upstream 94 / corpus 78,
    `round_bytes_float32` upstream 106 / corpus 90,
    `round_bytes_float64` upstream 118 / corpus 102) each carry
    a `///` doc block with a `# Panics\nPanics if
    \`bytes.len()\` is not a multiple of N.` section for N in
    {2, 4, 8}, but the body uses
    `bytes.as_chunks_mut::<N>().0` which never panics on
    non-multiple-of-N lengths — `slice::as_chunks_mut::<N>`
    (stabilised in Rust 1.88.0, 2025-06-26) returns
    `(&mut [[T; N]], &mut [T])` where the second tuple element
    is the remainder with length strictly less than N; the
    upstream code accesses only `.0` (the chunked arrays) and
    silently discards the remainder, so trailing bytes that
    don't form a complete N-byte chunk are ignored rather than
    triggering the documented panic. The textbook Tan SOSP
    2007 §3.2 Pattern B ("bad comment": panic claim without
    panicking constructs in the body) bug shape, previously
    not exercised by the audit corpus. Mapping against
    cntrdct's comment-code detector: spec F4 trigger (rendered
    doc string contains `panic` substring after
    `to_lowercase`) fires on each function because the doc's
    `# Panics\nPanics if ...` lowercases to a string
    containing `panic` twice; the body substring check at
    spec F4 looks for `panic!`, `unwrap`, `expect(`,
    `unreachable!`, `assert!`, `assert_eq!`, `assert_ne!`,
    `todo!`, `unimplemented!`, `debug_assert` and finds none
    of them in the `for chunk in bytes.as_chunks_mut::<N>().0
    { ... }` body (no `assert!` because
    `slice::as_chunks_mut`'s internal `assert!(N != 0)` is in
    std, not the call-site body; `as_chunks_mut` contains no
    substring match for any of the markers). All six entries
    are TP. The batch's value-add is dual: (a) shifts
    `comment-code` from single-pattern (Pattern C) coverage to
    two-pattern (Pattern B + Pattern C) coverage — the audit
    corpus now exercises 2 of the 3
    `docs/spec/comment-code-v0.md` patterns (Pattern A
    "Result / Option claim without matching return type" still
    owed), so a regression that breaks Pattern B but preserves
    Pattern C would now surface in the audit rather than going
    undetected; (b) the source-kind footprint stays at the
    existing six external kinds (this batch contributes under
    `github-commit`, already populated by batches
    1 / 3 / 4 / 11 / 12 / 13), so no new kind is invented and
    the source-list registry under README "Source list"
    remains unchanged. Selection rationale: zarrs is a
    long-lived (2023-) Rust implementation of the Zarr V3
    array-storage format, used as a Rust-native counterpart to
    Python's `zarr` library; the `bitround.rs` Pattern B
    family carries the wrong-documentation bug shape because
    the upstream code uses `slice::as_chunks_mut` (which
    silently drops the remainder rather than panicking) and
    the docstring was authored against an earlier mental model
    where the caller would have to provide a length-multiple-
    of-N buffer. A stable upstream commit and a stable bug
    shape across six instances — the same stability profile
    the batch-3 / 11 / 12 / 13 entries have. The
    `as_chunks_mut::<N>().0` idiom is the most interesting
    wrinkle in this batch: cntrdct's Pattern B body-substring
    check looks at the LITERAL body source, so even though
    `slice::as_chunks_mut`'s implementation contains an
    `assert!(N != 0)` at the std-library level, that text
    never appears in the call-site function body and Pattern B
    fires correctly. Updated audit numbers: `comment-code`
    tp=14 / fn=0 / 1.00 (up from 8/0/1.00, source_breakdown
    `github-commit` 14/14); `arg-swap`, `clone-drift`,
    `config-interaction`, `pr-miner`,
    `unreachable-after-terminator` unchanged; overall
    recall_upper_bound 0.49 (20 TP / 21 FN / 41 expected, up
    from 0.40 at batch 13). Corpus size 31 files (21 labelled
    + 10 density-support). Upward for the right reason: six
    new TP entries land cleanly within the existing 1.00
    detector AND extend the detector's pattern coverage from
    Pattern C only to Pattern B + Pattern C; the audit's
    denominator widens by 6 expected entries (from 35 to 41),
    so the percentage figure reflects the new labelled sample
    size and the new pattern coverage rather than detector
    improvement. Batch 14 also preserves the pr-miner mining
    margin because the new file is Rust — Python
    `{open} → {close}` mining-DB confidence stays at
    batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs
    (`get_ver`, `readfile`) remain TPs. SHA-256 is of the
    audit-corpus file as committed (minimal extract, per
    `benchmarks/audit-corpus/README.md` `Per-detector seed
    targets` item 4).
  - Fifteenth batch (done 2026-05-15): one expected entry
    shifting `comment-code` audit coverage from two-pattern
    (Pattern B + Pattern C, batches 3 / 11 / 12 / 13 / 14)
    to the complete three-pattern (Pattern A + Pattern B +
    Pattern C) triple by adding one TP from a sixth
    permissive-licensed Rust upstream — boundless-xyz/boundless
    at commit `1a2770b2d824df7d931f3fdf3907ae1633f9bc80`
    `crates/executor/src/backends/mod.rs` (Apache-2.0).
    `pub fn default_registry() -> Registry` at upstream line
    44 (corpus line 10) carries a `///` doc block whose
    prose contains the Pattern A trigger phrase `may fail`
    (per spec F3 the six case-folded substrings are
    `returns err`, `returns result`, `may fail`, `fallible`,
    `returns option`, `may return none`), but the function
    signature `() -> Registry` lacks the `Result` / `Option`
    substring required by F3's return-type negation, so the
    doc claim of fallibility is not propagated to the caller.
    The body absorbs constructor failures via
    `tracing::warn!` and returns a partially-populated
    `Registry` rather than propagating an error type — the
    textbook Tan SOSP 2007 §3.1 Pattern A
    ("Description Comments that describe what the function
    returns") bug shape, previously not exercised by the
    audit corpus. The entry is TP. The batch's value-add is
    dual: (a) shifts `comment-code` from two-pattern
    (Pattern B + Pattern C) to the complete three-pattern
    (Pattern A + Pattern B + Pattern C) coverage; (b)
    diversifies `comment-code`'s audit evidence to six
    upstreams (whisky-archive 4 Pattern C + tls-parser 2
    Pattern C + glium 1 Pattern C + pkg-config-rs 1
    Pattern C + zarrs 6 Pattern B + boundless 1 Pattern A).
    Updated audit numbers: `comment-code` tp=15 / fn=0 /
    1.00 (up from 14/0/1.00, source_breakdown
    `github-commit` 15/15); other five detectors unchanged;
    overall recall_upper_bound 0.50 (21 TP / 21 FN / 42
    expected, up from 0.49 at batch 14). Corpus size 32
    files. SHA-256 is of the audit-corpus file as committed
    (minimal extract, per `benchmarks/audit-corpus/README.md`
    `Per-detector seed targets` item 4).
  - Sixteenth batch (done 2026-05-15): two expected entries
    diversifying `comment-code` Pattern B audit coverage
    from a single upstream (zarrs only, batch 14) to two
    upstreams by adding two TPs from a seventh permissive-
    licensed Rust upstream — Amanieu/parking_lot at commit
    `d7828fff7b5d6327ae608e82db45f888b344449a`
    `core/src/parking_lot.rs` (MIT OR Apache-2.0,
    parking_lot_core-v0.9.12 release tag).
    `pub unsafe fn unpark_one` at upstream line 732 (corpus
    line 29) and `pub unsafe fn unpark_requeue` at upstream
    line 888 (corpus line 122) each carry a `///` doc block
    ending in `must not panic or call into any function in
    parking_lot.` Mapping against cntrdct's comment-code
    detector: spec F4 trigger (rendered doc string contains
    `panic` substring after `to_lowercase`) fires on both
    functions because the doc's `must not panic`
    lowercases to a string containing `panic`; the body
    substring check at spec F4 looks for `panic!`,
    `unwrap`, `expect(`, `unreachable!`, `assert!`,
    `assert_eq!`, `assert_ne!`, `todo!`, `unimplemented!`,
    `debug_assert` and finds none of them in either body
    (the bodies manipulate parking_lot's internal bucket
    queue via `lock_bucket`, `bucket.queue_head`,
    `link.set()`, `callback(result)`,
    `bucket.mutex.unlock()`, etc.; none of those identifiers
    contain a Pattern B marker substring). Both entries are
    TP. The two functions are top-level free `pub unsafe fn`
    items (not impl methods), so cntrdct's per-fn loop walks
    them correctly via the
    `root.children().filter(kind == "function_item")` scan
    in `src/detectors/comment_code.rs:147`. The doc here is
    a contract on callbacks (the callee must not panic)
    rather than a panic-prone implementation claim, but
    cntrdct's syntactic substring rule does not distinguish
    'must not panic' (callback contract) from 'panics if'
    (implementation claim) — both are spec F4 hits on the
    same trigger, and the labelled Pattern B denominator in
    the audit corpus now exercises both phrasings on two
    unrelated upstreams (Zarr-format codec helpers +
    parking_lot queue primitives). The batch's value-add is
    dual: (a) `comment-code` Pattern B diversifies from a
    single upstream (zarrs only at batch 14, 6/6 entries)
    to two upstreams on unrelated domains (zarrs Zarr-format
    data-type bindings 6 + parking_lot_core synchronization
    primitives 2), so a regression that broke one Pattern B
    bug shape while preserving the other would now surface
    in the audit rather than going undetected; (b) the
    source-kind footprint stays at the existing six external
    kinds (this batch contributes under `github-commit`,
    already populated by batches 1 / 3 / 4 / 11 / 12 / 13 /
    14 / 15), so no new kind is invented and the
    source-list registry under README "Source list" remains
    unchanged. Selection rationale: parking_lot_core is a
    long-lived (2016-) MIT OR Apache-2.0 synchronization-
    primitive crate used as the backing implementation for
    `std::sync::Mutex` (via the `parking_lot` family) and
    countless downstream crates; the `unpark_one` /
    `unpark_requeue` "must not panic" doc claim has remained
    stable across many releases and the panic-prohibition
    is genuinely a callback contract rather than an
    implementation claim — the audit corpus exercising both
    phrasings is a useful regression-detection breadth
    signal for spec F4. Updated audit numbers:
    `comment-code` tp=17 / fn=0 / 1.00 (up from 15/0/1.00,
    source_breakdown `github-commit` 17/17); `arg-swap`,
    `clone-drift`, `config-interaction`, `pr-miner`,
    `unreachable-after-terminator` unchanged; overall
    recall_upper_bound 0.52 (23 TP / 21 FN / 44 expected, up
    from 0.50 at batch 15). Corpus size 33 files (23
    labelled + 10 density-support). Upward for the right
    reason: two new TP entries land cleanly within the
    existing 1.00 detector AND break Pattern B's single-
    upstream dominance; the audit's denominator widens by 2
    expected entries (from 42 to 44), so the percentage
    figure reflects the new labelled sample size rather than
    detector improvement. Batch 16 also preserves the
    pr-miner mining margin because the new file is Rust —
    Python `{open} → {close}` mining-DB confidence stays at
    batch-10's 19/22 ≈ 0.864 ≥ 0.85, and both pr-miner TPs
    (`get_ver`, `readfile`) remain TPs. SHA-256 is of the
    audit-corpus file as committed (minimal extract, per
    `benchmarks/audit-corpus/README.md` `Per-detector seed
    targets` item 4).
  - Subsequent batches deepen coverage on the remaining 0.00
    detectors via either more labelled findings on the existing
    source kinds (now that pr-miner mines the
    `{open} → {close}` rule, future `open-never-closed`
    instances contribute as TPs rather than FN-by-sparsity
    provided each is accompanied by ≥ 4 paired density-
    support transactions to preserve the mining margin), or
    detector-side scope lifts under separate preregistrations:
    `unreachable-after-terminator` widening once F3 lifts to
    constant-condition / exception-typed reasoning; arg-swap /
    clone-drift / config-interaction TPs once the detectors'
    v0 scope is lifted by separate engineering with its own
    preregistration. `comment-code` is now at 1.00 on nine
    upstreams across all three
    `docs/spec/comment-code-v0.md` patterns (Pattern A from
    boundless batch 15, wasmtime batch 17, and rust-s3
    batch 18 — all three syntactic-Pattern-A sub-shapes
    (silent-absorb-and-log / documented-panic-on-failure /
    configuration-doc-references-external-fallibility)
    covered; Pattern B from zarrs batch 14 and
    parking_lot_core batch 16; Pattern C from batches
    3 / 11 / 12 / 13), so further batches on this detector
    would deepen existing patterns on additional upstreams
    (diminishing-returns regression-detection insurance for
    the existing three patterns on the existing sub-shape
    coverage) rather than introduce a fourth pattern or a
    fourth Pattern A sub-shape.
- Phase C — release-tag refresh discipline (done 2026-05-12):
  `benchmarks/audit-corpus/README.md` carries a new "Refresh
  discipline (Phase C)" section enumerating the on-tag procedure
  (re-run the audit, refresh the "Latest audit run" table,
  stage the README change in the same `chore(release): bump
  version` commit, no follow-up). `CLAUDE.md` "Release
  procedure" steps now include `cargo run --release --bin
  cntrdct -- calibrate --audit-recall benchmarks/audit-corpus`
  between the lockfile sync and the commit, and the
  non-negotiables list pins the same-commit rule and the no-op
  refresh policy (figures unchanged = no-op is fine; the
  discipline is the re-run, not the delta). No CI enforcement
  — the rationale tracks Q-13's: audit-recall is a property of
  the embedded priors and the shipped detector logic, both of
  which CI already gates, so a README-update gate would catch
  the same drift twice. The release-procedure non-negotiable is
  the gap-closer.

Q-15. SOTA baseline comparators

- Status: `[ ]`
- Goal: publish `cntrdct eval` with side-by-side precision /
  recall / F1 against state-of-the-art comparators on the same
  corpus. Pilot baselines: SourcererCC (Sajnani et al. 2016) for
  clone-drift and PyBugLab (Allamanis et al. 2021) for arg-swap.
  Each baseline ships as a Docker image so the comparison is
  reproducible from a clean environment.
- Acceptance: `cntrdct eval --baseline sourcerercc,pybuglab`
  produces a comparison table with cntrdct's numbers and each
  baseline's numbers; the table is linked from the README so the
  detector-level recall gap is on the record rather than
  implicit.
- Effort: 3-4 weeks.
- Depends on: Q-14 (so the corpus contains TPs the baselines can
  catch).

Q-16. cargo-mutants nightly mutation testing

- Status: `[x]` 2026-05-11
- Summary: `.github/workflows/mutants.yml` runs cargo-mutants on
  every UTC night at 06:00 (also on `workflow_dispatch`).
  `.cargo/mutants.toml` scopes the run to `src/detectors/**/*.rs`
  via `examine_globs`; the rest of the codebase is intentionally
  out of scope for this gate. The workflow installs cargo-mutants
  via `taiki-e/install-action`, runs `cargo mutants --no-shuffle
  -j 2`, treats exit code 2 (some missed) as expected, and tallies
  `mutants.out/{caught,missed,unviable,timeout}.txt` to compute a
  catch rate. The step writes a markdown table to
  `$GITHUB_STEP_SUMMARY` plus the verbatim missed-mutant list, then
  fails the job when `caught / (caught + missed) < 0.80`.
  `mutants.out/` is uploaded as an artifact (30-day retention) for
  off-runner inspection. `.gitignore` adds `/mutants.out/` and
  `/mutants.out.old/` so accidental local runs do not leak the
  per-mutant log dirs into commits.
- Caveat: cargo-mutants is too slow to validate locally (multi-hour
  runs even on six detectors), so the first nightly run on master is
  the real signal for whether the codebase already satisfies the
  80% gate. If the first run fails, follow-up work is to either
  strengthen the test suite at the unguarded judgement boundaries
  the missed-mutants list calls out, or temporarily relax the gate
  while the detector tests catch up — both are roadmap-scope
  decisions, not config tweaks.
- Evidence: Just, Jalali, Inozemtseva, Ernst, Holmes, Fraser (2014)
  "Are mutants a valid substitute for real faults in software
  testing?" FSE 2014 (mutation-detection ↔ real-bug detection
  agreement); cargo-mutants project documentation
  (<https://mutants.rs/>) for the per-mutant test-rerun semantics.

Future Q-series candidates (not yet scheduled):

- Apriori v1 → FP-growth in pr-miner. Already noted in
  `docs/spec/pr-miner-v0.md` future work; revisit once Q-15 is
  in place so before/after F1 numbers are publishable on a
  consistent baseline.
- Layer 3 ML-detector ensemble. Run PyBugLab / GraphCodeBERT
  alongside the LLM judge; preserves Layer 1-2 / Layer 4
  determinism while lifting the recall ceiling. This crosses
  the P3 boundary as currently written and would require a new
  OSF preregistration, so it stays out of the numbered Q-series
  until that prereg lands.

## Suggested execution order

Phase A (Tier 1, ~4-6 days total):

1. T1-1 GitHub Actions CI
2. T1-5 rustdoc on `cntrdct-core`
3. T1-2 crates.io metadata
4. T1-4 examples directory
5. T1-3 README polish
6. T1-6 LICENSE review
7. T1-7 GitHub Pages essay site

Phase B (small announcement; concurrent with Phase C):

7. Public read-only repository, no marketing yet.

Phase C (Tier 2 in parallel with Practical track):

8. T2-10 rayon parallelisation
9. T2-7 suppression mechanism
10. T2-11 cargo subcommand
11. T2-8 pre-built binaries
12. P-1 β corpus collection (Practical track)
13. P-3 SARIF validator integration
14. T2-9 GitHub Action wrapper

Phase D (Multi-language; interrupts the original Practical-track
sequence so new detectors are designed multi-language from day one):

15. M-6 citation policy doc (cheap; locks in the P1 extension first)
16. M-1 language abstraction foundation
17. M-2 pilot Python detector (`unreachable-after-terminator-py`)
18. M-3 cross-cutting detectors to Python
19. M-5 multi-language tooling surface
20. M-4 Python β corpus

Phase E (Practical-track items, resumed once Phase D lands):

21. P-4 ranker recalibration (now over Rust + Python corpora)
22. P-2 pr-miner detector (multi-language from inception, supersedes
    the original `pr-miner-rust` framing)
23. P-5 v0.2.0-beta release

Phase F (Tier 3 / 4 organically after launch):

24. T4-17 / T4-18 / T4-19 community scaffolding (landed)
24a. T4-20 / T4-21 deferred per maintainer decision
25. T3-15 git-cliff release-notes pipeline (landed 2026-05-08)
26. T3-16 telemetry-free assurance (landed 2026-05-08)
27. T3-14 distribution channels — cargo-binstall + Homebrew tap
    landed 2026-05-08; AUR dropped from scope
28. T3-12 LSP server — Phase 1 scaffolding + Phase 1.b document
    events + Finding -> Diagnostic mapping landed 2026-05-08;
    Phase 1.c per-URI didChange debouncing landed 2026-05-09;
    Phase 1.c+ per-URI generation counter landed 2026-05-09; Phase
    2 (vscode-cntrdct extension) next
29. T3-13 mdBook user guide (essay migration to external blog
    precedes Jekyll retirement, see T3-13 note)

Phase G (post-beta.1 quality-audit RC1 blockers; 1-2 days total,
required before the next release tag):

28. Q-1 SARIF detectors array missing pr-miner
29. Q-2 SARIF informationUri placeholder
30. Q-3 clone-drift doc-comment / value drift
31. Q-4 wiring consistency test
32. Q-5 SARIF Severity::Info mapping rationale

Phase H (RC1 governance and hygiene; 2-3 weeks, in parallel with
the Phase F community items):

33. Q-6 citation retraction monitor
34. Q-7 venue tier whitelist
35. Q-8 preregistration deviation log
36. Q-9 Python attribute-style suppression
37. Q-10 ParserProvider seam tightening

Phase I (RC2 / v0.2.0 methodology lift; 2-3 months):

38. Q-11 small-N statistical interval switching (landed 2026-05-10)
39. Q-12 LLM calibration post-hoc Platt fit (landed 2026-05-10)
40. Q-13 cross-model κ audit (landed 2026-05-11)
41. Q-14 recall-audit harness — Phase A scaffolding + Phase B
    first batch landed 2026-05-11; Phase B batch 2 (deepening
    `unreachable-after-terminator` recall measurement against
    additional rustc UI testset files), Phase B batch 3
    (broadening to a third detector, `comment-code`, via the
    Tan SOSP 2007 Pattern C bug in `sidan-lab/whisky-archive`),
    Phase C release-tag refresh discipline (README
    "Refresh discipline" section + CLAUDE.md release-procedure
    steps), Phase B batch 4 (introducing the
    `paper-appendix` source kind via three PyPIBugs ArgSwap
    entries on permissive-licensed Python repositories —
    `c137digital/unv_app` MIT, `mwouts/nbrmd` MIT,
    `markokr/rarfile` ISC; all three FN against cntrdct's
    narrow Rice-2017 arg-swap detector, surfacing the gap
    rather than inflating it), Phase B batch 5 (introducing the
    `clippy` source kind via two rust-clippy UI tests pinned at
    master commit `c4b8c6d4` —
    `tests/ui/if_same_then_else.rs` and
    `tests/ui/branches_sharing_code/shared_at_top.rs`; both FN
    against cntrdct's clone-drift detector by spec F2,
    introducing clone-drift to the corpus at
    recall_upper_bound 0.00 and surfacing the v0 fn-level
    scope choice), and Phase B batch 6 (introducing the
    `config-interaction` detector via the rustc UI test for
    the `cfg.attr.duplicates` Rust Reference behaviour —
    `rust-lang/rust@29b75901
    tests/ui/cfg/both-true-false.rs`; both `fn foo()` items FN
    against cntrdct's config-interaction detector by spec F5
    because atomic `true` / `false` lack the `not(...)`
    wrapper, introducing config-interaction at
    recall_upper_bound 0.00 and surfacing the v0
    require-`not(...)`-wrapper scope choice) all landed
    2026-05-12; Phase B batch 7 (introducing the `codeql`
    source kind via the CodeQL Python `UnreachableCode`
    query test fixture — `github/codeql@592c7c04
    python/ql/test/query-tests/Statements/unreachable/test.py`,
    MIT; six expected entries — one TP at the `for x in
    first_unreachable_stmt():` following `return 5` Python F3
    match, five FN on constant-condition branch and
    typed-exception unreachability bug shapes outside cntrdct's
    F3 terminator set) landed 2026-05-12;
    Phase B batch 8 (introducing the `semgrep` source kind AND
    the sixth and final cntrdct detector `pr-miner` to the
    corpus in the same commit via the Semgrep registry rule
    `open-never-closed` applied to TuGraph-family/tugraph-db
    `release/det_ver.py` Apache-2.0, closing the
    originally-named Phase B blockers without invoking a
    Semgrep Rules License v1.0 carve-out — only the rule's
    stable identity is cited and the redistributed target is
    permissive code the rule fires on; the entry is FN by spec
    F3 Apriori mining sparsity at the v0 audit-corpus density,
    NOT by spec F2 extractor scope) landed 2026-05-13;
    Phase B batch 9 (deepening the `pr-miner` denominator on the
    existing `semgrep` source kind via a second
    permissive-licensed `open-never-closed` instance — the same
    Semgrep registry rule applied to
    gregmuellegger/django-mobile@fafc3890 `setup.py`
    BSD-3-Clause; the FN at `def readfile(filename):` upstream
    line 13 is mining-sparsity-bound the same way batch 8's
    `get_ver` is, lifting pr-miner's denominator to 2 without
    crossing the F3 confidence threshold so the existing FN
    class is broadened rather than transformed into TP — the
    next pr-miner move shifts from denominator widening to
    numerator construction via paired open+close transactions on
    permissive Python upstreams) landed 2026-05-13; Phase B
    batch 10 (ten density-support files adding 18 paired
    open+close top-level Python transactions to lift the
    corpus-wide `{open} → {close}` mined-rule confidence from
    1/4 = 0.25 to 19/22 ≈ 0.864 ≥ 0.85; each density-support
    file ships with `expected: []` because the Semgrep
    `open-never-closed` labeller produces no findings on files
    where every open is closed, so the labeller's negative
    outcome is faithfully captured; spec F3 Apriori mines the
    rule, spec F4 scans the full transaction set for
    violations, and both batch-8 `get_ver` and batch-9
    `readfile` flip from FN to TP without any detector-side
    change — pr-miner moves from 0/2/0.00 to 2/0/1.00 and
    overall recall_upper_bound jumps from 0.26 to 0.32) landed
    2026-05-13; Phase B batch 11 (two `comment-code` TPs from
    a second permissive-licensed Rust upstream rusticata/tls-
    parser@65541559 `src/tls_handshake.rs` MIT OR Apache-2.0;
    both `pub fn parse_tls_handshake_next_protocol` and
    `pub fn parse_tls_handshake_msg_next_protocol` carry a
    `///` doc block ending in `Deprecated in favour of ALPN.`
    without `#[deprecated]` — the same Tan SOSP 2007 §3.2
    Pattern C bug shape the batch-3 `sidan-lab/whisky-archive`
    con_str* family flags. Diversifies `comment-code`'s audit
    evidence from a single upstream to two upstreams on
    unrelated domains; `comment-code` moves to 6/0/1.00 and
    overall recall_upper_bound to 0.36 (12 TP / 21 FN / 33
    expected). pr-miner mining margin preserved because the
    new file is Rust — Python `{open} → {close}` confidence
    stays at 19/22 ≈ 0.864) landed 2026-05-13; Phase B batch 12
    (one `comment-code` TP from a third permissive-licensed
    Rust upstream glium/glium@8d6fd34d
    `src/draw_parameters/mod.rs` Apache-2.0;
    `pub fn validate(...)` carries a single-line `///` doc
    block reading `DEPRECATED. Checks parameters and returns
    an error if something is wrong.` without `#[deprecated]` —
    the same Pattern C bug shape on a third unrelated domain
    (OpenGL bindings). Diversifies `comment-code`'s audit
    evidence to three upstreams (whisky-archive 4 +
    tls-parser 2 + glium 1); `comment-code` moves to 7/0/1.00
    and overall recall_upper_bound to 0.38 (13 TP / 21 FN /
    34 expected). pr-miner mining margin preserved because
    the new file is Rust) landed 2026-05-13; Phase B batch 13
    (one `comment-code` TP from a fourth permissive-licensed
    Rust upstream rust-lang/pkg-config-rs@f36d32a0 `src/lib.rs`
    MIT OR Apache-2.0; `pub fn find_library` carries a
    single-line `///` doc block reading `Deprecated in favor
    of the probe_library function` and a `#[doc(hidden)]`
    attribute but no `#[deprecated]` runtime attribute — the
    same Pattern C bug shape on a fourth unrelated domain
    (build-tool / system-package bindings). The
    `#[doc(hidden)]` attribute does NOT suppress Pattern C
    because the walker checks the literal first identifier
    (`doc` vs. `deprecated`), confirming Pattern C
    distinguishes visibility-hiding from runtime-lint
    enforcement. Diversifies `comment-code`'s audit evidence
    to four upstreams (whisky-archive 4 + tls-parser 2 +
    glium 1 + pkg-config-rs 1); `comment-code` moves to
    8/0/1.00 and overall recall_upper_bound to 0.40 (14 TP /
    21 FN / 35 expected). pr-miner mining margin preserved
    because the new file is Rust) landed 2026-05-14; Phase B
    batch 14 (six `comment-code` TPs shifting the detector's
    audit coverage from single-pattern Pattern C only to
    two-pattern Pattern B + Pattern C, via a fifth permissive-
    licensed Rust upstream zarrs/zarrs@3b944c57
    `zarrs_data_type/src/codec_traits/bitround.rs`
    MIT OR Apache-2.0; six top-level `pub fn` items
    `round_bytes_int16/int32/int64`,
    `round_bytes_float16/float32/float64` each carry a
    `# Panics\nPanics if \`bytes.len()\` is not a multiple of
    N.` doc claim for N in {2, 4, 8}, but the body uses
    `bytes.as_chunks_mut::<N>().0` which never panics on
    non-multiple-of-N lengths — `slice::as_chunks_mut::<N>`
    (stabilised in Rust 1.88.0, 2025-06-26) returns
    `(&mut [[T; N]], &mut [T])` where the second tuple
    element is the remainder; `.0` accesses only the chunked
    arrays so trailing bytes are silently dropped rather than
    triggering the documented panic — the textbook Tan SOSP
    2007 §3.2 Pattern B bug shape, previously not exercised
    by the audit corpus. Diversifies `comment-code`'s audit
    evidence to five upstreams (whisky-archive 4 +
    tls-parser 2 + glium 1 + pkg-config-rs 1 + zarrs 6) AND
    from single-pattern Pattern C to two-pattern
    Pattern B + Pattern C; `comment-code` moves to 14/0/1.00
    and overall recall_upper_bound to 0.49 (20 TP / 21 FN /
    41 expected). pr-miner mining margin preserved because
    the new file is Rust) landed 2026-05-15; Phase B batch 15
    (one `comment-code` TP shifting the detector's audit
    coverage from two-pattern Pattern B + Pattern C to the
    complete three-pattern Pattern A + Pattern B + Pattern C
    triple, via a sixth permissive-licensed Rust upstream
    boundless-xyz/boundless@1a2770b2
    `crates/executor/src/backends/mod.rs` Apache-2.0;
    `pub fn default_registry() -> Registry` at upstream line
    44 carries a `///` doc block whose prose contains the
    Pattern A trigger phrase `may fail` (per spec F3 the six
    case-folded substrings are `returns err`,
    `returns result`, `may fail`, `fallible`,
    `returns option`, `may return none`), but the function
    signature `() -> Registry` lacks the `Result` / `Option`
    substring required by F3's return-type negation, so the
    doc claim of fallibility is not propagated to the caller.
    The body absorbs constructor failures via
    `tracing::warn!` and returns a partially-populated
    `Registry` rather than propagating an error type — the
    textbook Tan SOSP 2007 §3.1 Pattern A bug shape
    ("Description Comments that describe what the function
    returns"), previously not exercised by the audit corpus;
    the companion documentation prose explains the
    absorb-and-log behaviour but does not move the failure
    information into the type system, so callers cannot
    distinguish a fully-populated `Registry` from a
    partially-populated one programmatically. Diversifies
    `comment-code`'s audit evidence to six upstreams
    (whisky-archive 4 Pattern C + tls-parser 2 Pattern C +
    glium 1 Pattern C + pkg-config-rs 1 Pattern C + zarrs 6
    Pattern B + boundless 1 Pattern A) AND from two-pattern
    Pattern B + Pattern C to the complete three-pattern
    Pattern A + Pattern B + Pattern C triple; `comment-code`
    moves to 15/0/1.00 and overall recall_upper_bound to
    0.50 (21 TP / 21 FN / 42 expected). pr-miner mining
    margin preserved because the new file is Rust) landed
    2026-05-15; Phase B batch 16 (two `comment-code`
    Pattern B TPs diversifying Pattern B audit coverage from
    a single upstream (zarrs only, batch 14) to two
    upstreams, via a seventh permissive-licensed Rust
    upstream Amanieu/parking_lot@d7828fff
    `core/src/parking_lot.rs` MIT OR Apache-2.0,
    parking_lot_core-v0.9.12 release tag;
    `pub unsafe fn unpark_one` at upstream line 732 (corpus
    line 29) and `pub unsafe fn unpark_requeue` at upstream
    line 888 (corpus line 122) each carry a `///` doc block
    ending in `must not panic or call into any function in
    parking_lot.` cntrdct's spec F4 trigger fires
    (`doc_lc.contains("panic")` matches both), and the body
    substring check finds none of `panic!` / `unwrap` /
    `expect(` / `unreachable!` / `assert!` / `assert_eq!` /
    `assert_ne!` / `todo!` / `unimplemented!` /
    `debug_assert` in either body — the same Tan SOSP 2007
    §3.2 Pattern B bug shape the batch-14 zarrs
    `round_bytes_*` family flags on the panic-mismatch
    direction. The doc here is a contract on callbacks
    (callee must not panic) rather than a panic-prone
    implementation claim, but cntrdct's syntactic substring
    rule does not distinguish 'must not panic' (callback
    contract) from 'panics if' (implementation claim) —
    both are spec F4 hits on the same trigger, and the
    labelled Pattern B denominator now exercises both
    phrasings on two unrelated upstreams (Zarr-format codec
    helpers + parking_lot queue primitives). Diversifies
    `comment-code`'s audit evidence to seven upstreams
    (whisky-archive 4 Pattern C + tls-parser 2 Pattern C +
    glium 1 Pattern C + pkg-config-rs 1 Pattern C + zarrs 6
    Pattern B + boundless 1 Pattern A + parking_lot_core 2
    Pattern B), breaking Pattern B's single-upstream
    dominance without introducing a new pattern;
    `comment-code` moves to 17/0/1.00 and overall
    recall_upper_bound to 0.52 (23 TP / 21 FN / 44
    expected). pr-miner mining margin preserved because the
    new file is Rust) landed 2026-05-15; Phase B batch 17
    (one `comment-code` Pattern A TP diversifying Pattern A
    audit coverage from a single upstream (boundless only,
    batch 15) to two upstreams, via an eighth permissive-
    licensed Rust upstream
    bytecodealliance/wasmtime@63330f11
    `cranelift/assembler-x64/src/fuzz.rs` Apache-2.0 WITH
    LLVM-exception. `pub fn roundtrip` at upstream line 26
    (corpus line 13) carries a `///` doc block whose
    `# Panics` section contains the Pattern A trigger phrase
    `may fail`, but the function signature
    `pub fn roundtrip(inst: &Inst<FuzzRegs>)` has unit return
    so F3's return-type negation passes and Pattern A fires;
    the body uses `unwrap` and `assert_eq!` (the fuzzer
    intentionally panics to express failure to the
    `arbitrary` harness), so spec F4 Pattern B's body-marker
    negation suppresses Pattern B even though the doc
    contains the substring `panic` — only Pattern A fires
    on this function. Introduces the second Pattern A
    sub-shape (documented-panic-on-failure: body uses
    `unwrap` / `assert_eq!` intentionally to surface failure
    via panic) contrasting batch 15's boundless silent-
    absorb-and-log sub-shape (body uses `tracing::warn!` and
    returns a partial `Registry`); the labelled Pattern A
    denominator in the audit corpus now exercises both
    sub-shapes on two unrelated upstreams (zkVM executor
    registry + assembler-fuzzer infrastructure).
    Diversifies `comment-code`'s audit evidence to eight
    upstreams (whisky-archive 4 Pattern C + tls-parser 2
    Pattern C + glium 1 Pattern C + pkg-config-rs 1 Pattern
    C + zarrs 6 Pattern B + boundless 1 Pattern A +
    parking_lot_core 2 Pattern B + wasmtime 1 Pattern A),
    breaking Pattern A's single-upstream dominance without
    introducing a new pattern; `comment-code` moves to
    18/0/1.00 and overall recall_upper_bound to 0.53 (24 TP
    / 21 FN / 45 expected). pr-miner mining margin preserved
    because the new file is Rust) landed 2026-05-15; Phase B
    batch 18 (one `comment-code` Pattern A TP diversifying
    Pattern A audit coverage from two upstreams (boundless +
    wasmtime) to three upstreams, via a ninth permissive-
    licensed Rust upstream durch/rust-s3@771be165
    `s3/src/lib.rs` MIT. `pub fn set_retries(retries: u8)` at
    upstream line 54 (corpus line 23) carries a `///` doc
    block whose first line reads `Sets the number of retries
    for operations that may fail and need to be retried.`;
    the substring `may fail` fires the same Pattern A check,
    the function signature `pub fn set_retries(retries: u8)`
    has unit return so F3's return-type negation passes, and
    the body
    `RETRIES.store(retries, std::sync::atomic::Ordering::SeqCst);`
    is unconditionally infallible so neither spec F4 Pattern B
    nor spec F5 Pattern C fires (the doc contains no `panic`
    or `deprecated` substrings) — only Pattern A fires on this
    function. Introduces the third Pattern A sub-shape
    (configuration-doc-references-external-fallibility: the
    doc's `may fail` substring describes downstream S3 request
    operations the configured retries protect against, not the
    function body itself, while the function body is
    unconditionally infallible) contrasting batch 15's
    boundless silent-absorb-and-log sub-shape and batch 17's
    wasmtime documented-panic-on-failure sub-shape; the
    labelled Pattern A denominator in the audit corpus now
    exercises all three syntactic-Pattern-A sub-shapes on
    three unrelated upstreams (zkVM executor registry +
    assembler-fuzzer infrastructure + S3-client configuration
    setter). Diversifies `comment-code`'s audit evidence to
    nine upstreams (whisky-archive 4 Pattern C + tls-parser 2
    Pattern C + glium 1 Pattern C + pkg-config-rs 1 Pattern C
    + zarrs 6 Pattern B + boundless 1 Pattern A +
    parking_lot_core 2 Pattern B + wasmtime 1 Pattern A +
    rust-s3 1 Pattern A), completing Pattern A's
    sub-shape coverage without introducing a new pattern;
    `comment-code` moves to 19/0/1.00 and overall
    recall_upper_bound to 0.54 (25 TP / 21 FN / 46 expected).
    pr-miner mining margin preserved because the new file is
    Rust) landed 2026-05-15; Phase B batch 19 (one
    `comment-code` Pattern B TP diversifying Pattern B audit
    coverage from two upstreams (zarrs + parking_lot_core) to
    three upstreams, via a tenth permissive-licensed Rust
    upstream vortex-data/vortex@4c1ae92d
    `vortex-buffer/src/bit/mod.rs` v0.70.0 release tag,
    Apache-2.0. `pub fn get_bit(buf: &[u8], index: usize) ->
    bool` at upstream line 33 (corpus line 11) carries a `///`
    doc block whose `# Panics` section reads `Panics if `index`
    is not between 0 and length of `buf * 8`.`; the substring
    `panic` fires the same Pattern B check, and the body
    `buf[index / 8] & (1 << (index % 8)) != 0` contains NONE of
    cntrdct's Pattern B body markers (panic-bang, unwrap,
    expect-paren, unreachable-bang, assert-bang, assert-eq-
    bang, assert-ne-bang, todo-bang, unimplemented-bang,
    debug-assert), so spec F4's body-marker negation passes and
    Pattern B fires. The body DOES panic on the documented
    out-of-bounds condition (slice bracket indexing on `&[u8]`
    panics through the `core::ops::Index` impl for `[T]` when
    `index / 8 >= buf.len()`), but the panic is implicit in
    the slice `Index` impl rather than expressed via any of
    cntrdct's syntactic body-marker substrings, so spec F4
    fires on syntactic substring rule rather than semantic
    correctness. Introduces the third Pattern B sub-shape
    (implicit-indexing-panic: body uses bracket indexing that
    panics on OOB through the slice `Index` impl without any
    of cntrdct's body-marker substrings) contrasting batch
    14's zarrs silent-drop-on-mismatch sub-shape (body uses
    `as_chunks_mut().0` which silently drops the trailing
    remainder rather than panicking) and batch 16's
    parking_lot_core callback-contract sub-shape (doc says
    "must not panic" as a contract on the supplied callback,
    function body has no panic-producing constructs); the
    labelled Pattern B denominator in the audit corpus now
    exercises all three syntactic-Pattern-B sub-shapes on
    three unrelated upstreams (Zarr-format codec helpers +
    parking_lot queue primitives + vortex-buffer bit-packed
    bitmap helpers). Diversifies `comment-code`'s audit
    evidence to ten upstreams (whisky-archive 4 Pattern C +
    tls-parser 2 Pattern C + glium 1 Pattern C + pkg-config-rs
    1 Pattern C + zarrs 6 Pattern B + boundless 1 Pattern A +
    parking_lot_core 2 Pattern B + wasmtime 1 Pattern A +
    rust-s3 1 Pattern A + vortex-buffer 1 Pattern B),
    completing Pattern B's sub-shape coverage without
    introducing a new pattern; `comment-code` moves to
    20/0/1.00 and overall recall_upper_bound to 0.55 (26 TP /
    21 FN / 47 expected). pr-miner mining margin preserved
    because the new file is Rust) landed 2026-05-15;
    Phase B batch 20 (two `comment-code` Pattern C TPs
    diversifying Pattern C audit coverage from four upstreams
    (whisky-archive 4 + tls-parser 2 + glium 1 +
    pkg-config-rs 1, batches 3 / 11 / 12 / 13) to five
    upstreams, via an eleventh permissive-licensed Rust
    upstream MystenLabs/sui@add9d472
    `crates/mysten-metrics/src/metered_channel.rs`,
    Apache-2.0. `pub fn channel<T>(size: usize, gauge:
    &IntGauge) -> (Sender<T>, Receiver<T>)` at upstream line
    321 (corpus line 8) carries a two-line `///` doc block
    whose second line reads `Deprecated: use
    `monitored_mpsc::channel` instead.`, and `pub fn
    channel_with_total<T>` at upstream line 339 (corpus line
    26) carries a single-line `///` doc block reading the
    same; both functions carry the `#[track_caller]`
    attribute (which propagates the caller location for panic
    reporting but does NOT trigger the Rust deprecation
    lints) and neither carries `#[deprecated]`, so spec F5
    Pattern C fires on both — the same Tan SOSP 2007 §3.2
    Pattern C bug shape batches 3 / 11 / 12 / 13 flag, on a
    fifth unrelated domain (async-channel metrics wrapper in
    Sui's blockchain runtime observability layer). The
    `#[track_caller]` attribute is structurally analogous to
    batch-13 pkg-config-rs's `#[doc(hidden)]` non-suppressive
    attribute: both are attributes present alongside the
    deprecation prose without triggering the `#[deprecated]`
    lint, confirming again that Pattern C distinguishes the
    literal first identifier of the attribute path
    (`track_caller` / `doc` vs. `deprecated`) rather than
    interpreting the attribute's behavioural semantics.
    Diversifies `comment-code`'s audit evidence to eleven
    upstreams (whisky-archive 4 Pattern C + tls-parser 2
    Pattern C + glium 1 Pattern C + pkg-config-rs 1 Pattern
    C + zarrs 6 Pattern B + boundless 1 Pattern A +
    parking_lot_core 2 Pattern B + wasmtime 1 Pattern A +
    rust-s3 1 Pattern A + vortex-buffer 1 Pattern B + sui
    mysten-metrics 2 Pattern C), lifting Pattern C from four
    upstreams to five without introducing a new pattern;
    `comment-code` moves to 22/0/1.00 and overall
    recall_upper_bound to 0.57 (28 TP / 21 FN / 49 expected).
    pr-miner mining margin preserved because the new file is
    Rust) landed 2026-05-15; Phase B batch 21 (one
    `comment-code` Pattern C TP diversifying Pattern C audit
    coverage from five upstreams (whisky-archive 4 + tls-parser
    2 + glium 1 + pkg-config-rs 1 + sui mysten-metrics 2,
    batches 3 / 11 / 12 / 13 / 20) to six upstreams, via a
    twelfth permissive-licensed Rust upstream
    mcgoo/vcpkg-rs@fa42994a `src/lib.rs`, MIT OR Apache-2.0.
    `pub fn probe_package(name: &str) -> Result<Library, Error>`
    at upstream line 293 (corpus line 7) carries a single-line
    `///` doc block reading `Deprecated in favor of the
    find_package function` and a `#[doc(hidden)]` non-
    suppressive attribute, but no `#[deprecated]` runtime
    attribute, so spec F5 Pattern C fires. Structurally pairs
    with batch-13 pkg-config-rs as the Windows side of the
    build-script system-package binding family: pkg-config-rs
    covers Unix pkg-config tool bindings; vcpkg-rs covers
    Microsoft Windows vcpkg C/C++ package manager bindings.
    Both upstreams independently exhibit the same Pattern C bug
    shape (`/// Deprecated in favor of X` single-line doc +
    `#[doc(hidden)]` non-suppressive attribute + body
    delegating to the replacement function) on functionally-
    paired native-library-discovery crates targeting different
    platform ecosystems. Diversifies `comment-code`'s audit
    evidence to twelve upstreams (whisky-archive 4 Pattern C +
    tls-parser 2 Pattern C + glium 1 Pattern C + pkg-config-rs
    1 Pattern C + zarrs 6 Pattern B + boundless 1 Pattern A +
    parking_lot_core 2 Pattern B + wasmtime 1 Pattern A +
    rust-s3 1 Pattern A + vortex-buffer 1 Pattern B + sui
    mysten-metrics 2 Pattern C + vcpkg-rs 1 Pattern C), lifting
    Pattern C from five upstreams to six without introducing a
    new pattern; `comment-code` moves to 23/0/1.00 and overall
    recall_upper_bound to 0.58 (29 TP / 21 FN / 50 expected).
    pr-miner mining margin preserved because the new file is
    Rust) landed 2026-05-16; Phase B batch 22 (one
    `comment-code` Pattern C TP diversifying Pattern C audit
    coverage from six upstreams (whisky-archive 4 + tls-parser
    2 + glium 1 + pkg-config-rs 1 + sui mysten-metrics 2 +
    vcpkg-rs 1, batches 3 / 11 / 12 / 13 / 20 / 21) to seven
    upstreams, via a thirteenth permissive-licensed Rust
    upstream overdrivenpotato/rust-vst2@244e14bd
    `src/interfaces.rs`, MIT. `pub fn process_deprecated(
    _effect: *mut AEffect, _inputs_raw: *mut *mut f32,
    _outputs_raw: *mut *mut f32, _samples: i32) { }` at
    upstream line 17 (corpus line 6) carries a single-line
    `///` doc block reading `Deprecated process function.`
    but does not carry the `#[deprecated]` runtime attribute,
    so spec F5 Pattern C fires. The function body is the empty
    block `{ }` retained as an ABI / API placeholder for
    callers that link against the old symbol while migrating to
    the replacement `process_replacing` /
    `process_replacing_f64` family — a new body-shape variant
    within Pattern C, contrasting prior body shapes that
    delegate to the replacement (batch-3 whisky-archive con_str
    → constr, batch-13 pkg-config-rs find_library →
    probe_library, batch-21 vcpkg-rs probe_package →
    find_package) and prior body shapes that retain the
    original implementation in-tree (batch-11 tls-parser
    parse_tls_handshake_*next_protocol family, batch-12 glium
    validate, batch-20 sui mysten-metrics channel /
    channel_with_total). cntrdct's spec F5 Pattern C check
    ignores the body — only the doc and adjacent attributes
    are inspected — so the stub-body case fires identically to
    delegate-body and in-tree-body cases, confirming the
    syntactic-only design. rust-vst2 is the audio / DSP plugin
    host domain (VST 2.4 API implementation in Rust for
    creating audio plugins and hosts), unrelated to the prior
    six Pattern C domains. Diversifies `comment-code`'s audit
    evidence to thirteen upstreams (whisky-archive 4 Pattern C
    + tls-parser 2 Pattern C + glium 1 Pattern C + pkg-config-
    rs 1 Pattern C + zarrs 6 Pattern B + boundless 1 Pattern A
    + parking_lot_core 2 Pattern B + wasmtime 1 Pattern A +
    rust-s3 1 Pattern A + vortex-buffer 1 Pattern B + sui
    mysten-metrics 2 Pattern C + vcpkg-rs 1 Pattern C +
    rust-vst2 1 Pattern C), lifting Pattern C from six
    upstreams to seven without introducing a new pattern;
    `comment-code` moves to 24/0/1.00 and overall
    recall_upper_bound to 0.59 (30 TP / 21 FN / 51 expected).
    pr-miner mining margin preserved because the new file is
    Rust) landed 2026-05-16; Phase B batch 23 (one
    `comment-code` Pattern C TP diversifying Pattern C audit
    coverage from seven upstreams (whisky-archive 4 +
    tls-parser 2 + glium 1 + pkg-config-rs 1 + sui
    mysten-metrics 2 + vcpkg-rs 1 + rust-vst2 1, batches 3 /
    11 / 12 / 13 / 20 / 21 / 22) to eight upstreams, via a
    fourteenth permissive-licensed Rust upstream
    always-further/nono@2e5504f2
    `crates/nono-cli/src/deprecated_schema.rs`, Apache-2.0.
    `pub fn warn_for_deprecated_flags(args:
    &[std::ffi::OsString])` at upstream line 196 (corpus line
    10) carries a four-line `///` doc block whose later half
    (after a blank `///` separator) reads `DEPRECATED: delete
    when all long-flag aliases in this module are removed / in
    v1.0.0. See module-level removal steps.` but does not
    carry the `#[deprecated]` runtime attribute, so spec F5
    Pattern C fires. Introduces the meta-deprecation-warning-
    emitter sub-shape within Pattern C: the function body
    iterates `detect_deprecated_flags(args)`, matches each
    canonical legacy spelling against its replacement
    (`--override-deny` → `--bypass-protection`), and calls
    `emit_deprecation_warning(...)` — its purpose is to emit
    per-call-site diagnostics for OTHER deprecated long-flag
    aliases routed through clap, while itself being marked
    deprecated in the module-level removal checklist; the
    function and the flags it warns about are co-scheduled for
    removal in v1.0.0, so the doc claim is forward-looking
    rather than legacy, contrasting prior body shapes that
    delegate to the replacement (batch-3 whisky-archive
    con_str → constr, batch-13 pkg-config-rs find_library →
    probe_library, batch-21 vcpkg-rs probe_package →
    find_package), prior body shapes that retain the original
    implementation in-tree (batch-11 tls-parser
    parse_tls_handshake_*next_protocol family, batch-12 glium
    validate, batch-20 sui mysten-metrics channel /
    channel_with_total), and the batch-22 rust-vst2 stub-body
    shape (empty `{ }` ABI placeholder). cntrdct's spec F5
    Pattern C check ignores the body — only the doc and
    adjacent attributes are inspected — so the meta-warning-
    emitter case fires identically to delegate-body,
    in-tree-body, and stub-body cases, confirming the
    syntactic-only design. nono is the capability-based
    sandbox CLI domain (fine-grained policy brokering for
    agent operating contexts, zero setup and zero latency),
    unrelated to the prior seven Pattern C domains.
    Diversifies `comment-code`'s audit evidence to fourteen
    upstreams (whisky-archive 4 Pattern C + tls-parser 2
    Pattern C + glium 1 Pattern C + pkg-config-rs 1 Pattern C
    + zarrs 6 Pattern B + boundless 1 Pattern A +
    parking_lot_core 2 Pattern B + wasmtime 1 Pattern A +
    rust-s3 1 Pattern A + vortex-buffer 1 Pattern B + sui
    mysten-metrics 2 Pattern C + vcpkg-rs 1 Pattern C +
    rust-vst2 1 Pattern C + nono 1 Pattern C), lifting Pattern
    C from seven upstreams to eight without introducing a new
    pattern; `comment-code` moves to 25/0/1.00 and overall
    recall_upper_bound to 0.60 (31 TP / 21 FN / 52 expected).
    pr-miner mining margin preserved because the new file is
    Rust) landed 2026-05-16; Phase B batch 24 (one
    `comment-code` Pattern C TP diversifying Pattern C audit
    coverage from eight upstreams (whisky-archive 4 +
    tls-parser 2 + glium 1 + pkg-config-rs 1 + sui
    mysten-metrics 2 + vcpkg-rs 1 + rust-vst2 1 + nono 1,
    batches 3 / 11 / 12 / 13 / 20 / 21 / 22 / 23) to nine
    upstreams, via a fifteenth permissive-licensed Rust
    upstream smol-machines/smolvm@019654bd
    `crates/smolvm-agent/src/storage.rs`, Apache-2.0.
    `pub fn export_layer(image_digest: &str, layer_index:
    usize) -> Result<PathBuf>` at upstream line 1418 (corpus
    line 10) carries a four-line `///` doc block whose later
    half (after a blank `///` separator) reads
    `DEPRECATED: Prefer streaming export via
    `find_layer_path()` + piped tar. This function creates a
    temp tar file that can fill the storage disk for large
    layers. Kept for backward compatibility.` but does not
    carry the `#[deprecated]` runtime attribute, so spec F5
    Pattern C fires. The function body falls into the existing
    in-tree-body sub-shape (calls
    `find_layer_path(image_digest, layer_index)?` to locate
    the layer source directory, builds a temporary tar path
    under `STORAGE_ROOT/tmp/`, spawns `tar -cf <tar_path> -C
    <layer_dir> .` via `Command::new("tar")`, and returns the
    resulting tar `PathBuf`) on a fourth unrelated upstream —
    broadening in-tree-body audit coverage beyond batch-11
    tls-parser, batch-12 glium, and batch-20 sui
    mysten-metrics while keeping Pattern C's body-shape
    footprint at the four shapes saturated by batches 22 and
    23 (delegate-body, in-tree-body, stub-body,
    meta-deprecation-warning-emitter). cntrdct's spec F5
    Pattern C check ignores the body — only the doc and
    adjacent attributes are inspected — so the new in-tree
    case fires identically to the prior in-tree,
    delegate-body, stub-body, and meta-warning-emitter cases,
    confirming the syntactic-only design. The function
    signature returns `Result<PathBuf>` so spec F3 Pattern A's
    return-type negation does not pass, but the doc contains
    no Pattern A trigger phrase either way so Pattern A does
    not fire; the doc contains no `panic` substring so spec
    F4 Pattern B does not fire — only Pattern C fires. smolvm
    is the portable, lightweight, self-contained VM image
    storage domain (the agent crate's storage module manages
    OCI-style image layers on disk for the VM runtime),
    unrelated to the prior eight Pattern C domains.
    Diversifies `comment-code`'s audit evidence to fifteen
    upstreams (whisky-archive 4 Pattern C + tls-parser 2
    Pattern C + glium 1 Pattern C + pkg-config-rs 1 Pattern C
    + zarrs 6 Pattern B + boundless 1 Pattern A +
    parking_lot_core 2 Pattern B + wasmtime 1 Pattern A +
    rust-s3 1 Pattern A + vortex-buffer 1 Pattern B + sui
    mysten-metrics 2 Pattern C + vcpkg-rs 1 Pattern C +
    rust-vst2 1 Pattern C + nono 1 Pattern C + smolvm 1
    Pattern C), lifting Pattern C from eight upstreams to
    nine without introducing a new pattern; `comment-code`
    moves to 26/0/1.00 and overall recall_upper_bound to
    0.60 (32 TP / 21 FN / 53 expected, raw 0.6038 vs. 0.5962
    at batch 23 — below the 0.05 movement threshold so no
    separate "Reading the figures" note is required per the
    refresh discipline). pr-miner mining margin preserved
    because the new file is Rust) landed 2026-05-17. With all
    six detectors and six external source kinds now live in
    the corpus, pr-miner's numerator-construction phase
    closed, comment-code exercised on all three
    `docs/spec/comment-code-v0.md` patterns AND both Pattern
    A's and Pattern B's single-upstream dominance broken at
    batches 17 and 16 respectively AND Pattern A's and Pattern
    B's sub-shape coverage completed at batches 18 and 19
    respectively (all three syntactic sub-shapes exercised on
    each pattern) AND Pattern C lifted from four to nine
    upstreams at batches 20, 21, 22, 23, and 24, further
    Phase B batches deepen existing pattern coverage on
    additional upstreams rather than introduce a new detector
    or pattern
42. Q-15 SOTA baseline comparators
43. Q-16 cargo-mutants nightly mutation testing (landed 2026-05-11)

The split between Phase A (Tier 1, blocking) and later phases is the
single most important boundary in this roadmap. Everything in Phase A
should be done before any external announcement; everything after
Phase A can be sequenced based on signal from early users. Phase G
plays the same role for the next release tag (RC1) as Phase A did
for the first announcement.
