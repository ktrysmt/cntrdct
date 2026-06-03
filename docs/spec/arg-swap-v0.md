# arg-swap detector v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Background

PR-Miner (Li & Zhou ESEC/FSE 2005) and "Detecting Argument Selection Defects"
(Rice et al ICSE 2017) detect calls where the programmer reversed argument
order. The Rice approach correlates parameter names from the function
definition with argument names at the call site.

cntrdct's arg-swap v0 follows the Rice approach in its simplest form: resolve
calls to definitions in the **same source file**, compare argument identifier
names against parameter names, and flag any call where a clean swap is the
best match.

## Scope

- Detector: arg-swap
- Language: Rust
- Granularity: top-level `fn` definitions + call sites
- Resolution: same-file only (no cross-crate, no method dispatch resolution)
- Parameter count: exactly 2 (binary functions)
- Argument shape: simple identifiers only

## Functional requirements

### F1 — Input

Accepts `&[IrFile]` (R-1 / ir-v0.md F4 override). Files where
`language != Language::Rust` are skipped.

### F2 — Definition extraction

Read each IrFile's `IrFn` entries directly (the converter parsed every
file once; the detector never reparses via `raw_tree()` — R-1.c'' Path
b). Rust registers only top-level functions (`!is_method`, matching the
v0.5.x root-level `function_item` walk); Python additionally registers
class methods (F4b). For each registered function record:
- function name (`IrFn.name`)
- parameter names from `IrFn.params`, in declaration order

Build the parameter list from `IrParam.kind`:
- drop `ParamKind::Receiver` (Python `self` / `cls`)
- reject the whole definition on any `ParamKind::Unsupported`
  (`*args`, `**kwargs`, tuple patterns, `/` and `*` separators, …)
- reject the whole definition on any `_`-prefixed `Plain` parameter

The arity-2 filter (and the duplicate-name no-op for Rust) is applied
later at resolution (F4), not at registration.

### F3 — Call-site extraction

Walk the raw tree-sitter tree (`IrFile::raw_tree()`) for every
`call_expression` (Rust) / `call` (Python) node, in document order,
across the whole file. Collect each call whose:
- function operand is a single bare identifier (Rust), or a bare
  identifier / single-segment `self.` / `cls.` method (Python, F3b),
  and whose
- arguments are all bare `identifier` nodes.

Record callee name, the identifier-name argument vector, argument
count, and location. Skip calls where any argument is non-identifier
(keyword args, splats, literals, nested calls).

Call-site enumeration is a raw-tree walk rather than an IR walk
(definition extraction in F2 stays on IR). The R-1.c'' Path (b) IR
migration narrowed enumeration to the converter-materialised shapes;
that walk could not reach calls buried in `IrExpr::Other` shapes
(`binary_operator`, closures, Python comprehensions / generators /
conditional expressions, f-strings, …), silently regressing arg-swap
recall on real code — a name-correlating swap inside any of those
shapes produced no finding where v0.5.x did. The T1 pinning
(ir-v0.md §F6 T1) did not catch the regression because the only such
call in the audit / wild corpora
(`benchmarks/audit-corpus/files/totalsegmentator_statistics.py:10`)
has no argument/parameter name correlation and so fires in neither
version. Enumeration was therefore reverted to the v0.5.x full raw-tree
walk (2026-06-03), which keeps the T1 pinning byte-identical and
restores detection of swaps nested in `Other` shapes. This is the
Pattern-B escape hatch pr-miner keeps (ir-v0.md §F5): full call-set
enumeration is not losslessly representable in the simplified IR. The
regression guard lives in `tests/detector_arg_swap.rs` T30 / T31.

### F3b — Method calls on `self` / `cls` (added 2026-05-21)

In Python, the function operand may also be an `attribute` node of
the shape `self.<name>` or `cls.<name>`. The attribute's identifier
becomes the callee, mirroring how the corresponding method is
registered in the per-file definition table (F4b). Deeper attribute
chains (`obj.foo`, `self.x.y`) remain out of v0 scope — resolving the
receiver type requires flow analysis. In Rust, method calls
(`obj.method(args)`) are still skipped: tree-sitter-rust's
`method_call_expression` is a distinct node kind from
`call_expression` and Rust's UFCS/trait-resolution adds further
ambiguity.

### F4 — Resolution

For each call site, look up its callee name in the per-file definition table.
- If exactly one definition with matching name and argument count = 2 exists,
  use it.
- If zero or more than one match, skip.

### F4b — Class methods (added 2026-05-21)

In Python, walk into every `class_definition` body and register its
`function_definition` (and `decorated_definition`-wrapped) methods
under their bare name. The implicit `self` or `cls` receiver is the
first parameter of any method per Python convention; F4b drops it
before the `_`-prefix rejection rule (F2) and the arity check (F4),
so a method declared `def foo(self, a, b):` registers with two
parameters `[a, b]` and matches the same call shape that F3b
accepts via `self.foo(args)`.

