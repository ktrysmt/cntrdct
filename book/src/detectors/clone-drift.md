# clone-drift

Flags a near-duplicate function whose AST has diverged from the
strict majority of its siblings in the same scope. The contradiction
is between "looks like a copy" and "doesn't actually do what its
copies do".

| Property | Value |
|---|---|
| Detector ID | `clone-drift` |
| Languages | Rust, Python |
| IEEE 1044-2009 class | Logic |
| Default severity | Warning |
| Spec | [`docs/spec/clone-drift-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/clone-drift-v0.md) |

## What it flags

```rust
fn poll_a(cx: &mut Context<'_>) -> Poll<()> {
    if let Some(x) = inner_a(cx) { return Poll::Ready(x); }
    Poll::Pending
}
fn poll_b(cx: &mut Context<'_>) -> Poll<()> {
    if let Some(x) = inner_b(cx) { return Poll::Ready(x); }
    Poll::Pending
}
fn poll_c(cx: &mut Context<'_>) -> Poll<()> {
    if let Some(x) = inner_c(cx) { return Poll::Ready(x); }
    Poll::Pending
}
fn poll_d(cx: &mut Context<'_>) -> Poll<()> {
    // <- flagged: drifted from the other three
    if inner_d(cx).is_some() { break; }
    Poll::Pending
}
```

`poll_a` / `poll_b` / `poll_c` normalise to identical AST node
sequences and form a dominant partition of size 3. `poll_d` belongs to
the same Jaccard ≥ 0.5 cluster but has a singleton normalised form
that differs by a small surgical edit (`return Poll::Ready(x)` →
`break`). The finding points at `poll_d`; `related` lists the three
canonical-form siblings.

## How the gates compose

The detector applies four ordered gates so the textbook "one of N
copies missed an update" pattern fires while designed N-variant
families and library-shape clusters do not:

| Gate | Purpose | Default |
|---|---|---|
| `MIN_FN_TOKENS = 22` | drop trivially short functions | constant |
| `SIMILARITY_THRESHOLD = 0.5` | pairwise Jaccard for cluster membership | constant |
| F5b scope-bounded clustering | clusters never cross crate / package boundaries | provenance-aware |
| F5c-i strict-majority | dominant partition must be > 50% of the cluster | constant |
| F5c-ii `NEAR_DUPLICATE_THRESHOLD = 0.7` | drifted singleton must be near-duplicate of the dominant exemplar | constant |
| F5d-i multi-singleton suppression | ≥ 2 singleton partitions ⇒ designed family ⇒ silent | constant |
| F5d-ii length-imbalance × weak-dominant gate | high length asymmetry under weak canonical evidence ⇒ silent | constants |
| F5d-iii small-cluster floor | `MIN_GROUP_SIZE` cluster at the resolution limit ⇒ silent | constants |

Gates F5b / F5c / F5d landed across P-6 and P-7 in response to the
wild β corpus residuals. Wild β clone-drift FP count is 0 in both
Rust and Python after F5d.

## Language coverage

- Rust: grandfathered citation (Juergens et al. ICSE 2009 on inconsistent
  clone changes); the broader Bettenburg MSR 2009 and Krinke ICSM 2007
  citations supply the F5c-ii drift gate's grounding.
- Python: confirmed citation. Assi, Hassan, Zou (TOSEM 2025, DOI
  10.1145/3721125) replicate NiCad / SourcererCC on nine Python deep-
  learning frameworks. SourcererCC also ships as the Q-15 SOTA
  baseline comparator for clone-drift (`baselines/sourcerercc/`).

## What it does not flag

- Clusters smaller than `MIN_GROUP_SIZE = 3`.
- Functions whose normalised AST has fewer than `MIN_FN_TOKENS = 22`
  tokens (small utility functions whose drift signal is too noisy).
- Functions inside `impl` / `trait` / `mod` blocks. Only top-level
  `fn` (Rust) and `function_definition` (Python) participate in v0.
- Cross-scope siblings. The F5b scope rule keeps clustering within a
  single crate / package / parent-directory namespace.

## Configuration

```toml
[detectors.clone-drift]
enabled = false
# Severity overrides apply at SARIF emission time (P5).
severity = "Note"
```

Per-language disable:

```toml
[languages.python]
suppress = ["clone-drift"]
```

See [In-source suppressions](../configuration/suppressions.md) for
the per-function attribute and comment forms.

## Related

- [arg-swap](./arg-swap.md) — different shape of same-file contradiction
  (parameter contract vs structural-clone consistency).
- [Statistical priors (P4)](../concepts/priors.md) — embedded
  clone-drift Wilson lower bound is 0.676 (8 TP / 0 FP) after P-7.
- [Severity and anomaly classes (P5)](../concepts/severity.md) —
  Logic anomaly class mapping.
