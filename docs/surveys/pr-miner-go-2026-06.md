# Literature survey: pr-miner → Go

Date: 2026-06-05
Detector: `pr-miner`
Target language: Go
Surveyor: cntrdct R-3.f PR (pr-miner Go grounding)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a
new language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has Go source as its
experimental subject for implicit-programming-rule / frequent-itemset
API-usage-rule mining and violation detection, or (b) is the
language-agnostic algorithm we already cite (frequent-itemset /
association-rule API-usage-rule mining, the PR-Miner family) PLUS an
independent peer-reviewed paper applying that algorithm to Go with
quantitative evaluation on a Go corpus, or (c) introduces a Go
API-usage-rule / specification-mining benchmark / dataset relevant to
the detection.

A hard rule constrains clause (a): subjects in Java / C / C++ /
JavaScript / TypeScript / Python ONLY do NOT satisfy clause (a) for
Go. Go must be an actual experimental subject — not a language merely
mentioned in passing, and not a tool's host language. Preprints and
released tools without a peer-reviewed quantitative evaluation do not
qualify; the strongest such candidates are recorded below as rejected.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search. It mirrors the TypeScript precedent
(`docs/surveys/pr-miner-typescript-2026-06.md`) and the Python
precedent (`docs/surveys/pr-miner-python-2026-05.md`), both of which
returned Unconfirmed.

## Detector concept (what must be grounded)

`pr-miner` mines pairwise implicit programming rules via bounded
Apriori frequent-itemset mining over the set of call-head
last-segments in each function, then flags functions that satisfy the
antecedent but violate the consequent (e.g. a function that calls
`beginTx()` but never `commitTx()`). The grounding question is
therefore narrow: does any peer-reviewed work mine implicit pairwise
call-co-occurrence rules from a Go corpus with frequent-itemset /
association-rule techniques and detect violations, with a quantitative
evaluation? A Go-subject bug study that does not run this class of
algorithm does not ground the detector's method; a frequent-itemset
miner evaluated only on non-Go corpora does not ground the Go
extension.

## Existing Rust citations

The detector currently cites:

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically
  Extracting Implicit Programming Rules and Detecting Violations in
  Large Software Code", ESEC/FSE 2005. Subjects: C/C++ (Linux
  kernel, PostgreSQL Server, Apache HTTP Server). Algorithm:
  frequent-itemset mining (Apriori-style) over intra-procedural call
  sets, followed by violation detection on the same database.

`li-zhou-fse-2005` is grandfathered as Rust-grounded under the
unrevised clause (b) when cntrdct shipped the v0 detectors. The Go
extension follows the strict clause (b): a secondary peer-reviewed Go
application of the algorithm is required, or a clause (a) / (c)
candidate.

A methodology reference (Apriori, Agrawal-Srikant VLDB 1994)
underlies the algorithm but is not itself a detector citation per
`citations-policy.md` (it does not introduce the detection concept).

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / web: queries `Go API usage rule mining
  specification mining violation detection`, `frequent itemset
  association rule mining implicit programming rules Go source code
  bug detection`, `Go API misuse empirical study mining patterns open
  source corpus precision recall`, `Golang error handling defer
  concurrency bug pattern mining detection empirical study`,
  `specification mining Go programs temporal API protocol inference
  automata`, `"Go" OR "Golang" frequent itemset mining function call
  pairs implicit rules violations corpus evaluation`, `Go linter
  static analysis study mining checker rules from Go repositories
  evaluation precision`, `PR-Miner replication Go corpus frequent
  itemset programming rules`.
- ACM Digital Library / IEEE Xplore / Springer: venues MSR / ICSE /
  FSE / ASE / ICSME / ISSTA / ASPLOS / Internetware / KSEM and
  TSE / TOSEM / JSS journals, keywords `Go` / `Golang` AND
  (`API misuse` OR `rule mining` OR `frequent itemset` OR
  `association rule` OR `programming rules` OR `specification
  mining`).
