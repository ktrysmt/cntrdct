# arg-swap

Flags a 2-argument call site whose argument names are swapped relative
to the same-file definition.

- **Rust citation:** Pradel & Sen "DeepBugs" FSE 2018.
- **Python citation:** Allamanis, Jackson-Flux, Brockschmidt
  NeurIPS 2021 (PyBugLab / PyPIBugs). Confirmed.
- **IEEE 1044-2009 class:** Logic / Computational.
- **Default severity:** Warning.

The v0 scope is intentionally narrow: only 2-argument calls where both
the call site and the definition live in the same file. Lifting the
scope to cross-file calls or to N > 2 is tracked under Future Q-series
candidates in the roadmap.

Spec:
[`docs/spec/arg-swap-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/arg-swap-v0.md).
