# Literature survey: clone-drift → Go

Date: 2026-06-05
Detector: `clone-drift`
Target language: Go
Surveyor: cntrdct R-3.f PR (clone-drift Go grounding)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has Go source as its
experimental subject for near-miss / inconsistent code-clone
detection, or (b) is the language-agnostic algorithm we already cite
(NiCad / the inconsistent-clone-change analysis) plus an independent
peer-reviewed paper applying that algorithm to Go with quantitative
evaluation on a Go corpus (both papers cited), or (c) introduces a Go
benchmark / dataset relevant to the detection.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search.

Note on the honesty bar (carried over from the Python and TypeScript
surveys and the R-2 / R-3 series): Java is NOT Go, C is NOT Go,
JavaScript / TypeScript is NOT Go. A corpus drawn from any of those
languages does not satisfy clause (a) for Go. Preprints (including
withdrawn ones), workshop notes below the IEEE/ACM peer-review tier,
community tools without a qualifying peer-reviewed paper (e.g. `dupl`,
`gocyclo`), tool documentation, and the cntrdct project itself do not
qualify under any clause. Go must be an actual experimental subject —
a "supported language" entry in a parser-coverage / scalability table
is not a quantitative clone-detection evaluation on a Go corpus.

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
  NOT include Go.

None of the four ground Go. The clone-drift detector declares
`Language::Go` in `supported_languages()` and runs the same
function-level NiCad-style pipeline on the language-agnostic IR
`normalised_tokens`, but emits `LanguageCitationStatus::Unconfirmed`
for Go findings pending this survey.

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / general web: queries `Go programming language code
  clone detection empirical study precision recall corpus`, `near-miss
  code clone detection Golang inconsistent changes Type-3 clones`, `Go
  language code clone benchmark dataset`, `code clone evolution
  inconsistent changes Go repositories empirical study maintenance`.
- ACM Digital Library / IEEE Xplore: venues ICSE/FSE/ASE/MSR/ICPC/
  SCAM/IWSC/SANER/ICSME/ISSTA/TOSEM/TSE/EMSE/JSS, keywords `Go` /
  `Golang` AND (`clone detection` OR `clone evolution` OR
  `inconsistent clone change` OR `clone drift` OR `clone fault`).
- arXiv cs.SE: same keywords, last 6 years.
- dblp / Semantic Scholar forward-citation ("cited by") graph of the
  Cordy-Roy NiCad, Bettenburg, and Krinke cluster, filtered for
  Go-language subjects.
- The multilingual-clone-detector cluster (MSCCD, TGMM, SourcererCC,
  Gitor, C4) checked for Go in their evaluation corpora.

## Candidates considered

### Wang, Gao, Jiang, Xing, Zhang, Ying, Gu, Sun, "Go-Clone: Graph-Embedding Based Clone Detector for Golang" (ISSTA 2019, Tool Demonstration track)

ACM SIGSOFT ISSTA 2019, Tool Demonstrations track.
https://dl.acm.org/doi/10.1145/3293882.3338996
https://conf.researchr.org/details/issta-2019/issta-2019-Tool-demonstrations/5/Go-Clone-Graph-Embedding-Based-Clone-Detector-for-Golang-
Tool repository: https://github.com/wangcong15/go-clone

This is the single strongest candidate found: a peer-reviewed
publication whose experimental subject genuinely IS Go. Go-Clone
parses Golang source into LLVM IR, computes a labelled semantic flow
graph (LSFG) per function, and trains a deep neural network to encode
LSFGs for similarity classification. It is evaluated on a constructed
Golang clone dataset of 6,110 commit versions from 48 GitHub
projects, reporting AUC 89.61% and accuracy 83.80%.

Verdict: REJECTED — strongest candidate, rejected on four
independent grounds.

- Venue tier / contribution type. Go-Clone is a two-page Tool
  Demonstration, not a full research paper. The tool itself is the
  contribution. This is the same class as the NiCad, SourcererCC, and
  `dupl` tools that the R-series policy excludes from clauses
  (a)/(b)/(c): a tool demonstration grounds a tool's availability, not
  an empirical finding about Go clones.
- Algorithm mismatch (clause b fails). Go-Clone is a deep-learning
  graph-embedding detector over LLVM-IR semantic flow graphs. It is
  NOT the NiCad normalised-token n-gram algorithm cntrdct cites
  (`cordy-roy-icpc-2008`). Clause (b) requires an independent paper
  applying *the cited* language-agnostic algorithm to Go. A different
  algorithm cannot serve as the NiCad application paper.
