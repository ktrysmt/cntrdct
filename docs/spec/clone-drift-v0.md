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
| T10 | one fn with parse error + valid drift fixture | invalid one skipped, drift still detected |

## Tunable constants (v0 defaults)

- `SIMILARITY_THRESHOLD = 0.5`
- `NGRAM_SIZE = 3`
- `MIN_GROUP_SIZE = 3`
- `MIN_FN_TOKENS = 22` — minimum normalized AST token count for a
  function to participate in clustering. Filters out trivially short
  utility functions (one-line returns, single `pass`, two-statement
  helpers) whose drift signal is too noisy to act on. Industrial NiCad
  / SourcererCC pipelines apply equivalent minimum-size gates; we
  expose ours as a tunable on the same surface as
  `SIMILARITY_THRESHOLD`.

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
