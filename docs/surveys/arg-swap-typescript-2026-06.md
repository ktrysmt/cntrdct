# Literature survey: arg-swap → TypeScript

Date: 2026-06-05
Detector: `arg-swap`
Target language: TypeScript
Surveyor: cntrdct R-2.f PR (arg-swap TypeScript grounding)

## Goal

Per `docs/spec/citations-policy.md`, when extending a detector to a new
language the implementer must locate at least one peer-reviewed
publication or established benchmark that (a) has TypeScript source as
its experimental subject, or (b) is the language-agnostic algorithm we
already cite plus an independent peer-reviewed paper applying it to
TypeScript with quantitative evaluation on a TypeScript corpus, or (c)
introduces a TypeScript benchmark / dataset relevant to the detection.

If no candidate satisfies any of (a) / (b) / (c), the language
extension still ships and emits findings with
`LanguageCitationStatus::Unconfirmed`. This document records the
search.

A hard constraint specific to this survey: JavaScript is NOT
TypeScript. They are distinct languages with distinct ASTs (TypeScript
adds type annotations, interfaces, enums, generics, decorators, and the
`.ts`/`.tsx` grammar that tree-sitter parses separately). A paper whose
experimental subjects are JavaScript files only does NOT satisfy clause
(a) for TypeScript, no matter how closely the bug class matches. This
distinction rejects the otherwise-strongest candidate (DeepBugs).

## Detector concept under grounding

`arg-swap` flags a 2-argument call site whose argument identifiers are
the reverse permutation of the callee's parameter names
(case-insensitive, with abbreviation-aware prefix matching) — i.e. a
same-typed swapped-argument defect detected by parameter-name
correlation, in the lineage of Rice et al. (ICSE 2017). The TypeScript
pipeline reuses the shared IR definition extraction plus a raw-tree
call walk (Pattern B); see `src/detectors/arg_swap.rs`.

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

None of these declares TypeScript as a grounded language. The TS
pipeline currently emits `LanguageCitationStatus::Unconfirmed` with
`citation_keys = ["li-zhou-fse-2005", "rice-icse-2017"]`.

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / general web: `TypeScript "argument swap" OR
  "swapped arguments" OR "argument selection defect" bug detection
  empirical evaluation`; `TypeScript name-based bug detection swapped
  arguments DeepBugs parameter name corpus peer reviewed`; `TypeScript
  neural bug detector swapped arguments call site precision recall
  ICSE FSE ASE 2023 2024 2025`.
- ACM Digital Library / IEEE Xplore (via web index): venue filter
  ICSE/FSE/ASE/MSR/OOPSLA 2017-2026, keywords `TypeScript` AND
  (`argument swap` OR `swapped arguments` OR `argument selection
  defect` OR `name-based bug detection`).
- dblp / publisher pages: Rice et al. (ICSE 2017) and Pradel & Sen
  (DeepBugs, OOPSLA 2018) "cited by" graph, filtered for TypeScript
  experimental subjects.
- arXiv cs.SE, last 5 years, same keywords (used only to confirm
  peer-review status / exclude preprint-only work; preprints do not
  qualify per the policy).

## Candidates considered

### Rice, Aftandilian, Jaspan, Johnston, Pradel, Arroyo-Paredes, "Detecting Argument Selection Defects" (ICSE 2017)

DOI 10.1145/3133928.
https://dl.acm.org/doi/10.1145/3133928
Open-access PDF: https://research.google.com/pubs/archive/46317.pdf

The originating paper for cntrdct's parameter-name-correlation
approach. Algorithm uses identifier names to flag wrong-argument
method calls. Evaluated at Google on 200 MLOC internal + 10 MLOC
external code; found defects in OpenJDK, ASM, MySQL JDBC.

Verdict: rejected for TypeScript clause (a). Experimental subjects are
Java. Already cited as the algorithmic basis (grandfathered Rust);
provides no TypeScript grounding. It is the language-agnostic primary
of a potential clause (b) pairing, but no qualifying TypeScript
secondary application exists (see Conclusion).

### Pradel, Sen, "DeepBugs: A Learning Approach to Name-based Bug Detection" (OOPSLA 2018)

DOI 10.1145/3276517. arXiv 1805.11683.
https://dl.acm.org/doi/abs/10.1145/3276517

Name-based bug-detection framework. Its `SwappedArgs` detector is the
closest published match to cntrdct's bug class — it flags accidentally
swapped function arguments (e.g. `setPoint(y, x)` for
`setPoint(x, y)`) using a learned semantic representation of
identifier names. Corpus: 100k training + 50k validation JavaScript
files, 68M LOC; reported accuracy 89–95%.