- Wrong anomaly (the drift half is ungrounded). Go-Clone addresses
  clone *detection* (is this pair a clone?). It says nothing about the
  drift / inconsistent-change signal (`bettenburg-msr-2009`,
  `krinke-icsm-2007`) that is clone-drift's actual anomaly — the
  detector flags the *diverged* member of a near-identical cluster,
  not the existence of a clone. Go-Clone neither studies inconsistent
  changes nor evaluates them on Go.
- Metric mismatch. The reported AUC / accuracy measure a learned
  binary classifier's quality, not near-miss clustering precision /
  recall in the NiCad lineage on a Go corpus. Even read charitably
  under clause (a), it grounds a semantic DL detector's
  classification quality, not the near-miss-clustering-plus-drift
  pipeline cntrdct ships.

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
execution cost). ANTLR ships a Go grammar, so Go is plausibly within
that scalability set.

Verdict: REJECTED.

- The quantitative detection-performance evaluation (recall via
  BigCloneBench, precision via manual sampling, plus CodeNet) is
  conducted on Java. The 20-language experiment measures parser /
  grammar-plugin coverage and execution cost — it is a language-
  scalability test, not a quantitative clone-detection evaluation on
  a Go corpus. A "supported language" entry in a scalability table
  does not meet clause (a)'s bar; no Go-specific quantitative clone
  result is reported.
- Clause (b) is not satisfied either: MSCCD implements its own
  ANTLR-token-based algorithm, NOT the NiCad algorithm cntrdct cites
  (`cordy-roy-icpc-2008`). Clause (b) requires an independent paper
  applying *the cited* language-agnostic algorithm to Go. MSCCD is a
  different algorithm, so it cannot serve as the NiCad application
  paper.
- It grounds (at most) clone *detection*, and says nothing about the
  drift / inconsistent-change signal (`bettenburg-msr-2009`,
  `krinke-icsm-2007`) on Go, which is the detector's actual anomaly.

### `dupl` / `golangci-lint` duplicate detection (mibk/dupl, golangci/dupl)

Community tools. https://github.com/mibk/dupl,
https://github.com/golangci/dupl

`dupl` is the canonical Go-native clone detector: it serialises Go
ASTs into a token stream (ignoring node values, keeping node types)
and finds duplicated subsequences via a suffix tree, the same Type-1/
Type-2 family clone-drift clusters. It is widely deployed through
`golangci-lint`.

Verdict: REJECTED. `dupl` is a community tool with no peer-reviewed
publication backing it — it has no associated ICSE/FSE/MSR/ISSTA paper
and no quantitative precision/recall study on a labelled Go corpus.
Per the policy and the arg-swap / Python / TypeScript precedent,
tools without a qualifying peer-reviewed paper do not satisfy any
clause. It is the most relevant *Go-native* artefact, but relevance is
not peer review, and it is noted here only as the strongest tool-class
near-miss.

### Bettenburg / Krinke / clone-genealogy and clone-fault empirical studies (forward-citation cluster)

Peer-reviewed empirical studies of inconsistent clone changes, clone
genealogies, and clone-related faults — the same evidential family as
`bettenburg-msr-2009` and `krinke-icsm-2007` (e.g. "An Empirical Study
on Inconsistent Changes to Code Clones at the Release Level"; clone-
lifetime / late-propagation studies; gCad near-miss genealogy work).
https://users.encs.concordia.ca/~shang/pubs/bettenburg-wcre09.pdf
https://dl.acm.org/doi/10.1109/WCRE.2007.7

These are the closest works to clone-drift's actual anomaly (the
drift / inconsistent-change signal).

Verdict: REJECTED for clause (a). Every such study surveyed uses
Java / C / C++ subjects (Linux, FreeBSD, Apache, automotive / Java
systems). None include Go subjects. The drift half of the detector
remains ungrounded on Go.

### "An Empirical Analysis of Git Commit Logs for Potential Inconsistency in Code Clones" (arXiv 2409.08555)

arXiv preprint analysing 45 Apache Software Foundation repositories.
https://arxiv.org/pdf/2409.08555

Studies the inconsistent-clone-change signal directly — topically the
closest to clone-drift — but the corpus is Apache (predominantly
Java) and the work is a preprint.

Verdict: REJECTED. Preprint, and subjects are not Go.

### Gitor (arXiv 2311.08778) and cross-language clone detection (C4, ICPC 2024; CLCDSA)

Gitor: https://arxiv.org/pdf/2311.08778
C4: https://xing-hu.github.io/assets/papers/icpc-c8.pdf

