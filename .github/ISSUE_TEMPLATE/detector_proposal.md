---
name: Detector proposal
about: Propose a new Layer 1 detector or a new language for an existing one
title: 'detector: '
labels: detector-proposal
assignees: ''
---

<!--
Read this first: cntrdct's most stringent design constraint (P1) is
that every detector must reference at least one peer-reviewed
publication or established benchmark that justifies the detection.
Detectors without a qualifying citation are rejected at startup.

This template enforces P1 at the proposal stage so we do not start
building a detector that cannot ship. If you cannot fill in the
"Citation" section, please open a discussion thread instead, or file a
feature request asking for guidance on finding prior art.
-->

## Detector id

<!-- kebab-case, e.g. `null-deref-after-check`. Must be unique across
the existing detectors listed in `CITATIONS.md`. -->

## Anomaly class

IEEE 1044-2009 anomaly classification. Pick exactly one:

- [ ] Logic
- [ ] Interface
- [ ] Data
- [ ] Documentation
- [ ] Performance
- [ ] Standards
- [ ] Other (explain below)

## Citation (required)

Primary citation that grounds this detector. P1 requires at least one
peer-reviewed publication or established benchmark; preprints, blog
posts, and the cntrdct project itself do not qualify.

- Citation key (proposed, last-author-venue-year style):
- Authors, title, venue, year:
- DOI or stable URL:
- Why this paper grounds the detection (1-3 sentences):
- One-sentence abstract or core claim:

## Citations-policy compliance

For each language this detector will support, name the
`docs/spec/citations-policy.md` clause that the citation satisfies:

- [ ] Clause (a): the cited paper's experimental subjects include code in the target language
- [ ] Clause (b): language-agnostic algorithm + an independent peer-reviewed application paper with quantitative evaluation on a corpus in the target language (cite both)
- [ ] Clause (c): the cited paper introduces a benchmark or dataset in the target language
- [ ] None — willing to ship with `LanguageCitationStatus::Unconfirmed` and a `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` recording the gap

## Languages targeted at v0

- [ ] Rust
- [ ] Python
- [ ] Other (specify):

For each ticked language, a survey under
`docs/surveys/<detector>-<lang>-<YYYY-MM>.md` is required before
implementation lands; clauses (a), (b), and (c) above gate the
strength of the resulting `LanguageCitationStatus`.

## Corpus commitment

- [ ] I will provide at least 8 positive cases per supported language under `benchmarks/corpus/files/` and at least 3 negative cases per supported language

## Detection algorithm sketch

<!-- 5-15 lines. What does the detector look at, and what does it
flag? AST shape, control-flow facts, comments vs body, identifier
similarity, etc. Pseudocode welcome. -->

## Expected false-positive failure modes

<!-- Where will this detector be wrong? Naming the obvious FP shapes
upfront helps the ranker calibration plan. Optional but useful. -->

## Open questions for the maintainers

<!-- Anything you want guidance on before drafting the spec. Optional. -->
