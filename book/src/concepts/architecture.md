# Four-layer architecture

cntrdct is organised into four layers with a deliberate separation
between deterministic and stochastic surfaces.

| Layer | Role | LLM? | Network? |
|---|---|---|---|
| Layer 1 | Tree-sitter-based detectors | No | No |
| Layer 2 | Statistical ranker (Wilson / Jeffreys × log-sibling-count) | No | No |
| Layer 3 | Optional LLM adjudicator | Yes | Yes (`scan --adjudicate`) |
| Layer 4 | SARIF 2.1.0 emitter | No | No |

The Layer 3 boundary is load-bearing: it is the only layer permitted
to open a socket, and only when `--adjudicate` is passed. The default
`scan` pipeline (Layers 1 → 2 → 4) is fully offline. The CI gate at
`.github/workflows/ci.yml` (`network-isolation` job) runs `cntrdct
scan` inside a Linux network namespace to enforce this structurally.

See also:

- [Network access policy (P3)](./network.md)
- [Statistical priors (P4)](./priors.md)
- [Severity and anomaly classes (P5)](./severity.md)
