# Literature survey: arg-swap → Go

Date: 2026-06-05
Detector: `arg-swap`
Target language: Go
Surveyor: cntrdct R-3.f PR (arg-swap Go grounding)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has Go source as its
experimental subject for the swapped-argument / argument-selection-defect
bug class, or (b) is the language-agnostic algorithm we already cite plus
an independent peer-reviewed paper applying it to Go with quantitative
evaluation on a Go corpus, or (c) introduces a Go benchmark / dataset
relevant to the detection.

If no candidate satisfies any of (a) / (b) / (c), the language extension
still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the search.

A hard constraint specific to this survey: Go is a distinct language with
its own tree-sitter grammar and its own compiler/AST. A paper whose
experimental subjects are Java / C / C++ / JavaScript / TypeScript /
Python ONLY does NOT satisfy clause (a) for Go, no matter how closely the
bug class matches. Go must be an actual experimental subject. This
distinction rejects every published swapped-argument work to date (Rice
2017 → Java, DeepBugs 2018 → JavaScript, SWAPD 2020 → C/C++, Allamanis
2021 → Python). Symmetrically, the genuine Go-subject bug studies that
exist target a different bug class (concurrency), so they cannot ground
this detector either.

Additional exclusions inherited from the policy and the task brief:
preprints (arXiv-only, not peer-reviewed) and community tools (linters,
`go vet`, `gopls`/`staticcheck` analyzers) do NOT qualify for (a) / (b) /
(c). The strongest such items are recorded below as rejected candidates
with rationale.

## Detector concept under grounding

`arg-swap` flags a 2-argument call site whose argument identifiers are
the reverse permutation of the callee's parameter names
(case-insensitive, with abbreviation-aware prefix matching) — i.e. a
same-typed swapped-argument defect detected by parameter-name
correlation, in the lineage of Rice et al. (ICSE 2017). The Go pipeline
reuses the shared IR definition extraction plus a raw-tree call walk
(Pattern B); see `src/detectors/arg_swap.rs`,
`src/parsers/go.rs`, and `src/detectors/pr_miner/extract_go.rs`.

## Existing citations

The detector currently cites:

- `li-zhou-fse-2005` — Z. Li, Y. Zhou, "PR-Miner: Automatically
  Extracting Implicit Programming Rules and Detecting Violations in
  Large Software Code", ESEC/FSE 2005. Subjects: C/C++.
  `languages: &[Language::Rust]`.
- `rice-icse-2017` — A. Rice, E. Aftandilian, C. Jaspan, E. Johnston,
  M. Pradel, Y. Arroyo-Paredes, "Detecting Argument Selection
  Defects", ICSE 2017 (publication of record PACMPL/OOPSLA, DOI
  10.1145/3133928). Subjects: Java (evaluated at Google on 200 MLOC
  internal + 10 MLOC external Java code). `languages: &[Language::Rust]`.
- `allamanis-neurips-2021` — M. Allamanis, H. Jackson-Flux,
  M. Brockschmidt, "Self-Supervised Bug Detection and Repair",
  NeurIPS 2021. Subjects: Python (PyBugLab + PyPIBugs).
  `languages: &[Language::Python]`.

None of these declares Go as a grounded language. The Go pipeline emits
`LanguageCitationStatus::Unconfirmed` with
`citation_keys = ["li-zhou-fse-2005", "rice-icse-2017"]` (the
algorithmic-lineage citations).

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / general web: `Go language "argument swap" OR
  "swapped arguments" OR "argument selection defect" bug detection
  static analysis`; `name-based bug detection Golang DeepBugs swapped
  arguments corpus evaluation`; `Golang learning-based bug detection
  benchmark dataset variable misuse argument 2023 2024 2025`.
- ACM Digital Library / IEEE Xplore (via web index): venue filter
  ICSE / FSE / ASE / MSR / OOPSLA / CGO / PLDI 2017-2026, keywords `Go`
  / `Golang` AND (`argument swap` OR `swapped arguments` OR `argument
  selection defect` OR `name-based bug detection`).
