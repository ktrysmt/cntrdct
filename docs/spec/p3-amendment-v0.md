# cntrdct P3 amendment v0 spec — Layer 0 LLM candidate generator

Status: APPROVED for implementation 2026-06-07. The review-before-build
gate (§12) is cleared and the two §11 design forks are accepted (Layer-0
prior keying on `(detector_id, origin)`; v0 ships an EMPTY Layer-0 prior
with a no-op fallback, deferring the labelled corpus to Phase B).
Post-R-4-review revision 2026-06-07 absorbing a 4-axis parallel review
(P3-integrity / architecture / spec-consistency / risk-completeness):
8 blockers + 8 majors + locator fixes. The absorbed decisions are
recorded in §12 "Review log" and folded into the body below; do not
re-litigate them.

REBUILD.md R-4 deliverable (carry-over from the retired `ROADMAP.md`
Q-17). v0.6.0 is explicitly out of scope (REBUILD.md §4 R-4); this
spec drafts the architectural amendment to design constraint P3 so the
IR call-site predicates landed in R-1 can anchor it. Implementation is
gated on a review of this document, the same way `ir-v0.md` (R-0)
absorbed an 11-blocker review before R-1 began. This revision clears
that gate's findings; §11 records the acceptance criteria the
implementation-approval decision is measured against.

## 1. What this amends

P3 as written (CLAUDE.md "Design constraints"; `adjudicator-v0.md`):

> only the Layer 3 adjudicator may invoke an LLM. Layers 1, 2, and 4
> are deterministic, including the Q-12 post-processing helper
> `apply_llm_calibration`. `reqwest` is reachable only from
> `src/adjudicator.rs::ReqwestClient` and the
> `build_default_adjudicator` constructor in `src/lib.rs`.

P3 is enforced three ways and the amendment must keep all three intact:

1. Code review — `reqwest` import sites are audited.
2. The `network-isolation` CI job (`.github/workflows/ci.yml`) runs
   `cntrdct scan benchmarks/corpus/files --format sarif
   --no-calibration` inside `sudo unshare --net`. The default scan
   path (walker → parsers → Layer 1 → Layer 2 → Layer 4) MUST complete
   with zero network access; any socket open fails with `ENETUNREACH` /
   `EAI_*`. There is no opt-out for `scan` / `calibrate` / `eval`.
3. The Q-12 calibration helper `apply_llm_calibration`
   (`src/llm_calibration.rs`) is deterministic even though it
   post-processes LLM verdicts.

The amendment introduces a new pipeline stage — Layer 0 — that runs an
LLM. P3 as written forbids this. The amendment narrows P3 from "only
Layer 3 may invoke an LLM" to "only Layer 0 and Layer 3 may invoke an
LLM, both only behind an explicit opt-in flag; the default
`scan` / `calibrate` / `eval` path stays deterministic and
network-free."

## 2. Why Layer 0 exists (motivation)

`arg-swap-v0.md` "Known recall upper bounds" §"Bound B —
name-correlation ceiling (F5)" records a structural recall ceiling
Layer 1 cannot cross by any deterministic refinement:

- `totalsegmentator_statistics.py:10` —
  `get_radiomics_features(ct_file, mask)` against the same-file
  signature `(seg_file, img_file)`. The argument identifiers share no
  equality or prefix with the parameter names, so F5 (cntrdct's
  name-correlation matcher) emits nothing.

`arg-swap-v0.md` cross-checked this against SwapD (Scott et al. ASE
2020, arXiv 2009.09117 §3.4), the published state-of-the-art for
*syntactic* name-correlation arg-swap detection: SwapD also misses it,
because after common-morpheme elimination the surviving morphemes
share no first character, collapsing its similarity metric to 0. The
spec's own conclusion:

> These are semantic swaps (CT vs. segmentation) that require reasoning
> beyond identifier morphology — the Allamanis et al. NeurIPS 2021
> self-supervised PyBugLab model or LLM adjudication, neither of which
> fits Layer 1's deterministic, citation-grounded contract.
>
> Bound B is the target of REBUILD.md R-4 (the P3 amendment for a
> Layer 0 LLM candidate generator running against IR call-site
> predicates); it is NOT an F5c-or-later Layer 1 spec amendment.

Layer 0 is the deliberate, P3-amended home for that reasoning. It does
not refine Layer 1; it sits *before* Layer 1 and originates candidate
findings that the deterministic layers would never have produced,
which then flow through the existing Layer 2 / 3 / 4 machinery.

Layer 3 alone cannot close Bound B: *pre-amendment*, the adjudicator
only ever sees `RankedFinding`s that Layer 1 already produced
(`adjudicator-v0.md` "Reads `RankedFinding`s emitted by Layer 2"). A
finding Layer 1 never emits is never adjudicated. Closing Bound B
therefore requires an *origination* path, not a *judgment* path — hence
Layer 0, not a Layer 3 extension. The amendment routes Layer 0
candidates *through* Layer 3 (§3.3, §4.1), so post-amendment the
adjudicator also sees `Layer0Llm`-origin findings; §2's pre-amendment
invariant is what motivates the new origination path, not a property
the amended pipeline preserves.

## 3. Design rationale (alternatives considered and dropped)

This section mirrors `cross-model-kappa-v0.md` "Design rationale": the
considered-and-dropped alternatives are recorded so a later session
does not re-litigate them.

### 3.1 LLM invocation mechanism: CLI shellout (chosen) vs. HTTP

Two ways to reach an LLM already ship in the repo:

- HTTP via `reqwest` — `AnthropicAdjudicator` / `ReqwestClient`
  (`src/adjudicator.rs`), used by `scan --adjudicate`. Opens a socket
  from cntrdct itself; constrained to `adjudicator.rs` by the P3
  reqwest-reachability rule.
