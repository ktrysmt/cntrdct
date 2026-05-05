# cntrdct research workspace

Academic / empirical-study workspace for cntrdct. Independent of the
technical workspace (`crates/`) at the repo root: no cargo dependencies
flow in either direction. Research artefacts that earn promotion into
the technical product are re-implemented there as a deliberate act,
not auto-imported.

## Layout

- `corpus-fetch/` — crates.io Sparse Index client and license filter
  used to assemble empirical-study corpora.
- `cli-research/` — `cntrdct-research` binary exposing fetch /
  aggregate / overlap / clippy-harness subcommands. These were
  formerly subcommands of the technical `cntrdct` CLI; they were
  extracted here in the research-split refactor so that research WIP
  cannot perturb the technical CLI's `Cargo.lock` or published
  surface.
- `projects/` — Track A / B / C empirical-study scaffolds.

## Local development

```bash
cd research
cargo build --workspace
cargo test --workspace
```

## Boundary

This workspace MUST NOT depend on `../crates/*`. Cross-workspace types
are duplicated rather than shared. Promotion of a research artefact
into the technical workspace is a manual act tracked under the
`promote(...)` commit-message prefix.