- dblp / publisher pages: Rice et al. (ICSE 2017) and Pradel & Sen
  (DeepBugs, OOPSLA 2018) "cited by" graph, filtered for Go
  experimental subjects.
- Go-specific empirical-bug literature: `empirical study real-world
  bugs Go programs benchmark dataset taxonomy`; the
  `system-pclub/go-concurrency-bugs` and GoBench artefacts.
- arXiv cs.SE / cs.PL, last 5 years, same keywords (used only to
  confirm peer-review status / exclude preprint-only work; preprints do
  not qualify per the policy).

## Candidates considered

### Rice, Aftandilian, Jaspan, Johnston, Pradel, Arroyo-Paredes, "Detecting Argument Selection Defects" (ICSE 2017)

DOI 10.1145/3133928.
https://dl.acm.org/doi/10.1145/3133928
Open-access PDF: https://research.google.com/pubs/archive/46317.pdf

The originating paper for cntrdct's parameter-name-correlation approach.
The algorithm matches identifier names to flag wrong-argument method
calls, then uses program-structure analysis to reject false positives.
Evaluated at Google on 200 MLOC internal + 10 MLOC external code; the
check ships in the open-source Error Prone project.

Verdict: rejected for Go clause (a). Experimental subjects are Java
(Error Prone is a `javac` plugin). Already cited as the algorithmic
basis (grandfathered Rust); provides no Go grounding. It is the
language-agnostic primary of a potential clause (b) pairing, but no
qualifying Go secondary application exists (see Conclusion).

### Pradel, Sen, "DeepBugs: A Learning Approach to Name-based Bug Detection" (OOPSLA 2018)

DOI 10.1145/3276517. arXiv 1805.11683.
https://dl.acm.org/doi/10.1145/3276517

Name-based bug-detection framework. Its `SwappedArgs` detector is the
closest published match to cntrdct's bug class — it flags accidentally
swapped function arguments (e.g. `setPoint(y, x)` for `setPoint(x, y)`)
using a learned semantic representation of identifier names. Corpus:
100k training + 50k validation JavaScript files, 68M LOC.

Verdict: rejected for Go clause (a). The experimental subjects are
JavaScript only. Under this survey's hard rule, JavaScript subjects do
not ground Go. The DeepBugs framework is in principle retargetable (it
learns from a token stream and could be re-trained on a Go corpus), but
no peer-reviewed paper applies it to a Go corpus with quantitative
evaluation.

### Davis, Kildea, et al., "A replication of 'DeepBugs: a learning approach to name-based bug detection'" (ESEC/FSE 2021, ROSE Festival)

DOI 10.1145/3468264.3477221.
https://dl.acm.org/doi/10.1145/3468264.3477221

Independent replication of the DeepBugs swapped-arguments detector on
the original JavaScript dataset; reproduces the reported accuracy within
a small margin.

Verdict: rejected. JavaScript-only; does not extend DeepBugs to Go.

### Liu, Foyen, Levin, "Out of Sight, Out of Place: Detecting and Assessing Swapped Arguments" (ASE 2020)

IEEE Xplore (ASE 2020). arXiv 2009.09117.
https://arxiv.org/abs/2009.09117

Static-analysis tool SWAPD uses natural-language information in
identifiers to flag mistakenly-swapped arguments at call sites.
Evaluated on 417M LOC of C and C++; reports 154 manually-vetted
real-world swap bugs.

Verdict: rejected for Go clause (a). Subjects are C and C++. The most
direct Rice-style lineage outside Java, but no Go subjects, and no
independent peer-reviewed Go application of SWAPD exists as of 2026-06.

### Allamanis, Jackson-Flux, Brockschmidt, "Self-Supervised Bug Detection and Repair" (NeurIPS 2021)

https://proceedings.neurips.cc/paper/2021 (PyBugLab / PyPIBugs).

Already cited by the detector for Python. Self-supervised detector whose
rewrite rules include argument-swap mutations; subjects are Python
(PyBugLab synthetic + PyPIBugs real-world).