- CLI shellout — `ClaudeCliAdjudicator` (`claude --print`) and
  `GeminiCliAdjudicator` (`gemini -p`), the Q-13 cross-model-kappa
  providers. cntrdct spawns a subprocess that itself talks to the
  network; cntrdct opens no socket. Both implement the object-safe
  `PromptDispatch` trait (`cross-model-kappa-v0.md` F1).

Chosen: CLI shellout, reusing `PromptDispatch`. Rationale:

1. Netns invariant. The `network-isolation` gate's invariant is "no
   code path in `cntrdct scan` makes a network call." CLI shellout
   preserves this literally even when Layer 0 is enabled: cntrdct
   spawns `claude --print`, which opens the socket. This is the exact
   shape CLAUDE.md already blesses for Q-13 — the `cross-model-kappa`
   subcommand "does NOT open a socket from cntrdct itself … it shells
   out to `claude --print` and `gemini -p`, which handle auth and HTTP
   themselves." Layer 0 inherits that carve-out instead of widening
   the reqwest-reachability surface.
2. No new auth surface. CLI providers authenticate via each CLI's own
   login (`claude` / `gemini` OAuth), "no API keys read by cntrdct"
   (CLAUDE.md). `scan --adjudicate`'s HTTP path needs an
   `ANTHROPIC_API_KEY`; Layer 0 should not add a second key.
3. Infrastructure reuse. `PromptDispatch`, the
   Skipped-on-auth-failure handling (`cross-model-kappa-v0.md` F2/F3),
   and the deterministic mock-fixture test pattern
   (`tests/cross_model_kappa.rs`) all transfer.

Trade-off accepted: CLI shellout is less methodologically clean than
raw HTTP (residual system prompts, agent-CLI persona — the same
confound `cross-model-kappa-v0.md` "Design rationale" point 2 documents
for Codex). For a *candidate generator* (proposals that Layer 3 then
adjudicates and Layer 2 then calibrates) this is acceptable; Layer 0
precision is a funnel, not a verdict.

Reqwest-reachability is preserved by *construction discipline*, not by
the type system: `AnthropicAdjudicator` (reqwest-backed) also
implements `PromptDispatch` (`src/adjudicator.rs`), so a
`Box<dyn PromptDispatch>` could in principle carry the HTTP provider
into the scan path. The Layer 0 driver (`src/candidate_llm.rs`, §4.3)
MUST therefore construct *only* the CLI providers — the
`build_audit_claude_cli_provider` / `build_audit_gemini_cli_provider`
pattern already used by Q-13 (`src/lib.rs`) — and MUST NOT reference
`build_default_adjudicator` or `ReqwestClient`. This is enforced
structurally by a `fmt`-job grep guard in `.github/workflows/ci.yml`
(the same shape as the existing `tree_sitter` / TBD greps) asserting
`candidate_llm.rs` references neither symbol, so the reqwest-reachable
set stays exactly `{adjudicator.rs::ReqwestClient,
build_default_adjudicator}` (absorbed: review M1/§6 P3).

### 3.2 Default-on vs. opt-in

Chosen: opt-in behind an explicit flag, default off. Rationale: the
`network-isolation` gate has "no opt-out for `scan` / `calibrate` /
`eval`," and the default `cntrdct scan` must remain runnable offline
(it is the end-user default, exercised in netns). Layer 0 therefore
mirrors `scan --adjudicate`: a flag the user must pass. The gate
continues to run the *unflagged* `scan` and stays green. The precise
structural twin of `--candidate-llm` is `--adjudicate` — both are flags
on the `scan` subcommand, and the netns job runs `scan` in its
flag-off form — so the flagged path is excluded from the netns gate by
the same logic that excludes `--adjudicate` (and, at the subcommand
level, `cross-model-kappa`): CLAUDE.md, "excluded from the netns gate
by design — it spawns subprocesses that themselves talk to the network,
same shape as `scan --adjudicate`".

Caveat on the netns assertion (absorbed: review M2). The netns gate
catches a socket opened *by cntrdct* (`ENETUNREACH`). Layer 0 opens no
cntrdct socket — it shells out — so if Layer 0 were ever accidentally
default-on, the netns run would NOT fail on a socket: the `claude
--print` subprocess would merely fail to reach the network, Layer 0
would yield no candidates, and a "zero `Layer0Llm` findings" assertion
would pass *trivially* via subprocess failure rather than because
Layer 0 stayed dormant. The netns SARIF assertion (§9) is therefore
defence-in-depth only; the authoritative guard against accidental
activation is a NON-netns, network-independent construction-probe test
(§9) asserting the default `scan` path constructs no Layer 0 provider.

### 3.3 Where Layer 0 candidates go: originate-then-adjudicate (chosen)

Chosen: Layer 0 emits low-confidence candidate `Finding`s that enter
the normal pipeline at Layer 2 (rank) → Layer 3 (adjudicate) →
Layer 4 (SARIF). Layer 0 *proposes*; Layer 3 *disposes*. Rationale:
keeping origination and judgment as separate LLM calls preserves the
two-stage discipline and lets the Q-13 cross-model κ audit measure
whether the *same* model both proposes and confirms (self-preference
risk — see §7 R3). Dropped alternative: Layer 0 emits findings
directly to SARIF, bypassing adjudication — rejected because an
unadjudicated LLM proposal has no precision floor and would flood
output.

