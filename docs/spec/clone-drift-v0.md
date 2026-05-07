# clone-drift detector v0 spec

Status: active draft, approved for TDD implementation 2026-05-02.

## Scope

- Detector: clone-drift only
- Language: Rust only
- Granularity: top-level `fn` definitions
- Output: `Vec<Finding>` from `Detector::detect`
- Out: CLI, SARIF, LLM, git history, multi-language, config file

## Functional requirements

### F1 — Input

Accepts `&[ParsedFile]` via `DetectContext`. Files with `language != "rust"` are
skipped without error. Empty input returns `Ok(vec![])`.

### F2 — Function extraction

Extracts every top-level `fn` definition from each ParsedFile. Functions inside
`impl`, `trait`, or `mod` blocks are out of scope for v0. Functions whose
normalized AST token sequence is shorter than `MIN_FN_TOKENS` are dropped
before clustering; their drift signal is too noisy to act on. This guard
mirrors the minimum-size filters in industrial NiCad and SourcererCC
pipelines.

### F3 — Type-3 clone grouping

For each pair of extracted functions, compute Jaccard similarity over the multiset
of n-grams (n = `NGRAM_SIZE`) of normalized AST node kinds (pre-order walk).
Functions with pairwise similarity ≥ `SIMILARITY_THRESHOLD` form Type-3 clone groups
via connected components. Group size must be ≥ `MIN_GROUP_SIZE` to be considered.

### F4 — Normalization

- Identifiers (variable, function, type, method names) → "IDENT"
- Integer literal → "LIT_INT"
- Float literal → "LIT_FLOAT"
- String literal → "LIT_STR"
- Char literal → "LIT_CHAR"
- Bool literal → "LIT_BOOL"
- Comments stripped
- Whitespace normalized

The normalized form is the sequence of AST node kinds (tree-sitter `kind()`) walking
pre-order, with leaves replaced per the rules above.

### F5 — Type-2 partitioning and drift signal

Within each Type-3 clone group of size ≥ `MIN_GROUP_SIZE`:
- Partition functions by exact equality of their normalized AST node sequence.
- A function is "drifted" iff:
  - It belongs to a partition of size 1, AND
  - At least one other partition in the same group has size ≥ 2.

### F6 — Finding shape

For each drifted function:
- `detector_id = "clone-drift"`
- `primary` = location of the drifted function
- `related` = locations of every function in the largest partition of the same group
- `message` = "function diverged from N similar siblings" (N = `related.len()`)
- `raw_severity = Warning`
- `evidence.citation_keys` MUST include at least one of:
  `cordy-roy-icpc-2008`, `bettenburg-msr-2009`, `krinke-icsm-2007`
- `evidence.raw` carries: similarity_threshold, group_size, partition_sizes,
  normalized_form_hash

### F7 — Output stability

Findings sorted by `(primary.file, primary.start_line)` lexicographically.

### F5b — Scope-bounded clustering (added 2026-05-07)

Clone-drift v0 originally pooled every function in the scan into a single
clustering namespace. The wild Rust β corpus exposed this as the dominant
FP source: 112 of 124 findings were cross-crate "siblings" — functions
whose AST normal form happened to match a function in an unrelated crate.
F5b restricts F3 / F5 to operate within a single scope at a time, defined
per file with a path-only inference (no filesystem I/O, preserving N3).

Scope key, first match wins:

1. Provenance header. If the file's first ~512 bytes contain a `// Source:`
   line referencing `https://static.crates.io/crates/<name>/...` (the
   wild β corpus convention; see `benchmarks/wild-corpus/README.md`), the
   scope key is `cratesio::<name>`. Equivalent forms for the Python wild
   corpus (`# Source: https://files.pythonhosted.org/.../packages/.../`)
   yield `pypi::<package>` keys.
