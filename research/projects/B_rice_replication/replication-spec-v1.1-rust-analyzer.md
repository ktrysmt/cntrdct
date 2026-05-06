# Rice 2017 replication specification v1.1 (DRAFT addendum): rust-analyzer-based type filter

Author: ktrysmt
Date: 2026-MM-DD (set when v1 lands and this addendum is finalised)
Project: cntrdct Track B (replication of Rice et al., ICSE 2017)
Status: DRAFT addendum. Composes with `replication-spec-v1.md` once
v1 lands (USR-3: paper read + v0 → v1 promotion). Until then,
composes with `replication-spec-v0.md` section 4.1 (the sketch of
the optional rust-analyzer-based add-on).
Composes-with: `replication-spec-v1.md` section 4.1.

## 0. Why an addendum, not an edit

`replication-spec-v1.md` will be frozen at the moment Rice 2017
placeholders are resolved. Material additions discovered after that
freeze go into a v1.x addendum that names the v1 file it composes
with rather than editing v1 in place; this preserves v1's status as
the audit copy of the verified Rice port. This file is the first such
addendum and adds the optional rust-analyzer-based type filter that
v1 section 4.1 will sketch but not specify.

The user has decided that the Rust-specific type-check survival
figure is scientifically interesting enough to spend the integration
effort. v0 section 9 marked this decision as "depends on the v1
reader's judgement"; the decision is now resolved in the affirmative.

The remaining open decisions before any implementation work starts
are: (a) how rust-analyzer is queried, and (b) what counts as "type
equivalence". This addendum fixes both.

## 1. Integration scope

Two viable architectures for surfacing parameter types from
rust-analyzer.

### 1.1 LSP-over-stdio

`cntrdct-research` spawns rust-analyzer as a child process and
communicates over the standard Language Server Protocol JSON-RPC
channel. The detector trace pass (per v0 section 5.2) emits a list
of `(file, span, parameter_index)` tuples; a new
`cntrdct-research rice-types` subcommand issues
`textDocument/hover` requests against each tuple, parses the
returned type strings, and writes a side-car JSON file that the
aggregator (deferred per v0 section 9) joins back to the trace
output.

Pros:

- No dependency on rust-analyzer's internal crate API. The LSP
  protocol is stable; the `ra_ap_*` crates are explicitly
  documented as unstable.
- `cntrdct-research`'s dependency surface adds only `serde_json`
  and a small JSON-RPC client.