Enforced, not merely intended (absorbed: review B5). Because
`--candidate-llm` and `--adjudicate` are otherwise independent flags
and `--adjudicate-top` caps adjudication at N (default 5,
`adjudicator-v0.md` F8), a naive design lets a `--candidate-llm`-only
run — or a candidate ranked below top-N — reach SARIF UNADJUDICATED,
which is exactly the rejected "bypass" design. The contract therefore
binds the two:

1. `--candidate-llm` REQUIRES `--adjudicate` (clap `requires`); passing
   the former without the latter is a usage error.
2. Every `Finding{origin: Layer0Llm}` is adjudicated regardless of
   `--adjudicate-top` (the top-N cap applies only to Layer-1-origin
   findings); a Layer 0 candidate that is not adjudicated (e.g. the
   provider returned `Skipped`) is SUPPRESSED from SARIF, never emitted
   raw. This guarantees the precision floor the dropped alternative
   lacked.

## 4. Architecture

### 4.1 Layer 0 placement

```
Layer 0 — LLM candidate generator     [NEW, opt-in, P3-amended]
            │  consumes IR call-site predicates (§5)
            │  emits candidate Finding{origin: Layer0Llm, ...}
            ▼
Layer 1 — deterministic detectors      (unchanged; runs in parallel)
            │
            ▼  Layer 0 candidates ∪ Layer 1 findings
Layer 2 — statistical ranker           (ranks the merged set)
            ▼
Layer 3 — LLM adjudicator              (adjudicates; opt-in --adjudicate)
            ▼
Layer 4 — SARIF emitter                (unchanged)
```

Layer 0 and Layer 1 are independent producers of `Finding`s. Layer 0
does not feed Layer 1 and Layer 1 does not feed Layer 0; their outputs
are merged before Layer 2. With Layer 0 off (the default) the merged
SET equals the Layer-1-only set exactly. Note this set-equality does
NOT by itself preserve T1 byte-identity (ir-v0.md F6 T1), which pins
per-finding *serialized bytes*: the new `Finding.origin` field must use
`skip_serializing_if` so default-origin findings serialise unchanged
(§4.3, B2). With that, Layer 1's T1 pinning is untouched.

### 4.2 CLI surface

`cntrdct scan <CORPUS> --candidate-llm[=<provider>] --adjudicate
[--candidate-llm-max-calls <N>]`
(working name; final flag name is an open question, §7 R5). Provider
defaults to `claude-cli`; `gemini-cli` selectable. The flag is additive
to `scan`; `calibrate` / `eval` gain no LLM path (P4 corpora stay
deterministically generated, §6). Without the flag, Layer 0 never runs
and `scan` is byte-identical to today.

Flag contract (absorbed: review B5/B6, M3):

- `--candidate-llm` REQUIRES `--adjudicate` (§3.3). Without it, clap
  rejects the invocation.
- `--candidate-llm-max-calls <N>` is a HARD ceiling on the number of
  LLM dispatches per scan (default a small conservative value, e.g.
  64). Unlike `cross-model-kappa`, which runs against a fixed small
  corpus, `scan` runs against arbitrary user trees, so the §7 R4
  deterministic pre-filter alone is not a bound. When the Bound B
  residue exceeds the cap, Layer 0 dispatches the first N (in
  deterministic file/byte order) and LOGS the number of call sites
  skipped (CLAUDE.md "no silent caps"). The skipped sites are not
  silently dropped findings — they are un-evaluated candidates,
  reported as such.
- Provider unavailable (CLI not installed / not logged in): `scan
  --candidate-llm` MUST degrade gracefully — emit the Layer-1-only
  findings, log a warning, and exit 0. A missing *optional* provider
  must never hard-fail a scan (contrast `cross-model-kappa`, an audit
  subcommand, which records `Skipped`). The Layer-1 findings are valid
  independent of Layer 0.

This stays inside the CLAUDE.md "CLI surface is split" contract:
`cntrdct` still exposes only `scan` / `calibrate` / `eval` /
`cross-model-kappa`; Layer 0 is a set of flags on the existing `scan`,
not a new subcommand.

### 4.3 Module layout (proposed, implementation-time)

- `src/candidate_llm.rs` — Layer 0 driver: call-site enumeration via
  the raw-tree Pattern-B walk (§5; NOT structured `IrCallSite`),
  predicate construction, prompt assembly, `PromptDispatch` invocation,
  and response parsing + VALIDATION into candidate `Finding`s. Reuses
  only the CLI `PromptDispatch` providers (§3.1; no `reqwest`, no
  `build_default_adjudicator`).
- `Finding` gains an `origin` discriminator
  (`Origin::{Layer1Deterministic, Layer0Llm}`, `Layer1Deterministic`
  the `Default`) so Layer 2/3/4 and SARIF can label provenance and the
  default path can be probed for "no Layer 0 provider constructed
  offline" (§9).

  Serialization (absorbed: review B2 — the original "byte-identical T1"
  claim was false without this). `Finding` derives `Serialize` and is
  serialized byte-for-byte into the T1 ir-pinning goldens
  (`tests/ir_pinning.rs`, `tests/fixtures/ir-pinning/<det>/
  {audit,wild-rust,wild-python}.json`) AND the Q-15 baseline fixtures.
  A normally-derived field would emit `"origin":"Layer1Deterministic"`
  into EVERY finding object and drift every golden, with the flag OFF —
  because T1 pins the per-finding *bytes*, which §4.1's "merged set ==
  Layer-1-only set" argument (about the finding SET) does not protect.
  The field MUST therefore carry
  `#[serde(default, skip_serializing_if = "Origin::is_layer1_default")]`
  so default-origin findings serialise identically to today; the T1 and
  baseline goldens then need no regeneration, and a §9 gate pins
  flag-off `Finding` serialization byte-identical to the pre-amendment
  output.

  Construction blast radius. `#[derive(Default)]` does NOT apply to
  struct-literal construction, so adding the field touches every
  `Finding { … }` literal site (~19–35: the seven detectors under
  `src/detectors/`, `src/config.rs`, `src/recall_audit.rs`,
  `src/adjudicator.rs`, `src/lsp.rs`, plus the test fixtures). The
  implementation should add `origin: Origin::default()` at the
  Layer-1 sites (or introduce a `Finding::builder` / `..Default`
  shorthand) and `Origin::Layer0Llm` in `candidate_llm.rs` only. The
  spec calls out the blast radius so it is budgeted, not discovered.

