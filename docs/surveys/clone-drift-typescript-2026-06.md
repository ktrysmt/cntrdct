# Literature survey: clone-drift → TypeScript

Date: 2026-06-05
Detector: `clone-drift`
Target language: TypeScript
Surveyor: cntrdct R-2.f PR (clone-drift)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has TypeScript source as
its experimental subject, or (b) is the language-agnostic algorithm we
already cite (NiCad / the inconsistent-clone-change analysis) plus an
independent peer-reviewed paper applying that algorithm to TypeScript
with quantitative evaluation on a TypeScript corpus (both papers
cited), or (c) introduces a TypeScript benchmark / dataset relevant to
the detection.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search.

Note on the honesty bar (carried over from the Python survey and the
R-2 series): JavaScript is NOT TypeScript. A JavaScript-only corpus
does not satisfy clause (a) for TypeScript. Preprints (including
withdrawn ones), workshop notes below the IEEE/ACM peer-review tier,
tool documentation, and the cntrdct project itself do not qualify
under any clause.

## Existing citations

The detector currently cites (see `src/detectors/clone_drift.rs`):

- `cordy-roy-icpc-2008` — J.R. Cordy, C.K. Roy, "The NiCad Clone
  Detector", ICPC 2008. Subjects: Java and C/C++. Defines NiCad's
  hybrid (text + AST) Type-3 near-miss clone detection algorithm,
  which cntrdct's normalised-token n-gram clustering is conceptually
  derived from. Confirmed for Python via `assi-tosem-2025`.
- `bettenburg-msr-2009` — N. Bettenburg, W. Shang, W. Ibrahim,
  B. Adams, Y. Zou, A.E. Hassan, "An Empirical Study on Inconsistent
  Changes to Code Clones at the Release Level", MSR 2009. Subjects:
  open-source Java / C systems. Establishes the empirical framing that
  drifted clones (inconsistent changes among near-clones) introduce
  defects — the bug class clone-drift flags.
- `krinke-icsm-2007` — J. Krinke, "A Study of Consistent and
  Inconsistent Changes to Code Clones", ICSM 2007. Subjects:
  open-source C / Java systems. Earlier longitudinal evidence for the
  same drift signal.
- `assi-tosem-2025` — M. Assi, S. Hassan, Y. Zou, "Unraveling Code
  Clone Dynamics in Deep Learning Frameworks", ACM TOSEM 2025.
  Grounds Python via clause (b) (NiCad + inconsistent-change framing
  applied to nine Python frameworks). Python-only; its subjects do
  NOT include TypeScript.

None of the four ground TypeScript. The clone-drift detector
declares `Language::TypeScript` in `supported_languages()` and runs
the same function-level NiCad-style pipeline on TypeScript IR, but
emits `LanguageCitationStatus::Unconfirmed` for TypeScript findings
pending this survey.

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / general web: queries `TypeScript "clone detection"
  empirical study quantitative evaluation`, `NiCad clone detection
  TypeScript corpus precision recall`, `TypeScript code clone
  inconsistent changes maintenance empirical study`, `TypeScript code
  clones Angular empirical study`, `code clone detection multilingual
  TypeScript tree-sitter precision recall corpus`.
- ACM Digital Library / IEEE Xplore: venues ICSE/FSE/ASE/MSR/ICPC/
  SCAM/IWSC/SANER/ICSME/TOSEM/TSE/EMSE/JSS, keywords `TypeScript` AND
  (`clone detection` OR `clone evolution` OR `inconsistent clone
  change` OR `clone drift` OR `clone fault`).
- arXiv cs.SE: same keywords, last 6 years.
- dblp / Semantic Scholar forward-citation ("cited by") graph of the
  Cordy-Roy NiCad, Bettenburg, and Krinke cluster, filtered for
  TypeScript-language subjects.
- The multilingual-clone-detector cluster (MSCCD, TGMM, SourcererCC,
  C4) checked for TypeScript in their evaluation corpora.

## Candidates considered

### Zhu, Yoshida, Kamiya, Choi, Takada, "Development and Benchmarking of Multilingual Code Clone Detector" (Journal of Systems and Software, 2024)