- The same approach generalises to non-Rust languages later
  (Track A's multi-language ambitions). Coupling Track B to
  rust-analyzer's LSP surface is far easier to undo than coupling
  to its internal crate API.

Cons:

- Type strings returned by rust-analyzer are display strings, not
  fully-resolved structural types. Equivalence comparison becomes
  string normalisation; section 2.1 specifies the rules.
- Subprocess startup is slow (single-digit seconds per crate).
  Wild-corpus runs of 100 to 1000 crates need batching. The
  per-call query cost is small in comparison.
- The Rust ecosystem's mature LSP client crates are server-side
  (`tower-lsp`, `lsp-server`); client-side options are sparser.
  The implementation will likely roll a minimal JSON-RPC client
  rather than depend on a heavy framework.

### 1.2 Direct ra_ap_* crate invocation

`cntrdct-research` depends on `ra_ap_ide`, `ra_ap_hir`,
`ra_ap_syntax`, and constructs an in-process analysis of each
crate. The trace pass emits the same tuples; type queries happen
via `ra_ap_hir::Type` resolution rather than over LSP.

Pros:

- Fully-resolved structural types, not display strings.
  Equivalence is `Type::could_be_unified` (or a narrower
  predicate); no string-normalisation problem.
- No subprocess: deterministic and cheap once the analysis is
  built.
- The trace + query path is a single Rust binary; the
  reproducibility story is one binary, one invocation, one output
  file.

Cons:

- `ra_ap_*` is explicitly unstable. Pinning to a specific
  rust-analyzer commit is required; bumping requires audit.
- `cntrdct-research`'s build surface roughly doubles; CI cycle
  lengthens materially.
- API churn risk over the multi-month replication horizon. A
  breaking change mid-study forces a choice between abandoning the
  in-flight measurement and freezing on a potentially buggy
  rust-analyzer revision.
- Type semantics become "whatever rust-analyzer happens to
  compute", not the rustc-canonical answer. This is mostly correct
  but is a quiet coupling that future readers may not notice.

### 1.3 Decision

This addendum specifies LSP-over-stdio (section 1.1) as the
primary integration.

Rationale:

- `ra_ap_*` instability is incompatible with a multi-month
  replication study's measurement-stability requirement.
- The LSP path's main weakness, display-string-based equivalence,
  is mitigable by predicate choice (section 2 below adopts a
  coarse predicate that is robust to display-string variance).
- The LSP path generalises to non-Rust languages. Track A's
  Phase 2+ ambitions are multi-language; not coupling Track B to
  rust-analyzer's internal API keeps that door open.

If during implementation the LSP path's accuracy proves
unacceptable (measured by section 3 below), the documented
fallback is to switch to `ra_ap_*` with an explicit version pin
and a Threats to Validity section in the eventual replication
paper. The switch is recorded as a new addendum (v1.2), not as an
edit to v1.1.

## 2. Type-equivalence predicate

The question the optional add-on answers is: "would rustc have
allowed a swap of these two arguments?" The narrowest correct
answer is "yes iff the two parameters' types unify under whatever
generic substitution rustc applied at the call site". That is
correct but expensive: it requires resolution at the call site, not
just at the declaration. A coarser answer is acceptable for the
replication's purpose, which is a survival-rate distribution
estimate, not a per-call defect oracle.

### 2.1 Display-string equivalence (adopted)

Two parameters' types are considered equivalent iff their normalised
display strings, as returned by rust-analyzer's
`textDocument/hover`, match exactly after the following
normalisation:

- Collapse runs of whitespace to a single space.
- Strip leading and trailing whitespace.
- Strip lifetime parameters: `&'a T` becomes `&T`,
  `Foo<'b, T>` becomes `Foo<T>`.
- Strip module qualification: `std::vec::Vec<u8>` becomes
  `Vec<u8>`. Use-site qualification differences are not material to
  the replication's survival-rate metric.

The normalisation rules MUST be recorded in the trace output as a
field `type_normalisation_version: "1"` so a future addendum that
extends or refines them is auditable.

False-positive direction: two parameters whose normalised display
strings match but which would not unify under rustc (for example
unrelated type aliases resolving to the same display name). These
count as "type-equivalent" and inflate the post-type-check survival
denominator. The over-report is small in practice (rustc rejects a
tiny fraction of arg pairs by alias mismatch alone); we accept this
conservative direction.

False-negative direction: two parameters whose types unify under
rustc but whose display strings differ (for example one written as
`Self::Item`, the other as the concrete instantiation). These are
excluded from the equivalent subset and shrink the survival
fraction. This is the more concerning direction. Mitigation: the
trace output records both the raw and the normalised display strings
so a manual audit pass during seed-corpus calibration (section 4)
can spot the most common false negatives.

### 2.2 Predicates considered and rejected

- Structural equivalence via `ra_ap_hir::Type`. Rejected per
  section 1.3 (`ra_ap_*` instability).
- Head-identifier equivalence (`Vec<u8>` equivalent to `Vec<u16>`
  because both have head `Vec`). Rejected as too coarse;
  generic-parameter differences materially affect the metric.
- Rustc-faithful unification at the call site. Rejected as too
  expensive; running rustc per call and parsing its diagnostic
  output is the implementation Rice's 38 percent figure is derived
  from in C++ / Java, but it is overkill for a survival-rate
  distribution. Reserved as a v1.2+ option if section 3
  measurements demand it.

### 2.3 Generic-parameter handling

The section 2.1 normalisation does not strip anything inside `< >`.
Two parameters of types `HashMap<String, u32>` and
`HashMap<&str, u32>` are NOT equivalent under section 2.1 even
though they share head `HashMap`. This is intended: Rice's
type-distinct subset reports cases where the types differ enough
that rustc would catch a swap; a same-head-different-parameters
pair is in that subset.

Lifetime parameters are stripped per section 2.1 because they are
not material to swap detection. Rust's borrow checker rejects
illegal swaps regardless of lifetime annotation; the replication
does not attempt to model that filter.

## 3. Reportable metric

The metric this addendum adds to the replication's output is a
single per-corpus fraction:

- Numerator: number of arg-swap candidates emitted by the trace
  path with reason `kept` (per v0 section 5.2 taxonomy) for which
  the section 2.1 type-equivalence predicate holds.
- Denominator: total number of arg-swap candidates emitted with
  reason `kept`.
- Reported with a Wilson 95 percent CI per Track A convention
  (`research/projects/A_1000_crate/scripts/phase1_kappa_wilson.py`
  exposes `wilson_ci`; the same module is the project-wide source
  of truth for binomial CI computation).

Secondary outputs, descriptive only and without CIs:

- Count of candidates excluded from section 3's denominator because
  the LSP query returned no type or returned an error. These rows
  appear in trace output under a new reason
  `skipped:lsp-no-type` introduced by this addendum and added to
  the v0 section 5.2 taxonomy as part of the v1.1 promotion.
- A flat list of the 20 most common pair-types in the
  type-equivalent subset and the 20 most common in the type-distinct
  subset. Descriptive sanity check; not part of the headline number.

The headline figure for the eventual replication paper is the
per-corpus fraction with its CI. It is reported alongside Rice's
38 percent (per v0 section 3 row 3) under the same table, with an
explicit note that the two figures are NOT directly comparable.
Rice's denominator is "candidates rejected by the host language
type checker"; this addendum's denominator is "candidates whose
parameters share a section 2.1-equivalent type". The paper text
must prevent the reader from reading the two as a like-for-like
comparison; the v1 promotion of this addendum will add a paragraph
to that effect in the methodology section.

## 4. Seed-corpus calibration

Before any wild-corpus run, the LSP integration is calibrated
against the same 10 positive seeds v0 section 6 specifies plus a
new set of type-distinct negatives:

- `arg_swap_001`..`arg_swap_010` (existing fixtures): every fixture
  must produce a successful LSP type query for both parameters and
  yield a type-equivalent result. All 10 fixtures use a single
  concrete type for both parameters by construction, so this is the
  type-equivalent baseline.
- New `arg_swap_type_distinct_001`..`arg_swap_type_distinct_005`
  (5 fixtures, technical-side artefact under
  `benchmarks/corpus/`): every fixture must produce a type-DISTINCT
  result. Coverage:
  - 001: distinct concrete types: `fn f(x: u32, y: u64)`.
  - 002: generic-parameter mismatch:
    `HashMap<String, u32>` vs `HashMap<&str, u32>`.
  - 003: reference vs value: `&String` vs `String`.
  - 004: trait object vs concrete: `&dyn Display` vs `String`.
  - 005: tuple-arity difference: `(u32, u32)` vs `(u32, u32, u32)`.

If either set fails, the LSP integration is incorrect and no
wild-corpus run starts. The seed pass is a hard gate per the
discipline established by v0 section 6.

The new type-distinct fixtures are technical-side artefacts; their
introduction is the trigger for a `promote(track-b): ...` commit
sequence per CLAUDE.md root section 3 once the addendum's optional
path is implemented. The fixtures themselves do not require this
addendum to be promoted first; they can ship under a `feat(corpus):
...` commit in the technical workspace independently.

## 5. Implementation hand-off

Implementation lives in the technical workspace, not in
`research/`. The patch series is:

1. `feat(detector-arg-swap)`: add the `RICE_TRACE=1` trace path
   per v0 section 5.2. Reason taxonomy frozen by v0 (this addendum
   adds only `skipped:lsp-no-type` to that taxonomy).
2. `feat(corpus)`: add `arg_swap_type_distinct_001`..`_005` under
   `benchmarks/corpus/`.
3. `promote(track-b)`: add a `cntrdct-research rice-types`
   subcommand that consumes the trace JSONL output, spawns
   rust-analyzer over LSP, and writes the type side-car JSON.
4. `promote(track-b)`: add a `cntrdct-research rice-aggregate`
   subcommand (also covers v0 section 9's deferred aggregator
   work) that joins the trace output and the type side-car and
   emits the per-KLOC, per-bucket, and survival-rate tables. The
   survival-rate table includes the section 3 metric with its
   Wilson CI.

The `cntrdct-research` subcommand additions stay on the research
side per CLAUDE.md (they extend `research/cli-research`, not
`crates/cli`). The detector trace path and corpus fixtures are
technical-side per the same.

## 6. Out of scope (v1.1)

- Wild-corpus runs at any size beyond the seed fixtures. A v1.2
  follow-up specifies the wild-corpus protocol once the seed pass
  validates the integration.
- `ra_ap_*` fallback path. Documented as the section 1.3
  contingency only; pursued as v1.2 if section 3 measurements are
  unacceptable.
- Cross-language extension (Java / C++ via their respective LSP
  servers). Track A's multi-language story remains independent.
- Method-call sites and trait-method dispatch. arg-swap-v0
  section F3 already excludes non-identifier callees; this
  addendum inherits that scope.

## 7. Promotion path

This addendum follows the same discipline as
`failure-modes-v1.md` and `rubric-v1-draft.md`:

1. Author drafts at
   `research/projects/B_rice_replication/replication-spec-v1.1-rust-analyzer.md`.
2. User confirms section 1.3 (LSP-over-stdio choice) and the
   section 2.1 normalisation rules.
3. Once v1 lands (USR-3 in the parent handoff), this file is
   promoted alongside it as a sibling addendum. NOT folded into v1.
4. Promotion target: `prereg/<YYYY-MM-DD>-replication-spec-v1.1.md`,
   with `Date:` matching v1's date and a `Supersedes:` header
   pointing to no predecessor (this is the first v1.x addendum).
5. The technical-side implementation work (section 5) starts after
   promotion. That work is a `promote(track-b): ...` commit series
   per CLAUDE.md root section 3.

## 8. References

- `research/projects/B_rice_replication/replication-spec-v0.md` —
  the scaffold this addendum extends. Section 4.1 is the specific
  attachment point.
- `docs/spec/arg-swap-v0.md` — cntrdct's arg-swap detector
  contract. Seed fixtures referenced in section 4 mirror the spec's
  test plan.
- `research/projects/A_1000_crate/scripts/phase1_kappa_wilson.py` —
  Wilson CI source of truth for the project.
- Language Server Protocol specification (LSP) — protocol stability
  is the rationale for section 1.1 over section 1.2.
- `rice-icse-2017` (CITATIONS.md) — Rice et al., "Detecting
  Argument Selection Defects", ICSE 2017. Directly cited by
  arg-swap-v0 section F6.