- Response validation (absorbed: review B7). The driver MUST treat the
  LLM response as untrusted: on malformed/unparseable JSON, a refusal,
  or an empty body → drop the candidate, log, and continue the scan
  (never panic, never exit non-zero); a proposed swap whose argument
  ordinals do not index into the predicate's `actual_args`, or that
  references args/params absent from the predicate, is rejected as
  malformed. §5's "cannot hallucinate" property bounds the *input*
  only; the *output* is constrained by this validation.

## 5. The call-site predicate interface

Layer 0 reasons over *predicates* — structured, citation-anchored
facts, never raw source dumped to a model. A predicate is the minimal
context an LLM needs to judge one candidate class without re-parsing.
For the arg-swap Bound B class (the v0 target), a predicate is:

```
CallSitePredicate {
    callee:        PathFact,          // call-head path (receiver chain + name)
    actual_args:   Vec<ArgFact>,      // positional args at the call site
    resolved_sig:  Option<Signature>, // same-file definition, if resolved
    call_location: Location,          // span of the call
}
ArgFact      { ordinal: usize, ident: Option<String>, expr_kind: &'static str }
Signature    { params: Vec<ParamFact> }
ParamFact    { ordinal: usize, name: String, kind: ParamKind, default: Option<String> }
```

Call-site ENUMERATION source (absorbed: review B1 — the decisive
blocker). The predicate must NOT be built by walking structured
`IrCallSite`s. The flagship Bound B call is
`totalsegmentator_statistics.py:10`,
`[get_radiomics_features(ct_file, mask) for mask in masks]` — the call
lives inside a `list_comprehension`, which the IR converter classifies
as `IrExprKind::Other` (`src/parsers/python.rs`, the catch-all `other`
arm) without recursing into the comprehension body, so NO `IrCallSite`
is ever materialised for it. This is the exact gap that forced
`arg-swap` itself to REVERT call-site enumeration to a raw tree-sitter
walk (`arg-swap-v0.md` §F3 "CORRECTION (2026-06-03)";
`src/detectors/arg_swap.rs` module header) — sourcing predicates from
`IrCallSite` would yield ZERO predicates for Layer 0's own motivating
FN. Layer 0 therefore enumerates call sites via the same raw-tree
Pattern-B walk `arg-swap` uses (ir-v0.md §F5 escape hatch) — ideally by
consuming arg-swap's existing resolved-but-unmatched call-site residue
directly, so the two share one enumeration path and one resolver.

Predicate FIELDS still derive from IR-equivalent data — `PathFact`
mirrors `IrPath` (receiver chain + last segment), `ArgFact.expr_kind`
mirrors the `IrExprKind` variant tag, `Signature`/`ParamFact` mirror
`IrFn.params` → `IrParam.{name, kind}` (`ParamKind ∈
{Plain, Receiver, Unsupported}`, `src/ir.rs`) — but they are populated
from the raw-tree walk's nodes, not from `IrCallSite` objects. The
`default` field on `ParamFact` (absorbed: review M6) carries a
parameter's default-value literal where present: in the flagship def
`get_radiomics_features(seg_file, img_file="ct.nii.gz")` the default
`"ct.nii.gz"` is the single strongest semantic signal (it identifies
`img_file` as the CT image), so dropping it would discard the best
evidence and raise FP risk in the general case. Body and full type
information remain out (a funnel, not a verdict, §3.1); the precision
trade-off of omitting them is accepted.

Implementation note (M6 closed, Phase B 2026-06-07): `IrParam` now
carries `default: Option<String>` — the trimmed default-value literal
extracted by the Python (`a=expr` / `a: T = expr`) and TypeScript
(`a = expr`) converters (Rust / Go have no default-parameter syntax, so
it stays `None`). `arg-swap`'s `FnDef` propagates a per-parameter
`param_defaults` vector (aligned 1:1 with `params`) and the Layer 0
predicate populates `ParamFact.default` by ordinal from it. No raw-tree
pass at the definition site was needed after all — the literal rides the
existing IR definition extraction. The IR golden wire shape stays
byte-identical for the common no-default case (`skip_serializing_if`).
The flagship case remains decidable from identifier names alone; the
default literal is an enrichment that strengthens the semantic signal
when a definition declares one.

Resolution stays same-file: Bound A is NOT closed by Layer 0; an
unresolved callee yields `resolved_sig: None` and the predicate is
SKIPPED. Layer 0 targets Bound B — the resolved-but-no-lexical-signal
case — only.