Verdict: rejected for Go clause (a). Python subjects; no Go corpus. The
PyPIBugs dataset is the Python analogue of a clause-(c) benchmark and has
no Go counterpart.

### Tu, Liu, Song, Zhang, "Understanding Real-World Concurrency Bugs in Go" (ASPLOS 2019)

DOI 10.1145/3297858.3304069.
https://dl.acm.org/doi/10.1145/3297858.3304069
Open-access PDF: https://songlh.github.io/paper/go-study.pdf
Artefact: https://github.com/system-pclub/go-concurrency-bugs

The first systematic study of concurrency bugs in real Go programs: 171
bugs mined from six popular open-source Go applications (Docker,
Kubernetes, etcd, CockroachDB, gRPC, BoltDB), taxonomised into blocking
(deadlock-style) and non-blocking (data-race-style) categories. Genuine,
large-scale Go experimental subjects.

Verdict: rejected on bug-class grounds. Although Go is unambiguously the
experimental subject (satisfying the language half of clause (a)), the
studied bug class is concurrency (misuse of goroutines, channels,
`sync` primitives), not same-typed argument-selection defects detected
by parameter-name correlation. The brief explicitly flags this paper as
concurrency, not arg-swap. It introduces no swapped-argument label set,
so it also fails clause (c) for this detector.

### Yuan, Li, Lu, Liu, Li, Xue, "GoBench: A Benchmark Suite of Real-World Go Concurrency Bugs" (CGO 2021)

DOI 10.1109/CGO51591.2021.9370317.
https://ieeexplore.ieee.org/document/9370317/
https://conf.researchr.org/details/cgo-2021/cgo-2021-papers/16/

A Go benchmark suite: GOREAL (82 bugs from nine real-world
applications) + GOKER (103 bugs as small reproducible kernels). The
closest thing to a clause-(c) Go bug dataset located.

Verdict: rejected for Go clause (c). The benchmark is entirely
concurrency bugs (blocking + non-blocking); it carries no
swapped-argument / argument-selection labels. A benchmark grounds a
detector only when its label set matches the detector's bug class, and
GoBench's does not. Genuine Go subjects, wrong defect category.

### Richter & Wehrheim, "How to Train Your Neural Bug Detector" (ASE 2023) and the distribution-shift cluster

DOI 10.1109/ASE56229.2023.00104 (peer-reviewed); plus "On Distribution
Shift in Learning-based Bug Detectors" (ICML 2022, arXiv 2204.10049) and
DeepMutants (arXiv 2107.06657).
https://arxiv.org/abs/2204.10049

This cluster studies training data for DeepBugs-style detectors
(including swapped-arguments) and distribution shift toward real bugs.

Verdict: rejected for Go clause (a). Experimental subjects are Python
(and JavaScript via the inherited DeepBugs corpora); none uses a Go
corpus. Relevant context for the swapped-arguments bug class but no Go
grounding.

### Cryptographic / API-misuse detectors for Go (e.g. "Evaluating Cryptographic API Misuse Detectors for Go", arXiv 2026)

arXiv 2604.24085.
https://arxiv.org/abs/2604.24085

A genuine Go-subject study, evaluating crypto-API-misuse detectors on Go
code.

Verdict: rejected on two independent grounds. (1) Provenance: arXiv-only
preprint at survey time; the policy excludes preprints. (2) Relevance:
the bug class is cryptographic API misuse, not same-typed argument-swap
defects; it isolates no swapped-argument category and ships no swap
labels. Neither (a) nor (c) is met even setting provenance aside.

### Go static-analysis tooling: `go vet`, `gopls` analyzers, `staticcheck`, GOA/SVENG-style analyzers

`go vet` and the `gopls` analyzer driver bundle pluggable checks; some
flag particular wrong-argument cases (e.g. `printf` format/argument
mismatches), and academic static-analysis frameworks for Go (lightweight
AST analyzer + interprocedural summary analyzer) have been published.
`staticcheck` adds further lint rules.

