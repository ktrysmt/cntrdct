# Citation policy (P1)

Every detector ships with at least one peer-reviewed citation. The
constraint — known internally as P1 — is the most stringent
guarantee cntrdct makes, and it is enforced structurally rather than
by convention.

## The rule

- Every Layer 1 detector must declare at least one citation. The
  citation must point at a peer-reviewed publication or established
  benchmark that justifies the detection.
- Multi-language detectors should ship at least one citation
  grounded in empirical work on each supported language. This is a
  SHOULD, not a MUST — see [Multi-language](#multi-language-extension)
  below for the boundary cases.
- Citations have DOIs where possible. The Q-6 retraction monitor
  uses the DOI to cross-check against Retraction Watch and Crossref
  Works.

## Enforcement points

| Point | Check |
|---|---|
| `core::register_detector` | Rejects any `Detector` whose `citations()` returns empty at startup. |
| `tests/citations_consistency.rs` | Every citation key declared by any detector resolves to an entry in [`CITATIONS.md`](https://github.com/ktrysmt/cntrdct/blob/master/CITATIONS.md). |
| `tests/citations_consistency.rs` | Each `supported_languages()` is either matched by a per-language citation or carries an explicit `unconfirmed:` annotation pointing at the survey notes. |
| `.github/workflows/citations.yml` | Q-6 retraction monitor (see below). |

A detector with empty `citations()` cannot register, so a P1
violation fails before scan even begins. The consistency test runs
on every push and PR.

## Multi-language extension

P1 was authored when cntrdct was Rust-only. The M-6 spec
([`docs/spec/citations-policy.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/citations-policy.md))
codifies the multi-language case. When a detector declares support
for a new language, the implementer runs a literature survey and
records its outcome as one of:

- `Confirmed` — a peer-reviewed citation grounded in empirical work
  on the target language exists, either because the cited paper's
  experimental subjects include that language, because a separate
  peer-reviewed paper applies the algorithm to that language with
  quantitative evaluation, or because the citation introduces a
  benchmark in that language.
- `Unconfirmed` — the survey returned no qualifying citation. The
  detector still ships; the gap is captured in
  `Evidence.language_citation_status` and exposed at SARIF emission
  time as `properties.languageCitationStatus`. SARIF consumers can
  filter or weight indirectly-grounded findings.

When the survey is `Unconfirmed`, the implementer must still record
the search under `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` — the
deliverable is the survey itself, not the citation. The existing
Rust citations on the v0 detectors are grandfathered, since they
predate the multi-language rule.

The current per-language statuses (excerpted from
[`docs/spec/citations-policy.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/citations-policy.md)):

| Detector | Language | Status |
|---|---|---|
| `unreachable-after-terminator` | Rust | Grandfathered |
| `unreachable-after-terminator` | Python | Unconfirmed |
| `comment-code` | Rust | Grandfathered |
| `comment-code` | Python | Unconfirmed |
| `arg-swap` | Rust | Grandfathered |
| `arg-swap` | Python | Confirmed (Allamanis et al. NeurIPS 2021, PyBugLab) |
| `clone-drift` | Rust | Grandfathered |
| `clone-drift` | Python | Confirmed (Assi et al. TOSEM 2025) |
| `pr-miner` | Rust | Grandfathered |
| `pr-miner` | Python | Unconfirmed |

## CITATIONS.md layout

[`CITATIONS.md`](https://github.com/ktrysmt/cntrdct/blob/master/CITATIONS.md)
is grouped by layer:

- Layer 1 — per-detector citations.
- Layer 2 — ranker statistical methods (Wilson interval,
  Brown-Cai-DasGupta 2001, Thulin 2014).
- Layer 3 — LLM adjudicator references (Platt 1999, Spiess et al.
  2025, the cross-model κ references).

Each entry carries the venue, year, DOI when available, and a
`Languages:` line declaring which languages the citation is
grounded in (M-6 extension).

## Q-6 retraction monitor

P1's "peer-reviewed prior art" assumption breaks if a cited paper is
retracted after shipping. The Q-6 monitor
(`scripts/check_retractions.py`) closes this loop on every CI run:

- DOIs are extracted from `CITATIONS.md` and from every
  `Citation { doi: Some("...") }` slot under `src/`.
- Each DOI is checked against (a) a cached Retraction Watch snapshot
  at `benchmarks/retraction-watch/cache.csv`, pinned by
  `cache.sha256`, and (b) Crossref Works' `update-to` field with
  `type: "retraction"`.
- A Mondays-06:00-UTC cron refreshes the cache via the Crossref Labs
  endpoint and opens a `chore(citations): refresh Retraction Watch
  cache` PR when the snapshot changes.
- A synthetic-DOI fixture at
  `tests/fixtures/retraction-watch/` pins the failure path so a
  future loosening of the matcher breaks CI rather than silently
  re-opening citations to retracted work.

See also:

- [`docs/spec/citations-policy.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/citations-policy.md)
  — the full multi-language citation rule.
- [`CITATIONS.md`](https://github.com/ktrysmt/cntrdct/blob/master/CITATIONS.md)
  — bibliography.
- [`docs/surveys/`](https://github.com/ktrysmt/cntrdct/tree/master/docs/surveys)
  — recorded literature surveys for `Unconfirmed` languages.
