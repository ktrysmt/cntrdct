# Literature survey: unreachable-after-terminator → Python

Date: 2026-05-05
Detector: `unreachable-after-terminator`
Target language: Python
Surveyor: cntrdct M-2 PR

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

- `hovemeyer-pugh-oopsla-2004` — D. Hovemeyer, W. Pugh, "Finding Bugs
  is Easy", OOPSLA 2004. Introduces the FindBugs UR (Unreachable
  code) bug pattern. Subjects: Java.
- `engler-sosp-2001` — D. Engler et al., "Bugs as Deviant Behavior",
  SOSP 2001. Establishes control-flow contradictions as
  high-confidence anomaly signals. Subjects: the Linux kernel and
  C systems code.

Both are grandfathered as Rust-grounded under the unrevised clause (b)
when cntrdct shipped v0; new languages follow the strict (b).

## Search

Databases / sources queried:

- Google Scholar: queries `python "unreachable code" static analysis
  empirical`, `python "dead code" detection peer reviewed`,
  `python "after return" static analysis`, `python "code smell"
  "dead code" detection corpus evaluation`.
- ACM Digital Library: filters venue ICSE/FSE/MSR/SANER/PLDI/POPL/OOPSLA
  2010-2025, keywords `Python` AND (`unreachable` OR `dead code` OR
  `control flow contradiction`).
- IEEE Xplore: same venues and keywords.
- dblp: author-name searches for the Hovemeyer-Pugh and Engler
  citation clusters' "cited by" graphs filtered for Python-language
  subjects.
- arXiv: cs.SE category, last 5 years, same keywords.

## Candidates considered

### Chen Zhifei et al., "Understanding metric-based detectable smells in Python software" (IST 2017)

DOI: 10.1016/j.infsof.2016.10.003. ScienceDirect:
https://www.sciencedirect.com/science/article/abs/pii/S0950584916301690

The paper enumerates ten Python code smells (Long Method, Long
Parameter List, Large Class, Long Scope Chaining, Long Base Class
List, Long Lambda Function, Long Ternary Conditional Expression,
Complex Container Comprehension, Multiply-Nested Container, Long
Message Chain) with metric-based detection on a corpus of 106
high-star GitHub Python projects.

Verdict: rejected. The smell catalogue does not include unreachable
code, dead code, or any control-flow contradiction class. Strong
peer-reviewed Python evaluation, wrong target pattern.

### Shivasankar et al., "PyExamine: A Comprehensive, Un-Opinionated Smell Detection Tool for Python" (MSR 2025)

arXiv: 2501.18327. IEEE: 11025624.

PyExamine reports 49 metrics across architectural, structural, and
code-level smells, evaluated on 7 case-study projects (with
accuracy 91.4% on code-level smells) and applied broadly to 183
Python projects. The paper enumerates "dead code" as one of the
detected smells.

Verdict: rejected on grounds (b). PyExamine evaluates Python code,
which would satisfy (a). However, the paper's "dead code"
definition follows the unused-symbol sense (the Vulture / Skylos
lineage: unused functions, classes, imports, variables). The
unreachable-after-terminator pattern is a different bug model —
control-flow contradiction at the AST-statement level, not
reachability of a definition from any call site. Citing PyExamine
as grounding for a UR-style detector would be the kind of
loose-fit citation the policy's R-B clause warns against
("Weak secondary papers: ... a workshop note that briefly mentions
the algorithm in passing"). The two bug models share a name in
the literature but differ algorithmically.

### Vulture (Seipp, jendrikseipp/vulture)

GitHub: https://github.com/jendrikseipp/vulture. Python community
tool, no peer-reviewed paper. Documents the same pattern
(unreachable code after `return` / `raise` / `break` / `continue`)
in its README.

Verdict: rejected. Tool, not a peer-reviewed publication. Cannot
satisfy (a) / (b) / (c).

### deadcode (albertas/deadcode), Skylos

Both community tools without peer-reviewed evaluations. Mentioned
as related work but not citable.

Verdict: rejected on the same grounds as Vulture.

### Hovemeyer-Pugh (OOPSLA 2004) cited-by graph

Of the ~3000 papers citing Hovemeyer-Pugh as of 2026, none in the
filtered top-200 (ranked by citation count and venue) presents a
quantitative evaluation of the FindBugs UR pattern on a Python
corpus. Multiple papers reuse the bug-pattern taxonomy for Java
analyses (SpotBugs lineage), several extend it to Android / Kotlin,
none target Python at the AST-statement level we need.

Verdict: no qualifying clause-(b) secondary application found.

### Engler-SOSP-2001 cited-by graph

Engler et al. has been extended to JavaScript (Pradel-Sen ECOOP
2018-style work), to C/C++ via Coverity, to kernel-level analyses,
but not to Python with the specific control-flow-contradiction
framing.

Verdict: no qualifying clause-(b) secondary application found.

### Bao et al., An Empirical Study of Python Code Smells (multiple venues)

Various code-smell empirical studies exist for Python (Vavrová &
Zaytsev 2017 "Does Python Smell Like Java?"; ResearchGate 311609982
"Detecting Code Smells in Python Programs"; the deep-learning
review surveys). None enumerate the unreachable-after-terminator
pattern as one of their detected smells.

Verdict: rejected — pattern not among detected smells.

### Other candidates examined and rejected

- DSD-Crasher (Csallner-Smaragdakis, ISSTA 2008) — Java only.
- PySonar2 (Wang, Cornell tech report) — type inference, not
  control-flow contradictions.
- PyT (NordSec 2018) — taint analysis for web app vulnerabilities.
- PyCG (Salis et al., MSR 2021) — call graph construction; reachability
  in the inter-procedural sense, not intra-block UR.
- mypy / Pyre / Pyright — type checkers; do not flag UR.
- "Bichhawat et al.", "Rice et al. ICSE 2017" — different
  domains / languages.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the
unreachable-after-terminator pattern on Python. The detector ships
its Python extension regardless: P1 itself remains satisfied (the
detector continues to declare the two grandfathered Rust citations,
which are non-empty), and the per-language gap is captured in
metadata.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(unreachable-after-terminator
  Python coverage: unconfirmed; survey notes at this file)` line
  under the detector's subsection so the consistency test sees an
  acknowledged gap rather than a silent one.
- The detector emits
  `LanguageCitationStatus::Unconfirmed` on every Python finding.
  SARIF consumers can filter or visually flag indirectly-grounded
  Python results via `properties.languageCitationStatus`.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies the FindBugs UR pattern to a Python
  corpus with quantitative evaluation (clause b candidate).
- A new Python benchmark / dataset specifically labels
  unreachable-statement defects (clause c candidate).
- The Hovemeyer-Pugh bug-pattern taxonomy is formally extended to
  Python in a published reference work.
