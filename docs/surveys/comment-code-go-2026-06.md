# Literature survey: comment-code → Go

Date: 2026-06-05
Detector: `comment-code`
Target language: Go
Surveyor: cntrdct R-3.f PR (comment-code Go grounding)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has Go source as its
experimental subject for comment/code inconsistency (doc-code mismatch)
detection, or (b) is the language-agnostic algorithm we already cite
plus an independent peer-reviewed paper applying it to Go with
quantitative evaluation on a Go corpus, or (c) introduces a Go
comment/doc-code-inconsistency benchmark / dataset relevant to the
detection.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search. The honest default is Unconfirmed — both prior surveys for this
same detector (`docs/surveys/comment-code-python-2026-05.md` and
`docs/surveys/comment-code-typescript-2026-06.md`) reached that
conclusion, and Go is held to the same bar.

Two honesty constraints govern this survey specifically:

- Java / C / C++ / JavaScript / TypeScript / Python subjects do NOT
  satisfy clause (a) for Go. The CCI lineage is Java-heavy and the
  iComment cluster is C/C++; a paper whose only subjects are those
  languages does not ground Go. A multi-language paper must use Go as
  an actual experimental subject for the inconsistency-detection task
  (not merely list Go in a pre-training corpus, and not merely claim
  Go as an inference capability).
- Preprints (arXiv-only, no venue acceptance), tool docs, linters, and
  blogs do not qualify under any clause; clause (b)'s secondary
  application must be peer-reviewed with quantitative Go-corpus
  evaluation.

## Existing citations

The detector currently cites:

- `tan-sosp-2007` — L. Tan, D. Yuan, G. Krishna, Y. Zhou,
  "/*iComment: Bugs or Bad Comments?*/", SOSP 2007. Subjects:
  C/C++ Linux kernel comments.
- `tan-pldi-2011` — L. Tan, Y. Zhou, Y. Padioleau,
  "@tComment / aComment: mining annotations from comments and code",
  PLDI/ICST-era doc-comment analysis. Subjects: C/C++ and Java
  (Javadoc) doc-comment testing.

Both are grandfathered as Rust-grounded under the unrevised clause (b)
when cntrdct shipped v0. Python and TypeScript coverage are
`unconfirmed` (see their surveys). New languages, including Go, follow
the strict clause (b).

