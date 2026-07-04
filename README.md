# cntrdct

[![CI](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml/badge.svg)](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cntrdct.svg)](https://crates.io/crates/cntrdct)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Evidence-based linter for logical contradictions and technical
inconsistencies in Rust, Python, TypeScript, and Go code. Every finding
cites the peer-reviewed paper that justifies the detection. Alpha; runs
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

## Usage

```sh
cntrdct scan ./src                    # JSON to stdout (default)
cntrdct scan ./src --format sarif     # SARIF 2.1.0 for code-scanning tools
cargo cntrdct scan ./src              # via cargo subcommand

# Optional Layer 3 LLM adjudication on the top-N findings. Off by
# default; see "Network access" below for the backends.
cntrdct scan ./src --adjudicate
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
| `python-unreachable-except` | a Python `except` handler shadowed by an earlier handler for a superclass exception |
| `build-tag-interaction-go` | a Go file carries two `//go:build` constraints whose predicates are structurally negations of each other |

See [`CITATIONS.md`](CITATIONS.md) for the bibliography behind each
detector.

## Language support

Detectors are selected per file by extension: `.rs` (Rust), `.py` /
`.pyi` (Python), `.ts` / `.mts` / `.cts` (TypeScript), `.tsx` (TypeScript
+ JSX, parsed with the TSX grammar), `.go` (Go). There is no
`--language` flag — a scan root may mix languages and each file is
routed to its own parser; unrecognised extensions are skipped.

| detector | Rust | Python | TypeScript | Go |
|---|:---:|:---:|:---:|:---:|
| `clone-drift` | yes | yes | yes | yes |
| `arg-swap` | yes | yes | yes | yes |
| `comment-code` | yes | yes | yes | yes |
| `unreachable-after-terminator` | yes | yes | yes | yes |
| `pr-miner` | yes | yes | yes | yes |
| `config-interaction` | yes | — | — | — |
| `python-unreachable-except` | — | yes | — | — |
| `build-tag-interaction-go` | — | — | — | yes |

Rust is the primary, fully-grounded target. For Python, TypeScript, and
Go the cross-cutting detectors carry `languageCitationStatus:
unconfirmed` in their SARIF output: the detection concept transfers but
no language-specific peer-reviewed citation has been confirmed yet (see
[`CITATIONS.md`](CITATIONS.md) and `docs/surveys/`). `.tsx` shares
TypeScript's detectors, citation grounding, and corpus.

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

TypeScript, `.tsx`, and Go use a `//` line comment (block `/* ... */`
form also works), with the same trailing / standalone semantics:

```typescript
doSomething(b, a); // cntrdct: allow(arg-swap)
```

An empty argument list (`cntrdct::allow()` / `cntrdct: allow()`) is the
catch-all that suppresses every detector for that item.

## Network access

`scan`, `calibrate`, and `eval` never open a socket — cntrdct runs
entirely offline by default. The only network access is opt-in Layer 3
LLM adjudication (`scan --adjudicate`), off unless you ask for it. The
default backend shells out to the Claude CLI on your existing
subscription (no API key); `--adjudicate-via=anthropic` uses the
Anthropic API with `ANTHROPIC_API_KEY` instead.

## Claude Code skill

With the binary on `PATH`, Claude Code users can invoke `/cntrdct` to
run a scan and have the top findings summarised in chat
(`.claude/skills/cntrdct/`).

## Editor / LSP integration

A companion `cntrdct-lsp` binary speaks the Language Server Protocol
and publishes diagnostics on `didOpen` / `didChange` / `didSave`. The
binary ships in the GitHub Releases archive alongside `cntrdct` and is
built behind an optional Cargo feature for source installs:

```sh
cargo install cntrdct --features lsp
```

A VS Code extension bundling the LSP lives in the separate
[`ktrysmt/vscode-cntrdct`](https://github.com/ktrysmt/vscode-cntrdct)
repo. Spec: [`docs/spec/lsp-v0.md`](docs/spec/lsp-v0.md).

## License

MIT. See [LICENSE](LICENSE).
