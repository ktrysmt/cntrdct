# Literature survey: comment-code → TypeScript

Date: 2026-06-05
Detector: `comment-code`
Target language: TypeScript
Surveyor: cntrdct R-2.f PR (comment-code TypeScript)

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
search. The honest default is Unconfirmed — the Python survey for this
same detector (`docs/surveys/comment-code-python-2026-05.md`) reached
that conclusion, and TypeScript is held to the same bar.

Two honesty constraints govern this survey specifically:

- JavaScript is NOT TypeScript. A paper whose only dynamic-language
  subject is JavaScript (or a CodeSearchNet split that lists JavaScript
  but not TypeScript) does not satisfy clause (a) for TypeScript.
- Preprints (arXiv-only, no venue acceptance), tool docs, and blogs do
  not qualify under any clause; clause (b)'s secondary application must
  be peer-reviewed with quantitative TypeScript-corpus evaluation.

## Existing citations

The detector currently cites:

- `tan-sosp-2007` — L. Tan, D. Yuan, G. Krishna, Y. Zhou,
  "/*iComment: Bugs or Bad Comments?*/", SOSP 2007. Subjects:
  C/C++ Linux kernel comments.
- `tan-pldi-2011` — L. Tan, Y. Zhou, Y. Padioleau,
  "aComment: Mining Annotations from Comments and Code to Detect
  Interrupt-related Concurrency Bugs", PLDI 2011. Subjects:
  C/C++ kernel concurrency annotations.

Both are grandfathered as Rust-grounded under the unrevised clause (b)
when cntrdct shipped v0. Python coverage is `unconfirmed` (see the
Python survey). New languages, including TypeScript, follow the strict
clause (b).

The shipped TypeScript pattern is `ts-throws`: the JSDoc `@throws` /
`@exception` tag or prose ("throws", "may throw", "will throw") claims
the function throws, but the function body contains no `throw`
statement. This is the iComment-style comment/code-inconsistency
concept (a documented promise contradicted by the implementation),
specialised to the TypeScript/JSDoc exception-documentation convention.

## Search

Databases / sources queried (2026-06-05):

- Google Scholar / general web: `TypeScript JSDoc comment code
  inconsistency detection empirical study`; `code comment inconsistency
  detection TypeScript corpus quantitative evaluation`; `JSDoc
  documentation mismatch JavaScript TypeScript static analysis peer
  reviewed`; `exception documentation @throws mismatch detection
  empirical study`.
- ACM DL / IEEE Xplore / dblp: venues ICSE/FSE/ICPC/MSR/SANER/AAAI/EACL,
  keywords `TypeScript` AND (`comment` OR `documentation` OR `JSDoc`)
  AND (`inconsistency` OR `mismatch`).
- arXiv (cs.SE): same keywords, last 2 years, to identify preprints and
  check whether any had since gained a peer-reviewed venue.
- iComment/aComment "cited by" graph filtered for TypeScript subjects.
- Exception-documentation cluster (`@throws` / Javadoc-vs-source
  mismatch studies) to match the `ts-throws` pattern specifically.

## Candidates considered

### DocPrism: Local Categorization and External Filtering to Identify Relevant Code-Documentation Inconsistencies

arXiv 2511.00215. https://arxiv.org/abs/2511.00215.