The shipped Go pattern is `go-panics`: a function's doc comment claims
the function panics (prose such as "panics if ...", "will panic",
"panics when") but the function body contains no divergent terminating
call — no `panic(...)`, `os.Exit(...)`, or `log.Fatal*(...)`.
Factory-shape returns suppress the finding (a constructor that returns
an error instead of panicking is not a contradiction). This is the
iComment-style comment/code-inconsistency concept (a documented promise
contradicted by the implementation), specialised to the Go convention
of documenting panic behaviour in the leading doc comment. There is no
dedicated language tag for panics in godoc — the Go team explicitly
declined to add one (golang/go issue #44056) — so the promise lives in
free-text prose, which is exactly what the detector parses.

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / general web: `Go golang code comment inconsistency
  detection empirical study godoc`; `comment code inconsistency
  detection Go corpus quantitative evaluation`; `godoc documentation
  code mismatch detection static analysis Go panics doc comment`;
  `empirical study comments Go programming language quality outdated
  documentation mining repositories`.
- ACM DL / IEEE Xplore / dblp: venues ICSE/FSE/ICPC/MSR/SANER/AAAI/EACL,
  keywords `Go` AND (`comment` OR `documentation` OR `godoc`) AND
  (`inconsistency` OR `mismatch` OR `outdated`).
- arXiv (cs.SE): same keywords, last 2 years, to identify preprints and
  check whether any had since gained a peer-reviewed venue.
- iComment/aComment "cited by" graph filtered for Go subjects.
- CodeSearchNet-derived CCI cluster: CodeSearchNet ships a Go split, so
  any peer-reviewed paper using the Go split for inconsistency
  detection (not summarisation) would qualify — checked explicitly.
- Go-specific tooling and proposals (godoc-lint, gopls analyzers,
  golang/go panic-annotation proposal) to confirm none is a
  peer-reviewed evaluation.

## Candidates considered

### Nguyen et al., "DocChecker: Bootstrapping Code Large Language Model for Detecting and Resolving Code-Comment Inconsistencies" (EACL 2024, Demonstrations)

EACL 2024 System Demonstrations. ACL Anthology:
https://aclanthology.org/2024.eacl-demo.20/. arXiv:
https://arxiv.org/abs/2306.06347.

The single most on-point candidate for Go, because DocChecker's
pre-training corpus is CodeSearchNet/CodeXGLUE, which DOES include a Go
split (`go.zip` is among the downloaded language archives), and the
shipped tool advertises inference over "10 popular programming
languages, including ... Golang ...". This is the one place in the CCI
literature where Go is named at all.

Verdict: rejected for clause (a). Two independent reasons.
(1) Wrong task for the Go data: Go appears only in the pre-training /
code-summarization pipeline and as an advertised inference capability.
The actual Inconsistency Code-Comment Detection (ICCD) evaluation uses
the post-hoc setting of Panthaplackel et al. (2021) — the Java
JITDATA-derived comment-method pairs — and reports a single aggregate
accuracy (72.3%) with no per-language breakdown and no Go test set. Go
is never an experimental subject for the detection task. (2) It is a
demonstration-track paper, not a full research-track evaluation; the
policy requires quantitative evaluation on a corpus in the target
language, which is absent for Go. "Listed in the pre-training corpus"
and "supported by the inference API" are precisely the in-passing
mentions clause (b) excludes.

### DocPrism: Local Categorization and External Filtering to Identify Relevant Code-Documentation Inconsistencies

arXiv 2511.00215. https://arxiv.org/html/2511.00215.

The most prominent recent multi-language code-documentation
inconsistency detector (LLM + "Local Categorization, External
Filtering"). It was the near-miss in the TypeScript survey because its
extension dataset spans TypeScript, C++, and Java with quantitative
precision.

Verdict: rejected, twice over. (1) Its multi-language extension dataset
is 1,491 functions — 494 TypeScript, 497 C++, 500 Java — plus a Python
base set. Go is NOT among DocPrism's evaluated languages, so it fails
clause (a) for Go on subject grounds (it failed for TypeScript only on
the preprint ground; for Go it fails on subject grounds as well).
(2) It is an arXiv preprint (submitted 2025-10-31) with no venue
acceptance as of this survey; `docs/spec/citations-policy.md`
explicitly excludes preprints. Either reason alone disqualifies it for
Go.

### Panthaplackel et al., "Deep Just-In-Time Inconsistency Detection Between Comments and Source Code" (AAAI 2021)

AAAI 2021 (peer-reviewed). arXiv: https://arxiv.org/pdf/2010.01625.
Artifact: https://github.com/panthap2/deep-jit-inconsistency-detection.

The foundational large-scale CCI benchmark (JITDATA) and the model that
most subsequent CCI work — including DocChecker and C4RLLaMA — builds
on.

Verdict: rejected for clause (a)/(c) on Go. The JITDATA corpus is Java.
As a benchmark it grounds Java, not Go, so it cannot satisfy clause (c)
here, and its subjects are not Go for clause (a). It is a candidate for
the clause (b) primary half (language-agnostic CCI detection), but no
independent peer-reviewed Go application completes the pair.

### Sahar et al., "Code Comment Inconsistency Detection and Rectification Using a Large Language Model" (C4RLLaMA, ICSE 2025)

ICSE 2025 Research Track (peer-reviewed). PDF:
https://people.cs.umass.edu/~brun/class/2024Fall/CS692P/idllm.pdf.

A full peer-reviewed research-track paper. C4RLLaMA fine-tunes
CodeLLaMA to both detect and rectify code-comment inconsistencies,
beating prior just-in-time and post-hoc CCI baselines. Exactly the
iComment-lineage detection task, and a strong publication.

Verdict: rejected for clause (a). Its experiments run on the
established just-in-time and post-hoc CCI benchmarks (the Panthaplackel
JITDATA dataset and post-hoc method-comment datasets), all Java
corpora. No Go subjects. It could serve as the primary half of a clause
(b) pair (language-agnostic CCI detection), but it provides no Go
evaluation itself and no independent peer-reviewed Go application was
found to complete the pair.

### Wen, Nagy, Bavota, Lanza, "A Large-Scale Empirical Study on Code-Comment Inconsistencies" (ICPC 2019)

ICPC 2019 (peer-reviewed). https://www.inf.usi.ch/lanza/Downloads/Wen2019a.pdf.
DL: https://dl.acm.org/doi/abs/10.1109/ICPC.2019.00019.

The strongest peer-reviewed empirical study in the iComment lineage;
mines 1.3 billion AST-level changes from 1,500 projects to taxonomise
inconsistency classes. Already evaluated and rejected in the Python and
TypeScript surveys.

Verdict: rejected for clause (a). The corpus is exclusively Java; no Go
subjects. Language-agnostic in principle (clause (b) primary), but no
independent peer-reviewed paper applies its taxonomy to a Go corpus
with quantitative evaluation.

### CCIBENCH / CoCC and recent CCI detection-repair preprints

Representative recent works in the JITDATA-successor cluster:
- "Are your comments outdated? Towards automatically detecting
  code-comment consistency" (CoCC) — https://arxiv.org/pdf/2403.00251.
- "Investigating the Impact of Code Comment Inconsistency on Bug
  Introducing" — https://arxiv.org/pdf/2409.10781 (introduces CCIBENCH,
  a refined JITDATA derivative).
- "CCISolver: End-to-End Detection and Repair of Method-Level
  Code-Comment Inconsistency" — https://arxiv.org/pdf/2506.20558.
- "Larger Is Not Always Better: Leveraging Structured Code Diffs for
  Comment Inconsistency Detection" — https://arxiv.org/html/2512.19883v1.

Verdict: rejected. Two reasons across the cluster. (1) All build on the
Java JITDATA / post-hoc CCI corpora (CCIBENCH is explicitly a refined
JITDATA derivative) — no Go corpus. (2) Each is an arXiv preprint
without confirmed peer-reviewed venue acceptance as of this survey, and
the policy excludes preprints.

### Code Comment Inconsistency Detection with BERT and Longformer; "...Based on Confidence Learning"

https://arxiv.org/abs/2207.14444 (BERT/Longformer);
https://www.researchgate.net/publication/377792240 (Confidence
Learning).