Verdict: rejected for TypeScript clause (a). The experimental subjects
are JavaScript only. Under this survey's hard JS≠TS rule, JavaScript
subjects do not ground TypeScript. The DeepBugs framework is in
principle retargetable, but no peer-reviewed paper applies it to a
TypeScript corpus with quantitative evaluation (the
`JetBrains-Research/DeepBugsPlugin` IDE plugin advertises some
multi-language support but is a community tool with no peer-reviewed
publication, so it cannot satisfy clause (b)).

### Davis et al., "A replication of 'DeepBugs: a learning approach to name-based bug detection'" (ESEC/FSE 2021, ROSE Festival)

DOI 10.1145/3468264.3477221.
https://dl.acm.org/doi/10.1145/3468264.3477221

Independent replication of the DeepBugs swapped-arguments detector on
the original 150k JavaScript dataset; reproduces accuracy within ~2%.

Verdict: rejected. JavaScript-only; does not extend DeepBugs to
TypeScript.

### Liu, Foyen, Levin, "Out of Sight, Out of Place: Detecting and Assessing Swapped Arguments" (ASE 2020)

IEEE Xplore (ASE 2020). arXiv 2009.09117.
https://arxiv.org/abs/2009.09117

Static-analysis tool SWAPD uses natural-language information in
identifiers to flag mistakenly-swapped arguments at call sites.
Evaluated on 417M LOC of C and C++; reports 154 manually-vetted
real-world swap bugs.

Verdict: rejected for TypeScript clause (a). Subjects are C and C++.
Strong direct lineage of the Rice-style method but no TypeScript
subjects, and no independent peer-reviewed TypeScript application of
SWAPD exists as of 2026-06.

### Chen, Hernández López, Mussbacher, Varró, "The Power of Types: Exploring the Impact of Type Checking on Neural Bug Detection in Dynamically Typed Languages" (ICSE 2025)

arXiv 2411.15368; accepted to ICSE 2025 Research Track (peer-reviewed).
https://arxiv.org/abs/2411.15368

Studies how type information affects neural bug detection in
dynamically typed languages. The most TypeScript-adjacent recent
peer-reviewed candidate found.

Verdict: rejected on two independent grounds. (1) The targeted bug
class is variable misuse, not swapped arguments — different detector.
(2) The experimental subjects are dynamically typed code (Python);
TypeScript is statically typed and is not an experimental subject. The
paper's relevance to "types" is conceptual, not a TypeScript-corpus
evaluation. Satisfies neither (a) nor the bug-class relevance the
Python precedent (Allamanis) required.

### Tang, Alimadadi, Sumner, "From Logic to Toolchains: An Empirical Study of Bugs in the TypeScript Ecosystem" (2026)

arXiv 2601.21186 (submitted 2026-01-29). cs.SE preprint.
https://arxiv.org/abs/2601.21186

A genuine TypeScript-subject empirical study: 633 bug reports across 16
open-source TypeScript repositories, building a fault taxonomy. This is
the only located work whose subjects are actual TypeScript code at
scale.

Verdict: rejected on two independent grounds. (1) Provenance: it is an
arXiv-only preprint at survey time; the citation policy excludes
preprints (clause (b) explicitly, and clause (a)/(c) by the
"peer-reviewed publication or established benchmark" requirement). (2)
Relevance: its fault taxonomy centres on tooling/configuration faults,
API misuses, and asynchronous error-handling; it does not isolate
swapped-arguments / argument-selection defects as a quantified category
and does not introduce a swap-labelled benchmark. Revisit if it is
accepted at a peer-reviewed venue AND a swap-related category with
quantitative data emerges.

### Gao, Bird, Barr, "To Type or Not to Type: Quantifying Detectable Bugs in JavaScript" (ICSE 2017)

DOI 10.1109/ICSE.2017.75.
https://dl.acm.org/doi/10.1109/ICSE.2017.75

Evaluates whether adding TypeScript / Flow type annotations would have
caught public bugs in JavaScript projects.

Verdict: rejected. Experimental subjects are JavaScript bugs;
TypeScript appears only as a candidate type system applied as a fix,
not as the analysed corpus. The studied defect class is type errors
caught by annotation, not same-typed argument swaps detected by
parameter-name correlation (which type checking precisely cannot catch
when the swapped parameters share a type). Neither bug class nor
subject language matches.

### Richter, Wehrheim, "How to Train Your Neural Bug Detector: Artificial vs Real Bugs" (ASE 2023) and related neural-mutation work

DOI 10.1109/ASE56229.2023.00104 (peer-reviewed); plus DeepMutants
(arXiv 2107.06657) and "On Distribution Shift in Learning-based Bug
Detectors" (ICML 2022, arXiv 2204.10049).

