# R-4 Layer 0 — end-to-end-recall measurement (2026-06)

Records the [MEASURE] end-to-end recall item from `REBUILD.md` R-4 and
`docs/spec/p3-amendment-v0.md` §9 — the figure deferred by
`r4-layer0-generation-recall-2026-06.md`. Generation recall (the Layer 0
proposer flagging the flagship Bound-B swap) was already 3/3; the open
question was whether a generated candidate SURVIVES Layer 3 adjudication
into the output, i.e. is caught **end-to-end**.

The enabling change (task 1): the CLI adjudicators
(`ClaudeCliAdjudicator`, `AgyCliAdjudicator`) now implement the
`Adjudicator` trait, and `scan --adjudicate --adjudicate-via=claude-cli|agy-cli`
runs Layer 3 over CLI subscription auth — no `ANTHROPIC_API_KEY`. This is
what unblocks the measurement without an API key.

Per R1 (LLM non-determinism) this is a recorded snapshot, not a gate.

## Target

`benchmarks/audit-corpus/files/totalsegmentator_statistics.py:10` — the
Bound-B semantic swap `get_radiomics_features(ct_file, mask)` against
`def get_radiomics_features(seg_file, img_file="ct.nii.gz")`. Layer 1
arg-swap cannot fire (no lexical name correlation in either direction —
arg-swap `recall_upper_bound` 0.25, Bound B). Layer 0 proposes it; Layer 3
must confirm it for end-to-end recall to register.

## Run 1 — claude-cli propose + claude-cli adjudicate (self-preference)

Context tuple (R1):

| Field | Value |
| --- | --- |
| Date | 2026-06-14 |
| Proposer | `--candidate-llm=claude-cli`, `claude` 2.1.177 (Claude Code), model `claude-sonnet-4-6` |
| Adjudicator | `--adjudicate --adjudicate-via=claude-cli`, same CLI/model |
| Self-preference | proposer + confirmer are both anthropic family → biased per `wataoka-2024`; run with `--allow-self-preference` |
| cntrdct binary | `target/debug/cntrdct`, task-1/task-2 working tree |

Command:

```sh
cntrdct scan benchmarks/audit-corpus/files/totalsegmentator_statistics.py \
  --candidate-llm=claude-cli --adjudicate --adjudicate-via=claude-cli \
  --allow-self-preference --format json
```

Result: **end-to-end CATCH.**

- Layer 0: `1 dispatched, 1 candidate, 0 over cap, 0 dropped`.
- Layer 3 verdict: `LikelyTruePositive`, confidence `0.88`
  ("ct_file (a CT scan) is passed to seg_file (expects a segmentation),
  while mask (a segmentation) is passed to img_file (expects an image);
  the default value "ct.nii.gz" for img_file confirms what it expects").
- The `Origin::Layer0Llm` arg-swap finding survives the B5 suppression
  filter (it was adjudicated) and is emitted at `:10` in the output.

So with an adjudicator wired, the flagship Bound-B FN that drags arg-swap
audit recall to 0.25 IS caught end-to-end. This is the reliable headline:
it confirms the task-1 CLI-adjudicator wiring works on subscription auth.

Caveat: the verdict is self-preference-biased (claude judging a claude
proposal). It demonstrates the wiring and that the candidate is judged a
true positive, but is not the unbiased number — see Run 2.

## Run 2 — claude-cli propose + agy(Gemini) adjudicate (cross-family, unbiased) — CATCH

The methodologically clean pairing routes the claude-cli proposal to a
NON-Anthropic confirmer so there is no self-preference: `agy -p`
(Antigravity) forced to a Gemini model. The self-preference guard allows
this pairing (`google` ≠ `anthropic`); no `--allow-self-preference`.

Context tuple (R1):

| Field | Value |
| --- | --- |
| Date | 2026-06-14 |
| Proposer | `--candidate-llm=claude-cli`, `claude` 2.1.177, model `claude-sonnet-4-6` |
| Adjudicator | `--adjudicate --adjudicate-via=agy-cli`, `agy` 1.0.8 (Antigravity), model `Gemini 3.5 Flash (Low)` |
| Self-preference | proposer anthropic, confirmer google → NO conflict (unbiased) |

```sh
AGY_CLI_MODEL_OVERRIDE="Gemini 3.5 Flash (Low)" \
cntrdct scan <flagship> --candidate-llm=claude-cli --adjudicate --adjudicate-via=agy-cli --format json
```

Result: **end-to-end CATCH (unbiased).**

- Layer 0 (claude-cli): `1 dispatched, 1 candidate`.
- Layer 3 (agy / Gemini 3.5 Flash Low): verdict `LikelyTruePositive`,
  confidence `0.95` ("The image file ct_file is passed to seg_file and the
  mask is passed to img_file, which swaps the image and segmentation
  parameters."). A DIFFERENT model family confirms the swap, so this is the
  self-preference-free number.
- The `Origin::Layer0Llm` arg-swap finding is emitted at `:10`.

### The agy integration bug found and fixed en route

Getting here required fixing a real cntrdct bug, not just an account
constraint. Initial cross-family runs returned empty / non-JSON
(`inner json parse error`) and the candidate was (correctly) suppressed.
Root cause, isolated by probing `agy` directly: `agy`'s `--print` / `-p`
flag takes the prompt as its VALUE (it is NOT a boolean). The original
`AgyCliAdjudicator` invoked `agy --print --model <m> <prompt>`, so
`--print` swallowed the literal `"--model"` as the prompt and the real
prompt was dropped as a stray positional — agy then replied chattily
("you passed `--model` as a command…") or emptily. Fixes landed:

1. Arg order corrected to `agy --model <m> --print <prompt>` (prompt is
   `--print`'s value; pinned by a regression assertion in
   `adjudicator.rs`'s agy dispatch test).
2. agy gets a forceful closed-book system prompt ([`AGY_SYSTEM_PROMPT`],
   folded into the body since `agy` has no `--system-prompt` flag) and a
   COMPACT, single-line, plain-text-evidence prompt
   ([`build_compact_prompt`] / `render_evidence_plain`) — the verbose
   `build_prompt` template (labelled fields + nested `EVIDENCE_RAW` JSON)
   trips agy's agentic persona. The proposer's own `llm_rationale` /
   `llm_confidence` are dropped from the agy prompt so the cross-family
   judge re-decides from facts (avoids a mini self-preference).

Separate, residual constraint (account, not code): free / not-fully-logged-in
Antigravity (`agy` cli.log: `not logged into Antigravity`) is
aggressively rate-limited, so bursts of calls still hang/throttle. Space
the calls (or use a logged-in / paid tier) for repeated runs; the single
measured run above succeeded with `Gemini 3.5 Flash (Low)`.

## Ledger summary

| Pairing | Self-preference | Outcome |
| --- | --- | --- |
| claude-cli propose + claude-cli adjudicate | yes (biased) | end-to-end CATCH, verdict LikelyTruePositive **0.88** |
| claude-cli propose + agy/Gemini 3.5 Flash (Low) adjudicate | no (unbiased) | end-to-end CATCH, verdict LikelyTruePositive **0.95** |

Bottom line: BOTH pairings catch the flagship Bound-B FN end-to-end. The
unbiased cross-family confirmation (claude proposes, Gemini disposes —
conf 0.95) is the headline: wiring the CLI adjudicator into
`scan --adjudicate` (task 1) plus adopting `agy` (task 2) lets the
arg-swap audit-recall lift (0.25 → Bound-A ceiling) be measured on
subscription auth with no self-preference bias and no `ANTHROPIC_API_KEY`.