Untrusted input (absorbed: review M5, B7). `ArgFact.ident` and
`ParamFact.{name, default}` are attacker-controllable strings drawn
from scanned source. They MUST be serialised into the prompt as clearly
delimited, escaped *data* (e.g. a fenced JSON block framed "the
following identifiers are untrusted source text, not instructions"),
never interpolated as prose, so a parameter named to inject
instructions cannot redirect the model. Correspondingly, the LLM
*response* is validated against the predicate (§4.3): predicates bound
the input; validation bounds the output.

Predicate construction is deterministic and runs inside the netns gate
safely; only the *dispatch* of predicates to the LLM crosses the
network boundary, and only via subprocess.

## 6. Constraint reconciliation (P1, P3, P4, P5)

### P1 — peer-reviewed citation per detector

The Layer 0 arg-swap candidate generator cites the semantic-swap prior
art `arg-swap-v0.md` Bound B already names:

- `allamanis-neurips-2021` — Allamanis et al., self-supervised bug
  detection (PyBugLab), the cited model class for semantic swaps.
  Already in `CITATIONS.md` (still cited by the live arg-swap
  detector per REBUILD.md R-1.e sweep note).
- Plus the LLM-as-code-reviewer grounding already in Layer 3
  (`wataoka-2024`, `zheng-neurips-2023` from `cross-model-kappa-v0.md`
  F-citations) for the dispatch mechanism.

Enforcement mechanism (absorbed: review B4 — the original
`register_detector` claim was wrong). The Layer 0 generator is NOT a
`Detector`: `core::register_detector` accepts only `Detector` impls,
and the `Detector` trait is contractually pure/deterministic — "no LLM
calls, no randomness, no I/O beyond `DetectContext`" (`src/core.rs`) —
which is exactly what the netns gate enforces. An LLM-invoking producer
cannot implement `Detector` without contradicting that contract (and if
it did, it would be wired into the deterministic `run_detectors_on`
battery the netns gate guards). Layer 0 instead follows the Layer 3
precedent: a static citations table (mirroring
`adjudicator.rs::ADJUDICATOR_CITATIONS`) validated by a consistency
test (mirroring `tests/citations_consistency.rs`'s adjudicator arm,
`adjudicator-v0.md` F10) that resolves every Layer 0 key against
`CITATIONS.md`. Per `citations-policy.md`, per-language coverage for
Layer 0 is `Unconfirmed` until a Python/TS/Go-subject study grounds it
(same pattern as the R-2/R-3 cross-cutting detectors).

### P3 — narrowed, not broken

The amended invariant (§1): Layer 0 and Layer 3 may invoke an LLM,
both opt-in; the default path is deterministic and network-free.
Concretely:

- `reqwest` reachability is UNCHANGED — Layer 0 uses CLI shellout
  (`PromptDispatch` CLI providers), not `reqwest`. The
  reqwest-reachable set stays `{adjudicator.rs::ReqwestClient,
  build_default_adjudicator}`, enforced by the §3.1 CI grep guard on
  `candidate_llm.rs` (not by the type system alone).
- The `network-isolation` job continues to run unflagged `scan` and
  stays green. A defence-in-depth assertion is added: the netns `scan`
  run emits zero `Finding{origin: Layer0Llm}`. The AUTHORITATIVE guard
  against accidental default-on activation is a separate
  network-independent construction-probe test (§3.2 caveat, §9), since
  the netns gate cannot distinguish "Layer 0 dormant" from "Layer 0 ran
  but its subprocess failed to reach the network".
- `--candidate-llm` is excluded from the netns gate by design, same as
  `--adjudicate` and (at the subcommand level) `cross-model-kappa`.

Wiring reconciliation (absorbed: review M3/Q-4). Layer 0 candidates
carry `detector_id = "arg-swap"` (so SARIF `ruleId`, the
`ALL_DETECTOR_IDS` ↔ rules-list invariant pinned by
`tests/wiring_consistency.rs`, and the recall-audit attribution all
work unchanged) and are distinguished from Layer-1 arg-swap findings by
`origin = Layer0Llm`, NOT by a new detector id. No new entry is added
to `ALL_DETECTOR_IDS`, so the default `scan` SARIF `tool.driver.rules`
set is byte-identical; the Q-4 wiring test is unaffected.

### P4 — priors from labelled corpora

Layer 0 candidate confidence is NOT a hardcoded constant. Two layers
of calibration, both corpus-derived:

- Layer 2 ranking of Layer 0 candidates uses a Layer-0-specific prior
  that is NOT shared with the Layer 1 arg-swap prior — an LLM-originated
  candidate has a different base rate than a deterministic F5 hit.

  Keying (absorbed: review B3 — the original spec was unrealizable).
  Priors today are keyed by `detector_id` ONLY (`src/ranker.rs`
  `self.priors.get(&f.detector_id)`; `compute_priors` aggregates by
  `detector_id`, `src/calibration.rs`), and neither Layer 2 nor the
  Q-12 Platt registry consults `origin`. Since Layer 0 candidates carry
  `detector_id = "arg-swap"` (§6 P3 wiring reconciliation), a
  `detector_id`-only prior would COLLIDE the Layer 0 base rate with
  Layer 1's. The amendment therefore re-keys the prior map and
  `compute_priors` on `(detector_id, origin)`; this is an explicit,
  spec-owned Layer 2 + calibration-schema change (the labelled-corpus
  schema gains an `origin` column). Layer-1 entries default to
  `origin = Layer1Deterministic`, so existing `priors-default.json`
  values are unchanged.

  v0 ships an EMPTY Layer-0 prior with a no-op fallback (absorbed:
  review B3 fork — decided). Building a *Bound-B-class* labelled corpus
  is a hard, unsolved problem (see OPEN below), so rather than block
  v0 on it — or author numbers in code, which P4 forbids — v0 ships no
  `(arg-swap, Layer0Llm)` prior entry and the ranker falls back to its
  existing prior-miss path (`related.len()`, `src/ranker.rs`), exactly
  the Q-12 pattern where `llm-calibration` ships an empty registry with
  a no-op fallback. The labelled Layer-0 corpus and its fitted prior
  are deferred to Phase B.

- Layer 3 verdict confidence on Layer 0 candidates is Platt-calibrated
  by the existing Q-12 `apply_llm_calibration` registry
  (`llm-calibration-v0.md`), which already ships empty + no-op
  fallback.

OPEN (§7 R2): a Phase-B Layer 0 labelled corpus must avoid the
labeller-bias loop `recall-audit-v0.md` warns about — the prior must
NOT be fit on Layer 0's own triaged output. The candidate external
anchor is the PyPIBugs swapped-args partition (Allamanis et al. NeurIPS
2021, the partition `recall-audit-v0.md` already names). UNVALIDATED
and explicitly flagged: PyPIBugs' swap partition is dominated by
*lexically-detectable* swaps, so it is not yet established that it
contains enough *Bound-B-class* (morphology-blind) examples to fit a
meaningful Layer-0 prior; quantifying that subset is a Phase-B
precondition. (The earlier draft's "OSV swaps" reference was dropped:
OSV is a vulnerability-advisory DB and carries no arg-swap-class
labels.) Because v0 ships an empty prior, this OPEN does not block the
v0 build — it gates Phase B only.

### P5 — severities map to IEEE 1044-2009

Unchanged. Layer 0 candidate `Finding`s carry a detector-defined
`raw_severity` and `anomaly_class` and map through the same
`src/sarif.rs` table at emission time, which keys on those two fields
only — never on `detector_id` or `origin` — so Layer 0 findings map
exactly like Layer 1's. Severity lives above IR (`ir-v0.md`
"Background"; REBUILD.md goal G4 / P5).