Transformer-based CCI detectors in the natural-language-inference
framing, evaluated on comment-method pairs.

Verdict: rejected for clause (a). Both evaluate on the Panthaplackel
comment-method (Java) corpora; neither uses Go subjects. Same
clause-(b)-primary-but-no-Go-secondary situation as above.

### Go tooling and the panic-annotation proposal (godoc-lint, gopls analyzers, golang/go#44056)

- `godoc-lint` — a linter for Go Doc Comments, now bundled in
  golangci-lint. https://github.com/godoc-lint/godoc-lint.
- gopls analyzers — Go's static-analysis framework / built-in passes.
  https://go.dev/gopls/analyzers.
- golang/go issue #44056 — "create a special annotation to indicate
  that a function panics". https://github.com/golang/go/issues/44056.

These are the most directly Go-relevant artefacts found, and #44056 is
notable because it confirms the `go-panics` pattern's premise: Go has
no machine-checkable panic annotation, so panic promises live in
free-text doc comments. `godoc-lint` enforces godoc style/structure
(presence, capitalisation, deprecation form), not doc-vs-implementation
contradiction.

Verdict: rejected. Tool documentation, a linter, and a GitHub proposal
issue — none is a peer-reviewed publication, none provides a
quantitative Go-corpus evaluation of doc-code inconsistency detection.
Cannot satisfy (a) / (b) / (c). Useful as design context only.

### iComment / aComment cited-by graph (Go filter)

Filtering the "cited by" graph of Tan et al. (2007, 2011) for Go
subjects yields the same shape as the Python and TypeScript surveys: a
Java-heavy replication/extension cluster (Wen 2019, Panthaplackel 2021,
C4RLLaMA 2025, the BERT/Longformer and Confidence-Learning detectors), a
C/C++ kernel cluster, and Android-Java work. No entry uses Go as an
experimental subject for comment/code inconsistency detection. The only
works that name Go at all are the CodeSearchNet-pre-trained DocChecker
(Go in pre-training only) and unrelated Go MSR studies (concurrency,
Stack Overflow Q&A, declined-proposal linking) that do not touch
doc-code inconsistency.

Verdict: no qualifying clause-(b) secondary application found.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the comment-code pattern on Go. The
peer-reviewed CCI literature (Wen ICPC 2019, Panthaplackel AAAI 2021,
C4RLLaMA ICSE 2025, the BERT/Longformer detector) is Java-only.
DocChecker (EACL 2024 demo) is the only work that names Go, but Go
appears solely in its pre-training corpus and inference capability — its
inconsistency-detection evaluation is the Java JITDATA post-hoc set, and
it is a demonstration-track paper besides. DocPrism — the multi-language
near-miss for TypeScript — does not evaluate Go at all and is an
unaccepted preprint. The Go-specific artefacts that exist (godoc-lint,
gopls, the golang/go panic-annotation proposal) are tooling, not
peer-reviewed evaluations. The detector ships its Go extension
regardless: P1 itself remains satisfied (the two grandfathered Rust
citations keep the citation set non-empty), and the per-language gap is
captured in metadata.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(comment-code Go coverage:
  unconfirmed; survey notes at
  docs/surveys/comment-code-go-2026-06.md)` line under the detector's
  subsection so the consistency test sees an acknowledged gap rather
  than a silent one. (This edit is performed by the integrating PR, not
  by this survey document.)
- The detector emits `LanguageCitationStatus::Unconfirmed` on every Go
  finding (already the case for the shipped `go-panics` pattern). SARIF
  consumers can filter or visually flag indirectly-grounded Go results
  via `properties.languageCitationStatus`.

## Revisit triggers

Re-run this survey if any of the following happens:

- DocPrism (arXiv 2511.00215) is accepted at a peer-reviewed venue AND
  its evaluation is extended to include a Go corpus — both would be
  required, since today it neither is accepted nor evaluates Go.
- A peer-reviewed paper applies iComment/CCI-style detection to a Go
  corpus with quantitative evaluation (clause b candidate, paired with
  the existing language-agnostic CCI citation, e.g. Panthaplackel AAAI
  2021 or C4RLLaMA ICSE 2025 as the primary half).
- A peer-reviewed Go comment/doc dataset specifically labels
  doc-vs-implementation contradictions — e.g. a CodeSearchNet-Go split
  re-annotated for inconsistency, or a panic-promise-vs-actual-panic
  corpus (clause c candidate).
- The golang/go panic-annotation proposal (#44056) is accepted and a
  peer-reviewed empirical study evaluates panic-documentation
  consistency on a Go corpus (would directly ground the `go-panics`
  pattern).
- A future DocChecker successor reports per-language Go detection
  numbers from a research-track (non-demo) evaluation.