2. Cargo project layout. If the path contains a `/src/` segment, the
   scope key is the substring up to and including the directory
   immediately before `/src/`. The same rule applies to `/tests/` and
   `/examples/` segments. Multi-crate workspaces (`crates/foo/src/...`,
   `crates/bar/src/...`) split into per-crate scopes naturally.
3. Filename `__` separator. If the file basename contains `__`, the
   scope key is the part before the first `__`. This is the wild β
   corpus's secondary fallback when the provenance header is missing.
4. Parent directory. Scope key is the file's parent path as a string,
   or the empty string when the file has no parent component.

Scopes never mix: clustering and partitioning run independently per
scope. F4 normalization, MIN_FN_TOKENS, MIN_GROUP_SIZE, and the
SIMILARITY_THRESHOLD all apply within a scope. The Finding shape
(F6) is unchanged.

Acceptance: on benchmarks/wild-corpus the cross-crate clustering
collapses; per-crate genuine drift findings remain. The narrowing
trades some recall (a single-crate clone group below MIN_GROUP_SIZE
within its scope no longer cross-pools to reach the threshold) for a
substantial precision gain on real-world code.

### F5c — Drift-signal tightening (added 2026-05-07)

F5b cut cross-scope FPs but left within-scope library-shape variants
flagged at high volume — 78 of the remaining 78 Rust wild-β findings
were intentional parser-combinator / formatter family members in
crates such as nom, syn, tracing-subscriber, and uuid. F5c adds two
inner gates that together carve out the textbook "drifted clone"
shape (Bettenburg et al., MSR 2009; Krinke, ICSM 2007) and discard
the family-of-variants shape.

(F5c-i) Strict-majority gate. The dominant exact-form partition must
cover a strict majority of the cluster, i.e.
`largest.len() * 2 > group.len()`. A 24-of-164 dominant partition
(nom-shape) does NOT qualify; a 9-of-10 dominant partition (the bug
pattern of "one of N copies missed an update") does.

(F5c-ii) Near-duplicate gate. The drifted singleton must have
Jaccard ≥ NEAR_DUPLICATE_THRESHOLD with the dominant exemplar's
n-grams. Cluster membership requires only pairwise Jaccard ≥
SIMILARITY_THRESHOLD with at least one neighbour, which lets a
structurally different function be transitively pulled in (e.g.
`encode_braced` joining `encode_simple` / `encode_hyphenated` in
uuid). The bug pattern requires the singleton to differ from the
canonical form by a small number of tokens; NEAR_DUPLICATE_THRESHOLD
(default 0.7) operationalises that.

Both gates are applied in `emit_findings_for_scope`. The ordering
is (i) before (ii) so that obvious family-of-variants shapes short-
circuit before the per-singleton Jaccard recompute. Evidence is
extended with `dominant_jaccard`, `near_duplicate_threshold`,
`drifted_len`, and `dominant_len` for downstream calibration.

Acceptance: on benchmarks/wild-corpus the within-scope FP count
drops from 78 to 3 (96% reduction). The 3 residuals (syn parse-API
family, tracing-subscriber `*_is_none` twins, uuid `encode_*`
formatter family) are designed library shapes; they have the same
shape as a real drift but are not bugs. They are documented as v0
limitations and labelled FP in the wild-corpus manifest.

### F5d — Sibling-family discriminator (added 2026-05-07)

After F5b / F5c the wild β corpus retained 5 residual FPs (Rust 3,
Python 2). Hand inspection showed all five share a structural
property that F5c does not capture: the cluster is a *designed
family of N parallel variants* rather than the textbook
"one of N copies missed an update" shape. F5d adds three
independent sub-gates, each carved from a residual class.