- dblp / Semantic Scholar: cited-by graph of `li-zhou-fse-2005`
  (PR-Miner), NAR-Miner (FSE 2018), and the AUGP / MUBench cluster,
  filtered for Go-language subjects.
- MUBench project (`https://github.com/stg-tud/MUBench`) checked for
  Go coverage.
- Go-specific defect-study cluster (the `system-pclub` Go-concurrency
  line, GCatch/GFix, GRace, BinGo) checked for frequent-itemset rule
  mining methodology.

## Candidates considered

### Tu, Liu, Song, Zhang, "Understanding Real-World Concurrency Bugs in Go" (ASPLOS 2019)

Peer-reviewed, Tier-A venue (24th International Conference on
Architectural Support for Programming Languages and Operating
Systems, ASPLOS '19, pp. 865-878, DOI 10.1145/3297858.3304069).
Subjects: genuine Go — 171 concurrency bugs manually collected and
categorised from six production-grade open-source Go applications
(Docker, Kubernetes, etcd, gRPC, CockroachDB, BoltDB). The clearest
clause-(a) candidate the search produced: Go is unambiguously the
experimental subject, and the study establishes that
misuse-of-channel and shared-memory pairing bugs are widespread in
real Go code.

Verdict: rejected as a qualifying citation; recorded as the strongest
Go-subject supporting-context candidate. It is a manual taxonomy /
empirical bug study, not a frequent-itemset / association-rule rule
miner — no algorithmic match for clause (b), and no clause-(a) match
for pr-miner's *method* (the policy's clause (a) for this detector
requires Go as the subject of implicit-rule / frequent-itemset mining,
not of any bug study). Its bug categories are hand-derived, not mined
as call-co-occurrence itemsets, and it ships no labelled benchmark of
API-pairing-rule violations (no clause (c) match). Citing it as
pr-miner's Go grounding would mislead readers about what algorithm
pr-miner actually runs — the same reasoning the TypeScript precedent
applied to Tang et al. (MSR '26) and the Python precedent applied to
Frantz et al. (TSE 2024). It grounds the detector's *relevance* to Go
but does not flip the status to Confirmed.
URL: <https://dl.acm.org/doi/abs/10.1145/3297858.3304069>
(PDF <https://songlh.github.io/paper/go-study.pdf>; bug corpus
<https://github.com/system-pclub/go-concurrency-bugs>)

### Liu, Zhu, Qin, Chen, Song, "Automatically Detecting and Fixing Concurrency Bugs in Go Software Systems" (GCatch/GFix, ASPLOS 2021)

Peer-reviewed, Tier-A venue (26th ASPLOS, DOI 10.1145/3445814.3446756).
Subjects: genuine Go — GCatch is a suite of static detectors evaluated
on 21 open-source Go projects (Docker, Kubernetes, gRPC, …), detecting
149 previously-unknown blocking-misuse-of-channel (BMOC) bugs plus 119
traditional concurrency bugs, with GFix synthesising patches for 124
BMOC bugs. Quantitative (true/false-positive counts reported).

Verdict: rejected. Go is a real subject and the evaluation is
quantitative, but the method is static analysis over a channel-state
constraint system solved with an SMT/constraint solver — not
frequent-itemset / association-rule mining of implicit co-occurrence
rules. No clause (b) algorithmic match. The pairing it reasons about
(send/receive, lock/unlock) is hard-coded from Go channel semantics,
not *mined* as a frequent itemset over call-head sets the way pr-miner
operates. It detects a fixed, language-specific bug class rather than
inferring rules from frequency. Not a benchmark/dataset paper either
(clause c). It is the strongest *quantitative Go-subject* candidate
but is algorithmically orthogonal to pr-miner.
URL: <https://dl.acm.org/doi/abs/10.1145/3445814.3446756>
(open PDF <https://par.nsf.gov/servlets/purl/10226304>;
tool <https://github.com/system-pclub/GCatch>)

### Wu, Xu, Qin, "Detecting API-Misuse Based on Pattern Mining via API Usage Graph with Parameters" (AUGP, KSEM 2023, LNCS)

The strongest algorithmic match found (same candidate the TypeScript
survey flagged). Extracts API usages as API Usage Graphs with
Parameters (AUGP) and mines binary API rules with the FP-growth
frequent-itemset algorithm — the same association-rule /
frequent-itemset family as PR-Miner — then flags co-occurrence and
order violations. Evaluated against five state-of-the-art detectors on
the public MUBench benchmark, reporting the highest precision and F1.

Verdict: rejected for Go coverage. The algorithm is the right family
for clause (b), but its quantitative evaluation is on MUBench, whose
subjects are Java. Clause (b) requires the secondary application paper
to evaluate on a corpus in the target language; this paper evaluates
on Java, not Go. No Go evaluation is present.
URL: <https://link.springer.com/chapter/10.1007/978-3-031-35257-7_21>
(DOI 10.1007/978-3-031-35257-7_21)

### Bian, Liang, Zhang, et al., "NAR-Miner: Discovering Negative Association Rules from Code for Bug Detection" (FSE 2018)

A direct descendant of PR-Miner and the closest *methodological*
sibling: it transforms program elements and their semantic
relationships within each function into transactions, mines frequent
and infrequent itemsets, infers negative association rules, and ranks
them by confidence and entropy to detect violations. This is squarely
the frequent-itemset / association-rule family pr-miner belongs to.

Verdict: rejected for Go coverage. Subjects are C — the evaluation is
on the Linux kernel and comparable C systems, the same corpus lineage
as the original PR-Miner. C subjects do not satisfy clause (a) for Go
(hard rule), and the paper is the *algorithm*, not an independent
application to a Go corpus, so it cannot serve as the clause-(b)
secondary citation for Go either.
URL: <https://hjjandy.github.io/docs/fse18_NARMiner.pdf>

### "Discovering API usage specifications for security detection using two-stage code mining" (Cybersecurity, Springer, 2024)

Two-stage approach: frequent API-set mining (frequent common API
identification + filtration to extract maximal frequent
context-sensitive API sequences) followed by an API relationship graph
built from symbolic path information to mine multi-API specifications.
The first stage is genuinely frequent-itemset-family mining of API
co-occurrence, so it was examined closely.

Verdict: rejected for Go coverage. The paper is paywalled behind a
Springer IDP redirect that the fetch could not clear, so the subject
corpus could not be confirmed as Go; the security-API-specification-
mining literature this paper sits in is overwhelmingly C/C++-centric
(binary / source security analysis), and nothing in the abstract or
indexing identifies Go as an experimental subject. Absent a confirmed
Go corpus and a quantitative Go evaluation, it satisfies neither
clause (a) nor clause (b) for Go. Recorded as a could-not-confirm
rejection rather than a match.
URL: <https://link.springer.com/article/10.1186/s42400-024-00224-w>
(DOI 10.1186/s42400-024-00224-w)

### Perracotta / temporal specification miners (Yang et al., ICSE 2006; and the FSA-inference line)

Temporal API-rule miners (Perracotta and successors) infer ordering
properties such as `a` must precede `b` from execution traces, then
flag deviations — conceptually adjacent to pr-miner's pairing rules.

Verdict: rejected on two independent grounds. (1) Subjects are Java /
C trace data, not Go. (2) The method is trace-based finite-state
automaton / temporal-property inference, not frequent-itemset mining
over static call sets — no clause (b) algorithmic match, and the
detection target (temporal ordering) differs from pr-miner's
co-occurrence pairing. No Go corpus is evaluated.
URL: <https://www.cs.virginia.edu/perracotta/>

### MUBench (Amann, Nadi, et al., MSR 2016; systematic-evaluation line)

MUBench is the established API-misuse-detector benchmark and would be
the natural clause-(c) anchor if it covered Go.

Verdict: rejected for clause (c). MUBench's misuse cases are drawn
from Java projects; it is a Java benchmark. No Go variant or Go misuse
cases exist in the dataset as of the search date.
URL: <https://github.com/stg-tud/MUBench>

### Go static-analysis tool ecosystem (Staticcheck, go-critic, go vet, errcheck)

Production Go linters encode hard-coded checks (some resembling
pairing rules, e.g. errcheck's unchecked-error detection) and operate
on real Go code at scale.

Verdict: rejected. These are engineering tools, not peer-reviewed
publications, and they do not *mine* rules via frequent-itemset /
association-rule techniques — their checks are hand-authored. Policy
excludes tools without a qualifying peer-reviewed quantitative
evaluation. No clause (a)/(b)/(c) match.
URL: <https://staticcheck.dev/>

### PR-Miner cited-by graph filtered for Go

The descendants of `li-zhou-fse-2005` that perform frequent-itemset /
association-rule API-rule mining — NAR-Miner (above), AUGP (above),
JADET-line object-usage anomaly miners, change-rule and graph-pattern
API-misuse detectors — evaluate on Java or C/C++. The Go-subject works
that surfaced (the ASPLOS '19 concurrency-bug study, the ASPLOS '21
GCatch detector, the broader `system-pclub` Go line) are manual
studies or static/dynamic concurrency analysers, not frequent-itemset
rule miners. The post-2022 long tail folds rule inference into
language-model fine-tuning, which does not invoke frequent-itemset
mining and therefore fails clause (b)'s algorithmic requirement.

Verdict: no clean qualifying clause-(b) Go application found.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the pr-miner pattern on Go:

- Clause (b) (the strongest target): the exact algorithm family —
  frequent-itemset / FP-growth / negative-association API-rule mining
  — has peer-reviewed instances (Wu et al. AUGP, KSEM 2023; NAR-Miner,
  FSE 2018), but their quantitative evaluations are on Java (MUBench)
  and C (Linux kernel) respectively. No peer-reviewed paper applies
  the algorithm to a Go corpus with quantitative evaluation.
- Clause (a): the strongest Go-subject candidates (Tu et al., ASPLOS
  '19; Liu et al. GCatch, ASPLOS '21) are a manual bug taxonomy and a
  static constraint-solver detector respectively. Both have Go as a
  genuine subject, but neither mines implicit call-pairing rules via
  frequent-itemset techniques; citing either as pr-miner's algorithmic
  grounding would misrepresent what the detector runs (the same
  reasoning the TypeScript precedent applied to Tang et al. and the
  Python precedent to Frantz et al.).
- Clause (c): the established API-misuse benchmark (MUBench) is Java;
  no Go API-pairing-rule-violation dataset exists.

This sits at roughly the same strength as the TypeScript result: the
ASPLOS '19 study firmly grounds "implicit pairing/channel bugs are
widespread in real Go code", and the AUGP/NAR-Miner line proves the
algorithm transfers in principle — but neither closes the gap the
policy requires, so the honest verdict remains Unconfirmed.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(pr-miner Go coverage:
  unconfirmed; survey notes at this file)` line under the detector's
  subsection so the consistency test sees an acknowledged gap rather
  than a silent one. (Owner: integrating PR, not this survey file.)
- The detector emits `LanguageCitationStatus::Unconfirmed` on every
  Go finding (consistent with `make_finding` mapping every non-Rust
  language to `Unconfirmed`). SARIF consumers can filter or visually
  flag indirectly-grounded Go results via
  `properties.languageCitationStatus`.
- The Rust subset stays `Confirmed` against `li-zhou-fse-2005` under
  the grandfather clause.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies frequent-itemset / association-rule /
  FP-growth API-rule mining to Go source code with quantitative
  evaluation on a rule-violation corpus (clause b candidate) — e.g. a
  Go extension/replication of the AUGP or NAR-Miner approach over a Go
  corpus such as the `system-pclub` Go projects.
- A new Go benchmark / dataset specifically labels API-pairing rule
  violations (clause c candidate; a MUBench-style corpus with Go
  misuse cases).
- A follow-up promotes the ASPLOS '19 Go concurrency-bug taxonomy into
  a labelled, mineable Go API-misuse benchmark, or pairs it with a
  frequent-itemset miner evaluated on the Go corpus.