## 7. Risks and open questions

R1. Determinism of `scan` output. Layer 1 `scan` is byte-identical
across runs; an LLM-driven Layer 0 is not (sampler stochasticity even
at temperature 0, per `cross-model-kappa-v0.md` "Design rationale"
point 3(b)). The non-stationarity is twofold: sampler stochasticity AND
external CLI/model drift — the CLI tools are "version-bumped silently"
(`cross-model-kappa-v0.md` N2 / "Design rationale" point 3(a)), the
reason Q-13 dropped continuous monitoring. Resolution: `--candidate-llm`
output is explicitly non-reproducible and labelled as such in SARIF
(`origin: Layer0Llm`); the deterministic guarantee is scoped to the
default path; T1 pinning never runs with the flag on; and any §9 recall
figure MUST be reported with the (CLI version, model id) tuple it was
measured against (a silent bump invalidates the number, same as Q-13).

R2. Labeller-bias loop for the P4 corpus (see §6 P4). The Layer 0
prior must not be fit on Layer 0's own triaged output. Anchor the
corpus to external ground truth per `recall-audit-v0.md`.

R3. Self-preference when one model both proposes (Layer 0) and
confirms (Layer 3). If `--candidate-llm=claude-cli` feeds
`--adjudicate` backed by Anthropic, the proposer confirms itself.
Resolution (IMPLEMENTED, Phase B 2026-06-07): the scan refuses
(exit 2) when the Layer 0 proposer and the Layer 3 adjudicator resolve
to the same model family. Families are classified coarsely by
provider-id / model substring (`candidate_llm::model_family`:
`claude`/`anthropic` → anthropic, `gemini`/`google` → google; unknown
fails open). The default Layer 3 adjudicator is Anthropic, so
`--candidate-llm=claude-cli` is blocked and `--candidate-llm=gemini-cli`
is allowed. `--allow-self-preference` overrides the guard with a logged
warning. Quantifying residual agreement on Layer 0 candidates via the
Q-13 cross-model κ audit remains available but is not wired into the
guard (a measurement, not a gate).

R4. Cost / fan-out. One LLM call per resolved-but-unmatched call site
is unbounded on a large corpus, and with `--adjudicate` required (§3.3)
each surviving candidate costs a SECOND LLM call. The pre-filter
(resolved same-file signature, arity ≥ 2, no F5 match) reduces but does
not BOUND fan-out. Resolution (decided, §4.2): the pre-filter narrows
the LLM to the Bound B residue, AND `--candidate-llm-max-calls <N>` is a
hard ceiling with explicit truncation-and-log semantics. Combined
worst-case cost is bounded at `2N` LLM calls per scan.

R5. Flag naming and provider selection surface (`--candidate-llm` /
`--candidate-llm-max-calls` working names). Decide alongside
`--adjudicate` / `--adjudicate-top` ergonomics so the opt-in LLM flags
are consistent. Implementation-time-deferrable (§11).

R6. Scope creep beyond arg-swap. Layer 0 generalises to any
detector with a semantic ceiling, but v0 should ship the arg-swap
Bound B generator only, as the single proof point — mirroring how R-2
shipped TypeScript as the single proof of the IR contribution model.

R7. Cost ceiling (absorbed from review). See R4 / §4.2: hard
`--candidate-llm-max-calls` cap, deterministic first-N selection, log
the skipped count. A §9 test asserts the cap bounds dispatch count.

R8. Candidate-response validation (absorbed from review). The LLM
response is untrusted: malformed/unparseable JSON, refusals, empty
bodies → drop + log + continue (never panic / non-zero exit); proposed
argument ordinals must index into `predicate.actual_args`, else reject
as malformed; candidates referencing args/params absent from the
predicate are rejected. §4.3 owns the contract; §9 owns the tests.

R9. Provider unavailability (absorbed from review). `claude` / `gemini`
CLI missing or not logged in must degrade gracefully on the end-user
`scan` verb — Layer-1 findings still emit, warning logged, exit 0
(§4.2). A missing optional provider must never hard-fail a scan.

