# Literature survey: pr-miner → TypeScript

Date: 2026-06-05
Detector: `pr-miner`
Target language: TypeScript
Surveyor: cntrdct R-2.f PR (team `r2f-ts-surveys`)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a
new language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has TypeScript source as
its experimental subject, or (b) is the language-agnostic algorithm
we already cite (frequent-itemset / association-rule API-usage-rule
mining, the PR-Miner family) PLUS an independent peer-reviewed paper
applying that algorithm to TypeScript with quantitative evaluation on
a TypeScript corpus, or (c) introduces a TypeScript benchmark /
dataset of API-usage-rule violations relevant to the detection.

JavaScript is not TypeScript: a paper whose subjects are JavaScript
(or whose only TypeScript content is type declarations used to
analyse JavaScript) does not satisfy clause (a) for TypeScript.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search. It mirrors the Python precedent
(`docs/surveys/pr-miner-python-2026-05.md`), which also returned
Unconfirmed.

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
TypeScript extension follows the strict clause (b): a secondary
peer-reviewed TypeScript application of the algorithm is required, or
a clause (a) / (c) candidate.

A methodology reference (Apriori, Agrawal-Srikant VLDB 1994)
underlies the algorithm but is not itself a detector citation per
`citations-policy.md` (it does not introduce the detection concept).

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / web: queries `TypeScript "PR-Miner"`, `TypeScript
  "frequent itemset" API rules mining`, `TypeScript "association
  rule" "API misuse" detection`, `TypeScript "implicit programming
  rules" mining`, `JavaScript TypeScript API usage pattern mining
  specification mining npm`, `TypeScript empirical study API misuse
  benchmark dataset mining bugs`, `frequent itemset mining implicit
  programming rules violation detection TypeScript precision recall`.
- ACM Digital Library / IEEE Xplore / Springer: venue MSR / ICSE /
  FSE / ASE / ICSME / ISSTA / TASE / KSEM and TSE / TOSEM / JSS
  journals, keywords `TypeScript` AND (`API misuse` OR `rule mining`
  OR `frequent itemset` OR `association rule` OR `programming
  rules`).
- dblp / Semantic Scholar: cited-by graph of `li-zhou-fse-2005` and
  the MUBench cluster, filtered for TypeScript-language subjects.
- MUBench project (`https://github.com/stg-tud/MUBench`) checked for
  TypeScript coverage.

## Candidates considered

### Wu, Xu, Qin, "Detecting API-Misuse Based on Pattern Mining via API Usage Graph with Parameters" (KSEM 2023, LNCS)

The strongest algorithmic match found. Extracts API usages as API
Usage Graphs with Parameters (AUGP) and mines binary API rules with
the FP-growth frequent-itemset algorithm — the same association-rule
/ frequent-itemset family as PR-Miner — then flags co-occurrence and
order violations. Evaluated against five state-of-the-art detectors
on the public MUBench benchmark, reporting the highest precision and
F1.

Verdict: rejected for TypeScript coverage. The algorithm is the right
family for clause (b), but its quantitative evaluation is on MUBench,
whose subjects are Java. Clause (b) requires the secondary
application paper to evaluate on a corpus in the target language;
this paper evaluates on Java, not TypeScript. No TypeScript
evaluation is present.
URL: <https://link.springer.com/chapter/10.1007/978-3-031-35257-7_21>
(DOI 10.1007/978-3-031-35257-7_21)

### Tang, Alimadadi, Sumner, "From Logic to Toolchains: An Empirical Study of Bugs in the TypeScript Ecosystem" (MSR 2026)

