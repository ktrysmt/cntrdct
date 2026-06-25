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
corpus.

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

## Labelling triage (v0.1, 2026-05-07)

Total: 11 files, 4 cntrdct findings post-FP-reduction (was 19 before
F5b/F5c on 2026-05-06).

Decision rule for `expected`: a finding is a true positive (TP) iff a
competent reviewer SHOULD investigate it as a possible bug. Idiomatic
patterns the detector misreads as bugs are false positives (FP).

### TP (0)

The previous triage labelled `charset_normalizer_utils.py:27
clone-drift` as TP (`is_accentuated` having eight OR conditions
diverged from its `is_X(character) -> bool` siblings). On strict
re-application of the labelling rubric §5.1
FP-1 ("primary and related share only the syntactic shape; their
conceptual roles differ"), `is_accentuated` is an accent-property
detector while its single-predicate siblings are script / category
detectors — different conceptual roles. The label is downgraded to
FP. The fix-4 F5c near-duplicate gate also no longer fires on
this position post-relabelling, so it does not appear in the
current finding set.

### FP (4)

#### clone-drift (2/2 FP)

- `charset_normalizer_utils.py:70` (`is_latin`) — single-predicate
  sibling, differs from the dominant exact form by carrying a
  `: str` type annotation on `description`. F5c-ii's near-duplicate
  Jaccard 0.78 clears the 0.7 threshold so the singleton still
  fires, but the rubric tags it FP-2 (style-only difference).
- `charset_normalizer_utils.py:194` (`is_arabic_isolated_form`) —
  compound predicate (`"ARABIC" in name AND "ISOLATED FORM" in
  name`), conceptually a stricter subset of `is_arabic`. FP-1
  (different conceptual role).

#### pr-miner (2/2 FP)

- `click_utils.py:326,338` — `get_binary_stream` / `get_text_stream`
  raise `TypeError` on dict-lookup miss. They have no `isinstance`
  check because the parameter is type-annotated `Literal['stdin',
  'stdout', 'stderr']`. The "12 of 14 similar functions call both"
  signal flags the asymmetry; the asymmetry is intentional.

#### comment-code (closed at detector level)

The 16 attrs `:raises:` factory FPs (`attrs_validators.py` cluster)
and the 2 `.. deprecated::` parameter-level FPs
(`attrs_make.py:93,1197`) were closed by F5b (factory-shape
suppression) and F5c (parameter-level deprecation peeking) on the
comment-code detector. Spec details:
`docs/spec/comment-code-v0.md` F5b/F5c.

#### clone-drift (closed at detector level)

The 2 single-predicate-vs-compound-predicate FPs that remain are
the residual after F5c-i's strict-majority gate filtered out the
parser-combinator-shape FPs. They survive because the
charset_normalizer family has a strict majority and all three
remaining candidates are near-duplicates of the dominant form.
Tightening this requires either bumping `MIN_FN_TOKENS` (would lose
real small-fn drift signals on the Rust pilot) or adding a
type-annotation-aware normalisation step (deferred to v0.x).

### FN (0)

The corpus does not currently include known-but-missed bugs from
upstream issue trackers. Expanding `expected` with such cases is
deferred to future iterations of the corpus (see "Limitations").

## Reported metrics

```
cntrdct eval benchmarks/wild-corpus-python
```

Current numbers (2026-05-07, against the v0.1 corpus after the
F5b/F5c FP reduction pass and rubric-strict relabelling):

| Detector       | TP | FP | FN | Precision | Recall | F1  |
|----------------|----|----|----|-----------|--------|-----|
| clone-drift    | 0  | 2  | 0  | 0.00      | 0.00   | 0.0 |
| pr-miner       | 0  | 2  | 0  | 0.00      | 0.00   | 0.0 |
| Overall        | 0  | 4  | 0  | 0.00      | 0.00   | 0.0 |

Detectors that did not fire on this corpus (`arg-swap`,
`config-interaction`, `comment-code`, `unreachable-after-terminator`)
are absent from the table — eval emits one row only for detectors
that produced findings.

These numbers reflect a corpus that is sparse on real bugs by
design. The seed corpus under `benchmarks/corpus/` reports
near-perfect numbers because every file is constructed to exhibit
the target pattern; the wild corpus's role is to expose detector
weaknesses on idiomatic Python code that wasn't written with
cntrdct in mind. Both are useful — the seed catches regressions,
the wild reveals where the detectors still need work.

## Limitations (v0)

- Tiny corpus: 11 files. Single-package weight (one bad file pattern)
  shifts metrics noticeably. Treat numbers as directional, not
  absolute.
- No `arg-swap`, `comment-code`, `config-interaction`, or
  `unreachable-after-terminator` findings — the selected packages
  happen not to exhibit those patterns. Recall signal for those
  detectors is therefore zero by default. Future iterations should
  add packages that contain known instances or inject a small set
  of targeted real-world fixtures.
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