R10. Prompt injection via source identifiers (absorbed from review).
`ArgFact.ident` / `ParamFact.{name, default}` are attacker-controlled.
They are serialised as escaped, explicitly-framed untrusted data, never
as prompt instructions (§5). A §9 fixture with an injection-shaped
identifier asserts the serialisation escapes it (the serialisation is
gated; the LLM verdict is not, given non-determinism).

## 8. Out of scope (v0)

- Layer 0 for any detector other than arg-swap (R6).
- Closing Bound A (cross-file / cross-module resolution) — Layer 0
  targets Bound B only; unresolved callees are skipped (§5).
- HTTP / `reqwest` dispatch for Layer 0 — CLI shellout only (§3.1).
- Default-on Layer 0 — opt-in only (§3.2).
- Layer 0 inside `calibrate` / `eval` — `scan --candidate-llm` only.
  This is a deliberate, load-bearing invariant (absorbed: review m1).
  `calibrate` and `eval` NEVER invoke an LLM: `calibrate` fits the
  Layer-0 prior (Phase B) from a PRE-labelled corpus, exactly as it
  fits every existing prior — no LLM call (this resolves the apparent
  §6 P4 ↔ §8 tension: "fitted by `cntrdct calibrate`" means fitted from
  pre-labelled data, not by calling a model). Keeping Layer 0 out of
  `eval` keeps `actual_total` and the R-1.f self-replication ledger /
  `eval --against` delta machinery deterministic; wiring Layer 0 into
  `eval` would silently break the ledger's drift signal, so it is
  forbidden.
- Continuous / nightly Layer 0 audit cadence — same reasoning as
  `cross-model-kappa-v0.md` "Design rationale" point 3 (model drift
  swamps the signal).
- The R-4-adjacent "Layer 3 ML-detector ensemble" (PyBugLab /
  GraphCodeBERT alongside the LLM judge) — REBUILD.md §6 subsumes it
  under this amendment but it is a separate, later spec.

## 9. Test plan sketch (implementation-time)

Each item is tagged [GATE] (hard CI gate) or [MEASURE] (recorded, not
gated — LLM non-determinism, R1). Modelled on ir-v0.md §F6's T1–T7
taxonomy; the [MEASURE] recall item is the T6 analogue, and the
error-path / parity / context-recording items fill the T2 / T1 / T7
analogues review-completeness flagged as missing.

- [GATE] Flag-off byte-identity (T1 analogue, absorbed: review B2). With
  Layer 0 OFF, `Finding` serialization is byte-identical to the
  pre-amendment output across all detectors — the existing T1 ir-pinning
  goldens and Q-15 baseline fixtures pass UNCHANGED (proves the
  `skip_serializing_if` on `origin` works). This is the load-bearing
  guard for the "byte-identical default" claim.
- [GATE] Default-path construction probe (absorbed: review M2). A
  network-independent test asserts the default (flag-off) `scan` path
  constructs NO Layer 0 provider — the authoritative accidental-
  activation guard the netns gate cannot provide.
- [GATE] Netns defence-in-depth. Unflagged `scan` in `unshare --net`
  emits zero `Layer0Llm` findings and opens no socket (extends the
  existing `network-isolation` job).
- [GATE] reqwest-reachability grep. The `fmt`-job grep guard (§3.1)
  asserts `candidate_llm.rs` references neither `build_default_adjudicator`
  nor `ReqwestClient`.
- [GATE] Predicate extraction goldens. Fixtures mapping the raw-tree
  walk → `CallSitePredicate` for the `totalsegmentator_statistics.py:10`
  Bound B case (asserting the comprehension-nested call IS enumerated,
  per B1) and ≥ one resolved-clean negative, asserted deterministically
  (no LLM). Includes a fixture carrying a default-value literal (M6) and
  an injection-shaped identifier asserting the serialisation escapes it
  (R10).
- [GATE] Dispatch + hand-off. Deterministic `PromptDispatch` mock (the
  `tests/cross_model_kappa.rs` `CannedDispatch` pattern) returning a
  fixed candidate verdict; assert candidate `Finding{origin: Layer0Llm}`
  shape, Layer 2 ranking (prior-miss fallback, since v0 ships no Layer-0
  prior), and Layer 3 hand-off (`build_prompt` is origin-agnostic).
- [GATE] Response validation (T2 analogue, absorbed: review B7/R8). Mock
  returns: malformed JSON → candidate dropped, scan continues, exit 0;
  out-of-range argument ordinal → rejected as malformed; refusal/empty →
  dropped. No panic, no non-zero exit in any case.
- [GATE] Cost cap (absorbed: review B6/R7). With `--candidate-llm-max-calls
  N` and a mock counting dispatches against a fixture with > N Bound B
  sites, assert exactly N dispatches and a logged skipped-count.
- [GATE] Provider-unavailable degradation (absorbed: review R9). With a
  provider stub reporting "unavailable", `scan --candidate-llm
  --adjudicate` emits Layer-1 findings, logs a warning, exits 0.
- [GATE] Flag contract. `--candidate-llm` without `--adjudicate` is a
  clap usage error; a non-adjudicated Layer 0 candidate is suppressed
  from SARIF (§3.3).
