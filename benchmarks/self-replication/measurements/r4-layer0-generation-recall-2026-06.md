# R-4 Layer 0 — generation-recall measurement (2026-06)

Records the [MEASURE] recall item from `docs/spec/p3-amendment-v0.md` §9
(the T6 analogue, intentionally not gated per R1 LLM non-determinism).
This run measures **generation recall** only — whether the Layer 0
proposer flags the flagship Bound-B swap. End-to-end recall (candidates
surviving Layer 3 adjudication into the output) is NOT measured here: it
requires an adjudicator (`ANTHROPIC_API_KEY` for the default Anthropic
HTTP adjudicator, or a small change wiring the `claude-cli` adjudicator
into `scan --adjudicate`), and is left deferred.

## Context tuple (R1)

Per `p3-amendment-v0.md` §7 R1, an LLM recall figure is only meaningful
with the (CLI version, model id) it was measured against. A silent CLI
or model bump invalidates the number.

| Field | Value |
| --- | --- |
| Date | 2026-06-14 |
| Proposer CLI | `claude` 2.1.177 (Claude Code) |
| Proposer model | `claude-sonnet-4-6` (`adjudicator::CLAUDE_CLI_MODEL` default; no model-override knob exists for the `claude-cli` provider) |
| Provider | `--candidate-llm=claude-cli` |
| Adjudicator | none (`ANTHROPIC_API_KEY` unset → adjudication skipped, candidates suppressed from output by B5) |
| cntrdct binary | `target/release/cntrdct`, built 2026-06-08 (post R-4 / Phase-B landing) |

## Target

`benchmarks/audit-corpus/files/totalsegmentator_statistics.py:10` —
the Bound-B semantic swap `get_radiomics_features(ct_file, mask)` against
the same-file definition `get_radiomics_features(seg_file, img_file=...)`.
The call sits inside a list comprehension, so it also exercises the B1
catch (comprehension-nested calls must be enumerated by the raw-tree
walk, not the structured `IrCallSite` view). See `arg-swap-v0.md`
"Bound B — name-correlation ceiling (F5)".

## Reproduction

```sh
# Layer 1 baseline (no Layer 0): the swap is a false negative.
./target/release/cntrdct scan \
  benchmarks/audit-corpus/files/totalsegmentator_statistics.py
# -> [] (0 findings)

# Layer 0 generation recall (proposer only; adjudication skipped, no key).
./target/release/cntrdct scan \
  --candidate-llm=claude-cli --adjudicate --allow-self-preference \
  --candidate-llm-max-calls 4 \
  benchmarks/audit-corpus/files/totalsegmentator_statistics.py
# stderr -> "Layer 0 candidate generator (claude-cli): 1 dispatched,
#            1 candidate(s), 0 skipped over cap, 0 dropped"
# stdout -> [] (the 1 candidate is suppressed: unadjudicated, B5)
```

`--allow-self-preference` is required because the proposer
(`claude-cli`, Anthropic family) and the default Layer 3 adjudicator
(Anthropic) share a model family (R3 guard). Self-preference does not
affect generation recall — generation is a property of the proposer
alone — so this number is clean; it would only bias end-to-end survival.

## Result

| Metric | Value |
| --- | --- |
| Layer 1 baseline findings on target | 0 (Bound-B FN, as specified) |
| Bound-B residue sites enumerated (pre-filter) | 1 (the comprehension-nested call; B1 confirmed) |
| Proposer candidates generated | 3/3 runs: 1 dispatched, 1 candidate, 0 over cap, 0 dropped |
| Generation recall on flagship | HIT (consistent across 3 runs) |
| Proposer LLM calls | 3 total (1 per run; subscription Pool-2, negligible) |

The Layer 0 proposer reliably (3/3) flags the morphology-blind semantic
swap that Layer 1's deterministic F5 structurally cannot catch. This is
the real-model proof that R-4's motivation (the arg-swap Bound B ceiling)
is addressable by a Layer 0 LLM candidate generator.

## Determinism note (R1)

Three runs, all 1/1. A single sample is weak evidence under LLM
non-determinism (sampler stochasticity + silent CLI/model drift); 3/3 is
a small-but-consistent point estimate, not a stable guarantee. Any future
re-measurement MUST re-record the (CLI version, model id) tuple — a
silent `claude` or model bump invalidates this figure.

## Still deferred

- End-to-end recall (the §9 [MEASURE] "0.25 → toward Bound A ceiling"
  figure): needs an adjudicator. Either set `ANTHROPIC_API_KEY` (Anthropic
  HTTP adjudicator, pay-as-you-go API billing, not subscription) or wire
  the `claude-cli` adjudicator into `scan --adjudicate` (subscription
  Pool-2, but self-preference on both proposer and confirmer).
- Phase-B labelled Layer-0 corpus + fitted `(arg-swap, Layer0Llm)` prior
  (B3/R2) — the labeller-bias / external-anchor problem is unchanged.