### F5 — Swap detection

Given a 2-arg call to a 2-arg definition:
- Let `(a0, a1)` = call argument names (lowercased).
- Let `(p0, p1)` = parameter names (lowercased).
- Define `name_matches(a, b)` = either `a == b` (F5a strict path)
  or one of `a` / `b` is a strict prefix of the other AND the
  shorter side is at least `PREFIX_MATCH_MIN_CHARS = 3` characters
  long (F5b prefix path). Equal-length distinct names never match.
- The call is **identity** iff `name_matches(a0, p0) AND
  name_matches(a1, p1)`.
- The call is **swapped** iff `name_matches(a0, p1) AND
  name_matches(a1, p0) AND NOT identity`.

F5b is added 2026-05-21. The matching kind that produced the
swap is recorded as `evidence.raw.match_kind = "strict"` for F5a
or `"prefix"` for F5b. The Rice et al. ICSE 2017 detector — already
cited as `rice-icse-2017` — uses abbreviation-aware matching to
recover swaps where the call site abbreviates the parameter name
(`dst` against `dstfn`, `inf` against `info`, the audit-corpus
`rarfile_set_attrs.py:14` shape).

### F6 — Finding shape

Each detected swap produces one Finding:
- `detector_id = "arg-swap"`
- `primary` = location of the swapped call
- `related = [location of the definition]`
- `message` = `"call argument order swapped relative to definition of `<name>`"`
- `raw_severity = Warning`
- `evidence.citation_keys` includes at least one of `li-zhou-fse-2005`,
  `rice-icse-2017`
- `evidence.raw` carries: `callee_name`, `parameter_names`, `argument_names`

### F7 — Output stability

Findings sorted by `(primary.file, primary.start_line)` lexicographically.

### F8 — Anomaly class

Every Finding emitted by arg-swap sets `anomaly_class = AnomalyClass::Interface`
(IEEE 1044-2009 §5.4). Rationale: a swapped-argument call is a violation of the
callee's parameter contract — the canonical "Interface" anomaly — not a logic
bug inside either function body. Surfaced in SARIF as
`result.properties.anomalyClass = "Interface"`.

## Non-functional requirements

### N1 — Determinism (P3)

Identical input produces identical output.

### N2 — Citation (P1)

Enforced by `register_detector`.

### N3 — No side effects

`detect()` performs no I/O.

### N4 — Robustness (N5 from clone-drift mirror)

Files that fail to parse are skipped silently in v0.

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | def `fn copy(dst, src)` + call `copy(src, dst)` | 1 Finding |
| T2 | def `fn copy(dst, src)` + call `copy(d, s)` | 0 Findings (no name match) |
| T3 | def `fn copy(dst, src)` + call `copy(dst, src)` | 0 Findings (correct order) |
| T4 | def `fn one(x)` + call `one(x)` | 0 Findings (single-arg out of scope) |
| T5 | def `fn three(a, b, c)` + call `three(c, b, a)` | 0 Findings (3-ary out of scope) |
| T6 | call with literal arg `copy(x, 42)` | 0 Findings |
| T7 | call to unknown function | 0 Findings |
| T8 | T1 fixture | every Finding has citation_key in {li-zhou-fse-2005, rice-icse-2017} |
| T9 | T1 fixture run twice | identical output |
| T10 | empty input | 0 Findings, no error |
| T11 | def + call in different files (same scan) | 1 Finding (cross-file resolution within scan) |
| T25 | Python class method declared with `self`, called via `self.copy(src, dst)` | 1 Finding, `match_kind = "strict"` (F3b + F4b) |
| T26 | rarfile shape: `self._set_attrs(dst, inf)` against `def _set_attrs(self, info, dstfn)` | 1 Finding, `match_kind = "prefix"` (F5b) |
| T27 | call `copy(s, d)` against def `copy(dst, src)` (single-letter args) | 0 Findings (F5b prefix floor) |
| T28 | call `copy(tar, src)` against def `copy(target_buf, source_buf)` (identity by prefix) | 0 Findings (F5b identity precedence) |
| T29 | classmethod with `cls` receiver | 1 Finding (F4b drops `cls` like `self`) |

## Non-goals (v0)

- N-ary swap detection (3+ params)
- Rust method calls (`obj.method(a, b)`) and Python attribute chains
  beyond the single-segment `self.<name>` / `cls.<name>` shape
  admitted by F3b
- Cross-crate / cross-module resolution beyond same scan
- Type-aware matching (params with same type are inherently swappable)
- Fuzzy name matching beyond the strict-prefix shape admitted by F5b
  (no Levenshtein, no edit distance, no semantic embeddings)