This cluster studies training data for DeepBugs-style detectors
(including swapped-arguments) and distribution shift to real bugs.

Verdict: rejected for TypeScript clause (a). Experimental subjects are
Python (and JavaScript via the inherited DeepBugs corpora); none uses a
TypeScript corpus. Relevant context for the swapped-arguments bug class
but no TypeScript grounding.

### pylint-equivalents for TS: ESLint, typescript-eslint, Biome, ts-prune, SonarJS/SonarTS

Community linters / type-aware tooling. typescript-eslint and the TS
compiler catch type-mismatched argument errors, but not the same-typed
swap pattern (identical parameter types pass the type check). No
peer-reviewed publication backs a TypeScript argument-swap rule with a
quantitative corpus evaluation.

Verdict: rejected. Tools, not peer-reviewed publications; cannot
satisfy (a) / (b) / (c). (Note: SonarTS was deprecated and folded into
SonarJS, underscoring that even tool support treats TS argument
analysis via the JS rule engine, not a TS-specific empirical study.)

### Rice et al. and Pradel-Sen "cited by" graphs

Filtered the top citing papers (by venue and citation count) of Rice
et al. (ICSE 2017) and DeepBugs (OOPSLA 2018) for TypeScript
experimental subjects. The graph is dominated by Java, C/C++,
JavaScript, and Python replications/extensions (plus Android/Kotlin
for Rice). No paper performs a TypeScript-corpus replication of the
parameter-name-correlation argument-swap method.

Verdict: no clause-(b) TypeScript secondary application surfaced.

## Conclusion

No peer-reviewed publication or established benchmark grounds the
`arg-swap` detector for TypeScript:

- Clause (a): no qualifying paper has TypeScript experimental subjects
  for the swapped-argument / argument-selection-defect bug class. The
  closest swapped-argument works are Java (Rice 2017), C/C++ (SWAPD
  2020), JavaScript (DeepBugs 2018, replication 2021), and Python
  (Allamanis 2021). JavaScript ≠ TypeScript, so DeepBugs does not
  transfer the grounding. The one genuine TypeScript-subject study
  (Tang et al. 2026) is an arXiv preprint and does not isolate
  argument swaps.
- Clause (b): the language-agnostic primaries we already cite
  (Rice 2017, Li-Zhou 2005) have no independent peer-reviewed
  TypeScript application with quantitative evaluation on a TypeScript
  corpus. Tool support (typescript-eslint, DeepBugsPlugin) is not
  peer-reviewed and is excluded.
- Clause (c): no TypeScript benchmark/dataset with swapped-argument
  labels exists. PyPIBugs (the Python analogue) has no TypeScript
  counterpart.

This is the honest, expected outcome — the same as 3 of 5 Python-era
surveys. The TypeScript extension ships under
`LanguageCitationStatus::Unconfirmed`; P1 is preserved because the
detector's overall citation set is non-empty and real.

## Decision

- Add NO TypeScript citation to `Citation::CITATIONS` in
  `src/detectors/arg_swap.rs`.
- The TypeScript pipeline keeps emitting
  `LanguageCitationStatus::Unconfirmed` with
  `citation_keys = ["li-zhou-fse-2005", "rice-icse-2017"]` (the
  algorithmic-lineage citations), so SARIF consumers see the grounding
  is indirect.
- `CITATIONS.md` adds the explicit-no-citation form under the arg-swap
  subsection, pointing readers at this survey:
  `(arg-swap TypeScript coverage: unconfirmed; survey notes at
  docs/surveys/arg-swap-typescript-2026-06.md)`.
  (Integration of CITATIONS.md is handled centrally by the integrator,
  not by this survey.)

## Revisit triggers

Re-run this survey if any of the following happens:

- A peer-reviewed paper applies a name-based / parameter-name
  argument-swap detector (DeepBugs-style or Rice-style) to a
  TypeScript corpus with quantitative evaluation — that would satisfy
  clause (b) (paired with `rice-icse-2017`) or clause (a) directly.
- A peer-reviewed TypeScript bug benchmark/dataset ships with a
  swapped-arguments / argument-selection label set — clause (c). Watch
  Tang et al. (arXiv 2601.21186) for acceptance at a peer-reviewed
  venue and for a swap-specific category.
- DeepBugs (Pradel & Sen) or SWAPD (Liu et al.) gains a peer-reviewed
  TypeScript replication with a TypeScript corpus.
- typescript-eslint or a successor publishes a peer-reviewed empirical
  evaluation of a same-typed argument-swap rule on TypeScript code
  (current tool docs are not peer-reviewed and do not qualify).
