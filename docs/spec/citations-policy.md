# Citation policy v1 spec (P1 extension for multi-language)

Status: draft, awaiting approval. Owner of M-6.

## Background

P1 (the project's most stringent constraint) says every detector
must reference at least one peer-reviewed publication or established
benchmark that justifies its detection. It is enforced at two
points:

- `cntrdct_core::register_detector` rejects detectors with empty
  `citations()` at startup.
- `crates/cli/tests/citations_consistency.rs` asserts every key
  declared by any detector resolves to a `CITATIONS.md` entry.

P1 was authored when cntrdct was Rust-only. The implicit contract
was "the citation justifies the detector existing"; it did not
constrain the language under analysis. Now that the M-series is
adding Python (and later TS / Go / Java), the question becomes:

If a Rust detector that cites e.g. Cordy & Roy (ICPC 2008, NiCad
clone detector) extends to Python, does the same citation suffice?

This document says no, and lays out a stricter rule.

## Rule

A detector that declares it supports a `Language` must provide at
least one citation grounded in empirical work on that language. A
citation is "grounded in" a language if any of the following holds:

(a) The cited paper's experimental subjects include code in that
    language, or
(b) The cited paper's algorithm is explicitly presented as
    language-agnostic and at least one independent published
    application of the algorithm to that language exists (and is
    also cited), or
(c) The cited paper introduces a benchmark / dataset in that
    language.

A single citation can satisfy the requirement for multiple
languages if (a), (b), or (c) holds for each.

The existing Rust citations on the v0 detectors are grandfathered
under (b): the cross-cutting concept papers (Cordy-Roy NiCad, Li-Zhou
PR-Miner, Tan iComment, Engler bugs-as-deviant-behavior) all present
language-agnostic algorithms whose Rust application is documented
via the cntrdct project itself. New languages do not enjoy the same
implicit blessing — they must produce an independent citation.

## Mechanical enforcement

The `Citation` struct gains a `languages` field declaring which
languages the citation is grounded in:

```rust
pub struct Citation {
    pub key: &'static str,
    // ... existing fields ...
    pub languages: &'static [Language],
}
```

`languages: &[]` means the citation is general / methodological and
does not satisfy any per-language requirement on its own (Wilson
score lower bound papers, IEEE 1044-2009, etc. fall here).

`register_detector` is extended:

```rust
pub fn register_detector(d: &dyn Detector) -> Result<(), DetectorError> {
    if d.citations().is_empty() {
        return Err(DetectorError::Config(format!(
            "detector {} has no citations (P1 violation)", d.id(),
        )));
    }
    for lang in d.supported_languages() {
        let has_lang_citation = d.citations()
            .iter()
            .any(|c| c.languages.contains(lang));
        if !has_lang_citation {
            return Err(DetectorError::Config(format!(
                "detector {} declares support for {:?} but has no citation grounded in that language (P1 multi-language violation)",
                d.id(), lang,
            )));
        }
    }
    Ok(())
}
```

`crates/cli/tests/citations_consistency.rs` extends to:

- Assert every detector's `supported_languages()` is covered by at
  least one of its citations' `languages`.
- Assert a deliberately under-cited fixture detector (a small struct
  in the test file claiming `Language::Python` with no
  Python-grounded citation) is rejected.

## Bibliography management

`CITATIONS.md` entries gain a `Languages:` line:

```
- Cordy, J. R. & Roy, C. K. (2008). The NiCad Clone Detector. ICPC.
  Languages: Java, C, Python (via Roy 2009 reapplication).
```

The line is parsed by `citations_consistency.rs` to cross-check
declarations in code. Mismatch (e.g. detector declares
`languages: &[Language::Python]` but the bibliography entry omits
Python) is a test failure.

## Survey requirement per new language

When adding a new language to a detector (M-3 et seq.), the
implementer must:

1. Identify or run a literature search for at least one paper that
   satisfies (a), (b), or (c) above for the target language.
2. Add the bibliography entry to `CITATIONS.md` with a `Languages:`
   line.
3. Update the detector's `Citation` static array, setting
   `languages` correctly.
4. Justify the choice in the PR description: why does this paper
   establish that the detection is meaningful for the new language?

For the M-series specifically, the survey is sized at roughly 1-3
days of literature work per (detector, language) pair. The Phase D
schedule budgets this implicitly inside each M-3 detector.

## Worked example

Adding Python support to `clone-drift`:

- Existing Rust citations: Cordy-Roy (NiCad, ICPC 2008),
  Bettenburg et al. (MSR 2009), Krinke (ICSM 2007). All on
  Java / C / C++.
- For Python, candidate: Selim et al. "Code clone detection in
  Python" (or a contemporary equivalent surfaced during the
  M-3-clone-drift PR survey).
- The new citation joins the static array with
  `languages: &[Language::Python]`. The Rust entries get their
  `languages` updated to reflect the languages they originally
  studied (e.g. Cordy-Roy → `&[Language::Java, Language::C]`).
- `clone-drift::supported_languages()` returns
  `&[Language::Rust, Language::Python]`; `register_detector`
  passes because each language has at least one matching citation.

## Non-goals

- Quality grading of citations (impact factor, citation count,
  venue tier). Any peer-reviewed venue or named benchmark counts.
- Recency requirements. A 1990s paper is acceptable if it actually
  grounds the detection.
- Translation of cited paper titles or abstracts. English-only.

## Risks

- Some detection ideas may have no Python-grounded prior art at
  all. In that case the implementer must either find an indirect
  precedent (a survey paper that mentions the technique in Python
  context) or shelve the (detector, language) pair until prior art
  emerges. Either is acceptable; shipping uncited is not.
- The bibliography file becomes a bottleneck if many languages
  land at once. Mitigation: M-series sequences languages one at a
  time; Phase E onwards adds languages serially.

## Approval

This policy is approved when:

1. The `Citation::languages` field, the `register_detector`
   extension, and the `citations_consistency` test extension are
   all reviewed (M-6 implementation PR).
2. The existing Rust citations have been retro-fitted with
   `languages` arrays without breaking any test.
