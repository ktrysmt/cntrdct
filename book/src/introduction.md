# Introduction

cntrdct is an evidence-based linter for logical contradictions and
technical inconsistencies in Rust and Python code. Every finding cites
the peer-reviewed paper that justifies the detection — no detector ships
without one.

This guide covers the design constraints, the six Layer 1 detectors,
how to configure and suppress findings, the supported workflows
(`scan`, `calibrate`, `eval`, `cross-model-kappa`), and the
integrations (GitHub Action, LSP, SARIF, Claude Code skill).

For installation and a one-line quickstart, jump to
[Getting started](./getting-started.md).

## Status

Alpha. The detector set, ranker priors, and SARIF mapping are stable
under SemVer; the LSP server and VS Code extension are still pre-1.0.

## Where to find things

- Source code, issues, and releases:
  [github.com/ktrysmt/cntrdct](https://github.com/ktrysmt/cntrdct).
- Bibliography behind each detector:
  [CITATIONS.md](https://github.com/ktrysmt/cntrdct/blob/master/CITATIONS.md).
- Engineering roadmap:
  [ROADMAP.md](https://github.com/ktrysmt/cntrdct/blob/master/ROADMAP.md).
- Per-feature specifications:
  [`docs/spec/`](https://github.com/ktrysmt/cntrdct/tree/master/docs/spec).