Gitor is a scalable global-sample-graph clone detector; C4 and CLCDSA
target cross-language clone pairs. Their corpora are built from
Java / Python / C# / C++ / C (BigCloneBench, AtCoder, Google CodeJam,
CLCDSA); Gitor is additionally an arXiv preprint.

Verdict: REJECTED. Go is not in the evaluation corpora; Gitor is
additionally a preprint, and none of these address the drift /
inconsistent-change signal.

### LLM-based clone-detection studies (arXiv 2511.01176, 2407.02402, GPTCloneBench, SemanticCloneBench)

arXiv 2511.01176 ("An Empirical Study of LLM-Based Code Clone
Detection"), arXiv 2407.02402, and the GPT/Semantic CloneBench
datasets.

Verdict: REJECTED. These are preprints and/or their benchmarks
(BigCloneBench, GPTCloneBench, SemanticCloneBench, GoogleCodeJam,
OJClone) are Java / C / C++ / Python derived; none is a Go clone
benchmark, and none addresses near-miss drift. Clause (c) is not
satisfied.

### Go-specific clone benchmark / dataset (clause c)

Searched explicitly for a published Go clone benchmark (analogue of
BigCloneBench for Go).

Verdict: REJECTED — none found. The Go-Clone dataset (6,110 commits
from 48 GitHub projects) is constructed for that tool-demonstration
and is not published as a standalone peer-reviewed benchmark with an
independent evaluation; Go-UT-Bench (arXiv 2511.10868) exists but is a
unit-test-generation dataset, not a clone benchmark. No peer-reviewed
Go clone benchmark / dataset exists.

## Conclusion

No candidate satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for cntrdct's clone-drift detector on
Go:

- The strongest candidate (Go-Clone, ISSTA 2019) genuinely uses Go as
  its experimental subject, but it (i) is a two-page tool
  demonstration in the same excluded class as NiCad / SourcererCC /
  `dupl`, (ii) implements a deep-learning graph-embedding algorithm,
  NOT the NiCad algorithm we cite — so clause (b) fails, (iii)
  addresses clone *detection* and is silent on the drift /
  inconsistent-change signal that is the detector's actual anomaly,
  and (iv) reports classifier AUC / accuracy rather than near-miss
  clustering precision / recall in our lineage. It does not honestly
  ground OUR detector.
- No peer-reviewed paper applies the NiCad algorithm we cite
  (`cordy-roy-icpc-2008`) to a Go corpus with quantitative
  evaluation — clause (b) fails (MSCCD is a different algorithm and
  reports its quantitative results on Java).
- No peer-reviewed study of inconsistent clone changes / clone faults
  uses Go subjects — the drift half of the detector is ungrounded on
  Go (Bettenburg / Krinke and their citation cluster are Java / C).
- No peer-reviewed Go clone benchmark / dataset exists — clause (c)
  fails.
- The most relevant Go-native artefact (`dupl`) is a community tool
  with no qualifying peer-reviewed paper.

The honest default applies: Go coverage for clone-drift is
Unconfirmed.

## Decision

- Do NOT add a Go-grounded citation to `Citation::CITATIONS` for
  clone-drift.
- Go findings continue to emit the cross-cutting concept keys
  (`cordy-roy-icpc-2008`, `bettenburg-msr-2009`, `krinke-icsm-2007`)
  with `LanguageCitationStatus::Unconfirmed` (matches the current
  `src/detectors/clone_drift.rs` behaviour).
- `CITATIONS.md` records the explicit-no-citation line for clone-drift
  Go coverage, pointing at this survey:
  `(clone-drift Go coverage: unconfirmed; survey notes at
  docs/surveys/clone-drift-go-2026-06.md)`.
  (Integration of this line is performed centrally by team-lead, not
  in this survey commit.)

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed full paper (IEEE/ACM/Elsevier/Springer tier) applies
  NiCad-style or SourcererCC-style near-miss clone detection to a Go
  corpus with quantitative evaluation — would satisfy clause (a)/(b).
- A peer-reviewed empirical study of inconsistent clone changes /
  clone faults / clone evolution uses Go subjects — would ground the
  drift signal directly.
- A peer-reviewed Go clone benchmark / dataset is published — would
  satisfy clause (c).
- The Go-Clone work is extended from the ISSTA 2019 tool demo into a
  full research paper with a NiCad-comparable near-miss evaluation and
  a drift / inconsistent-change analysis on Go.
- The MSCCD JSS work is extended with a confirmed, named Go
  quantitative clone-detection result on a Go corpus.
- `dupl` (or another Go-native detector) gains a backing peer-reviewed
  publication with a quantitative precision/recall study on a labelled
  Go corpus.