The single most on-point candidate found. DocPrism is a multi-language
code-documentation inconsistency detector (LLM + the "Local
Categorization, External Filtering" methodology) whose evaluation
explicitly spans Python, TypeScript, C++, and Java. Its extension
dataset reports 494 TypeScript functions and an overall precision of
0.62 across languages — i.e. it does perform quantitative
inconsistency detection with TypeScript among its subjects, which would
satisfy clause (a) on subject grounds.

Verdict: rejected. It is an arXiv preprint (submitted 2025-10-31) with
no "Comments:" field indicating conference or journal acceptance as of
this survey. `docs/spec/citations-policy.md` explicitly excludes
preprints ("Preprints, blog posts, and the cntrdct project itself do
not satisfy the secondary-application requirement"). This is the
clearest example of the honest-default rule biting: a TypeScript-subject
paper exists, but it has not yet cleared peer review, so it cannot
ground the language. This is the primary revisit trigger below.

### Sahar et al., "Code Comment Inconsistency Detection and Rectification Using a Large Language Model" (C4RLLaMA, ICSE 2025)

ICSE 2025 Research Track (peer-reviewed). IEEE Xplore:
https://ieeexplore.ieee.org/document/11029963/. ACM DL:
https://dl.acm.org/doi/10.1109/ICSE55347.2025.00035.

A full peer-reviewed research-track paper. C4RLLaMA fine-tunes
CodeLLaMA to both detect and rectify code-comment inconsistencies,
beating prior just-in-time and post-hoc CCI baselines. It is exactly
the iComment-lineage detection task and a strong publication.

Verdict: rejected for clause (a). Its experiments run on the
established just-in-time and post-hoc CCI benchmarks (the Panthaplackel
JIT dataset and the post-hoc method-comment datasets), all of which are
Java corpora. No TypeScript subjects. It could serve as the primary
half of a clause (b) pair (language-agnostic CCI detection), but it
provides no TypeScript evaluation itself and no independent
peer-reviewed TypeScript application was found to complete the pair.

### Panthaplackel et al., "Deep Just-In-Time Inconsistency Detection Between Comments and Source Code" (AAAI 2021)

AAAI 2021 (peer-reviewed). PDF:
https://users.ece.utexas.edu/~gligoric/papers/PanthaplackelETAL21InconsistencyDetection.pdf.
Artifact: https://github.com/panthap2/deep-jit-inconsistency-detection.

The foundational large-scale CCI benchmark (JITDATA) and the model that
most subsequent CCI work (including C4RLLaMA) builds on.

Verdict: rejected for clause (a)/(c) on TypeScript. The JITDATA corpus
is Java. As a benchmark it grounds Java, not TypeScript, so it cannot
satisfy clause (c) here, and its subjects are not TypeScript for clause
(a). Same clause-(b)-primary-but-no-TS-secondary situation as above.

### Wen, Nagy, Bavota, Lanza, "A Large-Scale Empirical Study on Code-Comment Inconsistencies" (ICPC 2019)

ICPC 2019 (peer-reviewed). https://www.inf.usi.ch/lanza/Downloads/Wen2019a.pdf.
DL: https://dl.acm.org/doi/abs/10.1109/ICPC.2019.00019.

The strongest peer-reviewed empirical study in the iComment lineage;
mines 1.3 billion AST-level changes from 1,500 projects to taxonomise
inconsistency classes. Already evaluated and rejected in the Python
survey.

Verdict: rejected for clause (a). The corpus is exclusively Java; no
TypeScript subjects. Language-agnostic in principle (clause (b)
primary), but no independent peer-reviewed paper applies its taxonomy
to a TypeScript corpus with quantitative evaluation.

### Nguyen et al., "DocChecker: Bootstrapping Code Large Language Model for Detecting and Resolving Code-Comment Inconsistencies" (EACL 2024, Demonstrations)

EACL 2024 System Demonstrations. https://aclanthology.org/2024.eacl-demo.20.pdf.

A multi-language CCI detection/resolution tool trained on CodeSearchNet.

Verdict: rejected. Two independent reasons. (1) CodeSearchNet covers
Go, Java, JavaScript, PHP, Python, and Ruby — TypeScript is not one of
its languages, and JavaScript is not TypeScript for clause (a). (2) It
is a demonstration-track paper, not a full research-track evaluation;
the policy requires quantitative evaluation on a corpus in the target
language, which is absent for TypeScript here.

### Exception-documentation / @throws-mismatch studies (Maven, Android API, Eclipse)

Representative peer-reviewed works in this cluster (matching the
`ts-throws` pattern in spirit):
- "The exception handling riddle: An empirical study on the Android
  API" — https://discovery.ucl.ac.uk/id/eprint/10102129/ (static
  analysis of documented-vs-thrown exceptions).
- "Should We Beware the Exceptions? An Empirical Study on the Eclipse
  Project" — https://ieeexplore.ieee.org/document/6821157/.

These directly target the documented-exception-vs-actual-throw mismatch
that `ts-throws` detects, which made them worth checking.

Verdict: rejected for clause (a). Every subject in this cluster is Java
(Javadoc `@throws`, Android/Eclipse Java APIs). None analyse TypeScript
or the JSDoc `@throws` convention. The detection concept transfers, but
the grounding language does not.

### Code2Doc: A Quality-First Curated Dataset for Code Documentation (arXiv 2025)

arXiv 2512.18748. https://arxiv.org/abs/2512.18748.

A curated function-documentation dataset that, per its description,
includes TypeScript and JavaScript function/documentation pairs
extracted with Tree-sitter — superficially a clause (c) candidate
(TypeScript dataset).

Verdict: rejected. Two reasons. (1) arXiv preprint, no venue acceptance
— excluded by policy. (2) Wrong task: it is a documentation-generation
quality dataset (function → high-quality doc), not a comment/code
inconsistency dataset. Clause (c) requires a dataset "relevant to the
detection"; a clean doc-generation corpus does not label
doc-vs-implementation contradictions.

### CCISolver (arXiv 2506.20558); "Larger Is Not Always Better: Leveraging Structured Code Diffs for Comment Inconsistency Detection" (arXiv 2512.19883)

https://arxiv.org/pdf/2506.20558 ; https://arxiv.org/pdf/2512.19883.

Recent CCI detection/repair preprints.

Verdict: rejected. Both are arXiv preprints without confirmed
peer-reviewed venue acceptance (policy excludes preprints), and both
build on the same Java JIT/post-hoc CCI benchmarks — no TypeScript
corpus.

### Community tools (TSDoc, TypeDoc, eslint-plugin-jsdoc, pydoclint analogues)

TSDoc/TypeDoc and `eslint-plugin-jsdoc` enforce JSDoc/TSDoc tag
correctness on TypeScript, including some `@throws`/tag-presence rules.

Verdict: rejected. Tool documentation and linters, not peer-reviewed
publications. Cannot satisfy (a) / (b) / (c).

### iComment / aComment cited-by graph (TypeScript filter)

Filtering the ~1500-paper "cited by" graph of Tan et al. (2007, 2011)
for TypeScript subjects yields the same shape as the Python survey: a
Java-heavy replication/extension cluster (Wen 2019, Ratol 2017, Steidl
2013, Panthaplackel 2021, C4RLLaMA 2025), a C/C++ kernel cluster, and
Android-Java work. The only entries that touch TypeScript at all are
the 2025 arXiv preprints above (DocPrism, CCISolver), none peer-reviewed.

Verdict: no qualifying clause-(b) secondary application found.

## Conclusion

No surveyed publication satisfies clause (a), (b), or (c) of
`docs/spec/citations-policy.md` for the comment-code pattern on
TypeScript. The peer-reviewed CCI literature (Wen ICPC 2019,
Panthaplackel AAAI 2021, C4RLLaMA ICSE 2025, DocChecker EACL 2024) is
Java-only (DocChecker adds JavaScript, which is not TypeScript). The
one work that genuinely evaluates on a TypeScript corpus — DocPrism —
is an arXiv preprint the policy explicitly excludes. The detector ships
its TypeScript extension regardless: P1 itself remains satisfied (the
two grandfathered Rust citations keep the citation set non-empty), and
the per-language gap is captured in metadata.

## Decision

- No new citation entry added to `Citation::CITATIONS`.
- `CITATIONS.md` adds an explicit `(comment-code TypeScript coverage:
  unconfirmed; survey notes at
  docs/surveys/comment-code-typescript-2026-06.md)` line under the
  detector's subsection so the consistency test sees an acknowledged
  gap rather than a silent one.
- The detector emits `LanguageCitationStatus::Unconfirmed` on every
  TypeScript finding (already the case for the shipped `ts-throws`
  pattern). SARIF consumers can filter or visually flag
  indirectly-grounded TypeScript results via
  `properties.languageCitationStatus`.

## Revisit triggers

Re-run this survey if any of the following happens:

- DocPrism (arXiv 2511.00215) is accepted at a peer-reviewed venue with
  its TypeScript evaluation intact — it would then be a strong clause
  (a) candidate (TypeScript subjects, quantitative precision reported).
- Any other peer-reviewed paper applies iComment/CCI-style detection to
  a TypeScript corpus with quantitative evaluation (clause b candidate,
  paired with the existing language-agnostic CCI citation).
- A peer-reviewed TypeScript/JSDoc dataset specifically labels
  doc-vs-implementation contradictions or `@throws`-vs-actual-throw
  mismatches (clause c candidate).
- A peer-reviewed JSDoc `@throws` / TSDoc exception-documentation
  mismatch study lands (directly grounds the shipped `ts-throws`
  pattern).
