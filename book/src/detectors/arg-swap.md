# arg-swap

Flags a two-argument call site whose argument identifier names match
the function's parameter names in *reversed* order. The contradiction
is between the call's positional layout and the callee's parameter
contract.

| Property | Value |
|---|---|
| Detector ID | `arg-swap` |
| Languages | Rust, Python |
| IEEE 1044-2009 class | Interface |
| Default severity | Warning |
| Spec | [`docs/spec/arg-swap-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/arg-swap-v0.md) |

## What it flags

```rust
fn copy(dst: &mut [u8], src: &[u8]) { /* ... */ }

fn main() {
    let mut dst = vec![0u8; 16];
    let src = b"hello";
    copy(src, &mut dst);   // <- flagged: argument order reversed
}
```

The detector resolves the call to the same-file definition of `copy`,
notices that the call's first identifier `src` matches the second
parameter name and vice versa, and emits a finding pointing at the
call site with the definition in `related`. The `evidence.raw` payload
carries `callee_name`, `parameter_names`, and `argument_names`.

## What it does not flag

- Calls whose arguments are not bare identifiers (literals,
  expressions, field accesses). `copy(some.dst, src)` is out of scope
  because the AST node is not a simple identifier.
- Single-argument or N ≥ 3 calls. Only binary functions participate.
- Method calls (`obj.copy(a, b)`) and qualified-path calls
  (`mod::copy(a, b)`).
- Cross-file resolution. The call and the definition must live in the
  same source file.
- Calls where the call-site identifier names are the same as the
  parameter names but in the right order, or where the names share no
  overlap at all.

The narrow scope is deliberate: arg-swap trades recall for precision
so the signal is actionable without an LLM in the loop.

## Language coverage

- Rust: confirmed citation. Pradel & Sen "DeepBugs" (FSE 2018) and
  Rice et al. "Detecting Argument Selection Defects" (ICSE 2017)
  ground the AST-level shape used here.
- Python: confirmed citation. Allamanis, Jackson-Flux, Brockschmidt
  (NeurIPS 2021, PyBugLab / PyPIBugs) covers the same swap pattern on
  PyPI corpora. PyBugLab also ships as the Q-15 SOTA baseline
  comparator for arg-swap (`baselines/pybuglab/`).

## Configuration

Suppress in-source:

- Rust: `#[cntrdct::allow(arg-swap)]` on the call site's enclosing
  function or item.
- Python: `# cntrdct: allow(arg-swap)` as a trailing or preceding
  comment (Q-9).

Suppress project-wide via `cntrdct.toml`:

```toml
[detectors.arg-swap]
enabled = false

[languages.python]
suppress = ["arg-swap"]
```

See [In-source suppressions](../configuration/suppressions.md) and
[cntrdct.toml reference](../configuration/cntrdct-toml.md).

## Related

- [Citation policy (P1)](../concepts/citations.md) — why every
  language addition needs its own grounding.
- [Statistical priors (P4)](../concepts/priors.md) — how the Layer 2
  ranker re-orders arg-swap findings by Wilson / Jeffreys lower bound
  against the calibrated corpus.
- [pr-miner](./pr-miner.md) — complementary "implicit pairing" rule
  miner; both detectors target the same family of API-contract
  violations from different angles.
