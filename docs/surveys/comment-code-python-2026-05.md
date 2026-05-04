# Literature survey: comment-code → Python

Date: 2026-05-05
Detector: `comment-code`
Target language: Python
Surveyor: cntrdct M-3 PR (comment-code first)

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

- `tan-sosp-2007` — L. Tan, D. Yuan, G. Krishna, Y. Zhou,
  "/*iComment: Bugs or Bad Comments?*/", SOSP 2007. Subjects:
  C/C++ Linux kernel comments.
- `tan-pldi-2011` — L. Tan, Y. Zhou, Y. Padioleau,
  "aComment: Mining Annotations from Comments and Code to Detect
  Interrupt-related Concurrency Bugs", PLDI 2011. Subjects:
  C/C++ kernel concurrency annotations.

Both are grandfathered as Rust-grounded under the unrevised clause (b)
when cntrdct shipped v0; new languages follow the strict (b).

## Search

Databases / sources queried:

- Google Scholar: queries `python "code-comment inconsistency"
  detection peer reviewed`, `python docstring "@raises" mismatch
  static analysis`, `python "comment code mismatch" iComment
  aComment`, `python docstring specification mismatch detection`.
- ACM Digital Library: filters venue ICSE/FSE/ICPC/MSR/SANER
  2010-2025, keywords `Python` AND (`comment` OR `docstring`) AND
  (`inconsistency` OR `mismatch` OR `bug`).
- IEEE Xplore: same venues and keywords.
- dblp: author-name searches in the iComment/aComment citation
  cluster's "cited by" graph filtered for Python-language subjects.
- arXiv: cs.SE category, last 5 years, same keywords.

## Candidates considered

### Wen, Nagy, Bavota, Lanza, "A Large-Scale Empirical Study on Code-Comment Inconsistencies" (ICPC 2019)

ICPC 2019. https://www.inf.usi.ch/lanza/Downloads/Wen2019a.pdf.

The strongest peer-reviewed empirical study in the iComment lineage
since the original Tan papers. Mines 1.3 billion AST-level changes
from 1,500 GitHub projects to taxonomise code-comment inconsistency
classes and quantify their bug-introducing impact (~1.5x compared
to consistent changes).

Verdict: rejected for clause (a). The corpus is exclusively Java
("1,500 open source Java projects"); Python is not analysed. The
algorithm is language-agnostic in principle, so the paper could be
the primary half of a clause (b) citation pair, but no independent
peer-reviewed paper applying the same AST-level inconsistency
taxonomy to a Python corpus with quantitative evaluation surfaced
in the search.

### Rani et al., "A Decade of Code Comment Quality Assessment" (Journal of Systems and Software 2022)

JSS 2022. PDF:
https://www.oscar.nierstrasz.org/files/publications/Rani22c-ADecadeOfCodeCommentQualityAssessment.pdf.

A systematic literature review of comment-quality assessment
research over 2011-2020. Surveys hundreds of studies, including
some Python-focused ones, but does not itself perform quantitative
detection on a Python corpus.

Verdict: rejected. Surveys are explicitly out of scope for clauses
(a)/(b)/(c) — the policy requires a paper whose experimental
subject is Python or whose algorithm has been demonstrated on
Python. A literature review pointing at other people's Python work
does not satisfy the requirement; the cited primary work would.

### Detecting Code Comment Inconsistencies using LLM and Program Analysis (FSE Companion 2024)

ACM DL: https://dl.acm.org/doi/10.1145/3663529.3664458 (FSE 2024
Companion / Demo / SRC track).

A 2024 short paper / demo. Combines LLM-based detection with
program analysis. Language coverage was not retrievable through
public abstracts; however, the venue is FSE Companion, which is
typically demo / SRC track and does not provide the same level of
empirical evaluation as full research-track papers. The policy
explicitly excludes works where "secondary application must include
quantitative evaluation on a corpus in the target language" cannot
be confirmed.

Verdict: rejected. Insufficient evidence of quantitative Python
corpus evaluation, and venue track ambiguity raises clause-(b)
loophole risk.

### Investigating the Impact of Code Comment Inconsistency on Bug Introducing (arXiv 2024)

arXiv 2409.10781. https://arxiv.org/html/2409.10781v1.

Verdict: rejected. arXiv preprint without confirmed peer-reviewed
venue acceptance. The policy explicitly excludes preprints
("Preprints, blog posts, and the cntrdct project itself do not
satisfy the secondary-application requirement").

### Completing Function Documentation Comments Using Structural Information (Empirical Software Engineering 2023)

Springer:
https://link.springer.com/article/10.1007/s10664-022-10284-6.

Generates rather than checks documentation. The task is the
inverse of comment-code mismatch detection.

Verdict: rejected. Wrong task — generation, not consistency
checking.

### pydoclint, pydocstyle, docchecker, C4RLLaMA

Community tools and / or LLM-fine-tuning artifacts without
peer-reviewed publications backing the specific detection rules.
pydoclint and pydocstyle are de-facto Python ecosystem standards
but are not academic publications.

Verdict: rejected. Tools, not peer-reviewed publications. Cannot
satisfy (a) / (b) / (c).

### iComment / aComment cited-by graph

Of the ~1500 papers citing Tan et al. (2007) and (2011) as of
2026, the filtered top-200 (ranked by citation count and venue)
yields:

- A cluster of Java-focused replications and extensions (Wen 2019,
  Ratol 2017, Steidl 2013, etc.).
- A C/C++ Linux-kernel cluster following the original aComment
  threads.
- A handful of Android (Java) papers.
- No cluster targeting Python with quantitative evaluation of the
  iComment / aComment specific patterns.

Verdict: no qualifying clause-(b) secondary application found.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the comment-code pattern on
Python. The detector ships its Python extension regardless: P1
itself remains satisfied (the detector continues to declare the two
grandfathered Rust citations, which are non-empty), and the
per-language gap is captured in metadata.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(comment-code Python coverage:
  unconfirmed; survey notes at this file)` line under the
  detector's subsection so the consistency test sees an
  acknowledged gap rather than a silent one.
- The detector emits
  `LanguageCitationStatus::Unconfirmed` on every Python finding.
  SARIF consumers can filter or visually flag indirectly-grounded
  Python results via `properties.languageCitationStatus`.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies iComment/aComment-style detection
  to a Python corpus with quantitative evaluation (clause b
  candidate).
- A new Python benchmark / dataset specifically labels
  doc-vs-implementation contradictions (clause c candidate).
- A peer-reviewed Python-corpus replication of Wen et al. (ICPC
  2019) is published.
