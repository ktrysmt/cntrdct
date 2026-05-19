# FAQ

## Does cntrdct send my code anywhere by default?

No. The default `scan` runs entirely offline. Only two opt-in flows
talk to the network: `scan --adjudicate` (Anthropic Messages API,
gated behind `ANTHROPIC_API_KEY`) and `cross-model-kappa` (shells out
to `claude --print` and `gemini -p`, which handle their own auth).
The `network-isolation` CI job enforces the property structurally on
every push and pull request. See
[Network access policy](./concepts/network.md).

## Why does every detector cite a paper?

cntrdct's value proposition is evidence-grounded findings: a reviewer
can follow each finding back to the peer-reviewed work that justifies
the detection rule. The constraint is enforced at registration time
and in CI (`tests/citations_consistency.rs`). See
[Citation policy (P1)](./concepts/citations.md).

## Can I run cntrdct on Python-only repositories?

Yes. Five of the six Layer 1 detectors support Python
(`unreachable-after-terminator`, `clone-drift`, `arg-swap`,
`comment-code`, `pr-miner`); `config-interaction` is Rust-only because
Python lacks a semantic analogue to `#[cfg(...)]`. See
[Configuration](./configuration/cntrdct-toml.md) for
`[languages.python]` enablement.

## Why are some Python citations marked Unconfirmed?

The citation-policy spec (`docs/spec/citations-policy.md`) requires at
least one citation grounded in empirical work on the target language.
Three detectors (`comment-code`, `pr-miner`,
`unreachable-after-terminator`) cite Rust empirical work under a
grandfather clause until a confirmed Python citation is identified.
See the per-detector pages for status.

## How do I report a false positive?

File an issue using the
[bug report template](https://github.com/ktrysmt/cntrdct/issues/new?template=bug_report.md).
Include the source snippet, the SARIF result block (or JSON finding),
and the cntrdct version. If the false positive reflects a corpus gap,
contributing a labelled negative under `benchmarks/` is the most
direct fix path.

## Is there a paid / hosted version?

No. cntrdct is MIT-licensed and runs locally.
