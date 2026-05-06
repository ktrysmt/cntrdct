# cntrdct Rust wild corpus

Real-world Rust β corpus for the P-1 milestone. Files are pinned
extracts from popular crates.io packages with permissive licenses;
ground truth is hand-labelled by triaging cntrdct's actual output
rather than constructed to exhibit a target pattern.

## Layout

```
wild-corpus/
├── README.md
├── manifest.jsonl
└── files/
    ├── aho_corasick__lib.rs
    ├── aho_corasick__automaton.rs
    ├── ...
```

`manifest.jsonl` follows the `cntrdct-eval` schema with three additive
fields per entry (`source`, `license`, `sha256`) so provenance is
auditable end-to-end. Schema is shared with the M-4 Python wild
corpus.

## Provenance

Every file under `files/` carries a 3-line header:

```
// Source: <static.crates.io URL of the upstream .crate file>
// License: <SPDX expression>
// Note: verbatim extract from upstream .crate
```

Rust line-comment syntax (`//`) is required so the cntrdct parser
accepts the file. The body below the header is byte-identical to the
upstream source (verified by re-running the fetcher). The `sha256`
recorded in `manifest.jsonl` is the hash of the file as committed
(header + body), so CI can detect drift without re-downloading.

## Refreshing the corpus

```sh
python3 scripts/fetch_rust_corpus.py --manifest-skeleton
```

The fetcher pins `(crate, version, file_path)` triples in its
`CORPUS` constant, downloads each crate's `.crate` tarball from
`static.crates.io` (after redirect from
`api.crates.io/v1/crates/<name>/<version>/download`), verifies the
tarball's SHA-256 against the canonical `cksum` recorded in the
crates.io sparse index (`https://index.crates.io/<shard>/<crate>`),
extracts the listed members, prepends the provenance header, and
writes the result to `files/`. Idempotent — re-runs produce
byte-identical output.

