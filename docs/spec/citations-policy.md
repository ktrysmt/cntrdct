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

## Non-goals

- Quality grading of citations (impact factor, citation count,
  venue tier). Any peer-reviewed venue or named benchmark counts.
- Recency requirements. A 1990s paper is acceptable if it actually
  grounds the detection.
- Translation of cited paper titles or abstracts. English-only.

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