- [MEASURE] Recall (T6 analogue). With the flag on against a mock that
  confirms the Bound B swap, `arg-swap` recall on the audit corpus rises
  from 0.25 toward the Bound A ceiling (the remaining FNs stay
  unresolved — Bound A, not Layer 0's target). Reported with the
  (CLI version, model id) tuple (R1); not gated.

## 10. Reading list

- This file.
- `arg-swap-v0.md` §"Known recall upper bounds" (Bound A / Bound B) —
  the motivation and the precise FN this closes.
- `cross-model-kappa-v0.md` §"Design rationale" + F1/F2/F3 — the
  CLI-shellout + `PromptDispatch` precedent reused wholesale.
- `adjudicator-v0.md` — Layer 3 contract Layer 0 hands off to.
- `ir-v0.md` §F1 — the `IrCallSite` / `IrPath` / `IrFn` / `IrParam`
  fields predicates are built from.
- `llm-calibration-v0.md` — Q-12 Platt registry reused for Layer 0
  candidate confidence.
- `recall-audit-v0.md` — the labeller-bias loop the Layer 0 P4 corpus
  must avoid.
- CLAUDE.md "Design constraints (P1, P3 - P5)" + REBUILD.md §4 R-4 /
  Glossary "Layer 0".

## 11. Approval criteria (gate for implementation)

This spec is approved for implementation when the implementation-approval
reviewer confirms the following. Mirrors R-0/ir-v0.md settling its
blockers before R-1 began.

Pre-build-blocking (must be resolved IN THIS SPEC before any code):

- B1 — call-site enumeration sources from the raw-tree Pattern-B walk,
  not `IrCallSite` (§5). RESOLVED in this revision.
- B2 — `origin` carries `skip_serializing_if`; flag-off byte-identity is
  a §9 gate (§4.3, §9). RESOLVED.
- B3 — Layer-0 prior keyed on `(detector_id, origin)`; v0 ships an empty
  prior + no-op fallback; corpus deferred to Phase B; OSV claim dropped
  (§6 P4). RESOLVED, subject to the reviewer accepting the two settled
  forks (prior keying; empty-prior-now).
- B4 — P1 enforcement via a static citations table + consistency test,
  not `register_detector` (§6 P1). RESOLVED.
- B5 — `--candidate-llm` requires `--adjudicate`; Layer 0 candidates are
  always adjudicated or suppressed (§3.3, §4.2). RESOLVED.
- B6 — hard `--candidate-llm-max-calls` cap with truncation-log (§4.2,
  R7). RESOLVED.
- B7 — response validation contract (§4.3, R8, §9). RESOLVED.
- B8 — these approval criteria exist (this section). RESOLVED.

Two settled forks the approval reviewer should explicitly accept or
redirect (both keep v0 shippable either way):

- Fork 1: Layer-0 prior keyed on `(detector_id, origin)` vs a distinct
  `arg-swap-llm` detector_id. Chosen `(detector_id, origin)` because a
  distinct id collides with the recall-audit attribution and the
  `ALL_DETECTOR_IDS`/SARIF-rules invariant (§6 P3/P4).
- Fork 2: v0 ships an empty Layer-0 prior with a no-op fallback,
  deferring the labelled corpus to Phase B, vs building the corpus for
  v0. Chosen empty-now because a Bound-B-class labelled corpus is an
  unsolved problem (§6 P4 OPEN) and authoring numbers in code violates
  P4.

Implementation-time-deferrable (decided during the build, not blocking
approval): R5 (flag/provider naming ergonomics) and the *content* of the
Phase-B labelled corpus (R2). The self-preference policy (R3) is no
longer deferred — Phase B (2026-06-07) implemented the
different-model-family guard with a `--allow-self-preference` override
(see §7 R3).

## 12. Review log (R-4 review-before-build gate, 2026-06-07)

A 4-axis parallel review (one reviewer per axis, the R-0 precedent)
returned NEEDS-REVISION on three of four axes; this revision absorbs the
findings. Axes and headline findings:

- P3-integrity — BLOCKERS: `origin` breaks T1 (B2); P4 prior unrealizable
  via `origin` (B3); P1 enforcement misattributed (B4). MAJORS: reqwest
  reachability unenforced structurally; netns assertion blind to
  subprocess; `candidate-llm-corpus` vs single-priors pipeline; Q-4
  wiring. VERIFIED: netns command quote, reqwest-reachable set, IR
  predicate field grounding, P5 mapping, `apply_llm_calibration`
  determinism.
- architecture — BLOCKERS: predicate source blind to the comprehension-
  nested flagship call (B1); `origin` breaks T1 byte-identity (B2).
  MAJORS: Layer 0 ≠ `Detector` (B4); Layer-2 prior keying (B3);
  originate-then-adjudicate unenforced (B5); predicate drops default
  literals (M6). CONFIRMED: Bound B characterization correct (the
  load-bearing claim); adjudicator hand-off works on a Layer 0 finding;
  Layer 2 does not crash on arbitrary origin.
- spec-consistency — APPROVE-WITH-CHANGES: every design-bearing
  cross-reference verified TRUE against the actual specs/source; two
  MINOR locator drifts fixed (`ir-v0.md G4` → REBUILD.md G4;
  `cross-model-kappa-v0.md §3.2/§3.3` → "Design rationale" points 2/3).
- risk-completeness — BLOCKERS: no cost cap (B6); no response-validation
  path (B7); P4 corpus rests on unvalidated PyPIBugs/OSV (B3); no gate
  acceptance criteria (B8). MAJORS: `--adjudicate` interaction (B5);
  `origin` serialization (B2); provider-unavailable degradation (R9);
  CLI/model non-stationarity (R1); prompt injection (R10). MINORS:
  self-replication ledger invariant made explicit (§8); §6 P4 ↔ §8
  reconciled; §9 gate-vs-measurement labels added.

All BLOCKERS resolved in the body above; §11 records the acceptance
criteria. The CLI-shellout-over-`PromptDispatch` core design, the
reqwest/netns claims, and the Bound B motivation were all confirmed
sound — the revision tightens consequences (calibration keying,
Finding-shape, flag contract, cost/validation), it does not change the
architecture.