JSS (Elsevier), peer-reviewed.
https://www.sciencedirect.com/science/article/pii/S0164121224002590
(journal extension of the ICPC 2022 paper "MSCCD: Grammar Pluggable
Clone Detection Based on ANTLR Parser Generation", DOI
10.1145/3524610.3529161, https://ieeexplore.ieee.org/document/9796305/).
Preprint: arXiv 2409.06176 (https://arxiv.org/abs/2409.06176).

MSCCD is a grammar-pluggable, ANTLR-based multilingual Type-3 clone
detector. Its language-extensibility experiment applied MSCCD to 20
modern languages (16 "perfectly supported", 4 supported at higher
execution cost); TypeScript is plausibly within that ANTLR-grammar
set, but neither the abstract, the GitHub README
(https://github.com/zhuwq585/MSCCD), nor the searchable text confirm
TypeScript by name or report any TypeScript-specific clone result.

Verdict: REJECTED.

- The quantitative detection-performance evaluation (recall via
  BigCloneBench, precision via manual sampling, plus CodeNet) is
  conducted on Java. The 20-language experiment measures parser /
  grammar-plugin coverage and execution cost — it is a language-
  scalability test, not a quantitative clone-detection evaluation on
  a TypeScript corpus. Clause (a) as applied in the R-2 series
  requires the cited paper's experimental subjects to include
  TypeScript code WITH quantitative clone evaluation; a "supported
  language" entry in a scalability table does not meet that bar, and
  TypeScript-specific quantitative results could not even be
  confirmed.
- Clause (b) is not satisfied either: MSCCD implements its own
  ANTLR-token-based algorithm, NOT the NiCad algorithm cntrdct cites
  (`cordy-roy-icpc-2008`). Clause (b) requires an independent paper
  applying *the cited* language-agnostic algorithm to TypeScript.
  MSCCD is a different algorithm, so it cannot serve as the NiCad
  application paper.
- It grounds (at most) clone *detection*, and says nothing about the
  drift / inconsistent-change signal (`bettenburg-msr-2009`,
  `krinke-icsm-2007`) on TypeScript, which is the detector's actual
  anomaly.

### "TGMM: Combining Parse Tree with GPU for Scalable Multilingual and Multi-Granularity Code Clone Detection" (arXiv 2403.18202)

arXiv preprint. https://arxiv.org/abs/2403.18202

A multilingual, multi-granularity Type-3 clone detector; some
secondary sources list TypeScript among the languages it can parse.

Verdict: REJECTED on two independent grounds.

- Preprint: no peer-reviewed venue. `citations-policy.md` clause (b)
  explicitly excludes preprints.
- WITHDRAWN by its own author, who states the paper "contains
  misleading experimental results". An unreliable, retracted preprint
  cannot ground any clause.

### Saini et al. / "Development nature matters: An empirical study of code clones in JavaScript applications" (Empirical Software Engineering, 2015)

Springer EMSE, DOI 10.1007/s10664-015-9368-6.
https://link.springer.com/article/10.1007/s10664-015-9368-6

Peer-reviewed empirical study of code clones, but the experimental
subjects are JavaScript applications.

Verdict: REJECTED for clause (a). JavaScript is not TypeScript
(R-series honesty rule). The corpus contains no TypeScript subjects.

### "Towards an Empirical Analysis of Code Cloning and Code Drift" (BENEVOL 2024 workshop)

CEUR-WS workshop proceedings.
https://ceur-ws.org/Vol-3941/BENEVOL2024_TECH_paper8.pdf

Discusses clone drift and touches multiple languages.

Verdict: REJECTED on venue tier. CEUR-WS workshop notes are
light-touch reviewed and fall below the "peer-reviewed publication"
bar the policy requires — the same standard applied in the Python
survey for this exact paper and in the arg-swap survey for community
tools. (Title-level relevance to "drift" does not change the venue
disqualification.)

### "An Empirical Analysis of Git Commit Logs for Potential Inconsistency in Code Clones" (arXiv 2409.08555)

arXiv preprint analysing 45 Apache Software Foundation repositories.
https://arxiv.org/pdf/2409.08555

Studies the inconsistent-clone-change signal directly — topically the
closest to clone-drift — but the corpus is Apache (predominantly
Java) and the work is a preprint.

Verdict: REJECTED. Preprint, and subjects are not TypeScript.

### Cross-language clone detection (C4, ICPC 2024; CLCDSA; "LLMs for cross-language code clone detection", arXiv 2408.04430)

C4: https://xing-hu.github.io/assets/papers/icpc-c8.pdf

These target cross-language clone pairs; their corpora are built from
Java / Python / C# / C++ (e.g. AtCoder, Google CodeJam, CLCDSA). The
LLM cross-language paper is an arXiv preprint.

Verdict: REJECTED. TypeScript is not in the evaluation corpora; the
LLM paper is additionally a preprint.

### Clone-fault / clone-genealogy empirical studies (Göde & Koschke; "On the Relationship of Inconsistent Software Clones and Faults", 2016; clone-stability studies)

Peer-reviewed empirical studies of inconsistent clone changes, clone
genealogies, and clone-related faults — the same evidential family as
`bettenburg-msr-2009` and `krinke-icsm-2007`.

Verdict: REJECTED for clause (a). Every such study surveyed uses
Java / C / C++ subjects (Linux, FreeBSD, automotive Java, Apache).
None include TypeScript.

### Community tools without peer-reviewed papers (jscpd, jsinspect, simian, PMD CPD)

JavaScript/TypeScript-capable clone tools exist, but they are tools,
not peer-reviewed publications with quantitative TypeScript
evaluation.

Verdict: REJECTED. Same standard as the Python and arg-swap surveys.

## Conclusion

No candidate satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for cntrdct's clone-drift detector on
TypeScript:

- No peer-reviewed paper applies the NiCad algorithm we cite
  (`cordy-roy-icpc-2008`) to a TypeScript corpus with quantitative
  evaluation — clause (b) fails.
- The strongest near-miss (MSCCD, JSS 2024) is a peer-reviewed
  multilingual clone detector, but it (i) uses a non-NiCad algorithm,
  (ii) reports its quantitative detection performance on Java rather
  than a TypeScript corpus, and (iii) does not address the drift /
  inconsistent-change signal — so it grounds neither clause (a) nor
  clause (b) for TypeScript honestly.
- No peer-reviewed study of inconsistent clone changes / clone faults
  uses TypeScript subjects — the drift half of the detector is
  ungrounded on TypeScript.
- No peer-reviewed TypeScript clone benchmark / dataset exists —
  clause (c) fails.

The honest default applies: TypeScript coverage for clone-drift is
Unconfirmed.

## Decision

- Do NOT add a TypeScript-grounded citation to
  `Citation::CITATIONS` for clone-drift.
- TypeScript findings continue to emit the cross-cutting concept keys
  (`cordy-roy-icpc-2008`, `bettenburg-msr-2009`, `krinke-icsm-2007`)
  with `LanguageCitationStatus::Unconfirmed` (matches the current
  `src/detectors/clone_drift.rs` behaviour).
- `CITATIONS.md` records the explicit-no-citation line for clone-drift
  TypeScript coverage, pointing at this survey:
  `(clone-drift TypeScript coverage: unconfirmed; survey notes at
  docs/surveys/clone-drift-typescript-2026-06.md)`.
  (Integration of this line is performed centrally by team-lead, not
  in this survey commit.)

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper (IEEE/ACM/Elsevier/Springer tier) applies
  NiCad-style or SourcererCC-style clone detection to a TypeScript
  corpus with quantitative evaluation — would satisfy clause (a)/(b).
- A peer-reviewed empirical study of inconsistent clone changes /
  clone faults / clone evolution uses TypeScript subjects — would
  ground the drift signal directly.
- A peer-reviewed TypeScript clone benchmark / dataset is published —
  would satisfy clause (c).
- The MSCCD JSS work is extended with a confirmed, named TypeScript
  quantitative clone-detection result on a TypeScript corpus.
- The TGMM preprint is re-issued, corrected, and accepted at a
  peer-reviewed venue with a TypeScript evaluation (currently
  withdrawn for misleading results).
