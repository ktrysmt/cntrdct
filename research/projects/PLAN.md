# cntrdct strategic plan

Last updated: 2026-05-03 (β commit `99f14a3`)

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
pr-miner-rust, SARIF validator, ranker recalibration, v0.2.0-beta
release) that originally lived under D2 of this plan. They sit in
ROADMAP.md because they are engineering deliverables; D2 here records
the strategic decision to pursue them, not their implementation
details.

## Cross-references

- `research/projects/A_1000_crate/README.md` — empirical study track.
- `research/projects/B_rice_replication/README.md` — Rice 2017 replication
  track.
- `research/projects/C_position/README.md` — position paper track.
- `prereg/2026-05-03-osf-prereg.md` — internal governance prereg.
- `docs/spec/` — per-detector specs (canonical TDD entry points).
- `CITATIONS.md` — Layer 1-4 prior-art bibliography.