(F5d-i) Multi-singleton suppression. A cluster carrying ≥ 2 size-1
partitions is the signature of an N-variant family — every variant
intentionally differs from every other along a small surgical edit.
The Python `charset_normalizer.utils` `is_<script>` family clusters
with `partition_sizes = [6, 1, 1, 1]`: six members share the
canonical `try / unicodedata.name / except / return X in name` body
and three variants each substitute a different substring (`"LATIN"`,
`"ARABIC" + "ISOLATED FORM"`, `"HANGUL"`, ...). When the bug pattern
is present at most one member is "missed" per cluster; ≥ 2 distinct
singletons in the same cluster is therefore the family-of-variants
shape and the entire cluster's singleton emission is suppressed.

(F5d-ii) Length-imbalance gate, conditioned on weak dominant-form
evidence. A real drifted clone differs from the dominant exemplar
by a small, surgical edit. Because the n-gram set is order- and
multiplicity-agnostic, repeated body blocks (`encode_braced` adding
a nested `struct` and a transmute) keep Jaccard high while bumping
the normalised length significantly. A high-Jaccard /
high-length-imbalance pair is the residual that F5c-ii cannot
resolve. We compute the asymmetry as
`|drift_len - dom_len| / max(drift_len, dom_len)` and suppress when
both (a) it exceeds `LENGTH_IMBALANCE_THRESHOLD` (default 0.15) and
(b) the dominant partition holds fewer than
`LENGTH_IMBALANCE_DOMINANT_FLOOR` functions (default 3, i.e. exactly
2 — the F5c-i strict-majority floor for a 3-fn cluster).

The dominant-floor conditioning is critical. With dominant size 2
the canonical-form evidence is structurally weak: the 2 dominant
members may themselves be a designed sibling pair (`layer_is_none` /
`subscriber_is_none` in tracing-subscriber) rather than 2 copies of
one canonical form, so length symmetry becomes the deciding signal.
With dominant size ≥ 3 the canonical-form evidence is strong; the
textbook bug pattern of "1 of N copies missed an update" (e.g.
seed-corpus `clone_drift_005` at length imbalance 0.258 with N = 4
poll-wrappers + 1 break-clause drift) fires unaffected. An
unconditional gate would have suppressed three seed-corpus TPs at
length imbalance 0.16, 0.18, 0.26 — empirically the same band the
wild β residuals sit in (0.186, 0.242), distinguished only by
dominant size. The wild β syn parse-API family at length imbalance
0.043 falls below the threshold; F5d-iii catches it.

