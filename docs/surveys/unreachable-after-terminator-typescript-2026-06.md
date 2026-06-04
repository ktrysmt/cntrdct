# Literature survey: unreachable-after-terminator → TypeScript

Date: 2026-06-05
Detector: `unreachable-after-terminator`
Target language: TypeScript
Surveyor: cntrdct R-2.f PR

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has TypeScript source as
its experimental subject, or (b) is the language-agnostic algorithm we
already cite plus an independent peer-reviewed paper applying it to
TypeScript with quantitative evaluation on a TypeScript corpus, or
(c) introduces a TypeScript benchmark / dataset relevant to the
detection.

JavaScript is explicitly not TypeScript: a JS-only-subject paper does
not satisfy clause (a) for TypeScript, and a JS application of our
cited algorithm does not satisfy clause (b).

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search.

## Detector concept

`unreachable-after-terminator` flags statically unreachable code that
follows a divergent terminator (return / throw / `process.exit(...)` /
break-or-continue / a branch where all arms diverge) within a block —
the FindBugs "UR — Unreachable code" pattern modelled as a control-flow
contradiction at the AST-statement level. The TypeScript scan
(`src/detectors/unreachable_after_terminator.rs::scan_typescript`) maps
`throw` → `Raise`, `process.exit(...)` → divergent process-exit, plus
the if-branch-merge and constant-condition rules. This is distinct from
the unused-symbol sense of "dead code" (unreferenced functions /
imports / variables, the ts-prune / Knip / Vulture lineage).

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
when cntrdct shipped v0; new languages follow the strict (b). For
TypeScript clause (b) we would need an independent peer-reviewed paper
applying the UR / control-flow-contradiction algorithm to a TypeScript
corpus with quantitative evaluation.

## Search

Databases / sources queried:

- Google Scholar / web: queries `TypeScript "unreachable code" "dead
  code" detection empirical study peer-reviewed`, `TypeScript code
  smells static analysis empirical study corpus evaluation`,
  `TypeScript empirical study static analysis warnings unreachable code
  after return throw control flow`, `ESLint no-unreachable rule
  TypeScript empirical evaluation precision recall study`.
- ACM Digital Library / IEEE Xplore (venue filter
  ICSE/FSE/MSR/SANER/ICSME/PLDI/POPL/OOPSLA/IST/TSE, 2015-2026):
  `TypeScript` AND (`unreachable` OR `dead code` OR `control flow
  contradiction`).
- dblp: "cited by" graphs of the Hovemeyer-Pugh and Engler citation
  clusters, filtered for TypeScript-language subjects.
- arXiv: cs.SE / cs.PL, last 5 years, same keyword set.

## Candidates considered

### "Detection of code smells in React with TypeScript applications" — SniffTSX (IST 2025)

ScienceDirect: https://www.sciencedirect.com/science/article/abs/pii/S0950584925001740
ResearchGate: https://www.researchgate.net/publication/397147118_Detection_of_code_smells_in_react_with_TypeScript_applications

Peer-reviewed (Information and Software Technology). Contributes a
catalog of six React-specific TypeScript code smells and the SniffTSX
detection tool, evaluated on real-world React+TypeScript projects
(reported accuracy 0.96 / precision 0.98 / recall 0.93 / F1 0.95). The
most frequent smells are Any Type, Multiple Booleans for State, and
Non-Null Assertions.

Verdict: rejected. TypeScript subjects (would satisfy (a)'s language
requirement), but the smell catalogue is React-API-shaped (typing and
state-management anti-patterns) and contains no unreachable-code,
dead-code, or control-flow-contradiction class. Right language, wrong
target pattern. Citing it for a UR-style detector would be the
loose-fit citation the policy's R-B clause warns against.

### "From Logic to Toolchains: An Empirical Study of Bugs in the TypeScript Ecosystem" (MSR 2026)

arXiv: https://arxiv.org/abs/2601.21186 (HTML: https://arxiv.org/html/2601.21186v1)

