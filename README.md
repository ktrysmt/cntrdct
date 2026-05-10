# cntrdct

[![CI](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml/badge.svg)](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cntrdct.svg)](https://crates.io/crates/cntrdct)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Evidence-based linter for logical contradictions and technical
inconsistencies in Rust (and Python) code. Every finding cites the
peer-reviewed paper that justifies the detection. Alpha; runs
entirely offline by default.

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
curl -fsSL https://raw.githubusercontent.com/ktrysmt/cntrdct/main/scripts/install.sh | bash
```

Cargo skips pre-releases by default; pass
`--version X.Y.Z-suffix` to install an `-rc.N` / `-beta.N`.

## Usage

```sh
cntrdct scan ./src                    # JSON to stdout (default)
cntrdct scan ./src --format sarif     # SARIF 2.1.0 for code-scanning tools
cargo cntrdct scan ./src              # via cargo subcommand

# Optional Layer 3 LLM adjudication on the top-N findings — sends those
# findings to the Anthropic Messages API. Off by default.
ANTHROPIC_API_KEY=... cntrdct scan ./src --adjudicate
```

`cntrdct --help` lists `calibrate` (recalibrate ranker priors, or fit
LLM-confidence Platt parameters with `--fit-platt`) and `eval`
(precision / recall on a labelled corpus). Three runnable end-to-end
examples live under [`examples/`](examples/).

## Detectors

| id | what it flags |
|---|---|
| `clone-drift` | a near-duplicate function whose AST diverged from a majority of its siblings |
| `arg-swap` | a 2-arg call whose argument names are swapped relative to the same-file definition |
| `comment-code` | doc comment claims a behaviour the implementation does not exhibit (Result / panic / deprecated patterns) |
| `unreachable-after-terminator` | a statement following `return` / `panic!()` / `unreachable!()` / `todo!()` / `break` / `continue` within the same block |
| `config-interaction` | a top-level item bears two `#[cfg(...)]` attributes whose predicates are structurally negations of each other |
| `pr-miner` | a call site violating an implicit programming rule mined via frequent-itemset analysis |

See [`CITATIONS.md`](CITATIONS.md) for the bibliography behind each
detector.

## Configuration

A `cntrdct.toml` at the scan root tunes severity, thresholds, and
per-path allow / deny rules. In-source suppression is also supported:

```rust
#[cntrdct::allow(clone-drift)]
fn looks_like_a_drifted_clone_but_is_intentional() { /* ... */ }
```

```python
# cntrdct: allow(arg-swap)
do_something(b, a)
```

## Network access

`scan`, `calibrate`, and `eval` never open a socket. Two subcommands
talk to the network and both are opt-in:

- `scan --adjudicate` — Layer 3 LLM adjudicator, gated behind
  `ANTHROPIC_API_KEY`, hits the Anthropic Messages API directly.
- `cross-model-kappa` — Q-13 cross-model audit. cntrdct itself does
  not open sockets here; it shells out to `claude --print` and
  `gemini -p`, and those CLIs handle their own auth (OAuth via
  `claude auth` / `gemini auth`). No API keys are read by cntrdct.

## Cross-model audit

`cntrdct cross-model-kappa <corpus.jsonl>` routes the same finding
set through `claude --print` and `gemini -p`, then reports pairwise
Cohen's κ per `(detector_id, anomaly_class)` cell. Cells with κ < 0.6
(Landis & Koch substantial-agreement floor) are flagged as
low-reliability adjudication regions. Both CLIs must be installed
and logged in (`claude auth`, `gemini auth`); a missing CLI surfaces
as a `skipped` provider in the audit JSON. Output goes to stdout by
default, or to `--output PATH` when set. Spec:
[`docs/spec/cross-model-kappa-v0.md`](docs/spec/cross-model-kappa-v0.md).

## Claude Code skill

With the binary on `PATH`, Claude Code users can invoke `/cntrdct` to
run a scan and have the top findings summarised in chat
(`.claude/skills/cntrdct/`).

## Further reading

- [The Linter that Cites Its Sources](https://ktrysmt.github.io/cntrdct/essays/citation-as-api/)
  — position essay on what evidence-based linting means in practice.

## License

MIT. See [LICENSE](LICENSE).
