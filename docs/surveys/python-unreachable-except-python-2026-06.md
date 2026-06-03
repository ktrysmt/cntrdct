# Literature survey: python-unreachable-except → Python

Date: 2026-06-03
Detector: `python-unreachable-except` (new, R-5 / F4f)
Target language: Python
Surveyor: cntrdct R-5 PR

## Goal

Per `docs/spec/citations-policy.md`, a detector declaring support for a
language should be grounded in at least one peer-reviewed publication that
(a) has source code in that language as its experimental subject, or (b) is a
language-agnostic algorithm we cite plus an independent peer-reviewed paper
applying it to the language with quantitative evaluation, or (c) introduces a
benchmark / dataset in that language.

`python-unreachable-except` is a brand-new, Python-only detector (not a
language extension of an existing one). P1 is satisfied by its two
concept-grounding citations regardless; this survey records the search for a
Python-subject grounding and its outcome.

## Detector citations (concept grounding, peer-reviewed)

- `hovemeyer-pugh-oopsla-2004` — D. Hovemeyer, W. Pugh, "Finding Bugs is Easy",
  OOPSLA 2004. FindBugs "UR — Unreachable code" bug pattern. Subjects: Java.
- `de-padua-shang-icpc-2017` — G. B. de Pádua, W. Shang, "Studying the
  Prevalence of Exception Handling Anti-Patterns", ICPC 2017, pp. 328-331,
  DOI 10.1109/ICPC.2017.1. Defines the "Unreachable Handler" anti-pattern
  (an `except` clause made unreachable because a broader clause precedes it).
  Subjects: 16 open-source Java and C#/.NET libraries.

Both are peer-reviewed and directly justify the detector concept. Neither has
Python source as a subject, so neither grounds the Python language per clause
(a)/(b)/(c).

## Search

Databases / sources queried:

- Google Scholar: `python "unreachable except" static analysis`,
  `python exception handling anti-pattern empirical`,
  `python "except" ordering subclass unreachable handler`,
  `python exception handling bug empirical study corpus`.
- ACM DL / IEEE Xplore: venues ICSE/FSE/MSR/SANER/ICPC/SCAM/EMSE 2015-2026,
  keywords `Python` AND (`exception handling` OR `unreachable handler` OR
  `except` ordering).
- dblp: "cited by" graph of de Pádua & Shang's exception-handling cluster,
  filtered for Python-subject follow-ups.
- arXiv cs.SE / cs.PL, last ~8 years.

## Candidates considered

### de Pádua & Shang follow-ups (SCAM 2017, MSR 2018)

"Revisiting Exception Handling Practices with Exception Flow Analysis"
(SCAM 2017) and "Studying the Relationship between Exception Handling
Practices and Post-release Defects" (MSR 2018). Both peer-reviewed and
on-point for the anti-pattern, but their subjects remain Java/C#. Do not
satisfy (a) for Python.

Verdict: rejected for Python grounding (wrong subject language); the ICPC 2017
sibling is already cited for concept grounding.

### Souza, Coelho, Correia, Lima, Teixeira, Neto — "Slithering Through Exception Handling Bugs in Python: Understanding Root Causes, Symptoms, and Fixes"

SSRN id 5592428. This is Python-subject and directly studies exception-handling
bugs and anti-patterns in Python, which would satisfy clause (a) on subject.

Verdict: rejected for now. The copy located is an SSRN posting with no
confirmable peer-reviewed venue; `citations-policy.md` (R-B mitigation) requires
peer-reviewed prior art and explicitly excludes preprints. Recorded as the
strongest revisit trigger: if/when this work appears in a peer-reviewed venue
(or its venue is confirmed), re-run this survey and promote Python grounding to
`Confirmed` with `Languages: Python`.

### arXiv:1704.00778 (de Pádua & Shang preprint)

The preprint of the ICPC 2017 paper. Cited via its peer-reviewed ICPC form
above; the preprint itself adds nothing for Python.

### Python community tools (Pylint E0701 / bad-except-order, CodeQL py/unreachable-except, flake8 plugins)

These implement exactly this check and confirm the pattern is well-established
in practice, but none is a peer-reviewed publication and so cannot satisfy
(a)/(b)/(c). Recorded as evidence the detection is sound and widely deployed.

Verdict: rejected (tools, not publications).

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`citations-policy.md` for the unreachable-`except`-handler pattern on Python.
The detector ships regardless: P1 is satisfied by the two peer-reviewed
concept-grounding citations, and the per-language gap is captured in metadata.

## Decision

- No Python-grounded entry added to `Citation::CITATIONS`; both citations carry
  `languages: &[]` (general / concept grounding).
- `CITATIONS.md` adds an explicit `(python-unreachable-except Python coverage:
  unconfirmed; survey notes at this file)` line under the detector subsection.
- The detector emits `LanguageCitationStatus::Unconfirmed` on every finding.

## Revisit triggers

- The Souza et al. "Slithering ..." Python exception-handling-bug study appears
  in a confirmable peer-reviewed venue (clause-a candidate; highest priority).
- A peer-reviewed paper labels unreachable-`except` defects on a Python corpus
  with quantitative evaluation (clause-b/c candidate).
