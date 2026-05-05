# cntrdct Python wild corpus

Real-world Python β corpus for the M-4 milestone. Files are pinned
extracts from popular PyPI packages with permissive licenses; ground
truth is hand-labelled by triaging cntrdct's actual output rather than
constructed to exhibit a target pattern.

## Layout

```
wild-corpus-python/
├── README.md
├── manifest.jsonl
└── files/
    ├── attrs_make.py
    ├── attrs_validators.py
    ├── ...
```

`manifest.jsonl` follows the `cntrdct-eval` schema with three additive
fields per entry (`source`, `license`, `sha256`) so provenance is
auditable end-to-end. The schema is shared with the future P-1 Rust β
corpus per ROADMAP.

## Provenance

Every file under `files/` carries a 3-line header:

```
# Source: <URL of the upstream PyPI sdist>
# License: <SPDX expression>
# Note: verbatim extract from upstream sdist
```

The body below the header is byte-identical to the upstream source
(verified by re-running the fetcher). The `sha256` recorded in
`manifest.jsonl` is the hash of the file as committed (header + body),
so CI can detect drift without re-downloading.

## Refreshing the corpus

```sh
python3 scripts/fetch_python_corpus.py
```

The fetcher pins `(package, version, file_path)` triples in its
`CORPUS` constant, downloads each package's sdist from PyPI, verifies
the tarball's SHA-256 against PyPI's reported digest, extracts the
listed members, prepends the provenance header, and writes the result
to `files/`. Idempotent — re-runs produce byte-identical output.

The maintainer then re-runs `cntrdct scan` over the corpus, triages new
findings, and updates `manifest.jsonl` by hand. Use
`--manifest-skeleton` to emit a starter skeleton with `expected: []`.

## Labelling triage (v0, 2026-05-06)

Total: 11 files, 19 cntrdct findings.

Decision rule for `expected`: a finding is a true positive (TP) iff a
competent reviewer SHOULD investigate it as a possible bug. Idiomatic
patterns the detector misreads as bugs are false positives (FP).

### TP (1)

- `charset_normalizer_utils.py:27 clone-drift` — `is_accentuated`
  diverges meaningfully from its `is_X(character) -> bool` siblings
  (eight OR conditions vs the family's typical one or two). Worth a
  reviewer's eye to confirm the divergence is intentional.

### FP (18)

- `charset_normalizer_utils.py:70` and `:194 clone-drift`. `is_latin`
  and `is_arabic_isolated_form` are simple sibling members of the
  family; the divergence is the substring being matched. Not
  bug-suspect.
- `attrs_make.py:93` and `:1197 comment-code`. The `attrib()` and
  `attrs()` functions' docstrings contain `.. deprecated::` directives
  that mark *parameters* as deprecated, not the function. The detector
  reads "deprecated" in the docstring as a function-level claim and
  fires when the function isn't decorated.
- `attrs_validators.py:128, 243, 320, 364, 411, 451, 493, 505, 517,
  529, 560, 591, 630, 688 comment-code` (14 findings). Each is a
  factory function returning a callable validator. The docstring's
  `:raises:` clause documents the behavior of the *returned validator*
  per the attrs idiom, but the factory body itself just constructs and
  returns the validator without raising. The detector flags the
  mismatch; the pattern is intentional.

### FN (0)

The corpus does not currently include known-but-missed bugs from
upstream issue trackers. Expanding `expected` with such cases is
deferred to future iterations of the corpus (see "Limitations").

## Reported metrics

```
cntrdct eval benchmarks/wild-corpus-python
```

Current numbers (2026-05-06, against the v0 corpus):

| Detector       | TP | FP | FN | Precision | Recall | F1  |
|----------------|----|----|----|-----------|--------|-----|
| clone-drift    | 1  | 2  | 0  | 0.33      | 1.00   | 0.5 |
| comment-code   | 0  | 16 | 0  | 0.00      | 0.00   | 0.0 |
| Overall        | 1  | 18 | 0  | 0.05      | 1.00   | 0.1 |

These numbers are intentionally not flattering. The seed corpus under
`benchmarks/corpus/` reports near-perfect numbers because every file
is constructed to exhibit the target pattern; the wild corpus exposes
the detectors' weaknesses on idiomatic Python code that wasn't written
with cntrdct in mind. Both are useful — the seed catches regressions,
the wild reveals where the detectors need work.

## Limitations (v0)

- Tiny corpus: 11 files. Single-package weight (one bad file pattern)
  shifts metrics noticeably. Treat numbers as directional, not
  absolute.
- No `arg-swap` or `unreachable-after-terminator` findings yet — the
  selected packages happen not to exhibit those patterns. Recall
  signal for those detectors is therefore zero by default. Future
  iterations should add packages that contain known instances or
  inject a small set of targeted real-world fixtures.
- No FN entries: every expected entry happens to be matched. To make
  recall non-trivial we need to label specific upstream-known bugs
  the detector misses, which requires bug-tracker spelunking.
- `idna_uts46data.py` was deliberately excluded — it's a generated
  Unicode lookup table, not code, and produces 68 clone-drift hits
  that all classify as FP and swamp the signal. The exclusion is
  documented in `scripts/fetch_python_corpus.py`.

## License notes

Every vendored file is redistributed under its upstream license (MIT,
BSD-3-Clause). The corresponding SPDX identifiers are recorded in both
the file's provenance header and the `license` field of its manifest
entry. Upstream copyright notices in the original files are preserved
verbatim; we do not strip them.