Peer-reviewed (23rd International Conference on Mining Software
Repositories, MSR '26). Subjects: TypeScript — 633 bug reports
(commits, issues, PRs) mined from 16 actively maintained open-source
TypeScript repositories, manually annotated into an 11-category fault
taxonomy. "API Misuse" is the single most prevalent category (14.5%
of annotated faults), defined as incorrect usage of internal or
third-party APIs.

Verdict: rejected as a qualifying citation; recorded as the strongest
supporting-context candidate. This is the clearest clause-(a)
candidate the search produced — its subjects are genuine TypeScript
code and it establishes that API misuse is the leading bug category
in the TypeScript ecosystem. However, it is a manual bug taxonomy
study: it does not mine implicit API-pairing rules with
frequent-itemset / association-rule techniques (no algorithmic match
for clause (b)), and its "dataset" is categorised bug reports rather
than a labelled benchmark of API-pairing rule violations (no clause
(c) match). Its "API Misuse" label is a broad bucket — wrong method,
wrong argument, wrong protocol — not the specific co-call pairing
violation (`a` present, paired `b` absent) that pr-miner detects.
Following the Python precedent's standard, which rejected the
clause-(a)-defensible Frantz et al. (TSE 2024) on the same
algorithmic-mismatch grounds, citing this taxonomy study as
pr-miner's TypeScript grounding would mislead readers about what
algorithm pr-miner actually runs. It grounds the detector's relevance
to TypeScript but does not flip the status to Confirmed.
URL: <https://arxiv.org/abs/2601.21186> (MSR '26;
<https://www.semanticscholar.org/paper/From-Logic-to-Toolchains:-An-Empirical-Study-of-in-Tang-Alimadadi/d8f67f8dd1c4f6ed6372264864b113bdac240d89>)

### "An empirical study on bugs in TypeScript programming language" (Journal of Systems and Software, 2025)

Peer-reviewed journal (Elsevier JSS, 2025, DOI
10.1016/j.jss.2025.112445). Collected 49,375 closed issues from the
TypeScript project and analysed ~8,800 fixed bug reports.

Verdict: rejected. The subjects are bugs in the TypeScript compiler /
language implementation itself (the `microsoft/TypeScript`
repository's issue tracker), not API-pairing rule violations mined
from TypeScript application code. No frequent-itemset / association-
rule mining; no API-usage-rule benchmark. It is at best clause-(a)
adjacent (TypeScript is the subject domain) but has no algorithmic or
benchmark overlap with pr-miner's detection.
URL: <https://www.sciencedirect.com/science/article/abs/pii/S016412122500113X>
(DOI 10.1016/j.jss.2025.112445)

### Park, Ryu, et al., "JavaScript API Misuse Detection by Using TypeScript" (MODULARITY '14 companion / extended abstract)

Title mentions TypeScript, so examined closely. The method uses
TypeScript declaration files from DefinitelyTyped as type
specifications and extends the SAFE static-analysis framework to flag
errors in JavaScript programs that import libraries such as jQuery
and MooTools.

Verdict: rejected on three independent grounds. (1) The analysed
subjects are JavaScript programs, not TypeScript code — TypeScript
appears only as the source of type declarations, which does not
satisfy clause (a) ("subjects must include actual TypeScript code").
(2) It performs type-based static analysis, not frequent-itemset /
association-rule rule mining — no clause (b) algorithmic match. (3)
It is a companion-volume extended abstract (workshop-tier), below the
peer-reviewed-with-quantitative-evaluation bar.
URL: <https://dl.acm.org/doi/abs/10.1145/2584469.2584472>
(<https://plrg.korea.ac.kr/assets/data/publication/mod-src14.pdf>)

### Sahai et al. / ICSE 2014, "Mining interprocedural, data-oriented usage patterns in JavaScript web applications"

Mines interprocedural API usage patterns from JavaScript web
applications. Algorithmically adjacent to PR-Miner-style usage-rule
mining.

Verdict: rejected for TypeScript coverage. Subjects are JavaScript,
not TypeScript. JavaScript is not TypeScript per the policy and the
team's explicit rule; no TypeScript corpus is evaluated.
URL: <https://dl.acm.org/doi/10.1145/2568225.2568302>

### MUBench / MUBench-Pipe (Amann, Nadi, et al., MSR 2016; systematic evaluation arXiv→ICSE/journal line)

MUBench is the established API-misuse-detector benchmark and would be
the natural clause-(c) anchor if it covered TypeScript.

Verdict: rejected for clause (c). MUBench's 89 misuses are drawn from
Java projects; it is a Java benchmark. No TypeScript variant or
TypeScript misuse cases exist in the dataset as of the search date.
URL: <https://github.com/stg-tud/MUBench>
(<https://ieeexplore.ieee.org/document/7832926/>)

### PR-Miner cited-by graph filtered for TypeScript

The descendants of `li-zhou-fse-2005` (PR-Miner) that perform
frequent-itemset / association-rule API-rule mining — AUGP (above),
JADET-line object-usage anomaly miners, change-rule and
graph-pattern API-misuse detectors — evaluate on Java or C/C++. The
TypeScript-subject works that surfaced (the MSR '26 taxonomy, the JSS
'25 compiler-bug study) are empirical bug studies, not rule miners.
The post-2022 long tail folds rule inference into language-model
fine-tuning, which does not invoke frequent-itemset mining and
therefore fails clause (b)'s algorithmic requirement.

Verdict: no clean qualifying clause-(b) TypeScript application found.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the pr-miner pattern on
TypeScript:

- Clause (b) (the strongest target): the exact algorithm family —
  frequent-itemset / FP-growth API-rule mining — has a peer-reviewed
  secondary application (Wu et al., AUGP, KSEM 2023), but its
  quantitative evaluation is on Java (MUBench), not TypeScript. No
  peer-reviewed paper applies the algorithm to a TypeScript corpus
  with quantitative evaluation.
- Clause (a): the strongest TypeScript-subject candidate (Tang et
  al., MSR '26) is a manual bug taxonomy, not a rule miner; citing it
  as pr-miner's algorithmic grounding would misrepresent what the
  detector runs (same reasoning the Python precedent applied to
  Frantz et al.).
- Clause (c): the established API-misuse benchmark (MUBench) is Java;
  no TypeScript API-pairing-rule-violation dataset exists.

This is one notch stronger than the Python result — the MSR '26 study
firmly grounds "API misuse is the leading bug category on real
TypeScript code", and the AUGP paper proves the algorithm transfers
in principle — but neither closes the gap the policy requires, so the
honest verdict remains Unconfirmed.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(pr-miner TypeScript coverage:
  unconfirmed; survey notes at this file)` line under the detector's
  subsection so the consistency test sees an acknowledged gap rather
  than a silent one.
- The detector emits `LanguageCitationStatus::Unconfirmed` on every
  TypeScript finding (already the behaviour: `make_finding` maps every
  non-Rust language to `Unconfirmed`). SARIF consumers can filter or
  visually flag indirectly-grounded TypeScript results via
  `properties.languageCitationStatus`.
- The `[Language::Rust, Language::Python, Language::TypeScript]`
  declaration ships with this status; the Rust subset stays
  `Confirmed` against `li-zhou-fse-2005` under the grandfather clause.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies frequent-itemset / association-rule /
  FP-growth API-rule mining to TypeScript source code with
  quantitative evaluation on a rule-violation corpus (clause b
  candidate) — e.g. a TypeScript extension/replication of the AUGP
  approach.
- A new TypeScript benchmark / dataset specifically labels
  API-pairing rule violations (clause c candidate; a MUBench-style
  corpus with TypeScript misuse cases).
- An MSR '26 follow-up promotes the "API Misuse" taxonomy category
  into a labelled, mineable TypeScript API-misuse benchmark.