## Known recall upper bounds (recorded 2026-05-21, refined 2026-06-03)

The audit corpus at `benchmarks/audit-corpus/` carries four
expected `arg-swap` entries (one `github-commit`, three
`paper-appendix`). cntrdct recovers one — the
`rarfile_set_attrs.py:14` case — via F3b + F4b + F5b, giving
`recall_upper_bound = 0.25` (1/4). All three FNs are genuine
swaps (verified by upstream fix commits / PyPIBugs labels), not
labelling errors. They fall into two distinct, structural recall
bounds — not v0 scope choices that a later spec amendment closes
cheaply. The 2026-06-03 audit (`docs/spec/recall-audit-v0.md`
"arg-swap / clone-drift FN triage") confirmed both, and that the
F3 call-enumeration regression fix does NOT move this corpus
figure (see bound B).

### Bound A — same-file resolution (F4)

F4 resolves a callee only against definitions in the same scan unit
(SHOULD: any file in the same scan; the corpus entries are
single-file, so the imported definition is absent). Two FNs are
unresolvable for this reason — the callee is imported from a module
not present in the corpus file, so F4 finds zero candidate
definitions and skips before F5 ever runs:

- `unv_app_settings.py:41` — `update_dict_recur(...)` imported from
  `unv.utils.collections` (upstream line 7).
- `nbrmd_test_ipynb_to_R.py:26` — `compare_notebooks(...)` imported
  from `jupytext.compare` (upstream line 5).

This is the resolution model stated in Scope ("same-file only") and
Non-goals ("cross-crate / cross-module resolution beyond same
scan"). T11 shows cntrdct DOES resolve cross-file when both the
definition and call files are in the scan; the bound is that the
audit entries package only the call-site file. Closing it would
require either packaging the imported module into the corpus entry
(a labelling/corpus change) or whole-program / import-following
resolution (out of Layer 1's per-file deterministic contract).

### Bound B — name-correlation ceiling (F5)

The third FN resolves cleanly (definition is same-file) but carries
no lexical signal F5 can use:

- `totalsegmentator_statistics.py:10` —
  `get_radiomics_features(ct_file, mask)` against same-file
  signature `(seg_file, img_file)`. The argument identifiers share
  no equality or prefix with the parameter names, so neither
  identity nor swap matches and F5 emits nothing.

This was cross-checked against the cover-based checker in Scott et
al. ASE 2020 (SwapD, arXiv 2009.09117, §3.4), the published
state-of-the-art for syntactic name-correlation arg-swap detection,
which would also miss all three even given the signatures: after
common-morpheme elimination the surviving morphemes share no first
character, so SwapD's similarity metric (zero when first characters
differ; §3.2) collapses every coverage to 0 and the dual threshold
`(<α₁=0.5, >α₂=0.75)` cannot be satisfied. These are semantic swaps
(CT vs. segmentation) that require reasoning beyond identifier
morphology — the Allamanis et al. NeurIPS 2021 self-supervised
PyBugLab model or LLM adjudication, neither of which fits Layer 1's
deterministic, citation-grounded contract.

Bound B is the target of REBUILD.md R-4 (the P3 amendment for a
Layer 0 LLM candidate generator running against IR call-site
predicates); it is NOT an F5c-or-later Layer 1 spec amendment.

Note on the F3 regression (2026-06-03): the call-enumeration revert
(F3) does not change this corpus's `recall_upper_bound`, because the
one audit call hidden in an `IrExpr::Other` shape
(`totalsegmentator_statistics.py:10`, inside a list comprehension)
is itself bound B (no name correlation) and so fires in neither the
pre- nor post-fix detector. The fix recovers real-world recall for
name-correlating swaps nested in `Other` expression shapes, which no
audit/wild fixture exercised — hence the regression's invisibility
to the T1 gate.

## References (P1)

- `li-zhou-fse-2005` — Li & Zhou, "PR-Miner: Automatically Extracting Implicit
  Programming Rules and Detecting Violations in Large Software Code", ESEC/FSE 2005
- `rice-icse-2017` — Rice, Aftandilian, Jaspan, Johnston, Pradel, Arroyo-Paredes,
  "Detecting Argument Selection Defects", ICSE 2017
- `scott-ase-2020` (consulted but not added to `CITATIONS`) — Scott,
  Ranieri, Kot, Kashyap, "Out of Sight, Out of Place: Detecting and
  Assessing Swapped Arguments", ASE 2020 (arXiv 2009.09117). Read
  alongside the audit-corpus FN review; SwapD's cover-based checker
  also misses the three FN cases above, which is why the
  name-correlation ceiling section above documents this and the
  detector does not adopt SwapD's algorithm in v0.
