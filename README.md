# cntrdct

[![CI](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml/badge.svg)](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cntrdct.svg)](https://crates.io/crates/cntrdct)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Evidence-based linter for logical contradictions and technical
inconsistencies in Rust and Python code. Every finding cites the
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

## Self-replication ledger

cntrdct tracks its own precision / recall / F1 across releases instead
of comparing against external state-of-the-art tools. The
head-to-head-against-PyBugLab / SourcererCC framing was retired: the
pre-trained weights and comparison infrastructure those projects
depend on are not distributed in an installable form, so a reproducible
external comparison was unrealisable.

Each release commits an eval snapshot under
`benchmarks/self-replication/v<release>/cntrdct.jsonl` — one
`cntrdct eval` JSON object per tracked corpus, one per line (JSONL):

```sh
mkdir -p benchmarks/self-replication/v<release>
for c in audit-corpus wild-corpus wild-corpus-python; do
    cntrdct eval "benchmarks/$c" | jq -c .
done > benchmarks/self-replication/v<release>/cntrdct.jsonl
```

(`jq -c` compacts each report to a single line; any JSON minifier works.)
Each line carries a `corpus` field so the lines self-identify across
releases. The wild corpora are unlabelled, so their precision / recall
land at zero — their useful signal is `actual_total` drift (did a change
make more or fewer findings fire?).

At release time, a reviewer reads the per-detector F1 / precision /
recall delta against the previous tag's snapshot with `--against`:

```sh
cntrdct eval benchmarks/audit-corpus \
    --against benchmarks/self-replication/v<prev>/cntrdct.jsonl
```

This prints the delta of the current run against the matching `corpus`
line in the previous snapshot (a baseline, with no delta, when no line
matches). The ledger is refreshed manually per release and carries no
CI gate.

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

The VS Code extension bundling the LSP is tracked in the
[todo](#todo) section. Spec:
[`docs/spec/lsp-v0.md`](docs/spec/lsp-v0.md).

## License

MIT. See [LICENSE](LICENSE).

## todo

Outstanding work remaining after the v0.6.0 rebuild:

- VS Code extension (R-6) — lives in the separate
  [`ktrysmt/vscode-cntrdct`](https://github.com/ktrysmt/vscode-cntrdct)
  repo. Phase 1 (LSP) and Phase 2 (extension scaffolding + a headless
  end-to-end test) have landed; remaining work is the Marketplace
  listing (`docs/spec/lsp-v0.md` step 7), an in-editor F5 end-to-end run
  against the real `cntrdct-lsp` binary (the headless test is a
  surrogate, not a replacement), an extension icon, and reconciling the
  extension's pinned default server version with the latest release.
- Layer 0 LLM candidate generator (R-4, Phase B) — a labelled Layer-0
  confidence corpus and a fitted Layer-0 prior remain deferred; v0 ships
  an empty prior with a no-op fallback.
