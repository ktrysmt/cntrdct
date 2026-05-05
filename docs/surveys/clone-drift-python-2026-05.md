# Literature survey: clone-drift → Python

Date: 2026-05-05
Detector: `clone-drift`
Target language: Python
Surveyor: cntrdct M-3 PR (clone-drift)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has Python source as its
experimental subject, or (b) is the language-agnostic algorithm we
already cite plus an independent peer-reviewed paper applying it to
Python with quantitative evaluation, or (c) introduces a Python
benchmark / dataset relevant to the detection.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search.

## Existing Rust citations

The detector currently cites:

- `cordy-roy-icpc-2008` — J.R. Cordy, C.K. Roy, "The NiCad Clone
  Detector", ICPC 2008. Subjects: Java and C/C++. Defines NiCad's
  hybrid (text + AST) Type-3 near-miss clone detection algorithm,
  which cntrdct's Type-3 + Type-2 partition pipeline is conceptually
  derived from.
- `bettenburg-msr-2009` — N. Bettenburg, W. Shang, W. Ibrahim,
  B. Adams, Y. Zou, A.E. Hassan, "An Empirical Study on Inconsistent
  Changes to Code Clones at the Release Level", MSR 2009. Subjects:
  three open-source Java / C systems. Establishes the empirical
  framing that drifted clones (inconsistent changes among near-clones)
  introduce defects between releases, which is the bug class
  cntrdct's clone-drift detector flags.
- `krinke-icsm-2007` — J. Krinke, "A Study of Consistent and
  Inconsistent Changes to Code Clones", ICSM 2007. Subjects: five
  open-source C / Java systems. Earlier longitudinal evidence for
  the same drift signal.

All three are grandfathered as Rust-grounded under the unrevised
clause (b) when cntrdct shipped v0; new languages follow the strict (b).

## Search

Databases / sources queried:

- Google Scholar: queries `Python "clone detection" empirical
  evolution`, `Python clone drift inconsistent change empirical`,
  `clone evolution Python software repositories`, `NiCad Python
  evaluation precision recall`.
- ACM Digital Library: filters venue ICSE/FSE/ASE/MSR/ICPC/SCAM/IWSC/
  TOSEM/TSE/EMSE 2010-2025, keywords `Python` AND (`clone evolution`
  OR `inconsistent clone change` OR `clone drift` OR `clone bug`).
- IEEE Xplore: same venues and keywords, plus the IWSC workshop
  series 2007-2024.
- arXiv: cs.SE category, last 5 years, same keywords.
- dblp: author-name searches in the Cordy-Roy / Bettenburg /
  Krinke citation cluster's "cited by" graph filtered for
  Python-language subjects.
- The detector's existing citation cluster's forward-citation graph
  (Semantic Scholar API), filtered for venue tier ICSE/FSE/ASE/
  TOSEM/TSE/EMSE/MSR/SCAM/IWSC and Python-language subjects.

## Candidates considered

### Assi, Hassan, Zou, "Unraveling Code Clone Dynamics in Deep Learning Frameworks" (ACM TOSEM 2025)

ACM TOSEM, DOI 10.1145/3721125. Preprint arXiv 2404.17046 (April 2024).
https://dl.acm.org/doi/10.1145/3721125

Empirical study of code clones across nine open-source Python deep
learning frameworks (TensorFlow, Paddle, PyTorch, Aesara, Ray, MXNet,
Keras, Jax, BentoML). Methodology applies NiCad — the algorithm
introduced by `cordy-roy-icpc-2008` — to Python source files (file
extension `.py`) and SourcererCC for cross-framework token-based
clone detection. Tracks long-term clone-coverage evolution across
releases (PyTorch declines from 27% to <3% over 20 releases;
TensorFlow/Jax/Ray remain stable within 5%), within-release cloning
patterns, and bug-fixing activity in clones across release histories.
The "Serpentine" evolutionary trend is empirically associated with
elevated bug-fixing activity in cloned fragments — directly the
inconsistent-change framing of `bettenburg-msr-2009` and
`krinke-icsm-2007`, transposed to Python.

Verdict: ACCEPTED.

- Clause (b) is satisfied for `cordy-roy-icpc-2008`: NiCad is the
  cited language-agnostic algorithm; this paper is an independent
  peer-reviewed publication that applies NiCad to Python with
  quantitative evaluation (clone coverage percentages, bug-fix
  frequency by evolutionary trend, cross-framework clone count).
  The work is not a preprint (TOSEM is peer-reviewed), not a blog,
  not the cntrdct project; it is published in a top-tier SE journal.
- Clause (b) also extends the Bettenburg/Krinke drift framing to
  Python, since the bug-fixing-in-clones analysis is the same
  empirical primitive carried into a Python corpus.
- Algorithmic identity holds: NiCad in this study uses the same
  text-and-pretty-print Type-3 detection cntrdct cites; cntrdct's
  detector uses tree-sitter token sequences with the same Type-3
  + Type-2 partition logic, which is in the same algorithmic family.
- Citation key: `assi-tosem-2025`.

### Roy & Cordy, "Adventures in NICAD: A Ten-Year Retrospective" + "NiCad: A Modern Clone Detector" (Springer 2021)