(F5d-iii) Small-cluster floor. A cluster at exactly `MIN_GROUP_SIZE`
whose dominant exemplar's normalised length is within
`SMALL_CLUSTER_TOKEN_BUFFER` (default 2) of `MIN_FN_TOKENS` is at the
detector's resolution limit — signature normalization (parameters,
type bounds, where clauses) dominates the n-gram set, so any
single-token body shift in the singleton looks like a drift even
when the three siblings are independently designed delegate
wrappers. The wild β syn parse-API family (`parse` / `parse2` /
`parse_str`) is exactly this shape (group_size = 3, dominant
normalises to 22 tokens). The +2 buffer admits genuine fixtures
(t5's dominant exemplar normalises to ≈ 35 tokens).

All three gates are applied in `emit_findings_for_scope`. F5d-i and
F5d-iii are evaluated once per cluster (cheap), before the per-
singleton loop; F5d-ii is evaluated per singleton, after the F5c-ii
Jaccard recompute (so an obvious low-Jaccard variant short-circuits
first). Evidence is extended with `length_imbalance`,
`length_imbalance_threshold`, and `singleton_count` for downstream
calibration and audit.

Acceptance: on benchmarks/wild-corpus the clone-drift residual FP
count drops from 3 to 0 in Rust and from 2 to 0 in Python (the wild
β corpus's labelled FPs at `syn__lib.rs:961`,
`tracing_subscriber__layer_mod.rs:1547`, `uuid__fmt.rs:280`,
`charset_normalizer_utils.py:70`, `:194` no longer fire).
`tests/detector_clone_drift.rs` t29 (F5d-i), t30 (F5d-ii), t31
(F5d-iii) pin the new gates structurally; t1–t28 remain green.

### F8 — Anomaly class

Every Finding emitted by clone-drift sets `anomaly_class = AnomalyClass::Logic`
(IEEE 1044-2009 §5.4). Rationale: divergence between near-duplicate functions is
an inconsistency in computational behaviour — the canonical "Logic" anomaly
class — rather than an interface mismatch (Interface), a data-shape problem
(Data), or a comment/spec drift (Documentation). Surfaced in SARIF as
`result.properties.anomalyClass = "Logic"`.

## Non-functional requirements

### N1 — Determinism (P3)

Identical input produces identical output, including ordering. No LLM, no network,
no `SystemTime`, no random seeds.

### N2 — Citation (P1)

Enforced by `register_detector` in `cntrdct-core`.

### N3 — No side effects

`detect()` performs no I/O, no logging.

### N4 — Performance (target only, not gated)

10K LOC of Rust code processed in < 30s on a single core.

### N5 — Robustness

Files that fail to parse are skipped silently in v0. Logging channel will be added
in β.

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | 4 identical fns + 1 with extra `&&` clause | 1 Finding, primary = the modified fn, related.len() == 4 |
| T2 | 5 fns identical after normalization (only ident names differ) | 0 Findings |
| T3 | 2 of version A + 2 of version B | 0 Findings |
| T4 | 2 identical fns (group size < `MIN_GROUP_SIZE`) | 0 Findings |
| T5 | 1 of version A + 2 of version B | 1 Finding, primary = version A |
| T6 | 2 unrelated fns (similarity < threshold) | 0 Findings |
| T7 | T1 input | every Finding's `evidence.citation_keys` ⊇ one of the recognized keys |
| T8 | T1 input run twice | identical `Vec<Finding>` |
| T9 | empty input | 0 Findings, no error |
| T20 | 4 identical fns under `crateA/src/*.rs` + 1 drifted under `crateB/src/*.rs` | 0 Findings (different scopes, F5b) |
| T21 | 4 identical fns + 1 drifted, all with `// Source: https://static.crates.io/crates/foo/...` provenance | 1 Finding (same scope) |
| T22 | 4 identical fns provenance-tagged `foo`, 1 provenance-tagged `bar` | 0 Findings (different cratesio scopes) |
| T23 | 4 identical `foo__a.rs`/`foo__b.rs`/... + 1 drifted `bar__e.rs` (no provenance) | 0 Findings (different `__`-prefix scopes) |
| T24 | T1 fixture (bare names, no path / provenance) | 1 Finding (all share parent-dir scope; backward-compatible) |
| T25 | 4-fn cluster split [2, 1, 1] (no strict majority) | 0 Findings (F5c-i) |
| T26 | 4-fn cluster split [3, 1] (strict majority + small drift) | 1 Finding |
| T27 | 5-fn cluster, dominant pair + 1 structurally different singleton | 0 Findings (F5c-ii: dominant_jaccard < NEAR_DUPLICATE_THRESHOLD) |
| T28 | T1 fixture | every Finding's `evidence.raw` carries `dominant_jaccard` and `near_duplicate_threshold` |
| T29 | 4 base fns + 2 distinct drifted singletons in same scope | 0 Findings (F5d-i: multi-singleton family) |
| T30 | 2 base fns + 1 repeated-body singleton (dominant size 2, length asymmetry > 0.15) | 0 Findings (F5d-ii: weak-dominant length-imbalance) |
| T30b | 4 base fns + 1 break-clause drifted singleton (dominant size 4, length asymmetry > 0.15) | 1 Finding (F5d-ii exemption: strong dominant) |
| T31 | 3 tiny delegate wrappers at MIN_GROUP_SIZE with dominant normalised length ≤ MIN_FN_TOKENS + 2 | 0 Findings (F5d-iii: small-cluster floor) |
| T10 | one fn with parse error + valid drift fixture | invalid one skipped, drift still detected |

## Tunable constants (v0 defaults)

- `SIMILARITY_THRESHOLD = 0.5` — pairwise Jaccard cutoff for cluster
  membership.
- `NGRAM_SIZE = 3`
- `MIN_GROUP_SIZE = 3`
- `MIN_FN_TOKENS = 22` — minimum normalized AST token count for a
  function to participate in clustering. Filters out trivially short
  utility functions (one-line returns, single `pass`, two-statement
  helpers) whose drift signal is too noisy to act on. Industrial NiCad
  / SourcererCC pipelines apply equivalent minimum-size gates; we
  expose ours as a tunable on the same surface as
  `SIMILARITY_THRESHOLD`.
- `NEAR_DUPLICATE_THRESHOLD = 0.7` — F5c-ii Jaccard cutoff between a
  drifted singleton and the dominant exemplar. Higher than
  `SIMILARITY_THRESHOLD` because cluster membership is a
  transitive-chain property, while the drift signal is a direct
  near-clone property. Empirically tuned so the Python pilot drift
  fixture (Jaccard 0.78) clears it while structural variants such
  as nom@1309 (0.53) and nom@1330 (0.66) do not.
- `LENGTH_IMBALANCE_THRESHOLD = 0.15` — F5d-ii ceiling on the
  normalised-token-length asymmetry between a drifted singleton and
  the dominant exemplar. The gate fires only in conjunction with
  `LENGTH_IMBALANCE_DOMINANT_FLOOR` (see below). Tuned so the wild β
  residuals (uuid `encode_*` 0.242, tracing-subscriber `*_is_none`
  0.186) are caught under weak dominant evidence while genuine drift
  fixtures with weak dominants (FN_VARIANT_A vs FN_VARIANT_B at
  ≈ 0.11) clear the gate.
- `LENGTH_IMBALANCE_DOMINANT_FLOOR = 3` — F5d-ii dominant-size
  conditioner. The length-imbalance gate triggers only when
  `largest.len() < LENGTH_IMBALANCE_DOMINANT_FLOOR`, i.e. exactly 2
  (the F5c-i strict-majority floor for a 3-fn cluster). Larger
  dominant partitions carry strong canonical-form evidence and are
  exempt; the textbook bug pattern of "1 of N copies missed an
  update" with N ≥ 3 fires unaffected even at length imbalance > 0.15
  (seed-corpus `clone_drift_005` at 0.258 with N = 4 stays a TP).
- `SMALL_CLUSTER_TOKEN_BUFFER = 2` — F5d-iii buffer above
  `MIN_FN_TOKENS`. A cluster at exactly `MIN_GROUP_SIZE` whose
  dominant exemplar normalises to ≤ `MIN_FN_TOKENS +
  SMALL_CLUSTER_TOKEN_BUFFER` tokens is at the detector's resolution
  limit and is suppressed. The +2 buffer admits genuine fixtures
  (t5's dominant exemplar at ≈ 35 tokens) while suppressing the syn
  parse-API family (dominant 22 tokens).

Exposed as `pub const` in `cntrdct-detector-clone-drift` for tuning without API
change. Real-world calibration belongs to Layer 2 (ranker), not these constants.

## Non-goals (v0)

- Multi-language
- Statement-block granularity
- Functions inside `impl` / `trait` / `mod`
- Strict Type-3 weighted ranking
- Git history / SZZ
- LSP integration
- SARIF emission
- LLM adjudication
- Configuration override

## References (P1)

- `cordy-roy-icpc-2008` — Cordy & Roy, "The NiCad Clone Detector", ICPC 2008
- `bettenburg-msr-2009` — Bettenburg et al., "Inconsistent Changes to Code Clones at the Release Level", MSR 2009
- `krinke-icsm-2007` — Krinke, "Consistent and Inconsistent Changes to Code Clones", ICSM 2007
