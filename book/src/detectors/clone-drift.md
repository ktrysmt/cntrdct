# clone-drift

Flags a near-duplicate function whose AST has diverged from the
majority of its siblings in the same scope.

- **Rust citation:** Juergens, Deissenboeck, Hummel, Wagner ICSE 2009.
- **Python citation:** Assi, Hassan, Zou TOSEM 2025 (NiCad /
  SourcererCC on nine Python DL frameworks). Confirmed.
- **IEEE 1044-2009 class:** Logic.
- **Default severity:** Warning.

The v0 detector clusters functions by AST shape with a Jaccard ≥ 0.7
gate, requires `MIN_FN_TOKENS = 22`, and only fires when a
strict-majority partition exists. The F5d sibling-family discriminator
(three sub-gates) closes the residual within-scope false positives
identified in P-7.

Spec:
[`docs/spec/clone-drift-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/clone-drift-v0.md).