Book chapter (Springer Nature, 2021), DOI 10.1007/978-981-16-1927-4_3.

Retrospective summarising NiCad's evolution since ICPC 2008. Mentions
multi-language support including Python but does not present new
quantitative Python evaluation; it surveys prior work. The Python
material is cross-references to other publications, not a new
empirical study.

Verdict: rejected. No quantitative Python evaluation of its own;
fails clause (b) as a stand-alone citation. (The Assi 2025 paper
above is the actual independent application required by clause (b).)

### Roy & Cordy "An Empirical Study of Function Clones in Open Source Software" (SCAM 2008) + follow-ups

SCAM 2008. Subjects: predominantly C and Java systems. Establishes
function-level clone density baselines, but the experimental
subjects do not include Python.

Verdict: rejected for clause (a). Not Python.

### Pate, Tairas, Kraft, "Clone evolution: a systematic review" (J. Software: Evolution and Process, 2013)

DOI 10.1002/smr.579.

Systematic review of clone-evolution research. Discusses Python
as part of the broader landscape but is itself a review, not an
empirical study with a Python corpus.

Verdict: rejected. Reviews evidence but does not generate
Python-grounded empirical results. Does not satisfy any clause as
a stand-alone Python citation.

### Saha, Roy, Schneider, "An Empirical Study on Clone Evolution by Analyzing Clone Lifetime" (IEEE 2019)

IEEE Xplore 8665850. Empirical study of clone evolution across
12-66 versions of four open-source projects. Subjects are Java
and C/C++; Python is not in the corpus.

Verdict: rejected for clause (a). Subjects are not Python.

### "Towards an Empirical Analysis of Code Cloning and Code Drift" (BENEVOL 2024 workshop)

CEUR-WS workshop note. Workshop proceedings without IEEE/ACM
peer-review tier. Does discuss Python but the venue does not
satisfy the "peer-reviewed publication" bar in
`citations-policy.md` clause (b). CEUR-WS workshop notes are
typically light-touch reviewed; we apply the same standard that
rejected the DeepBugsPlugin community tool in the arg-swap survey.

Verdict: rejected on venue tier. Workshop note, not peer-reviewed
in the strict sense the policy requires.

### "An Empirical Study on Cross-language Clone Bugs" (ICSE 2024 Posters)

ICSE 2024 posters track, DOI 10.1145/3639478.3643075. Studies
"mirror bugs" between Java and C# implementations of four projects.
Python is not in the corpus.

Verdict: rejected for clause (a). Subjects are Java and C#, not
Python.

### Liu, Wang, Liu, "Large Language Models for cross-language code clone detection" (arXiv 2408.04430)

arXiv preprint. Per `citations-policy.md` clause (b): "Preprints,
blog posts, and the cntrdct project itself do not satisfy the
secondary-application requirement." This paper does not list a
peer-reviewed venue at the time of the survey.

Verdict: rejected. Preprint.

### Multilingual Code Clone Detector benchmarking studies (e.g., arXiv 2409.06176)

arXiv preprint. Same disqualification as above.

Verdict: rejected. Preprint.

### Tools without peer-reviewed publications (e.g., DeepBugsPlugin, jscpd, simian, pylint)

Community tools. The arg-swap survey applied the same standard:
tools without peer-reviewed papers do not satisfy (a) / (b) / (c).

Verdict: rejected.

## Conclusion

Assi, Hassan, Zou (TOSEM 2025) — "Unraveling Code Clone Dynamics in
Deep Learning Frameworks" — satisfies clause (b) of
`docs/spec/citations-policy.md` for cntrdct's clone-drift detector
on Python. NiCad (cited via `cordy-roy-icpc-2008`) is the
language-agnostic algorithm; the TOSEM paper is the required
independent peer-reviewed application to Python with quantitative
evaluation across nine real-world Python codebases. The bug-fixing
analysis transposes the inconsistent-change framing of
`bettenburg-msr-2009` and `krinke-icsm-2007` to Python.

This is the worked example from `citations-policy.md` line 207
("a separate paper applies NiCad to Python with measured
precision/recall on a Python corpus") realised concretely.

## Decision

- Add `assi-tosem-2025` to `Citation::CITATIONS` with
  `languages: &[Language::Python]`.
- Python findings emit `citation_keys` that include
  `assi-tosem-2025` alongside the existing Rust-grounded citations,
  and carry `LanguageCitationStatus::Confirmed`.
- `CITATIONS.md` adds a bibliography entry under the clone-drift
  subsection with `Languages: Python`.
- A fresh dated formal preregistration revision is published per
  the convention established by the arg-swap PR.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed empirical study published in a tier-1 venue
  reports that NiCad-style clone-drift signals do not reproduce on
  Python at expected rates, which would weaken the clause-(b)
  grounding.
- A peer-reviewed Python-specific clone-drift detector (with
  quantitative target-language evaluation) is published, which
  would supersede the indirect grounding with a direct one.
- The Assi 2025 paper is retracted or substantially criticised in
  a peer-reviewed venue, requiring fallback to Unconfirmed status
  pending a replacement.
