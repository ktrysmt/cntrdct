# pr-miner detector v0 spec

Status: approved 2026-05-06.

Owner of `ROADMAP.md` P-2 (multi-language reframing of the original
`pr-miner-rust` slot).

## Scope

- Detector: `pr-miner` (not `pr-miner-rust` — multi-language from
  inception per Phase E note in `ROADMAP.md`).
- Languages: Rust and Python.
- Granularity: top-level function definitions; rule items are
  function-call sites inside each function's body.
- Output: `Vec<Finding>` from `Detector::detect`.
- Out: cross-function temporal ordering (use-before-def, lock
  pairing as a sequence), inter-procedural analysis, library API
  rules sourced from external docs, type-aware rule mining,
  configuration override.

## Citation grounding (P1 / citations-policy.md)

Primary algorithm citation:

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically
  Extracting Implicit Programming Rules and Detecting Violations in
  Large Software Code", ESEC/FSE 2005. The paper's frequent-itemset
  framing is what this detector implements; we are NOT lifting a
  single hand-coded rule. Original subjects: C/C++ (Linux kernel,
  PostgreSQL, Apache).

Per-language grounding:

- Rust: `li-zhou-fse-2005` is grandfathered under
  `citations-policy.md` clause (b) (algorithm is language-agnostic;
  cntrdct itself counts as the secondary application for the
  grandfathered Rust detectors). Status: `Confirmed`.
- Python: requires a separate survey under
  `docs/surveys/pr-miner-python-{date}.md` per
  `citations-policy.md`. If the survey returns no candidate
  satisfying clauses (a) / (b) / (c), the Python extension ships
  with `LanguageCitationStatus::Unconfirmed`, exactly as
  `comment-code` and `unreachable-after-terminator` did. The survey
  will start with: Acharya & Xie (FSE 2007 "Mining API patterns as
  partial orders from source code"), Wasylkowski et al. (ESEC/FSE
  2007 "Detecting Object Usage Anomalies"), and PyBugLab
  (`allamanis-neurips-2021`, already cited for arg-swap; the
  PyPIBugs evaluation corpus may carry rule-violation cases
  relevant here).

## Functional requirements

### F1 — Input

Accepts `&[ParsedFile]` via `DetectContext`. Per
`multilang-v0.md` F6 Pattern A, the detector dispatches on
`file.language` internally. Files of unsupported languages are
skipped without error. Empty input returns `Ok(vec![])`.

### F2 — Function and call-site extraction

For each `ParsedFile` of a supported language, walk the syntax tree
and emit one `Transaction` per top-level function definition.
A `Transaction` is the multiset of `Item`s extracted from the body
of that function.

Per-language extraction tables:

| Language | function-defining node | call-site node | item value |
|---|---|---|---|
| Rust   | `function_item`        | `call_expression` whose head is a `path`/`identifier` | the path's last segment as a string |
| Python | `function_definition`, `decorated_definition` (incl. `async def`) | `call` whose function is an `identifier` or `attribute` | the attribute's last `.name` segment, or the bare identifier |

Items with non-identifier heads (closures, dynamic dispatch,
`obj[index]()` etc.) are dropped in v0. Methods are reduced to
their last-segment name (`obj.foo(...)` → item `foo`); this
matches PR-Miner's "function name only" formulation and trades
some precision for tractable rule mining over modest corpora.

