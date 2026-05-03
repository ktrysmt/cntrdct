# cntrdct benchmarks

Labelled corpus and ground-truth manifests used by the `cntrdct eval`
harness. Spec: `../docs/spec/eval-v0.md`.

## Layout

```
benchmarks/
├── README.md
└── corpus/
    ├── manifest.jsonl
    └── files/
        ├── <case>.rs
        └── ...
```

`manifest.jsonl` is one JSON object per non-blank, non-`//` line of the
form:

```
{"file": "files/<case>.rs", "expected": [{"detector_id": "<id>", "line": <N>}]}
```

A case with `"expected": []` represents a true negative — a file the
detectors should produce no findings on.

## Provenance

Every source file in `corpus/files/` carries a header comment of the form

```
// Source: <URL of the originating crate or paper>
// License: <SPDX expression>
// Note: <verbatim | excerpted | with-injected-defect>
```

so the corpus is auditable end-to-end. Verbatim extracts (typical for
negatives) preserve the upstream license, which is also recorded in the
header. Files marked with an injected defect note the specific mutation
that introduced the pattern (for example, "the call at line N reverses
the arguments to exhibit the arg-swap pattern documented in Rice et al.,
ICSE 2017").

## Adding a new case

1. Write the source under `corpus/files/<descriptive_name>.rs`,
   including a provenance header.
2. Run `cargo run --bin cntrdct -- scan benchmarks/corpus/files/<name>.rs`
   to confirm the detector fires on the expected line (or, for a
   negative, that no detector fires).
3. Add a `manifest.jsonl` entry whose `expected` array reflects the
   findings you consider true positives. Lines you intentionally want
   to stay unflagged become FN candidates only if the detectors miss
   them.
4. Run `cargo run --bin cntrdct -- eval benchmarks/corpus` to see the
   updated metrics.
5. Run `cargo test -p cntrdct-cli --test corpus_shape` to confirm the
   prereg-numeric corpus contract still holds.

## Status

- 50 source files: 10 positives per registered Layer 1 detector
  (`arg-swap`, `clone-drift`, `comment-code`,
  `unreachable-after-terminator`) plus 10 true negatives drawn from
  production crates (anyhow, serde, ripgrep).
- The numeric shape of this corpus is preregistered under
  `../prereg/2026-05-03-osf-prereg.md` and enforced by
  `crates/cli/tests/corpus_shape.rs`.

## Caveats

- Positive fixtures are constructed to exhibit the target pattern;
  precision and recall reported by `cntrdct eval` on this corpus are
  therefore an upper bound on real-world precision / recall and are
  appropriate only for regression testing and detector tuning. The β
  phase will rerun the harness against a separately-collected,
  unbiased corpus per the preregistration's stopping rule.
- Files in the corpus are not guaranteed to type-check: tree-sitter
  parses syntactically and the detectors are AST-level, so semantic
  validity is not required. Provenance headers note when a file
  diverges from the upstream source.
