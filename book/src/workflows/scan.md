# scan

`cntrdct scan <PATH>` runs the four-layer pipeline against a directory
or file. Default output is JSON to stdout; `--format sarif` switches
to SARIF 2.1.0.

```sh
cntrdct scan ./src
cntrdct scan ./src --format sarif > findings.sarif
cargo cntrdct scan ./src                     # cargo subcommand shim
```

## Flags

| Flag | Effect |
|---|---|
| `--format json\|sarif` | Output format (default: JSON). |
| `--config <PATH>` | Override the `cntrdct.toml` config file. When omitted, the scan root is searched for `cntrdct.toml`. |
| `--priors <PATH>` | Override the Layer 2 priors file (bypasses the default per-user cache lookup). |
| `--no-calibration` | Force the uncalibrated ranker even if priors are present. |
| `--adjudicate` | Route the top-N findings through the Layer 3 LLM adjudicator (requires `ANTHROPIC_API_KEY`). |
| `--adjudicate-top <N>` | Number of top-ranked findings to adjudicate when `--adjudicate` is set (default: 5). |

`scan` itself never opens a socket. Only the optional `--adjudicate`
flag does — see [Network access policy](../concepts/network.md).
