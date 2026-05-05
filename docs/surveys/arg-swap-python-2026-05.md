# Literature survey: arg-swap → Python

Date: 2026-05-05
Detector: `arg-swap`
Target language: Python
Surveyor: cntrdct M-3 PR (arg-swap)

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

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically
  Extracting Implicit Programming Rules and Detecting Violations in
  Large Software Code", ESEC/FSE 2005. Subjects: C/C++.
- `rice-icse-2017` — A. Rice, E. Aftandilian, C. Jaspan, E. Johnston,
  M. Pradel, Y. Arroyo-Paredes, "Detecting Argument Selection
  Defects", ICSE 2017. Subjects: Java and C++.

Both are grandfathered as Rust-grounded under the unrevised clause (b)
when cntrdct shipped v0; new languages follow the strict (b).

## Search

Databases / sources queried:

- Google Scholar: queries `Python "argument swap" OR "swapped
  arguments" static analysis empirical`, `Python argument selection
  defect detection peer reviewed`, `DeepBugs Python replication
  argument swap`, `PyPIBugs argument swapping evaluation`.
- ACM Digital Library: filters venue ICSE/FSE/ASE/MSR/OOPSLA
  2010-2025, keywords `Python` AND (`argument swap` OR `swapped
  arguments` OR `argument selection defect`).
- IEEE Xplore: same venues and keywords.
- NeurIPS / ICLR / ICML proceedings: `bug detection` AND `Python`
  AND `argument`.
- dblp: author-name searches in the Rice / Li-Zhou / Pradel-Sen
  citation clusters' "cited by" graph filtered for Python-language
  subjects.
- arXiv: cs.SE category, last 5 years, same keywords.

## Candidates considered

### Allamanis, Jackson-Flux, Brockschmidt, "Self-Supervised Bug Detection and Repair" (NeurIPS 2021)

NeurIPS 2021. arXiv 2105.12787.
https://proceedings.neurips.cc/paper/2021/hash/ea96efc03b9a050d895110db8c4af057-Abstract.html

Introduces BugLab, a self-supervised co-training approach for bug
detection and repair, and PyBugLab — its Python implementation. The
Python implementation explicitly targets four bug classes, of which
"Argument Swapping" is one (the others are Variable Misuse, Wrong
Operator, Wrong Literal). The paper also releases PyPIBugs, a
manually curated Python dataset of 2,374 real-world bugs collected
from PyPI packages, used as the evaluation corpus. Quantitative
results report a 30% improvement over baseline name-based detectors
on PyPIBugs and 19 previously unknown bugs found in open-source
Python projects.

Verdict: ACCEPTED.

- Clause (a) is satisfied directly: the paper's experimental
  subjects are Python code (PyPIBugs and follow-up open-source
  Python projects).
- Clause (c) is also satisfied: PyPIBugs is a peer-reviewed Python
  benchmark whose label set includes argument swaps.
- The detection algorithm (graph-neural-network co-training) differs
  from cntrdct's syntactic Rice-style parameter-name match, but the
  bug class is identical (caller passes positional arguments in
  reverse order). Per `citations-policy.md` clause (a), the
  experimental subject overlap is sufficient grounding; algorithmic
  identity is not required.
- Citation key: `allamanis-neurips-2021`.

### Liu, Foyen, Levin, "Out of Sight, Out of Place: Detecting and Assessing Swapped Arguments" (ASE 2020)

IEEE Xplore 9252035. arXiv 2009.09117.

Static-analysis tool SwapD that uses natural-language information in
program identifiers (variable / function / macro names) to detect
mistakenly-swapped arguments at call sites. Evaluated on a corpus of
417 million lines of C and C++ code; reports 154 manually-vetted
real-world swap bugs.

Verdict: rejected. Strong direct lineage of the Rice-style approach,
but the experimental subjects are C and C++ — not Python. Could be
cited as Rust-grounded under the original v0 grandfather clause but
it does not satisfy (a) for Python and there is no independent
peer-reviewed Python application of SwapD published as of 2026-05.