The fetcher also rejects any source file whose first 200 bytes
contain `@generated`. Auto-generated visitor / fold / debug code
(syn's `src/gen/*.rs` is the canonical case) produces wholesale
clone-drift false positives by construction; the marker check is a
fail-safe so future CORPUS additions cannot silently include such a
file.

The maintainer then re-runs `cntrdct scan` over the corpus, triages
new findings, and updates `manifest.jsonl` by hand. Use
`--manifest-skeleton` to emit a starter `manifest.skeleton.jsonl`
with `expected: []`.

## Selection rules

- Source: crates.io top-by-downloads list
  (`https://crates.io/api/v1/crates?sort=downloads&per_page=100`),
  pages 1-2.
- License filter: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, or
  permissive combinations thereof (e.g. `MIT OR Apache-2.0`,
  `Unlicense OR MIT`, `Apache-2.0 OR ISC OR MIT`). GPL / LGPL / MPL /
  proprietary / `CDLA-Permissive-2.0` excluded.
- Crates skipped:
  - Pure-FFI / generated bindings (libc, windows-sys, web-sys,
    js-sys, linux-raw-sys, generic-array): code is mostly auto-
    generated; signal value poor.
  - Crates with old `Unlicense/MIT` slash-form declarations not
    recognised by the SPDX permissive-only check (walkdir,
    same-file, num-bigint, serde_urlencoded). Excluded for safety;
    can be re-included once the parser learns the slash form.
  - Pre-1.0 alpha lines (smallvec 2.0.0-alpha.12).
  - Tiny utility crates with no substantive logic (cfg-if, autocfg,
    rustversion, equivalent, pin-utils, scopeguard, fnv).
- Per-crate inside the `.crate` tarball: vendor 5-10 substantive
  `.rs` files. Skip `build.rs`, `tests/`, `examples/`, `benches/`,
  `fuzz/`, `src/bin/`, generated unicode tables (`unicode_tables/`,
  `case_folding`, `general_category`, `property_bool`),
  Huffman lookup tables, files < 500 bytes, and any file whose first
  200 bytes contain `@generated`.

## Labelling triage (v0.1, 2026-05-07)

Total: 270 files, 24 cntrdct findings post-FP-reduction (was 124
before F4b/F4c/F5b/F5c on 2026-05-06; see `prereg/2026-05-07-...`
for the supersession trail).

Decision rule for `expected`: a finding is a true positive (TP) iff
a competent reviewer SHOULD investigate it as a possible bug.
Idiomatic patterns the detector misreads as bugs are false positives
(FP).

### TP (0)

The triage produced zero true positives. Every remaining finding
falls into the three residual idiom-misread categories below; the
high-volume cfg-gated, cross-crate, and Sphinx-`:raises:` patterns
that previously dominated have been closed at the detector level.

### FP (24)

#### unreachable-after-terminator (0/0 FP)

The detector no longer fires on this corpus. F4b
(spec/unreachable-after-terminator-v0.md) closes the cfg-gated
alternative-return pattern that produced all ten previous findings:

```rust
#[cfg(feature = "preserve_order")]
return self.swap_remove(key);
#[cfg(not(feature = "preserve_order"))]
return self.map.remove(key);
```

F4c additionally suppresses hoisted item declarations after a
terminator (`semver__identifier.rs:377` was the canonical case: a
nested `unsafe fn decode_len_cold(...)` declared after `return
unsafe { decode_len_cold(ptr) };`).

#### comment-code (2/2 FP)

Both findings still fire on `parking_lot_core__parking_lot.rs` at
lines 736 and 892. The doc comments document the `validate` /
`callback` parameter's contract ("must not panic"). The detector
reads the function-level docstring's "panic" mention as a CLAIM
that this function panics under some condition; the surrounding
text is actually a constraint on the caller-supplied closure.
Closing this on the Rust side is deferred to v0.x; the analogous
Python pattern (`:raises:` parameter-level documentation) was
closed by F5b/F5c (comment-code) on the Python pilot.

#### clone-drift (3/3 FP)

F5b restricts clustering to within-scope (per-crate via path-only
inference); F5c adds two within-scope tightening gates. The 3
residuals are designed library shapes that locally satisfy the
drift signal but are not bugs:

- `syn__lib.rs:961` — `parse_str` is the third member of a
  designed `parse` / `parse2` / `parse_str` API family.
- `tracing_subscriber__layer_mod.rs:1547` — `subscriber_is_none`
  is the deliberate twin of `layer_is_none` / a third
  `*_is_none` helper in `filter_layer_filters_mod.rs`.
- `uuid__fmt.rs:280` — `encode_braced` is structurally larger
  than its `encode_simple` / `encode_hyphenated` siblings (it
  builds an inner `Braced` struct), but the n-gram skeleton is
  similar enough to satisfy the cluster-membership floor.

The previous within-scope cohort of 78 findings (parser-combinator
families in nom, winnow, regex_syntax, etc.) was closed by F5c-i's
strict-majority filter and F5c-ii's near-duplicate gate. Prior to
F5b, an additional 34 findings clustered cross-crate; those were
closed by scope bounding.

#### pr-miner (19/19 FP)

The pr-miner detector did not run on the v0 corpus (was shipped
later). All 19 wild findings are intentional patterns:

- error-constructor functions that always return `Err` by design
  (`flate2__mem.rs` `decompress_failed` / `decompress_need_dict`
  / `compress_failed`, `serde_json__read.rs:864` `error`,
  `serde__private_de.rs:28` `missing_field`).
- functions that delegate to a `Result`-returning helper without
  a literal `Ok(...)` constructor (`chrono__format_parse.rs:20,33`
  `set_weekday_with_*`, `chrono__format_parsed.rs:1176`
  `resolve_week_date` via `.ok_or(...)`, `regex_syntax__unicode.rs`
  `script` / `script_extension` / `gc` / `general_category` family).
- input-shape-validating parsers (`uuid__parser.rs:151,171,181`
  `try_parse` / `parse_braced` / `parse_urn` returning `Err` on
  bad shape, otherwise delegating to `parse_hyphenated`).
- void-return functions whose `Err(...)` mentions are pattern-
  matches or state-construction calls, not Result returns
  (`mio__sys_windows_named_pipe.rs:940` `write_done`).

Tightening pr-miner to recognise these shapes is deferred to v0.x.

### FN (0)

The corpus does not currently include known-but-missed bugs from
upstream issue trackers. Expanding `expected` with such cases is
deferred to future iterations of the corpus (see "Limitations").

## Reported metrics

```
cntrdct eval benchmarks/wild-corpus
```

Current numbers (2026-05-07, against the v0.1 corpus after the
F4b/F4c/F5b/F5c FP reduction pass):

| Detector                        | TP | FP | FN | Precision | Recall | F1  |
|---------------------------------|----|----|----|-----------|--------|-----|
| clone-drift                     | 0  | 3  | 0  | 0.00      | 0.00   | 0.0 |
| comment-code                    | 0  | 2  | 0  | 0.00      | 0.00   | 0.0 |
| pr-miner                        | 0  | 19 | 0  | 0.00      | 0.00   | 0.0 |
| Overall                         | 0  | 24 | 0  | 0.00      | 0.00   | 0.0 |

Detectors that did not fire on this corpus (`arg-swap`,
`config-interaction`, `unreachable-after-terminator`) are absent
from the table — eval emits one row only for detectors that
produced findings.

These numbers reflect a corpus that is sparse on real bugs by
design — the top-100-by-downloads list is heavily reviewed code.
The seed corpus under `benchmarks/corpus/` reports near-perfect
numbers because every file is constructed to exhibit the target
pattern; the wild corpus's role is to expose detector weaknesses
on idiomatic Rust library code that wasn't written with cntrdct
in mind. Both are useful — the seed catches regressions, the
wild reveals where the detectors still need work.

The remaining v0.1 weaknesses on this corpus:

- `comment-code` v0 misreads "callback must not panic" as a
  function-level panic claim. Disambiguating the subject of a
  panic claim (this function vs caller-supplied closure) is a v0.x
  candidate.
- `clone-drift` v0.1 still flags 3 designed-library-shape variants
  (syn parse-API family, tracing-subscriber `*_is_none` twins,
  uuid `encode_*` formatter family). Tightening MIN_FN_TOKENS or
  adding a token-length-balance filter is a v0.x candidate.
- `pr-miner` v0 has no model for error-constructor functions or
  delegating wrappers. Recognising `fn x() -> Result<T> { Err(...) }`
  as an intentional shape (one-statement body) and `.ok_or()` as a
  Result-construction call is a v0.x candidate.

These weaknesses are now P-4-visible: the labelled findings feed
`scripts/build_priors_corpus.py` and the calibration run assigns
correspondingly low `posterior_tp` / `wilson_lower_95` to the
three detectors that still fire on Rust wild code.

## Limitations (v0)

- Coverage skewed toward parser combinators, iterators, and
  container types: top-by-downloads crates over-represent these
  domains. Adding diversity (web frameworks, async runtimes,
  databases, CLI tools beyond clap) is a future iteration.
- 270 files is small relative to the 1000-crate Track A target;
  the per-detector precision numbers are directional, not
  statistically tight. A 1000-file expansion is feasible by widening
  the per-crate file count from 5-10 to 15-25.
- No FN entries: every expected entry happens to be matched. To
  make recall non-trivial we need to label specific upstream-known
  bugs the detector misses, which requires bug-tracker spelunking
  per crate.
- syn's `src/gen/*.rs` files were excluded after the first scan
  produced 290 wholesale FPs from auto-generated visitor methods.
  The `@generated` marker check in the fetcher is the fail-safe
  for future cases.

## License notes

Every vendored file is redistributed under its upstream license
(MIT, Apache-2.0, BSD-3-Clause, Unlicense OR MIT, etc.). The
corresponding SPDX identifiers are recorded in both the file's
provenance header and the `license` field of its manifest entry.
Upstream copyright notices in the original files are preserved
verbatim; we do not strip them.

License breakdown across the 270 files:

| SPDX                            | Count |
|---------------------------------|-------|
| MIT OR Apache-2.0               | 174   |
| MIT                             | 48    |
| Apache-2.0 OR MIT               | 24    |
| Unlicense OR MIT                | 16    |
| Apache-2.0 OR ISC OR MIT        | 8     |

## Crate roster

36 crates pinned. Versions reflect crates.io top-of-trunk at
2026-05-06.

aho-corasick 1.1.4, base64 0.22.1, bitflags 2.11.1, chrono 0.4.44,
clap 4.6.1, clap_builder 4.6.0, flate2 1.1.9, h2 0.4.14,
hashbrown 0.17.0, http 1.4.0, hyper 1.9.0, idna 1.1.0,
indexmap 2.14.0, itertools 0.14.0, memchr 2.8.0, mio 1.2.0,
nom 8.0.0, object 0.39.1, once_cell 1.21.4, parking_lot_core 0.9.12,
regex 1.12.3, regex-automata 0.4.14, regex-syntax 0.8.10,
rustls 0.23.40, semver 1.0.28, serde 1.0.228, serde_json 1.0.149,
syn 2.0.117 (excluding `src/gen/*.rs`), thiserror 2.0.18,
time 0.3.47, toml 1.1.2+spec-1.1.0, toml_edit 0.25.11+spec-1.1.0,
tracing-subscriber 0.3.23, url 2.5.8, uuid 1.23.1, winnow 1.0.2.
