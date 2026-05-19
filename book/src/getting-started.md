# Getting started

## Install

```sh
# crates.io
cargo install cntrdct

# macOS / Linux via Homebrew
brew tap ktrysmt/cntrdct
brew install cntrdct

# pre-built archive (no compile); requires cargo-binstall
cargo binstall cntrdct

# install script (Linux x86_64/aarch64, macOS aarch64, Windows x86_64)
curl -fsSL https://raw.githubusercontent.com/ktrysmt/cntrdct/master/scripts/install.sh | bash
```

The `cargo install` path additionally ships a `cargo-cntrdct` shim so
`cargo cntrdct scan` works alongside `cntrdct scan`.

## First scan

```sh
cntrdct scan ./src                    # JSON to stdout (default)
cntrdct scan ./src --format sarif     # SARIF 2.1.0 for code-scanning tools
```

The default scan is fully offline: walker → tree-sitter parsers →
Layer 1 detectors → Layer 2 ranker → Layer 4 SARIF emitter. No network
access, no telemetry. See
[Network access policy](./concepts/network.md) for the design property
and the CI gate that enforces it.

## Optional: LLM adjudication

`scan --adjudicate` routes the top-N findings through the Layer 3
Anthropic Messages API for a second-opinion verdict. Gated behind the
`ANTHROPIC_API_KEY` environment variable; off by default.

```sh
ANTHROPIC_API_KEY=... cntrdct scan ./src --adjudicate
```

See [scan](./workflows/scan.md) for the full flag reference.

## Examples

Three runnable end-to-end examples live under
[`examples/`](https://github.com/ktrysmt/cntrdct/tree/master/examples):
a plain scan, a calibration run against a labelled corpus, and an
adjudication run against a mock API.