### Pradel, Sen, "DeepBugs: A Learning Approach to Name-based Bug Detection" (OOPSLA 2018)

DOI 10.1145/3276517. arXiv 1805.11683.

Name-based bug detection framework. Three trained detectors target
swapped function arguments, incorrect binary operators, and
incorrect operands. Experimental subjects are 150,000 JavaScript
files (100k train / 50k validation), 68.6 million LOC.

Verdict: rejected for Python clause (a). Subjects are JavaScript
only. The companion `JetBrains-Research/DeepBugsPlugin` IntelliJ
plugin claims Python support, but that is a community tool without
a peer-reviewed publication; it does not satisfy the secondary
peer-reviewed application requirement of clause (b).

### He, Zhou, "A replication of 'DeepBugs: a learning approach to name-based bug detection'" (ESEC/FSE 2021 ROSE Festival)

DOI 10.1145/3468264.3477221.

Partial replication of Pradel-Sen 2018 for the swapped-arguments
detector on the original 150k JavaScript dataset.

Verdict: rejected. Replication is JavaScript-only. Does not extend
DeepBugs to Python.

### Castagna, Lanvin, Petrucciani et al., "Reconciling Type Annotations and Argument Selection in Pair-Programming Environments" (various)

Not located as a peer-reviewed publication in any of the venue
indexes searched. Mentioned in passing in survey result snippets;
unable to verify.

Verdict: rejected on grounds of unverifiable provenance.

### pylint / Pyflakes / Bandit / Ruff / mypy / Pyright

Community tools and / or type-checkers without peer-reviewed
publications backing the specific argument-swap detection rule.
mypy and Pyright catch type-mismatched argument errors but not
the same-typed swap pattern. Ruff offers no swap rule.

Verdict: rejected. Tools, not peer-reviewed publications. Cannot
satisfy (a) / (b) / (c) directly.

### Rice et al ICSE 2017 cited-by graph

Of the ~700 papers citing Rice et al as of 2026, the filtered
top-100 (ranked by citation count and venue) yields no
Python-corpus replication of the parameter-name correlation method
specifically. Java and C++ replications dominate, plus extensions
to Android and Kotlin.

Verdict: no clause-(b) Python secondary application from this
graph beyond Allamanis 2021 (which is captured under its own entry
above and accepted).

### Li-Zhou FSE 2005 cited-by graph

PR-Miner has been extended to many languages (Java, JavaScript,
Smart Contracts, Linux drivers) but a Python replication that
specifically targets argument-swap as a bug class did not surface
in the search beyond Allamanis 2021.

Verdict: same as above.

## Conclusion

Allamanis et al. NeurIPS 2021 (PyBugLab + PyPIBugs) satisfies
clauses (a) and (c) of `docs/spec/citations-policy.md` for the
arg-swap pattern on Python. The detector adds this as a Python
citation; Python findings emit
`LanguageCitationStatus::Confirmed`.

## Decision

- Add `allamanis-neurips-2021` to `Citation::CITATIONS` with
  `languages: &[Language::Python]`.
- Python findings emit `citation_keys` that include
  `allamanis-neurips-2021` alongside the existing Rust-grounded
  citations, and carry `LanguageCitationStatus::Confirmed`.
- `CITATIONS.md` adds a bibliography entry under the arg-swap
  subsection with `Languages: Python`.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed Python application of SwapD (Liu et al. ASE 2020)
  or DeepBugs (Pradel-Sen OOPSLA 2018) is published, which would
  add a clause-(b) citation alongside the existing clause-(a)
  Allamanis citation.
- A peer-reviewed empirical study of Python argument-swap bug
  prevalence using parameter-name correlation (cntrdct's actual
  algorithm) is published, which would tighten the algorithmic
  match between cited grounding and shipped detector.
- A successor or replacement of PyPIBugs is published that
  redefines or removes the argument-swapping label set.