Functions whose body produces fewer than `MIN_TRANSACTION_ITEMS`
distinct items are dropped before mining (their support signal is
too sparse; mirrors Li-Zhou's noise-suppression filter).

### F3 — Frequent-itemset mining

Implements Apriori (Agrawal & Srikant, VLDB 1994; cited as a
methodology reference, not a separate detector citation) bounded
to itemsets of size ≤ `MAX_ITEMSET_SIZE`. Inputs:

- The transaction database built in F2 over ALL supported-language
  files in the input set (single shared database; we do NOT mine
  per-language databases in v0 because the Rust+Python corpora are
  small enough that splitting would starve both, and rules are
  about the same items modulo namespace).
- Parameters: `MIN_SUPPORT` (relative, fraction of transactions),
  `MIN_CONFIDENCE` (rule confidence threshold),
  `MAX_ITEMSET_SIZE`.

Output: a set of association rules `LHS → RHS` where `LHS` and
`RHS` are disjoint itemsets, `support(LHS ∪ RHS) ≥ MIN_SUPPORT`, and
`confidence(LHS → RHS) = support(LHS ∪ RHS) / support(LHS) ≥
MIN_CONFIDENCE`.

For v0 we restrict `MAX_ITEMSET_SIZE = 2`, so rules are exactly
`{a} → {b}`. This matches the "implicit pairing" pattern Li-Zhou's
own evaluation focused on (lock/unlock, alloc/free,
fopen/fclose).

### F4 — Violation detection

For each mined rule `{a} → {b}`:

- Scan every function in the input set BEFORE the
  `MIN_TRANSACTION_ITEMS` filter (i.e. the full extracted set, not
  the mining-filtered subset). The filter exists only to suppress
  noise during rule discovery; a function whose body contains `a`
  alone is exactly the violation pattern Li-Zhou is designed to
  surface, and dropping it before checking the rule would defeat
  the detector.
- A function T VIOLATES the rule iff `a ∈ items(T)` and `b ∉
  items(T)`.
- The function is the violation site.

Per F3's `MAX_ITEMSET_SIZE = 2`, the violation predicate stays
Boolean per function. When `MAX_ITEMSET_SIZE` is raised post-v0,
the predicate generalises to `LHS ⊆ items(T) ∧ ¬(RHS ⊆ items(T))`.

### F4b — Item-cardinality post-filter (added 2026-05-21)

R7 (item-cardinality post-filter) ships as a fourth gate inside
`mine_pairs` (`src/detectors/pr_miner/apriori.rs`). For each candidate
pair `{a, b}` that has cleared F3's support and confidence floors,
the rule is DROPPED iff either side's marginal cardinality strictly
exceeds `MAX_ITEM_CARDINALITY * |T|`, where `|T|` is the mining-
database size and `MAX_ITEM_CARDINALITY` defaults to `0.5`.

The gate is grounded in Li-Zhou §3.2 ("we filter common library
calls"): the algorithm's own discussion of stop-list-style filtering
treats universal items as outside the paired-API operating envelope.
Items that "everyone" calls describe co-occurrence in a shared
context rather than a contract, so a rule involving them is
statistical noise.

Strict `>` boundary: an item at exactly the threshold survives.
Distinguishes "exceeds N%" from "matches or exceeds N%".

Empirical effect on shipped corpora (v0.4.3 measurement):

| Corpus | Rule | Cardinality | Caught by F4b at 0.5? |
|---|---|---|---|
| Synthetic stdlib-shape fixture | `Err -> Ok` (24 of 32 tx have `Err`) | 0.75 | yes |
| `benchmarks/wild-corpus/` | `Err -> Ok` (132 of 650 tx have `Err`) | 0.20 | **no** |
| `benchmarks/wild-corpus-python/` | `TypeError -> isinstance` (14 of 98 tx have `TypeError`) | 0.14 | **no** |

The empirical FM-A pathology in the shipped wild corpora sits below
the spec-aligned threshold. F4b at `MAX_ITEM_CARDINALITY = 0.5`
catches only the upper-bound shape (items that are truly universal
in the mining DB); on its own it does not move the v0.1 calibration
numbers. F4c (R6 stop-list, added the same day) does — see below
for the joint reading.

F4b lands as the algorithmic primitive (so the cardinality dial
becomes a first-class knob the calibrator can move per release)
without claiming to close FM-A in v0. The choice between F4b-bis
and R6 is deferred to a future Q-series entry.

### F4c — Per-language stop-list (added 2026-05-21)

R6 (per-language stop-list) ships in `src/detectors/pr_miner/mod.rs`
as `RUST_STOPLIST` and `PYTHON_STOPLIST`. Items in the list for a
given language are dropped from the corresponding transactions
*before* mining and *before* the violation scan — same in-place
filter, same pass — so they neither participate in rule discovery
nor in violator emission. Citation grounding: Li-Zhou ESEC/FSE 2005
§3.2 ("we filter common library calls"). The lists are intentionally
narrow at v0:

- Rust: `Err`, `Ok`, `Some`, `None`. Closes the empirical wild-
  corpus `Err -> Ok` rule. `new` / `from` / `into` are deliberately
  *not* included because the last-segment representation makes them
  collide with legitimate user-defined constructors and converters.
- Python: built-in exception classes (`TypeError`, `ValueError`,
  `KeyError`, `Exception`, ...), built-in introspection
  (`isinstance`, `issubclass`, `getattr` / `setattr` / `hasattr`,
  `callable`), iteration / sequence helpers (`len`, `range`, `iter`,
  `next`, `enumerate`, `zip`, `map`, `filter`), and inheritance
  / I/O / debug primitives (`super`, `print`, `repr`). Closes the
  empirical wild-corpus `TypeError -> isinstance` rule.

Empirical effect (re-measured 2026-05-21 against the same corpora
that defined the v0.1 FM-A baseline):

| Corpus | Before R6 | After R6 |
|---|---|---|
| `benchmarks/wild-corpus/` | 19 pr-miner FPs (all `Err -> Ok`) | 0 |
| `benchmarks/wild-corpus-python/` | 2 pr-miner FPs (`TypeError -> isinstance`) | 0 |
| `benchmarks/corpus/` | 16 TPs + 2 cross-fixture FPs | 16 TPs + 2 cross-fixture FPs |

R6 alone closes FM-A entirely. The remaining 2 seed-corpus FPs are
the R8 cross-fixture pollution shape (`close_handle -> open_handle`
on `unreachable_python_002.py`, `push -> new` on
`clone_drift_009.rs`) — they survive R6 because neither item is in
the stop-list. F4d below closes them.

### F4d — Manifest-driven pr-miner eligibility (R8, added 2026-05-21)

`benchmarks/corpus/manifest.jsonl` is a mixed-fixture corpus: each
file is a positive for exactly one detector, and the per-detector
fixtures share an `files/` subtree. When pr-miner scans the corpus,
identifiers reused across peer-detector fixtures (`push` and `new`
appearing in many `clone_drift_*.rs`; `close_handle` in one
`unreachable_python_*.py`) leak rules from peer detectors' positives
into pr-miner's mining DB and surface as violators on the same
peer-detector files.

The R8 fix is corpus-side: `ManifestEntry` gains an optional
`pr_miner_eligible: Option<bool>` field. When `Some(false)`,
`scripts/build_priors_corpus.py` drops every pr-miner finding on
that file from the labelled corpus, so it never enters the
calibration TP / FP tallies. Default (`None` / `Some(true)`) is the
backward-compatible "eligible" interpretation.

Scope:

- The detector itself does *not* read the manifest. Real-world
  `cntrdct scan` runs (where there is no manifest) are unaffected
  by R8.
- The flag applies only to the post-scan labelling pipeline.
- The 90 non-pr-miner-positive entries under `benchmarks/corpus/`
  are bulk-tagged `pr_miner_eligible: false`. The 16 pr-miner
  positive fixtures plus the 6 `pr_miner_*_negative_*` files stay
  unflagged (default eligible).

Empirical effect: pr-miner labelled-corpus FP count drops from 2 to
0 against `benchmarks/corpus/`. Combined with F4c above, pr-miner
moves from 16 TP / 22 FP (`posterior_tp 0.425`) to 16 TP / 0 FP
(`posterior_tp 0.944`, Jeffreys at n=16) — the same band as the
other 0-FP detectors.

### F4e — Python noise-filter extraction carve-outs (added 2026-05-21)

The audit-corpus review against PyPIBugs-style fixtures surfaced
two recurring Python FP shapes that are noise in the Li-Zhou §3.2
sense — the call is syntactically present but semantically does
NOT participate in the paired-API contract the rule encodes.
Both carve-outs apply at extraction time (F2) so the call never
enters `Transaction.items` for either the mining DB or the
violation scan, mirroring how F4c handles the same class of noise
via a stop-list rather than a post-filter.

(F4e-i) Context-manager cleanup synthesis. A `call` whose
immediate parent is a Python `with_clause` / `with_item` is the
resource the surrounding `with` block manages. Python's
`with EXPR as NAME:` contract guarantees that `EXPR.__exit__`
runs at block exit; for the canonical resource pairs the
extractor synthesises the matching cleanup identifier into
`Transaction.items` so paired-API rules (e.g. `{open} -> {close}`,
`{acquire} -> {release}`) are satisfied by the implicit cleanup
without any syntactic cleanup call in the body. The pairs the
extractor recognises in v0 are:

| context-manager call head | synthesised cleanup |
|---|---|
| `open` | `close` |
| `acquire` | `release` |

The original `call` head is NOT dropped — the open / acquire still
appears in `items` so a function that opens AND legitimately calls
`close` explicitly still satisfies both directions of the rule
(`{open} -> {close}` and `{close} -> {open}`). Earlier drafts of
F4e-i dropped the call instead of synthesising the cleanup; that
spec produced a new FP class on shapes like
`with open(p) as fh: ...; fh.close()` (defensive double-close
idiom — the function lost its `open` item and tripped the
`{close} -> {open}` and `{write} -> {open}` rules in the
opposite direction). The synthesise-only formulation avoids both
the original FN and the introduced FP. Heads not on the pair list
(generic `with` over a non-standard context manager, e.g.
`with database_session() as session:`) leave `items` unchanged.

Pinned by `audit-corpus/files/nbrmd_test_ipynb_to_R.py:21` (the
canonical FP shape: `with open(nb_file) as fp:` with no syntactic
`close()` in the body) and
`audit-corpus/files/carla_import.py:161`
(`with open(...) as fh: ...; fh.close()` — must NOT trip the
inverse `close -> open` rule after synthesis). Both pinned by
tests t-prm-fp-1 and t-prm-fp-4.

(F4e-ii) Non-identifier-chain attribute receivers. An `attribute`
call whose `object` is anything other than an "identifier chain"
(a recursive `attribute(identifier_chain, identifier)` or a single
`identifier`) is dropped. Subscript receivers (`pieces[-1].write(x)`),
call receivers (`get_handler().write(x)`), parenthesised
expressions and arithmetic results are all skipped — they describe
write-like operations on transient or computed objects whose
type the extractor cannot resolve, and conflating them with the
file/socket/lock idioms PR-Miner is mining inflates FP rate
without recovering any TP.

Pinned by `audit-corpus/files/rosrust_genaction.py:50`
(`parse_action_spec` calls `pieces[-1].write(l + '\n')` on
`io.StringIO` instances; `write` here is not the file-handle
write that the mined `write -> close` / `write -> open` rules
target) and tests t-prm-fp-2 / t-prm-fp-3.

Both carve-outs are scoped to the Python extractor. The Rust
extractor is unaffected — its analogous patterns surface different
empirical FP shapes (overloaded `walk` / `kind` / `filter` /
`collect` across tree-sitter and iterator ecosystems) that need
separate analysis and a different stop-list extension. That work
is preregistered in `ROADMAP.md` as a follow-up to F4e.

### F5 — Finding shape

For each violating function:

- `detector_id = "pr-miner"`
- `primary` = location of the violating function (start line / col
  of the function definition)
- `related` = locations of ALL functions in the database that
  satisfy the rule (i.e. contain both `a` and `b`). Capped at
  `MAX_RELATED` to keep findings tractable in large corpora; the
  cap is documented in `evidence.raw`.
- `message` = format!(`"function calls {a} but never {b}; {N} of {M} similar functions ({percent}%) call both"`)
  where N / M / percent are the rule's support / confidence numbers.
- `raw_severity = Warning`
- `evidence.citation_keys` MUST include `li-zhou-fse-2005`.
  Python findings additionally include the survey-resolved
  Python citation key when `Confirmed`; otherwise none.
- `evidence.language_citation_status` follows the survey result:
  `Confirmed` for Rust always; `Confirmed` or `Unconfirmed` for
  Python depending on survey outcome.
- `evidence.raw` carries: `rule_lhs`, `rule_rhs`, `support`,
  `confidence`, `transaction_count`, `related_capped`.

### F6 — Output stability

Findings sorted by `(primary.file, primary.start_line, rule_lhs,
rule_rhs)` lexicographically. Ties on the first three are vanishingly
rare (at most one rule per function head); the rule fields are added
to the sort key purely for absolute determinism.

### F7 — Anomaly class

Every Finding emitted by pr-miner sets `anomaly_class =
AnomalyClass::Logic` (IEEE 1044-2009 §5.4). Rationale: a missing
companion call to a paired-API operation is a logic-level
inconsistency, not an interface mismatch, data-shape problem, or
documentation drift.

### F8 — Suppression hooks

Honours the existing T2-7 suppression surface unchanged:

- `#[cntrdct::allow(pr-miner)]` on the violating function (Rust).
- `cntrdct.toml [detectors.pr-miner] enabled = false`.
- `cntrdct.toml [languages.python] suppress = ["pr-miner"]` (M-5).

No new suppression keyword.

## Non-functional requirements

### N1 — Determinism (P3)

Identical input produces identical output, including ordering.
Apriori is deterministic given a stable iteration order over the
transaction database; we sort the items inside each transaction
and the transactions among themselves before mining to keep BTree
/ HashMap iteration order from leaking into rule discovery.

### N2 — Citation (P1)

Enforced by `register_detector` and the per-language extension at
`tests/citations_consistency.rs`.

### N3 — No side effects

`detect()` performs no I/O, no logging, no network.

### N4 — Performance (target only, not gated)

10K LOC of mixed Rust+Python processed in < 60s on a single core,
mining included. The Apriori step dominates; for v0 corpora (both
shipped seed corpora total ≪ 10K LOC) wall time is expected ≪ 5s.

### N5 — Robustness

Files that fail to parse are skipped silently in v0 (consistent
with the other detectors). When the transaction database is
smaller than `MIN_DATABASE_SIZE`, mining returns no rules and no
findings — pr-miner has no signal at corpus sizes where every
itemset is a singleton.

## Test plan

Every fixture below pads its scenario with filler functions whose
bodies contain two distinct identifiers each (`alpha(); beta();` etc.)
so the total transaction count is `≥ MIN_DATABASE_SIZE = 20`. The
fillers' identifier pairs are chosen so they neither participate in
the scenario rule nor cross `MIN_SUPPORT` themselves; their sole
purpose is to clear the database-size gate. Scenario function counts
listed in the table refer to the rule-relevant subset, not the total.

| ID | Description | Expected |
|---|---|---|
| T1 | 9 Rust fns each calling `acquire(); release()`; 1 fn calling `acquire(); helper()` (no release) | 1 Finding, primary = the lone fn, related.len() == 9 |
| T2 | 9 Python fns each calling `open(...); close()`; 1 fn calling `open(...); helper()` (no close) | 1 Finding, primary = the lone fn |
| T3 | 5 Rust fns + 5 Python fns calling `lock(); unlock()`; 1 Rust fn calling `lock(); helper()` only | 1 Finding (rule mined cross-language) |
| T4 | 9 fns each calling `acquire(); helper()` only; 1 fn calling `acquire(); release()` | 0 Findings (no rule mined: confidence too low) |
| T5 | 4 fns calling pair, 16 calling neither | 0 Findings (support below threshold) |
| T6 | T1 corpus run twice | identical `Vec<Finding>` |
| T7 | T1 finding | `evidence.citation_keys` includes `li-zhou-fse-2005` |
| T8 | T2 finding | `evidence.language_citation_status` matches survey outcome |
| T9 | empty input | 0 Findings, no error |
| T10 | one fn with parse error + valid T1 fixture | invalid one skipped, violation still detected |
| T11 | rule `{a} → {b}` and rule `{b} → {a}` both qualify on the same corpus | both rules' violations are reported (no merging) |
| T12 | `MAX_RELATED` exceeded | `related.len() == MAX_RELATED`; `evidence.raw.related_capped == true` |
| T13 | mixed-language transaction database where Rust calls `lock()` and Python calls `acquire()` (different identifiers) | no spurious rule across the synonym pair |
| T14 | violating fn is decorated with `#[cntrdct::allow(pr-miner)]` (Rust) | violation suppressed |
| T15 | `cntrdct.toml [languages.python] suppress = ["pr-miner"]` and corpus has Python violations only | 0 Findings |
| t-prm-fp-1 | `def f(p): with open(p) as fp: data = fp.read()` | items contain `open` AND synthesised `close` (F4e-i) |
| t-prm-fp-2 | `def f(): pieces[-1].write(x); other_call()` | `write` dropped (F4e-ii subscript receiver); `other_call` kept |
| t-prm-fp-3 | `def f(): get_handler().write(x)` | `get_handler` kept; `write` dropped (F4e-ii call receiver) |
| t-prm-fp-4 | `def f(p): with open(p) as fh: fh.write(x); fh.close()` | items contain both `open` and `close` (F4e-i must not drop) |
| t-prm-fp-5 | `def f(lock): with lock.acquire() as l: do_stuff()` | items contain `acquire` AND synthesised `release` (F4e-i) |
| t-prm-fp-6 | `def f(): with session() as s: do_stuff()` | items contain `session`; no synthesis (head not on pair table) |
| t-prm-fp-7 | `def f(self): self.foo.lock(x)` | items contain `lock` (F4e-ii identifier-chain receiver kept) |

## Tunable constants (v0 defaults)

- `MIN_SUPPORT = 0.05` — rule's joint itemset must appear in ≥ 5% of
  transactions. Below this threshold the rule is statistical noise;
  Li-Zhou used 0.01 for kernel-scale corpora but defaults higher
  for the modest corpus sizes cntrdct ships against.
- `MIN_CONFIDENCE = 0.85` — when LHS appears, RHS must follow in ≥
  85% of transactions for the rule to count. Picks up "almost
  always paired" patterns without flagging every co-incidence.
- `MAX_ITEMSET_SIZE = 2` — pairs only in v0 (see F3).
- `MIN_TRANSACTION_ITEMS = 2` — functions whose body has < 2 distinct
  call items contribute nothing to mining and are dropped.
- `MIN_DATABASE_SIZE = 20` — below 20 transactions the Apriori
  output is too unstable to act on; detector returns no findings.
- `MAX_RELATED = 32` — caps `Finding.related` so a corpus with 1000
  satisfying functions does not produce a 1000-element related
  array for every violation.
- `MAX_ITEM_CARDINALITY = 0.5` — F4b R7 cardinality post-filter.
  Drops rules whose `lhs` or `rhs` strictly exceeds this fraction of
  the mining database. v0 default catches only the upper-bound
  ("everyone calls it") shape; the empirical wild-corpus FM-A
  pathology sits below this threshold and is left for follow-up
  (F4b-bis lower threshold or R6 stop-list — see F4b discussion).

Exposed as `pub const` in `cntrdct-detector-pr-miner` for tuning
without API change. Real-world calibration belongs to Layer 2
(ranker), not these constants.

## Detector dispatch (multilang-v0.md F6 Pattern A)

Pattern A: a single `cntrdct-detector-pr-miner` crate parameterised
by `Language`. Per-language differences (function-defining nodes,
call-site nodes, name extraction) live in private helpers. The
public surface is one `Detector` impl with
`supported_languages() = &[Language::Rust, Language::Python]`.

Rationale: pr-miner's bug pattern ("a function violates an implicit
rule mined from its peers") is the same concept across languages;
only the AST mechanics differ. Same call as `arg-swap` and
`clone-drift` — Pattern A applies.

## Corpus contribution

P-2 acceptance demands `≥ 8 positive cases for the new detector` in
the seed corpus. Spec extends this to `≥ 8 per supported
language` to honour the M-6 / citations-policy.md framing:

- 8 Rust positives under `benchmarks/corpus/files/pr_miner_NNN.rs`
  exhibiting realistic API-pairing rule violations (lock/unlock,
  fopen/fclose, mutex acquire/release patterns).
- 8 Python positives under
  `benchmarks/corpus/files/pr_miner_python_NNN.py` exhibiting the
  Python equivalents (`__enter__/__exit__` outside `with`,
  `acquire/release`, `open/close`).
- ≥ 3 negatives per language: functions that participate in mined
  rules and DO satisfy them.

`tests/corpus_shape.rs` is extended to count the new
detector and require ≥ 8 positives.

## Migration sequence

1. Survey doc `docs/surveys/pr-miner-python-2026-MM.md` lands
   first. This is the lowest-risk artefact and locks in the
   `Confirmed` / `Unconfirmed` Python citation status before the
   detector ships.
2. Algorithm crate `cntrdct-detector-pr-miner` ships with Rust-only
   support against ≥ 8 positives. Python `supported_languages()`
   left out of v0.0; Detector trait `supported_languages()` returns
   `&[Language::Rust]` initially.
3. Python dispatch + 8 Python positives + Python citation (or
   `Unconfirmed` flag) land together in v0.1. This is when
   `supported_languages()` widens to both languages.
4. Layer 2 ranker recalibration runs on the extended corpus once
   v0.1 is in. P-4's labelled-corpus pipeline picks up the new
   detector automatically — no Layer 2 code changes needed.

The two-step landing keeps each PR reviewable (the algorithm and
the language addition are separable concerns) and makes a partial
revert tractable if the Python pilot reveals issues with item
extraction.

## Non-goals (v0)

- Multi-itemset rules (`MAX_ITEMSET_SIZE > 2`).
- Inter-procedural rules (function-call ordering across function
  bounds).
- Type-aware rule mining (e.g. "every `Mutex<T>::lock()` call
  follows a successful `Mutex::new`").
- API-doc-driven rules (e.g. mining rules from rustdoc /
  Sphinx).
- Languages beyond Rust and Python.
- LSP integration for the new detector.
- Configurable rule whitelisting beyond the existing per-detector
  suppression surface.

## Risks and open questions

R1. Apriori's worst-case complexity is exponential in
`MAX_ITEMSET_SIZE`. We bound at 2, so the worst case is
O(items² × transactions) — fine for the corpora cntrdct targets,
but if the constant is later raised an FP-growth or
ECLAT-based implementation may be required.

R2. Reducing methods to "last segment of the dotted path" loses
information (`a.lock()` and `b.lock()` map to the same item).
Empirically this is what Li-Zhou did and the simplification keeps
the rule database tractable on small corpora; the mistake mode is
spurious cross-context rules that future spec revisions can address
by switching to fully-qualified paths once we have larger corpora.

R3. The Python survey may return nothing publishable. Planned
fallback: ship the Python extension with
`LanguageCitationStatus::Unconfirmed`, exactly per the
`comment-code` precedent. The detector still satisfies P1 because
its overall citation set is non-empty (`li-zhou-fse-2005`).

R4. Mixed-language transaction database may surface a rule that
holds in one language but not the other (e.g. a Rust-only
`new() → drop()` pattern that has no Python analogue). v0 accepts
this trade-off — the rule applies to whatever language has matching
transactions; the suppression surface (per-language `[suppress]`)
gives users an escape hatch. Post-v0, splitting the database by
language is a candidate refinement.

R5. The seed corpus may not be large enough for `MIN_DATABASE_SIZE
= 20` to be hit pre-shipping. If so, the v0.0 release is mining-
inactive on the seed corpus alone and the test plan above
synthesises a tempdir corpus large enough to trigger mining. The
real signal arrives once the wild corpora (M-4 and the future
P-1) populate.

## Empirical FP analysis (v0.1, 2026-05; closed 2026-05-21)

> **Resolution status:** the 22 FPs documented below are *closed* as
> of 2026-05-21 by F4c (R6 stop-list) + F4d (R8 manifest eligibility).
> Re-calibration against the same corpora moves pr-miner from 16 TP /
> 22 FP / `posterior_tp = 0.425` (Wilson at n=38) to 16 TP / 0 FP /
> `posterior_tp = 0.944` (Jeffreys at n=16, since the FP count drop
> takes the sample below the Q-11 small-sample threshold). The v0.1
> baseline is retained below for historical reference and for any
> future analysis that compares against a known regression target.

Recalibration after step 3 (`chore(priors): recalibrate against
pr-miner v0.1 corpus`) labels 16 TP / 22 FP across `benchmarks/corpus`,
`benchmarks/wild-corpus-python`, and `benchmarks/wild-corpus`
(`posterior_tp = 0.43`, `wilson_lower_95 = 0.28`). The 22 FPs cluster
into two failure modes that the v0 design did not anticipate.

FM-A. Stdlib-constructor / builtin co-occurrence (21 of 22 FPs).
- Rust: rule `Err -> Ok` with 19 violations across `chrono`, `flate2`,
  `mio`, `regex_syntax`, `serde`, `serde_json`, `uuid`. Mechanism:
  `Result`'s `Err(...)` and `Ok(...)` are tree-sitter
  `call_expression`s reduced to the items `Err` and `Ok` per F2's
  last-segment rule. They co-occur in the majority of fallible Rust
  functions, so Apriori mines the pair with high confidence; functions
  that early-return only `Err(...)` (typical short delegators that
  forward to a helper for the success path, e.g.
  `uuid__parser.rs::parse_braced`) are flagged as violators even
  though no API contract is being broken.
- Python: rule `TypeError -> isinstance` with 2 violations in
  `click_utils.py::get_binary_stream`, `::get_text_stream`. Mechanism:
  Python validators frequently combine `isinstance(x, T)` with
  `raise TypeError(...)`; functions that raise `TypeError` after a
  different shape of guard (e.g. `if opener is None:`) are flagged.

FM-B. Cross-fixture name collision (1 of 22 FPs).
- Rule `close_handle -> open_handle` (the reverse direction of the
  `pr_miner_python_001/003.py` open_handle/close_handle pair) flags
  `unreachable_python_002.py::parse_header`, which legitimately calls
  `close_handle()` for that fixture's own scenario but never
  `open_handle()`. The pr-miner positive corpus's heavy use of the
  pair makes the reverse rule mineable; absence of `open_handle` in a
  peer-detector fixture surfaces as a violation.

FM-A drives the bulk of the FP count and is what depresses
`posterior_tp` from a paired-API-driven ceiling near 0.7-0.8 down to
0.43. FM-B is corpus-specific to the fixture-rich seed.

## v1 mitigations under consideration

R6. Per-language stop-list of constructors / builtins.
**Landed 2026-05-21** as F4c (see above). Implementation ships
`RUST_STOPLIST` and `PYTHON_STOPLIST` constants in
`src/detectors/pr_miner/mod.rs` with last-segment matching; the
filter runs unconditionally inside `detect()` between extraction
and mining. The v0 lists are narrow (Rust:
`Err` / `Ok` / `Some` / `None`; Python: built-in exception classes
+ introspection + iteration helpers + `super` / `print`) — broader
items like `new` / `from` / `into` are deliberately excluded to
avoid collisions with legitimate user-defined paired APIs. The
spec-noted `cntrdct.toml [detectors.pr-miner] stoplist` user-
override surface is *not yet* implemented; the default lists handle
the observed wild-corpus FM-A on their own and the override knob is
deferred until external usage demonstrates a need.

R7. Optional fully-qualified item granularity (refines R2).
Switching from "last segment" to the full path (`core::result::Result::Err`
instead of `Err`) would distinguish stdlib constructors from
user-defined symbols at zero stop-list maintenance cost. Cost: the
extractor needs to resolve paths, which tree-sitter alone cannot do
for Rust use-statements / re-exports. A pragmatic compromise: keep
last-segment as default but add an item-cardinality post-filter
(drop rules whose `lhs` or `rhs` exceeds N% of all transactions —
items that "everyone" calls are by definition not paired-API
candidates). Eliminates most of FM-A without a manual list.

Status (2026-05-21): the item-cardinality post-filter ships as F4b
at `MAX_ITEM_CARDINALITY = 0.5`. The algorithmic primitive is in
place and unit-tested; empirically the wild-corpus FM-A items sit
below the spec-aligned threshold (Rust `Err` at 0.20, Python
`TypeError` at 0.14), so F4b at 0.5 leaves the v0.1 calibration
numbers unchanged. See F4b for the table and the two open follow-up
paths (F4b-bis lower threshold + fixture rework, or R6 stop-list).
The fully-qualified-path variant of R7 remains unimplemented at
v0.4.4.

R8. Cross-fixture isolation in mixed-fixture corpora.
**Landed 2026-05-21** as F4d. Implementation took option (a):
`ManifestEntry` gains an optional `pr_miner_eligible: Option<bool>`
field with `serde(default)` so the schema bump is backward-
compatible. `scripts/build_priors_corpus.py` reads the field and
drops pr-miner findings on ineligible files from the labelled JSONL
before calibration consumes it. 90 non-pr-miner-positive entries
under `benchmarks/corpus/manifest.jsonl` are bulk-tagged
`pr_miner_eligible: false`; the 16 pr-miner positive fixtures plus
the 6 `pr_miner_*_negative_*` files stay unflagged (default
eligible). Option (b) (Layer-2 de-weighting) was not pursued — the
corpus-side fix is more honest about what pr-miner sees and does not
introduce a new ranker-side coupling between detector and
manifest.

## Compatibility

- `cntrdct-core` unchanged. The `Detector` trait surface already
  accommodates the new detector; `register_detector` continues to
  enforce P1.
- The pr_miner detector ships as a new module under
  `src/detectors/pr_miner/` at v0.1.0.
- `src/lib.rs` registers the new detector and bumps to v0.3.0
  alongside the v0.1 Python extension.
- `cntrdct.toml` schema unchanged. `[detectors.pr-miner]` works
  via the existing per-detector override surface.
- SARIF emitter unchanged — the new detector flows through
  unchanged because the `Detector` trait is the only contract.
- Corpus manifest format unchanged (already extended in M-4).

## References (P1)

- `li-zhou-fse-2005` — Li & Zhou, "PR-Miner: Automatically
  Extracting Implicit Programming Rules and Detecting Violations
  in Large Software Code", ESEC/FSE 2005. Primary citation.
- `agrawal-vldb-1994` — Agrawal & Srikant, "Fast Algorithms for
  Mining Association Rules", VLDB 1994. Methodology reference for
  the Apriori algorithm we implement; not a separate detector
  citation per `citations-policy.md` since it does not introduce
  the detector concept.
- Pending Python citation, contingent on the survey landing in
  step 1 of the migration sequence above.

## Approval

Approved 2026-05-06 with revisions R-A (test-fixture sizing
relative to `MIN_DATABASE_SIZE = 20`) and R-B (F4 violation scan
applies before the `MIN_TRANSACTION_ITEMS` filter) folded in.
Implementation proceeds against this approved spec.
