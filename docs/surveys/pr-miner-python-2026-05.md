# Literature survey: pr-miner → Python

Date: 2026-05-06
Detector: `pr-miner`
Target language: Python
Surveyor: cntrdct P-2 PR

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a
new language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has Python source as
its experimental subject, or (b) is the language-agnostic algorithm
we already cite plus an independent peer-reviewed paper applying it
to Python with quantitative evaluation, or (c) introduces a Python
benchmark / dataset relevant to the detection.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search.

## Existing Rust citations

The detector currently cites:

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically
  Extracting Implicit Programming Rules and Detecting Violations in
  Large Software Code", ESEC/FSE 2005. Subjects: C/C++ (Linux
  kernel, PostgreSQL Server, Apache HTTP Server). Algorithm:
  frequent-itemset mining (Apriori-style) over intra-procedural call
  sets, followed by violation detection on the same database.

`li-zhou-fse-2005` is grandfathered as Rust-grounded under the
unrevised clause (b) when cntrdct shipped the v0 detectors. The
Python extension follows the strict clause (b): a secondary peer-
reviewed Python application of the algorithm is required, or a
clause (a) / (c) candidate.

A methodology reference (`agrawal-vldb-1994`, Apriori) accompanies
`li-zhou-fse-2005` in the spec but is not itself a detector
citation per `citations-policy.md` (it does not introduce the
detection concept).

## Search

Databases / sources queried (2026-05-06):

- Google Scholar: queries `python "PR-Miner" replication`, `python
  "frequent itemset" source code "API rules"`, `python "association
  rule" "API misuse" detection`, `python "implicit programming
  rules" mining`, `python "specification mining" rule violation
  static analysis`.
- ACM Digital Library: filters venue ICSE / FSE / ASE / ICSME / MSR
  / ISSTA 2010-2026, keywords `Python` AND (`API misuse` OR `rule
  mining` OR `frequent itemset` OR `programming rules`).
- IEEE Xplore: same venues + TSE / TOSEM journals, same keywords.
- dblp: cited-by graph of `li-zhou-fse-2005`,
  `acharya-fse-2007`, `wasylkowski-fse-2007`, filtered for Python-
  language subjects.
- arXiv: cs.SE category, last 6 years, same keywords.
- ASERG mining-software-engineering bibliography
  (`https://sites.google.com/site/asergrp/projects/codemining`).

## Candidates considered

### Acharya, Xie, Pei, Xu, "Mining API Patterns as Partial Orders from Source Code" (ESEC/FSE 2007)

Mines partial-order specifications of API usage from C source
(X11, OpenSSL). Generalises PR-Miner's intra-procedural item
mining to ordering relations.

Verdict: rejected for Python coverage. Subjects are C; no
peer-reviewed independent application to Python found in the
cited-by graph as of the search date. Clause (b) requires the
secondary application to exist and be peer-reviewed; absence is
fatal.

### Wasylkowski, Zeller, Lindig, "Detecting Object Usage Anomalies" (ESEC/FSE 2007)

Mines temporal object-usage specifications from Java code (AspectJ
and four other Java systems) and flags deviations as anomalies.
Algorithmically related to PR-Miner (rule mining + violation
flagging) but the rule shape is different (object-protocol partial
orders rather than intra-procedural co-call pairs).

Verdict: rejected for Python coverage. Subjects are Java; the
JADET prototype was never published with Python evaluation. dblp's
Wasylkowski cluster shows no Python-subjects follow-up.

### Engler, Chen, Chou, "Bugs as Deviant Behavior" (SOSP 2001)

The conceptual ancestor of PR-Miner — bugs framed as deviations
from inferred patterns. Subjects: C (Linux kernel, OpenBSD).
Already cited in CITATIONS.md for `clone-drift` and others.

Verdict: rejected for Python coverage. C-only subjects; no
peer-reviewed Python replication of the deviant-behavior framing
found in the search. Even if found, the framing is not
algorithmically tied to frequent-itemset mining specifically —
satisfying clause (b) for Engler would justify a different
detector (`clone-drift` or a generic deviance miner), not
pr-miner.

### Allamanis, Jackson-Flux, Brockschmidt, "Self-Supervised Bug Detection and Repair" (PyBugLab / PyPIBugs, NeurIPS 2021)

Introduces PyPIBugs, a 2374-bug Python benchmark drawn from PyPI
package commits, and BugLab, a self-supervised model trained on
synthetic-bug injection. Targets "stupid simple bugs" (variable
misuse, wrong operator, wrong literal, swapped arguments, wrong
assignment, etc.).

NeurIPS is peer-reviewed. PyPIBugs is publicly hosted at
`https://www.microsoft.com/en-us/download/103554`. Already cited in
`CITATIONS.md` under `arg-swap` (the swapped-argument category in
PyPIBugs is the direct ground truth for that detector).

Verdict for pr-miner: borderline reject. The PyPIBugs taxonomy is
local-token-level mistakes rather than API-pairing rule violations.
PR-Miner's prototypical bug ("function calls `acquire()` but never
`release()`") is not a labelled category in PyPIBugs. Citing it
under pr-miner would conflate two distinct bug families and dilute
the evidence the citation is supposed to provide.

### Frantz, Xiao, Pias, Meng, Yao, "Methods and Benchmark for Detecting Cryptographic API Misuses in Python" (IEEE TSE, 2024)

Peer-reviewed (IEEE Transactions on Software Engineering, 2024).
Subjects: Python source code targeting cryptographic libraries.
Builds a benchmark of 18 misuse patterns with benign / vulnerable
/ non-usage cases per pattern, evaluates Bandit, Semgrep, and
Dlint as detectors.