Peer-reviewed and accepted at the 23rd International Conference on
Mining Software Repositories (MSR '26), Rio de Janeiro. Analyses 633
bug reports from 16 open-source TypeScript projects (Angular, Vue,
Chakra-UI, NestJS, Insomnia, ...) and derives an eleven-category bug
taxonomy: Asynchrony/Event, Error Handling, Missing Case, Missing
Feature, Runtime Exception, Tooling/Config, Type Error, Logic Error,
API Misuse, Test Fault, UI Bug.

Verdict: rejected. Strong peer-reviewed TypeScript subject base, but
the taxonomy is built from observable faults in bug reports, not from
static code patterns; unreachable-code / code-after-terminator is not a
category. No quantitative evaluation of the UR pattern on the corpus.
Does not satisfy (a) for our pattern, nor (b), nor (c).

### Malavolta et al., "JavaScript Dead Code Identification, Elimination, and Empirical Assessment" (IEEE TSE 2023)

arXiv: https://arxiv.org/abs/2308.16729
DOI: 10.1109/TSE.2023.3267848

Peer-reviewed (IEEE Transactions on Software Engineering). Empirical
study on 30 mobile web applications measuring the run-time overhead
(energy, performance, network, resource usage) of dead code.

Verdict: rejected on two independent grounds. (1) The subjects are
JavaScript, not TypeScript — JS does not satisfy (a) or (b) for a
TypeScript extension per the policy. (2) "Dead code" here means
"code implementing unused functionalities" (the unused-symbol /
reachability-from-entry-point sense), a different bug model from the
intra-block control-flow contradiction the detector flags.

### "A Calculus for Unreachable Code" (arXiv 2407.04917, 2024)

arXiv: https://arxiv.org/abs/2407.04917

A formal PL-theory paper: the first calculus for unreachable code,
correctness proved via a logical relation, related to transformations
in Racket and LLVM.

Verdict: rejected. (1) Preprint only — no peer-reviewed venue listed.
(2) Theory paper with no TypeScript corpus and no quantitative
(precision/recall) evaluation. Satisfies none of (a) / (b) / (c).

### "A Multi-Study Investigation Into Dead Code" (IEEE TSE 2018)

PDF: https://www.cs.wm.edu/~denys/pubs/TSE'18-DeadCode.pdf

Peer-reviewed (IEEE TSE). Studies when/why developers introduce dead
code, how they perceive it, and whether it is harmful, via interviews
and experiments at Basilicata and William & Mary.

Verdict: rejected. Subjects are Java; "dead code" is the unused/unused-
functionality sense, not unreachable-after-terminator. Neither the
language nor the pattern matches.

### ESLint `no-unreachable` rule

ESLint docs: https://eslint.org/docs/latest/rules/no-unreachable
typescript-eslint #1041: https://github.com/typescript-eslint/typescript-eslint/issues/1041

The `no-unreachable` rule disallows code after return / throw /
continue / break — exactly the UR pattern. Notably the TypeScript
compiler itself flags this (error TS7027), and `@typescript-eslint`'s
recommended config disables `no-unreachable` because the compiler
already covers it.

Verdict: rejected. Tool / compiler feature, not a peer-reviewed
publication; no empirical study with quantitative TypeScript-corpus
evaluation was found. Cannot satisfy (a) / (b) / (c). (Recorded here
because it confirms the pattern is well-established in TypeScript
tooling, which is relevant to a future clause-(b) search but is not
itself citable.)

### Community dead-code tools (ts-prune, Knip, dead-code-checker, Vulture)

ts-prune / Knip / denisoed/dead-code-checker (TypeScript), Vulture
(Python). Community tools, no peer-reviewed evaluation, and all target
the unused-symbol sense rather than the control-flow contradiction.

Verdict: rejected on the same grounds as the Python survey's Vulture
entry — tools, not publications, and wrong bug model.

### DCE-LLM / dead-code-poisoning papers (arXiv 2506.11076, 2502.20246)

LLM-based dead-code elimination and dead-code-poisoning detection in
code-generation datasets.

Verdict: rejected. Preprints; not TypeScript-subject; concern
dead-code elimination / dataset poisoning, not static unreachable-after-
terminator detection.

### Citation cited-by graphs

- Hovemeyer-Pugh (OOPSLA 2004): no paper in the filtered top results
  presents a quantitative evaluation of the FindBugs UR pattern on a
  TypeScript corpus. The lineage stays Java (SpotBugs) and branches to
  Android / Kotlin, not TypeScript at the AST-statement level.
- Engler-SOSP-2001: extended to JavaScript (Pradel-Sen-style work),
  C/C++ (Coverity), and kernels, but no TypeScript control-flow-
  contradiction application with quantitative TS-corpus evaluation was
  found.

Verdict: no qualifying clause-(b) secondary application found for
TypeScript.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the unreachable-after-terminator
pattern on TypeScript. The closest peer-reviewed TypeScript-subject
works (SniffTSX, IST 2025; the MSR '26 TypeScript bug study) do not
enumerate unreachable-code / control-flow contradiction. The
peer-reviewed dead-code works (Malavolta TSE 2023; the TSE '18
multi-study) are either JavaScript/Java or use the unused-symbol sense.
The UR pattern is well-supported in TypeScript tooling (the TSC TS7027
check, ESLint `no-unreachable`), but tooling and compiler features are
not peer-reviewed publications and so cannot ground the citation.

Mirroring the Python precedent, the honest default applies: TypeScript
coverage is Unconfirmed. The detector ships its TypeScript extension
regardless — P1 remains satisfied (the two grandfathered Rust citations
are non-empty), and the per-language gap is captured in metadata.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(unreachable-after-terminator
  TypeScript coverage: unconfirmed; survey notes at this file)` line
  under the detector's subsection so the consistency test sees an
  acknowledged gap rather than a silent one.
- The detector emits `LanguageCitationStatus::Unconfirmed` on every
  TypeScript finding (already the case in `scan_typescript`). SARIF
  consumers can filter or visually flag indirectly-grounded TypeScript
  results via `properties.languageCitationStatus`.

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies the FindBugs UR / control-flow-
  contradiction pattern to a TypeScript corpus with quantitative
  evaluation (clause b candidate).
- A new TypeScript benchmark / dataset specifically labels
  unreachable-statement / code-after-terminator defects (clause c
  candidate).
- A peer-reviewed empirical study of TypeScript code smells or
  static-analysis warnings adds an unreachable-code category with a
  TypeScript-corpus evaluation (clause a candidate).
- The Hovemeyer-Pugh bug-pattern taxonomy is formally extended to
  TypeScript in a published reference work.
