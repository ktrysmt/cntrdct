# cntrdct strategic plan

Last updated: 2026-05-18 (v0.3.0 stable shipped, closing the v0.2.0-beta/rc
release cycle on the technical side; research-side USR-1..3 gates remain
open. β paper structure and the strategic framing below stay current.)

This document is the north-star for cntrdct's post-β direction. It records
decisions made during the β-completion session and the three independent
work tracks that follow. Each track has its own subdirectory under
`projects/` with a self-contained README intended for resumption in a
fresh session.

## Status of the codebase

- Five Layer 1 detectors registered: `clone-drift`, `arg-swap`,
  `comment-code`, `unreachable-after-terminator`, `config-interaction`.
- Layer 2 (uncalibrated + calibrated rankers), Layer 3 (Anthropic
  adjudicator), Layer 4 (SARIF emitter) all wired end-to-end.
- `cntrdct eval` reports per-detector precision/recall/F1 against a
  manifest-labelled corpus. `cntrdct calibrate` builds priors from a
  labelled JSONL corpus.
- 58-file synthetic seed corpus under `benchmarks/corpus/`. Per-detector
  precision = recall = F1 = 1.0 (corpus-induced upper bound; the corpus
  is constructed to match detector output, not collected externally).
- Internal preregistration draft at `prereg/2026-05-03-osf-prereg.md`,
  enforced by `crates/cli/tests/{prereg,corpus_shape,citations}_*.rs`.

## Decisions

### D1 — Drop OSF preregistration submission

The internal prereg stays as a repository governance document but is NOT
submitted to OSF. Rationale: the detectors are reimplementations of
published patterns (no novelty), the corpus is synthetic (P=R=1 is a
sanity check, not a finding), no baseline comparison, sample size two
orders of magnitude below ICSE/FSE-class empirical studies. Submitting a
preregistration without a study endpoint is performative.

The prereg file remains in tree because:

- It pins the design constraints (P1-P5) for current and future
  detectors.
- The structural tests guard against silent drift between prereg and
  code.
- Future research (tracks A or B below) can lift this document into a
  real OSF submission without rewriting.

### D2 — Two parallel work tracks: practical and academic

Practical track (the primary effort):

1. β corpus collection from real-world Rust crates (regression-test
   purpose, not academic).
2. Implement pr-miner full version (statistical implicit-rule mining)
   as a sixth Layer 1 detector.
3. SARIF output validation against the OASIS official schema via
   `sarif-multitool` in CI.
4. Layer 2 ranker recalibration once the β corpus exists.
5. β release tagging and `crates.io` publish.

Academic track (optional, deferred, parallelisable):

- Track A — empirical study on `crates.io` top-N (highest ceiling).
- Track B — Rice 2017 replication in Rust (focused, lower effort).
- Track C — position paper on evidence-based linter design pattern
  (essay-class, lowest effort, highest leverage as a starter).

Both tracks share the same codebase. Academic work does not block the
practical track.

### D3 — OSS readiness as a first-class concern

Post-β, the project ships as an OSS Rust tool. The implementation
roadmap for OSS readiness (CI, releases, documentation, contribution
flow) is treated as an engineering deliverable equal in importance to
detector additions. See `OSS_ROADMAP` section below.

## Strategic recommendation

Order of execution, optimised for total leverage and lowest risk:

1. Complete the practical track (corpus, pr-miner, SARIF, calibration,
   release). 4-8 weeks part-time. Produces a shippable, useful OSS tool.
2. Write track C as a blog post. 1-2 weekends. Tests appetite for the
   methodological argument and produces a reference URL.
3. If C lands well, run a track A pilot (top 100 crates only). 1-2
   weekends. Pilot result decides whether to scale to top 1000 and
   write a paper.
4. Track B is a fallback if A's pilot underperforms — replication is
   cheaper to write up and venues for replications are receptive even
   when novelty is constrained.

## OSS readiness roadmap

Moved to `ROADMAP.md` at the repository root. That file is the
canonical, line-itemised view of the engineering work, including
acceptance criteria, effort estimates, and dependency ordering. Update
it (not this file) when individual items are picked up or completed.

The shape, in summary:

- Tier 1 — usable OSS, blocking for any external announcement.
- Tier 2 — adoption-grade, drives external usage.
- Tier 3 — polish, post-launch.
- Tier 4 — community, opens contribution funnel.

ROADMAP.md also contains the "Practical track" items (β corpus,
pr-miner-rust, SARIF validator, ranker recalibration, the v0.2.0-beta /
v0.2.0-rc / v0.3.0 release sequence — all closed on the technical side)
that originally lived under D2 of this plan. They sit in ROADMAP.md
because they are engineering deliverables; D2 here records the
strategic decision to pursue them, not their implementation details.

## β paper structure (2026-05-06 update)

The "academic track" framing in D2 has resolved into an active β
paper effort with three contributions, each grounded in one of the
research tracks:

