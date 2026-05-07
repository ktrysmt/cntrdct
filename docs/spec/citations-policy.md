# Citation policy v1 spec (P1 extension for multi-language)

Status: approved 2026-05-04. Owner of M-6.

## Background

P1 (the project's most stringent constraint) says every detector
must reference at least one peer-reviewed publication or established
benchmark that justifies its detection. It is enforced at two
points:

- `cntrdct_core::register_detector` rejects detectors with empty
  `citations()` at startup.
- `crates/cli/tests/citations_consistency.rs` asserts every key
  declared by any detector resolves to a `CITATIONS.md` entry.

P1 was authored when cntrdct was Rust-only. The implicit contract
was "the citation justifies the detector existing"; it did not
constrain the language under analysis. Now that the M-series is
adding Python (and later TS / Go / Java), the question becomes:

If a Rust detector that cites e.g. Cordy & Roy (ICPC 2008, NiCad
clone detector) extends to Python, does the same citation suffice?

This document says: best-effort yes. The detector still needs at
least one citation overall (P1 is unchanged), and the implementer
must perform a literature survey for the new language and record
its result. But finding a per-language citation is a SHOULD, not
a MUST — when the survey returns nothing publishable, the language
extension still ships, with the gap surfaced in metadata so SARIF
consumers know the grounding is indirect.

## Rule

P1 (unchanged): every detector must declare at least one citation
in `Citation::citations()`. `register_detector` continues to reject
empty citation sets.

P1 multi-language extension (new, SHOULD-level):

When a detector declares support for a `Language`, the implementer
should produce at least one citation grounded in empirical work on
that language. A citation is "grounded in" a language if any of the
following holds:

(a) The cited paper's experimental subjects include code in that
    language, or
(b) The cited paper's algorithm is presented as language-agnostic
    AND at least one independent peer-reviewed publication has
    applied the algorithm to the target language with quantitative
    evaluation on a corpus in that language. Both papers are cited.
    Preprints, blog posts, and the cntrdct project itself do not
    satisfy the secondary-application requirement, or
(c) The cited paper introduces a benchmark / dataset in that
    language.

A single citation can satisfy the requirement for multiple
languages if (a), (b), or (c) holds for each.

When the survey for a (detector, language) pair returns no
candidate that satisfies (a), (b), or (c), the implementer:

- Documents the survey effort in the PR description (databases
  searched, query terms, reasoning behind rejecting candidates).
- Sets the per-language citation status to `unconfirmed` (see
  Mechanical enforcement below).
- Ships the language extension regardless. The detector continues
  to satisfy P1 because its overall citation set is non-empty.

The existing Rust citations on the v0 detectors are grandfathered
under (b): the cross-cutting concept papers (Cordy-Roy NiCad, Li-Zhou
PR-Miner, Tan iComment, Engler bugs-as-deviant-behavior) all present
language-agnostic algorithms whose Rust application is documented
via the cntrdct project itself. We accept this grandfather clause
for the existing five detectors; new languages and new detectors
follow the strict (b) above.

## Mechanical enforcement

The `Citation` struct gains a `languages` field declaring which
languages the citation is grounded in:

```rust
pub struct Citation {
    pub key: &'static str,
    // ... existing fields ...
    pub languages: &'static [Language],
}
```

`languages: &[]` means the citation is general / methodological and
does not satisfy any per-language requirement on its own (Wilson
score lower bound papers, IEEE 1044-2009, etc. fall here).

`register_detector` keeps the P1 hard gate, does not add a
per-language hard gate:

```rust
pub fn register_detector(d: &dyn Detector) -> Result<(), DetectorError> {
    if d.citations().is_empty() {
        return Err(DetectorError::Config(format!(
            "detector {} has no citations (P1 violation)", d.id(),
        )));
    }
    // Per-language coverage is checked by the consistency test, not
    // by the registration call. Missing per-language citations are
    // reported at scan time via Finding.evidence metadata, not by
    // refusing to register the detector.
    Ok(())
}
```

`crates/cli/tests/citations_consistency.rs` extends to:

- Assert every detector's `supported_languages()` is documented in
  CITATIONS.md (each language has either a matching citation or an
  explicit `unconfirmed:` annotation pointing at the survey notes).
- Assert a deliberately under-cited fixture detector (a small struct
  in the test file claiming `Language::Python` with no
  Python-grounded citation and no `unconfirmed` annotation) is
  flagged as a warning. The test does not fail; it surfaces the
  status.
- Assert no two citations for the same detector share a `key`
  (catches retro-fit copy-paste mistakes).

## Finding metadata

`Finding.evidence` gains a per-language citation status field
populated by the detector at emission time:

```rust
pub struct Evidence {
    pub citation_keys: Vec<&'static str>,
    pub raw: serde_json::Value,
    pub language_citation_status: LanguageCitationStatus,
}

pub enum LanguageCitationStatus {
    /// At least one citation in `citation_keys` is grounded in the
    /// finding's source language per (a), (b), or (c).
    Confirmed,
    /// All cited works ground a different language; the survey
    /// returned no per-language match. The detector still applies
    /// because the underlying concept transfers, but the grounding
    /// is indirect.
    Unconfirmed,
}
```

The SARIF emitter copies this into `properties.languageCitationStatus`
on each result, so SARIF consumers can filter or visually flag
indirectly-grounded findings.

## Bibliography management

`CITATIONS.md` entries gain a `Languages:` line:

```
- Cordy, J. R. & Roy, C. K. (2008). The NiCad Clone Detector. ICPC.
  Languages: Java, C
- Selim, G. M. K., Foo, K. C., Zou, Y. (2010). Enhancing source-based
  clone detection using intermediate representation. WCRE.
  Languages: Java
- (clone-drift Python coverage: unconfirmed; survey notes at
  docs/surveys/clone-drift-python-2026-05.md)
```

The third line is the explicit-no-citation form: it points readers
at the survey that documented why no qualifying citation was found.
The consistency test parses both kinds.

## Survey requirement per new language

When adding a new language to a detector (M-3 et seq.), the
implementer must:

1. Run a literature search for at least one paper that satisfies
   (a), (b), or (c) above for the target language. Reasonable
   sources: Google Scholar, ACM DL, IEEE Xplore, dblp, the
   detector's existing citation cluster's "cited by" graph.
2. If a qualifying citation is found:
   - Add the bibliography entry to `CITATIONS.md` with a
     `Languages:` line.
   - Update the detector's `Citation` static array, setting
     `languages` correctly.
3. If no qualifying citation is found:
   - Write `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` recording
     the search (databases, queries, candidate papers and reasons
     for rejection). Cite this file in `CITATIONS.md` per the
     example above.
   - Set the detector's per-emission `LanguageCitationStatus` for
     this language to `Unconfirmed`.
4. Justify the choice in the PR description: which papers were
   considered, why the chosen one suffices (or why none did).

For the M-series specifically, the survey is sized at roughly 1-3
days of literature work per (detector, language) pair. The Phase D
schedule budgets this implicitly inside each M-3 detector.

## Worked example

Adding Python support to `clone-drift`:

- Existing Rust citations: Cordy-Roy (NiCad, ICPC 2008),
  Bettenburg et al. (MSR 2009), Krinke (ICSM 2007). All on
  Java / C / C++.
- For Python, run the survey. Suppose it surfaces Selim et al.
  (2010, WCRE) which extends NiCad-style clone detection to Java
  with a quantitative evaluation, AND a separate paper applies
  NiCad to Python with measured precision/recall on a Python
  corpus. Citation (b) is satisfied for Python.
- The new Python paper joins the static array with
  `languages: &[Language::Python]`. The Rust entries get their
  `languages` updated to reflect the languages they originally
  studied (e.g. Cordy-Roy → `&[Language::Java, Language::C]`).
- `clone-drift::supported_languages()` returns
  `&[Language::Rust, Language::Python]`; the Python finding
  emits `language_citation_status: Confirmed`.

If the survey instead finds no qualifying Python application of
NiCad-style clone detection, the language extension still ships:

- The detector's array gains no Python citation.
- A `docs/surveys/clone-drift-python-2026-05.md` file records the
  search.
- `CITATIONS.md` adds the explicit-no-citation line pointing at the
  survey.
- Each Python finding emits
  `language_citation_status: Unconfirmed`. SARIF consumers see this
  in `properties.languageCitationStatus` and can choose to weight
  unconfirmed findings lower or surface them as exploratory.

## Why best-effort and not strict

A strict per-language citation MUST has two failure modes that hurt
us more than they help:

- It blocks language extensions for detectors whose target language
  has no published prior work — pessimising recall and forcing us
  to either ship nothing or fabricate weak citations to satisfy the
  rule.
- It puts the worst-case PR effort at "do unbounded literature
  survey until you find something publishable", which incentivises
  weak citations.

A best-effort SHOULD with explicit metadata:

- Always allows the language extension to ship.
- Captures the survey effort as a deliverable (the
  `docs/surveys/...` file), which has independent value.
- Surfaces the grounding strength to downstream consumers via
  `language_citation_status`, so the user — not the project — gets
  to decide how much weight to give an indirectly-grounded finding.
- Preserves P1 itself: the detector still has at least one citation
  overall, and that citation is real.

## Risks

R-A. Detector multiplication amplifies noise

cntrdct's design encourages many independent detectors. This is
deliberate (multi-perspective coverage improves recall, the ranker
filters precision). But the noise floor scales with N detectors:

- Effective FP rate on uncalibrated installs ≈ Σ per-detector FP
  rate. Layer 2 (CalibratedRanker) compresses this once priors
  ship; the uncalibrated default does not.
- Performance scales as detector × file. Rayon mitigates wall-clock
  cost, not total CPU.
- Same source line may be flagged by multiple detectors. No dedup
  today; users see duplicate-looking annotations.
- Citation maintenance: every new detector adds at least one
  citation. 50 detectors → 50+ entries to keep in sync with their
  `languages` arrays.

Counters built into the design:

- Ranker (Layer 2) ranks FP-prone findings down once a labelled
  corpus exists.
- Adjudicator (Layer 3) is the human-out-of-loop final filter on
  top-N.
- `cntrdct.toml` and `#[cntrdct::allow]` give project-level and
  in-source escape hatches.

Counters not yet built (future work, surfaced in ROADMAP for
visibility, no commitment date):

- Per-finding deduplication when multiple detectors flag the same
  primary location.
- "Default profile" of curated detectors enabled out-of-box, with
  the rest opt-in via `cntrdct.toml`. Lets us ship many detectors
  without surfacing them all by default.

R-B. Loophole in clause (b)

The (b) clause as drafted ("language-agnostic algorithm + independent
application paper") could be exploited to claim arbitrary language
support via thin secondary citations:

- Self-reference: the cntrdct project itself could be cited as the
  "Rust application" of e.g. Cordy-Roy. Recursive and
  evidentially empty.
- Weak secondary papers: an arXiv preprint or workshop note that
  briefly mentions the algorithm in passing in the target language.
- Future-work mentions: the original paper's "we plan to apply this
  to X" being treated as evidence X is covered.

Mitigation in the rule above:

- Secondary application must be peer-reviewed (no preprint, no
  blog, no future-work mentions).
- Secondary application must include quantitative evaluation on a
  corpus in the target language (no in-passing mentions).
- The cntrdct project itself does not satisfy the secondary
  requirement (explicit exclusion).

Residual exposure: a peer-reviewed paper that meets the bar but
has been criticised in subsequent literature could still be cited.
Catching this requires reviewer judgment rather than mechanical
rules; we accept that.

R-C. Bibliography becomes a bottleneck

The bibliography file becomes a bottleneck if many languages land
at once. Mitigation: M-series sequences languages one at a time;
Phase E onwards adds languages serially.

## Venue tier whitelist (added 2026-05-07, Q-7)

P1's "peer-reviewed prior art" requirement is enforced at the
citation-key level by `register_detector` and
`tests/citations_consistency.rs`. The original draft (2026-05-04)
explicitly declined to grade venues, on the assumption that any
peer-reviewed venue or named benchmark counted equally. That
uniform treatment opens R-B (loophole in clause (b)) wider than
necessary: a future detector could meet P1 by citing thin
secondary applications on the bare minimum of peer-reviewed
venues — a workshop note carries the same authority as an ICSE
main-track paper.

Q-7 codifies a venue tier whitelist that is checked mechanically
by the consistency test. Every citation shipped on a registered
detector must classify into Tier-A or Tier-B below. Unknown
venues fail the test, forcing either an explicit addition to the
whitelist (which prompts review) or a different citation. Tier-C
is documented but starts empty; it exists as a forward-compatible
release valve for grandfather clauses without loosening today's
bar.

### Tier-A — top-tier peer-reviewed venues

Software engineering:

- ICSE — International Conference on Software Engineering
- FSE / ESEC/FSE — ACM SIGSOFT Symposium on the Foundations of
  Software Engineering (joint with ESEC every other year)
- ASE — International Conference on Automated Software Engineering
- ISSTA — International Symposium on Software Testing and Analysis
- ACM TOSEM — Transactions on Software Engineering and Methodology
- IEEE TSE — Transactions on Software Engineering
- EMSE — Empirical Software Engineering (Springer journal)

Programming languages and systems:

- OOPSLA — Object-Oriented Programming, Systems, Languages and
  Applications (since merged into SPLASH)
- PLDI — Programming Language Design and Implementation
- POPL — Principles of Programming Languages
- SOSP — Symposium on Operating Systems Principles
- OSDI — Operating Systems Design and Implementation
- EuroSys — European Conference on Computer Systems

Adjacent (where software-engineering-relevant work appears):

- NeurIPS — Neural Information Processing Systems (e.g.
  PyBugLab / PyPIBugs)
- ICML — International Conference on Machine Learning
- USENIX Security, IEEE S&P, ACM CCS — top-tier security venues
  with software-engineering-relevant publications

### Tier-B — established peer-reviewed venues

- ICPC — International Conference on Program Comprehension
- ICSM / ICSME — International Conference on Software Maintenance
  (and Evolution)
- MSR — Mining Software Repositories
- SANER — Software Analysis, Evolution and Reengineering
  (CSMR + WCRE merger)
- WCRE — Working Conference on Reverse Engineering (SANER
  predecessor)
- SCAM — IEEE Working Conference on Source Code Analysis and
  Manipulation
- ICST — IEEE International Conference on Software Testing,
  Verification and Validation
- ISSRE — International Symposium on Software Reliability
  Engineering
- JSS — Journal of Systems and Software (Elsevier)
- IST — Information and Software Technology (Elsevier)

### Tier-C — peer-reviewed but not in A or B

Reserved for grandfather clauses (workshop venues, regional
conferences with documented quantitative-evaluation rigour, etc.).
Currently empty; entries are added explicitly when a specific
citation's review judges the venue acceptable. Tier-C entries emit
a CI warning rather than a failure so that grandfather clauses
remain workable without re-baselining the entire bibliography.

### Mechanical enforcement

`tests/citations_consistency.rs` carries
`every_shipped_detector_citation_has_known_tier`. It splits each
detector's citation venue string on non-alphanumeric characters,
lowercases the tokens, and checks for a Tier-A or Tier-B match
(token-equality for acronyms, substring match for multi-word
journal names). Unknown venues fail the test.

The fabrication path is pinned by
`fabricated_fixture_venue_is_rejected`: the fixture detector's
venue (`"Fixture"`) must remain unrecognised so a future loosening
of the matcher fails this test rather than silently lowering the
bar.

The matcher's whitelist lives in `tests/citations_consistency.rs`;
the spec text above is the canonical record. Adding a new venue
requires updating both — one without the other fails the test
deliberately.

## Retraction monitor (added 2026-05-07, Q-6)

P1's "peer-reviewed prior art" requirement assumes that the cited
work remains in good standing. Retraction is the failure mode the
venue tier whitelist (Q-7 above) does not catch: a paper accepted at
a Tier-A venue can be retracted years later for fabrication,
plagiarism, or methodological collapse, and a citation that was
sound when shipped becomes invalid the moment the retraction notice
publishes. Q-6 closes this loop by making CI fail closed on
retractions.

Mechanism:

- `scripts/check_retractions.py` walks `CITATIONS.md` and every
  `Citation { ..., doi: Some("...") }` slot in `src/**/*.rs`,
  unions the DOIs, and looks each one up against two sources.
- Cache (offline floor): `benchmarks/retraction-watch/cache.csv` is
  a snapshot of the Crossref-Labs-hosted Retraction Watch dataset.
  `benchmarks/retraction-watch/cache.sha256` pins it; a mismatch
  fails the script unless `--no-verify-cache` is passed (the
  refresh job uses that escape hatch immediately before rewriting
  the pin).
- Crossref Works (online ceiling): `https://api.crossref.org/works/<doi>`
  is checked for an `update-to` entry with `type == "retraction"`
  or a top-level `subtype == "retraction"`. Failure modes (timeout,
  4xx/5xx, JSON decode error) are silently demoted; the cache is
  the authoritative source if the live lookup fails.

Refresh:

- The Crossref Labs Retraction Watch endpoint
  (https://api.labs.crossref.org/data/retractionwatch) requires an
  `email` parameter so they can rate-limit and contact heavy users.
  `.github/workflows/citations.yml` reads it from the
  `RETRACTION_WATCH_EMAIL` repository secret; if the secret is
  unset, the refresh job warns and exits cleanly so the gate never
  blocks on missing credentials.
- The refresh runs Mondays at 06:00 UTC. When the cache or pin
  changes, `peter-evans/create-pull-request@v6` opens a
  `chore(citations): refresh Retraction Watch cache` PR. The
  weekly cadence matches Retraction Watch's publishing rhythm
  without producing PR noise.

Failure-path pin:

- `tests/fixtures/retraction-watch/{citations.md,cache.csv,cache.sha256}`
  plants a synthetic DOI under the unassigned `10.99999/` prefix
  and lists it as retracted. The workflow's `fixture smoke test`
  step runs the script over this fixture with `--no-network` and
  asserts exit code 1; a future loosening of the matcher (e.g. a
  DOI normalisation bug, a missing CSV column lookup, a swallowed
  cache hit) breaks this assertion rather than silently lowering
  the bar.

Non-coverage:

- A retracted-but-not-yet-indexed paper is not caught until either
  the Crossref `update-to` record publishes or the next weekly
  Retraction Watch refresh lands. The two-source design narrows
  this window but does not eliminate it.
- Citations without DOIs (e.g. `oasis-sarif-2.1.0`,
  `ieee-1044-2009`) are not checkable. Adding a DOI to a
  bibliography entry whose registry assigns one is encouraged but
  not enforced.

## Non-goals

- Recency requirements. A 1990s paper is acceptable if it actually
  grounds the detection.
- Translation of cited paper titles or abstracts. English-only.
- Quality grading of citations beyond the venue tier whitelist
  above (impact factor, h5-index, citation counts).

## Approval

Approved 2026-05-04 with the following decisions:

- Per-language citations are SHOULD, not MUST. Detectors ship with
  the new language even when the survey returns nothing; the gap
  is captured in `LanguageCitationStatus::Unconfirmed` metadata.
- Clause (b) is tightened: peer-reviewed secondary application with
  quantitative target-language evaluation; preprints / blogs /
  cntrdct itself do not qualify.
- Survey effort is mandatory and recorded under
  `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` whether or not it
  produces a citation.
- The `Citation::languages` field, the `Finding.evidence.language_citation_status`
  field, and the consistency test extension all land in M-6.
- Existing Rust citations are grandfathered under the unrevised
  (b); new languages and new detectors follow the strict (b).