Methodology: hand-curated rule patterns + forward / backward
slicing. Rule MINING is not part of the contribution; the rules
are author-authored.

Verdict: rejected for clause (b). Clause (b) demands secondary
peer-reviewed application of the cited algorithm (frequent-itemset
mining); Frantz et al. do not mine rules. The paper is
clause-(a) defensible — its experimental subjects are Python and
the bug class (API misuse) overlaps pr-miner's detection target —
but the algorithmic mismatch is severe enough that citing it as
pr-miner's Python grounding would mislead readers about what
algorithm pr-miner actually runs. Marked as a clause-(c)-adjacent
candidate (introduces a Python misuse benchmark) but the benchmark
is crypto-specific, not the API-pairing pattern pr-miner targets.

### Anonymised, "An Empirical Study of API Misuses of Data-Centric Libraries" (ACM venue, 2024; arXiv 2408.15853)

Peer-reviewed ACM-prefix DOI (10.1145/3674805.3686685). Subjects:
NumPy, pandas, scikit-learn, Matplotlib, seaborn (Python).
Methodology: manual analysis of 345 Stack Overflow posts and 358
GitHub commits to enumerate 49 API-misuse cases, organised into a
new taxonomy with a "data dependency" axis.

Verdict: rejected for clause (b). Manual taxonomy from QA + commit
mining is not the rule-mining algorithm pr-miner implements. The
paper is a clause-(a) candidate: it grounds the existence of API
misuse as a problem in Python data libraries. As pr-miner's
algorithmic citation it is a poor fit (no algorithm overlap), but
the taxonomy could justify the detector's relevance to Python
practitioners. Not strong enough on its own to flip the citation
status from Unconfirmed; recorded as supporting context.

### Widyasari et al., "BugsInPy: A Database of Existing Bugs in Python Programs" (ESEC/FSE 2020 Tool Demos)

Introduces BugsInPy: 493 real bugs from 17 Python projects
(machine learning, dev tools, scientific computing, web frameworks)
with reproducer test cases. Inspired by Defects4J.

Verdict: rejected for clause (c). The dataset's bug labels are
generic (test-failure-reproducing diffs, no fine-grained bug-class
schema). pr-miner's specific target (API-pairing rule violations)
cannot be cleanly extracted from BugsInPy without manual re-
labelling of every entry. Recording it as available context but
not as a qualifying clause-(c) citation.

### Cordy, "A Language-Agnostic Framework for Mining Static Analysis Rules from Code Changes" (ICSE-SEIP 2023)

Mines static-analysis rules from version-control history rather
than from a single code snapshot. Language-agnostic by
construction.

Verdict: paper not accessible from the search infrastructure as of
the search date (DOI 10.1109/ICSE-SEIP58684.2023.00035 returned
403 to automated fetches). Recorded as a future-revisit candidate;
the abstract did not surface a Python-specific evaluation but the
"language-agnostic" framing suggests at least language-portable
methodology. Not used in the current verdict because the paper's
contents could not be confirmed.

### iComment / aComment / PR-Miner cited-by graph filtered for Python

Of the ~3000 papers citing `li-zhou-fse-2005` as of 2026, the
filtered top-200 (ranked by venue and citation count) yielded the
clusters above plus a long tail of:

- Java / C++ replications and extensions (e.g. JADET descendants,
  Engler-style miners on Linux), which do not contribute to
  Python coverage.
- Configuration / NLP-rule-mining works that are off-target for
  pr-miner's source-code rule scope.
- LLM-era work (post-2022) that subsumes rule mining into
  language-model fine-tuning; these do not invoke frequent-itemset
  mining and therefore do not satisfy clause (b)'s algorithmic
  requirement.

Verdict: no clean qualifying clause-(b) Python application found.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the pr-miner pattern on
Python. The closest candidates (Frantz TSE 2024, the data-centric
libraries empirical study, PyPIBugs / PyBugLab) are clause-(a) /
clause-(c) adjacent — they ground "API misuse on Python source"
as a real research target — but each fails the algorithmic match
required by clause (b) (none mines rules with frequent-itemset /
association-rule techniques) and none labels the specific
API-pairing-violation bug class pr-miner detects in a way that
would let us count it as a clause-(c) benchmark.

The detector ships its Python extension regardless: P1 itself
remains satisfied (the detector continues to declare
`li-zhou-fse-2005`, which is non-empty), and the per-language gap
is captured in metadata via `LanguageCitationStatus::Unconfirmed`
exactly as `comment-code` and `unreachable-after-terminator`
ship today.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(pr-miner Python coverage:
  unconfirmed; survey notes at this file)` line under the
  detector's subsection so the consistency test sees an
  acknowledged gap rather than a silent one.
- The detector emits `LanguageCitationStatus::Unconfirmed` on every
  Python finding. SARIF consumers can filter or visually flag
  indirectly-grounded Python results via
  `properties.languageCitationStatus`.
- v0.1 ships the `[Language::Rust, Language::Python]` declaration
  with this status; the Rust subset stays `Confirmed` against
  `li-zhou-fse-2005` under the grandfather clause.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies frequent-itemset / association-rule
  mining to Python source code with quantitative evaluation on a
  rule-violation corpus (clause b candidate).
- A new Python benchmark / dataset specifically labels API-pairing
  rule violations (clause c candidate; PyPIBugs-style but with
  pairing labels rather than stupid-simple-bug labels).
- The "Language-Agnostic Framework for Mining Static Analysis Rules
  from Code Changes" (Cordy, ICSE-SEIP 2023) full text becomes
  accessible and includes a Python-corpus evaluation.
- A peer-reviewed extension of Frantz et al. broadens the
  cryptographic API misuse benchmark to general API-pairing
  patterns on Python.
