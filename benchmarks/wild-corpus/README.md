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

## Labelling triage (v0, 2026-05-06)

Total: 270 files, 124 cntrdct findings.

Decision rule for `expected`: a finding is a true positive (TP) iff
a competent reviewer SHOULD investigate it as a possible bug.
Idiomatic patterns the detector misreads as bugs are false positives
(FP).

### TP (0)

The triage produced zero true positives. Every finding fell into
one of the three idiom-misread categories below.

### FP (124)

#### unreachable-after-terminator (10/10 FP)

All ten findings flag the same Rust idiom: cfg-gated alternative
returns / panics, e.g.:

```rust
#[cfg(feature = "preserve_order")]
return self.swap_remove(key);
#[cfg(not(feature = "preserve_order"))]
return self.map.remove(key);
```

Exactly one branch is active per cfg evaluation; both lines are
NOT simultaneously present in any compiled binary. The detector's
AST walk treats them as if they were sequential statements.
Locations: `clap_builder__output_help_template.rs:1120`,
`semver__identifier.rs:377`, `serde_json__map.rs:168`,
`serde_json__map.rs:191`, `serde_json__map.rs:954`,
`serde_json__map.rs:1017`, `syn__item.rs:2648`,
`tracing_subscriber__fmt_fmt_layer.rs:343`,
`tracing_subscriber__fmt_format_json.rs:497`,
`winnow__error.rs:322`. Two of the ten use the
`unreachable!()` / `panic!()` macro as the terminator (the macro
itself is the dead-code-elimination marker).

#### comment-code (2/2 FP)

Both findings fire on `parking_lot_core__parking_lot.rs` at lines
736 and 892. The doc comments document the `validate` / `callback`
parameter's contract ("must not panic"). The detector reads the
function-level docstring's "panic" mention as a CLAIM that this
function panics under some condition; the surrounding text is
actually a constraint on the caller-supplied closure. The pattern
is the Rust analogue of the attrs `:raises:` factory pattern that
showed up as 14 FPs in the M-4 Python wild corpus.

#### clone-drift (112/112 FP)

Every clone-drift finding reports "function diverged from 24
similar siblings" — the global similarity pool caps at 25 (1
primary + 24 related per `MAX_RELATED`). The pool spans the entire
corpus across crate boundaries; the "siblings" of a `nom`
combinator include `syn` parse helpers and `serde_json`
constructors purely on signature shape (e.g.
`pub fn x(input) -> Result<Output>`). Cross-crate divergence in a
shape-shared pool is not bug-suspect — each crate has its own
conventions and a parser combinator is supposed to look like a
parser combinator.

The 112 findings concentrate on parser-combinator / iterator-
combinator libraries where the detector's shape similarity is
maximally fooled: nom (37), winnow (28), itertools (21),
regex_syntax (7), memchr (5), base64 (6), serde_json (5), and
single-finding outliers in 14 other files.

### FN (0)

The corpus does not currently include known-but-missed bugs from
upstream issue trackers. Expanding `expected` with such cases is
deferred to future iterations of the corpus (see "Limitations").

## Reported metrics

```
cntrdct eval benchmarks/wild-corpus
```

Current numbers (2026-05-06, against the v0 corpus):

| Detector                        | TP | FP  | FN | Precision | Recall | F1  |
|---------------------------------|----|-----|----|-----------|--------|-----|
| clone-drift                     | 0  | 112 | 0  | 0.00      | 0.00   | 0.0 |
| comment-code                    | 0  | 2   | 0  | 0.00      | 0.00   | 0.0 |
| unreachable-after-terminator    | 0  | 10  | 0  | 0.00      | 0.00   | 0.0 |
| Overall                         | 0  | 124 | 0  | 0.00      | 0.00   | 0.0 |

Detectors that did not fire on this corpus (`arg-swap`,
`pr-miner` once shipped) are absent from the table — eval emits one
row only for detectors that produced findings.

These numbers are intentionally not flattering. The seed corpus
under `benchmarks/corpus/` reports near-perfect numbers because
every file is constructed to exhibit the target pattern; the wild
corpus exposes the detectors' weaknesses on idiomatic Rust library
code that wasn't written with cntrdct in mind. Both are useful —
the seed catches regressions, the wild reveals where the detectors
need work. Concretely:

- `unreachable-after-terminator` v0 has no cfg-attribute model.
  Modelling cfg gates is a v0.x candidate.
- `comment-code` v0 misreads "callback must not panic" as a
  function-level panic claim. Disambiguating the subject of a
  panic claim (this function vs caller-supplied closure) is a v0.x
  candidate.
- `clone-drift` v0 with `MAX_RELATED = 24` and a global sibling
  pool reports cross-crate "divergences" that are not meaningful.
  Restricting the pool to intra-crate or intra-file siblings, or
  raising the support threshold, is a v0.x candidate.

These three weaknesses are now P-4-visible: the labelled findings
feed `scripts/build_priors_corpus.py` and the next calibration run
will assign correspondingly low `posterior_tp` / `wilson_lower_95`
to all three detectors on Rust wild code.

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
