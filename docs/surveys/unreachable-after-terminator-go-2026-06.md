# Literature survey: unreachable-after-terminator → Go

Date: 2026-06-05
Detector: `unreachable-after-terminator`
Target language: Go
Surveyor: cntrdct R-3.f PR (unreachable-after-terminator Go grounding)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has Go source as its
experimental subject for unreachable / dead-code-after-terminator
detection, or (b) is the language-agnostic algorithm we already cite
plus an independent peer-reviewed paper applying it to Go with
quantitative evaluation on a Go corpus, or (c) introduces a Go
benchmark / dataset relevant to the detection.

A subject in Java / C / C++ / JavaScript / TypeScript / Python only
does NOT satisfy clause (a) for Go: Go must be an actual experimental
subject. A Java / C application of our cited algorithm does not satisfy
clause (b) for Go either. Preprints, blog posts, and tools (`go vet`,
`staticcheck`, the `deadcode` command) do not satisfy any clause; the
strongest are recorded below as rejected candidates.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search.

## Detector concept

`unreachable-after-terminator` flags a statement that statically
follows a divergent terminator within the same block — the FindBugs
"UR — Unreachable code" pattern modelled as a control-flow
contradiction at the AST-statement level. For Go the divergent
terminators are `return`, `panic(...)`, `os.Exit(...)`,
`log.Fatal*(...)`, `break`, `continue`, and an `if`/`else` where both
branches diverge; plus the F4e constant-condition rule
(`if false { ... }`). The Go scan
(`src/detectors/unreachable_after_terminator.rs`) maps `panic(...)` and
the `os.Exit` / `log.Fatal*` non-returning calls to a divergent
process-exit, plus the branch-merge and constant-condition rules.

This is distinct from the unused-symbol sense of "dead code"
(unreferenced functions / packages / variables — the lineage of the
Go `deadcode` command, which uses Rapid Type Analysis to find functions
unreachable from `main`). The detector targets the intra-block
control-flow contradiction, not reachability-from-entry-point.

## Existing Rust citations

The detector currently cites:

- `hovemeyer-pugh-oopsla-2004` — D. Hovemeyer, W. Pugh, "Finding Bugs
  is Easy", OOPSLA 2004. Introduces the FindBugs UR (Unreachable
  code) bug pattern. Subjects: Java.
- `engler-sosp-2001` — D. Engler et al., "Bugs as Deviant Behavior:
  A General Approach to Inferring Errors in Systems Code", SOSP 2001.
  Establishes control-flow contradictions as high-confidence anomaly
  signals. Subjects: the Linux kernel and C systems code.

Both are grandfathered as Rust-grounded under the unrevised clause (b)
when cntrdct shipped v0; new languages follow the strict (b). For Go
clause (b) we would need an independent peer-reviewed paper applying
the UR / control-flow-contradiction algorithm to a Go corpus with
quantitative evaluation. Neither existing citation has Go subjects.

## Search

Databases / sources queried:

- Google Scholar / web: `Go unreachable code dead code detection
  empirical study peer-reviewed`, `empirical study of bugs in Go
  static analysis`, `Go dead code benchmark dataset unreachable
  statement defect detection corpus`, `Go control flow unreachable
  after return panic empirical evaluation precision recall`.
- ScienceDirect / Elsevier (Journal of Systems and Software,
  Information and Software Technology): Go AND (linter OR unreachable
  OR dead code).
- arXiv: cs.SE / cs.PL, Go-subject static-analysis and bug-study
  keyword set, last 6 years.
- The Go Wiki "Research Papers" list (go.dev/wiki/ResearchPapers) and
  the Hovemeyer-Pugh / Engler "cited by" clusters, filtered for
  Go-language subjects.
- staticcheck issue tracker and gopls analyzer docs (to confirm the
  pattern is tooling-established in Go, not as citable evidence).

## Candidates considered

### Wu & Clause, "An Empirical Assessment of Go Linters on Real-World Issues" (JSS 2026)

SSRN: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=5208109
ScienceDirect: https://www.sciencedirect.com/science/article/abs/pii/S0164121226000312