1. Empirical study (Track A): per-detector precision and FP
   failure-mode distribution on a labelled sample drawn from the
   top 1000 crates by lifetime download. Output: §6/§7 of the β
   paper, plus a recalibrated `priors.json` shipped with cntrdct
   v0.x.0. Owner: `research/projects/A_1000_crate/`.
2. Replication study (Track B): Rust-side replication of Rice
   et al. (ICSE 2017) on arg-swap defect density and post-
   type-check survival. Independent empirical contribution that
   tests whether Rice's C++/Java numbers generalise to Rust.
   Output: Track B chapter of the β paper. Owner:
   `research/projects/B_rice_replication/`.
3. Position essay (Track C): argument that peer-reviewed citation,
   treated as a typed and enforced component of every detector,
   produces useful design constraints across an analyser. Target
   venue: SPLASH Onward! Essays. Stage 2 essay drafted at
   `research/projects/C_position/stage2-essay-v0.md` (currently
   ~4075 words, inside the 4000-6000 target). Output: a separate
   essay-class submission, NOT a chapter of the β paper.

Tracks A and B feed the same paper as independent empirical
contributions (different corpora, different hypotheses). Track C
is a separate submission and can land independently.

The cautious sequencing in the Strategic recommendation section
above (run Track C as a blog, pilot Track A on top 100, then
maybe scale to 1000) has been superseded by the actual execution:
Track C is a peer-reviewed essay (not a blog), Track A targets
top 1000 directly (not a 100-crate pilot), and Track B is being
prepared synchronously rather than as a fallback. The supersession
is documented here, not in the recommendation block, because the
recommendation block records the prior thinking and is retained
as audit trail of how the strategy evolved.

## Gating user decisions (USR-1..3)

Three decisions that only the user can make. Each gates a track's
empirical execution; until resolved, the corresponding track
stays in DRAFT and no labelled data is produced. The agent
session can prepare commits and authoring around these gates but
cannot resolve the underlying decisions.

### USR-1 — rubric v1 §0 confirmation

File: `research/projects/A_1000_crate/rubric-v1-draft.md` §0.
Five sub-items: tiebreak protocol adoption, rater-2 (cntrdct
non-affiliated) recruitment, rater-3 (tiebreaker) selection rule,
date stamp, and the `build_phase1_csv.py` extension scope. Once
resolved, the rubric promotes to
`prereg/<DATE>-labelling-rubric-v1.md` per rubric-v1-draft §13.

Unblocks: Track A Phase 1 labelling (200-300 findings × 2
raters); Cohen's κ figure against the κ ≥ 0.6 acceptance
threshold; per-detector precision tables; recalibrated priors
that ship with cntrdct v0.x.0.

Bottleneck: rater-2 recruitment is the long-pole sub-item; the
remaining four sub-items are 30-minute decisions.

### USR-2 — failure-modes v1 promotion (synced with USR-1)

File: `research/projects/A_1000_crate/failure-modes-v1.md`.
Promotes alongside USR-1 to
`prereg/<DATE>-failure-modes-v1.md`. Codifies the controlled
vocabulary (5 detectors × 4-5 modes, plus the shared
`cross-file-context-resolved` mode) for FP failure-mode
classification.

Unblocks: the `failure_mode` column in Phase 1 CSV gets
populated; the aggregator at
`research/projects/A_1000_crate/scripts/phase1_failure_modes_aggregate.py`
runs against real data; the β paper §Threats / §Discussion gets
its FP-distribution table.

Methodological value: a pre-registered FP taxonomy resists
post-hoc cherry-picking critiques. Without USR-2 the FP
categories in the β paper are vulnerable to the "you chose
categories to match the result" attack.

### USR-3 — Rice 2017 paper read + replication-spec v1

Files: `research/projects/B_rice_replication/replication-spec-v0.md`
plus 12 `[verify Rice §X]` placeholders to resolve against the
actual paper. Once resolved, an agent session can author
`replication-spec-v1.md` from the user's verification notes and
then promote it.

Unblocks: Track B as a whole (corpus assembly, RICE_TRACE
implementation, candidate aggregation, paper write-up); the
v1.1 rust-analyzer addendum at
`research/projects/B_rice_replication/replication-spec-v1.1-rust-analyzer.md`
is currently composed-with v0 but switches to compose-with v1
the moment v1 lands.

Cost: ~3-5 hours of focused paper reading by someone with paper
access. The agent session cannot perform this read.

Sequencing: USR-3 is parallelisable with USR-1; the two tracks
are independent. USR-2 is downstream of USR-1.

## Cross-references

- `research/projects/A_1000_crate/README.md` — empirical study track.
- `research/projects/B_rice_replication/README.md` — Rice 2017 replication
  track.
- `research/projects/C_position/README.md` — position paper track.
- `prereg/2026-05-03-osf-prereg.md` — internal governance prereg.
- `docs/spec/` — per-detector specs (canonical TDD entry points).
- `CITATIONS.md` — Layer 1-4 prior-art bibliography.