Verdict: rejected. These are tools / linters, not peer-reviewed
publications grounding a same-typed parameter-name-correlation
argument-swap rule, and where an academic framework exists its evaluated
defect classes are not the arg-swap bug class with a quantitative
swap-labelled Go corpus. The Rice-style check itself ships only in Error
Prone (Java), not in the Go toolchain. None satisfies (a) / (b) / (c).

### Rice et al. and Pradel-Sen "cited by" graphs

Filtered the top citing papers (by venue and citation count) of Rice et
al. (ICSE 2017) and DeepBugs (OOPSLA 2018) for Go experimental subjects.
The graph is dominated by Java (Error Prone lineage), C/C++ (SWAPD),
JavaScript (DeepBugs replications), and Python (Allamanis,
distribution-shift cluster). No paper performs a Go-corpus replication of
the parameter-name-correlation argument-swap method.

Verdict: no clause-(b) Go secondary application surfaced.

## Conclusion

No peer-reviewed publication or established benchmark grounds the
`arg-swap` detector for Go:

- Clause (a): no qualifying paper has Go experimental subjects for the
  swapped-argument / argument-selection-defect bug class. The closest
  swapped-argument works are Java (Rice 2017), C/C++ (SWAPD 2020),
  JavaScript (DeepBugs 2018, replication 2021), and Python (Allamanis
  2021). None has Go subjects, and the hard rule blocks transferring
  their grounding. Conversely, the genuine Go-subject bug studies (Tu et
  al. ASPLOS 2019, GoBench CGO 2021) are concurrency, a different defect
  class.
- Clause (b): the language-agnostic primaries we already cite (Rice
  2017, Li-Zhou 2005) have no independent peer-reviewed Go application
  with quantitative evaluation on a Go corpus. Tool support (`go vet`,
  `gopls`, `staticcheck`) is not peer-reviewed and is excluded.
- Clause (c): no Go benchmark/dataset with swapped-argument labels
  exists. The Go benchmark that does exist (GoBench) carries only
  concurrency-bug labels; PyPIBugs (the Python analogue with swap
  mutations) has no Go counterpart.

This is the honest, expected outcome — matching 3 of 5 Python-era
surveys and 5 of 5 TypeScript surveys. The Go extension ships under
`LanguageCitationStatus::Unconfirmed`; P1 is preserved because the
detector's overall citation set is non-empty and real.

## Decision

- Add NO Go citation to `Citation::CITATIONS` in
  `src/detectors/arg_swap.rs`.
- The Go pipeline keeps emitting `LanguageCitationStatus::Unconfirmed`
  with `citation_keys = ["li-zhou-fse-2005", "rice-icse-2017"]` (the
  algorithmic-lineage citations), so SARIF consumers see the grounding
  is indirect.
- `CITATIONS.md` adds the explicit-no-citation form under the arg-swap
  subsection, pointing readers at this survey:
  `(arg-swap Go coverage: unconfirmed; survey notes at
  docs/surveys/arg-swap-go-2026-06.md)`.
  (Integration of CITATIONS.md is handled centrally by the integrator,
  not by this survey.)

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies a name-based / parameter-name
  argument-swap detector (DeepBugs-style or Rice-style) to a Go corpus
  with quantitative evaluation — that would satisfy clause (b) (paired
  with `rice-icse-2017`) or clause (a) directly.
- A peer-reviewed Go bug benchmark/dataset ships with a
  swapped-arguments / argument-selection label set — clause (c). GoBench
  or a successor adding a non-concurrency argument-defect category would
  qualify.
- DeepBugs (Pradel & Sen) or SWAPD (Liu et al.) gains a peer-reviewed Go
  replication with a Go corpus.
- An Error Prone-equivalent argument-selection check for Go (e.g. a
  `gopls`/`go vet` analyzer) is published with a peer-reviewed empirical
  evaluation on a Go corpus (current Go tooling docs are not
  peer-reviewed and do not qualify).