Peer-reviewed (The Journal of Systems and Software, vol. 236, 2026).
Authors Jianwei Wu and James Clause evaluate existing Go linters
against real-world issues drawn from the industrial development
workflow at MathWorks. The headline result is that the linters are
often unable to detect the issues developers actually hit, and even
when they fire they rarely guide the developer to a valid fix.

Verdict: rejected. This is a genuine Go-subject static-analysis study
(would satisfy clause (a)'s language requirement), but its unit of
study is linter efficacy against developer-reported workflow issues
broadly, not the unreachable-code / code-after-terminator pattern. It
does not isolate a UR / control-flow-contradiction category nor report
quantitative precision/recall for unreachable-statement detection on a
Go corpus. Right language, wrong target dimension — the same
loose-fit rejection the TypeScript survey applied to the MSR '26 TS
bug study.

### Shirai et al., "Does Programming Language Matter? An Empirical Study of Fuzzing Bug Detection" (MSR 2026)

arXiv: https://arxiv.org/abs/2602.05312 (HTML: https://arxiv.org/html/2602.05312v1)

Peer-reviewed, accepted at the 23rd International Conference on Mining
Software Repositories (MSR '26). Analyses 559 OSS-Fuzz projects across
six languages — C, C++, Go, Java, Python, Rust — including 74 Go
projects with 3,180 detected issues, and classifies faults at the CWE
Pillar level (Resource Management CWE-664, Incorrect Calculation
CWE-682, Control Flow CWE-691, Protection Mechanism CWE-693,
Exceptional Handling CWE-703, Neutralization CWE-707, Coding Standards
CWE-710).

Verdict: rejected. Go is a real experimental subject (clause (a)
language requirement met), and there is a broad "Control Flow
(CWE-691)" pillar, but the study does not report unreachable-code /
dead-code-after-terminator as a measured category, and CWE-691 is far
wider than the UR pattern (it covers missing/incorrect control-flow
logic generally). No quantitative evaluation of the
code-after-terminator contradiction on the Go corpus. Does not satisfy
(a) for our pattern, nor (b), nor (c).

### Yeboah & Popoola, "Efficacy of static analysis tools for software defect detection on open-source projects" (arXiv 2405.12333, 2024)

arXiv: https://arxiv.org/abs/2405.12333

Compares SonarQube, PMD, Checkstyle, and FindBugs using
precision / recall / F1.

Verdict: rejected. Subjects are Java, C++, and Python — Go is not
studied. A non-Go subject cannot satisfy (a) or (b) for a Go
extension per the policy. (Also, the tools studied are Java/JVM-
oriented and the bug model is not isolated to the UR pattern.)

### Tu, Liu, Song & Zhang, "Understanding Real-World Concurrency Bugs in Go" (ASPLOS 2019)

PDF: https://songlh.github.io/paper/go-study.pdf

Peer-reviewed (ASPLOS 2019). The first systematic study of Go
concurrency bugs, analysing six production-grade Go projects (Docker,
Kubernetes, etcd, gRPC, CockroachDB, BoltDB) and categorising
blocking / non-blocking concurrency bugs.

Verdict: rejected. Strong Go-subject empirical base, but the taxonomy
is entirely about concurrency (channels, locks, WaitGroups), not
static unreachable-code / code-after-terminator. Right language, wrong
bug model. Satisfies none of (a) / (b) / (c) for this detector.

### Lauinger et al. / "Breaking Type-Safety in Go" + "Uncovering Unsafe Go in the Wild" (2020)

arXiv: https://arxiv.org/abs/2006.09973 , https://arxiv.org/abs/2010.11242

Go-subject empirical studies of the `unsafe` package usage in the
wild.

Verdict: rejected. Go subjects, but the studied phenomenon is
`unsafe`-package memory-safety risk, with no unreachable-code /
control-flow-contradiction category and no quantitative evaluation of
the UR pattern. Wrong target pattern.

### The Go `deadcode` command and `go vet` / staticcheck unreachable check

Go blog: https://go.dev/blog/deadcode
Package: https://pkg.go.dev/golang.org/x/tools/cmd/deadcode
gopls analyzers: https://go.dev/gopls/analyzers

The `deadcode` command (golang.org/x/tools) uses Rapid Type Analysis
to report functions unreachable from `main`. `go vet`'s `unreachable`
analyzer and staticcheck (e.g. SA4006 "value never used", and the
unreachable-statement check) flag code after a terminator and unused
assignments — exactly adjacent to the UR pattern. The Go spec proposal
golang/go#71553 even discusses teaching the compiler about
non-returning functions so tools can extend "terminating statement"
recognition to `panic`-like calls.

Verdict: rejected as citations. These are tools / compiler features and
a language proposal, not peer-reviewed publications, and no empirical
study with a quantitative Go-corpus evaluation of the unreachable
pattern was found behind them. Recorded because they confirm the
pattern is well-established and standard in Go tooling — relevant to a
future clause-(b) search but not itself citable. Note also that
`deadcode` targets the unused-symbol sense, a different bug model from
the intra-block contradiction this detector flags.

### Dead-code-elimination / poisoning preprints (DCE-LLM arXiv 2506.11076; DePA arXiv 2502.20246)

LLM-based dead-code elimination and dead-code-poisoning detection in
code-generation datasets.

Verdict: rejected. Preprints; not Go-subject for the UR pattern;
concern dead-code elimination / dataset poisoning, not static
unreachable-after-terminator detection. Same grounds as the TypeScript
survey's rejection of these.

### Citation cited-by graphs

- Hovemeyer-Pugh (OOPSLA 2004): no paper in the filtered results
  presents a quantitative evaluation of the FindBugs UR pattern on a
  Go corpus. The SpotBugs lineage stays on the JVM (Java / Kotlin /
  Android) and does not branch to Go at the AST-statement level.
- Engler-SOSP-2001: extended to C/C++ (Coverity) and OS kernels, but
  no Go control-flow-contradiction application with quantitative
  Go-corpus evaluation was found.

Verdict: no qualifying clause-(b) secondary application found for Go.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the unreachable-after-terminator
pattern on Go. The strongest peer-reviewed Go-subject static-analysis
works (Wu & Clause's Go-linters assessment, JSS 2026; the MSR '26
fuzzing-bug study; the Go concurrency-bug and `unsafe`-package studies)
either study a different bug model (concurrency, `unsafe`, broad linter
efficacy, CWE-691 control-flow generally) or do not isolate an
unreachable-code / code-after-terminator category with quantitative
evaluation. The other dead-code works are non-Go, preprints, or use
the unused-symbol sense. The UR pattern is well-supported in Go tooling
(`go vet` unreachable, staticcheck, the `deadcode` command, proposal
golang/go#71553), but tooling, compiler features, and language
proposals are not peer-reviewed publications and cannot ground the
citation.

Mirroring the TypeScript and Python precedents, the honest default
applies: Go coverage is Unconfirmed. The detector ships its Go
extension regardless — P1 remains satisfied (the two grandfathered Rust
citations are non-empty), and the per-language gap is captured in
metadata.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` should add an explicit `(unreachable-after-terminator
  Go coverage: unconfirmed; survey notes at this file)` line under the
  detector's subsection so the consistency test sees an acknowledged
  gap rather than a silent one. (This survey does not edit
  `CITATIONS.md`; the PR that wires Go support makes that one-line
  edit.)
- The detector emits `LanguageCitationStatus::Unconfirmed` on every Go
  finding. SARIF consumers can filter or visually flag
  indirectly-grounded Go results via
  `properties.languageCitationStatus`.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies the FindBugs UR / control-flow-
  contradiction pattern to a Go corpus with quantitative evaluation
  (clause b candidate).
- A new Go benchmark / dataset specifically labels
  unreachable-statement / code-after-terminator defects (clause c
  candidate).
- A peer-reviewed empirical study of Go bugs or static-analysis
  warnings adds an unreachable-code category with a Go-corpus
  quantitative evaluation (clause a candidate) — e.g. a follow-up to
  the Wu & Clause Go-linters assessment that breaks out the
  unreachable check.
- The Hovemeyer-Pugh bug-pattern taxonomy is formally extended to Go
  in a published reference work.
