# SourcererCC baseline — upstream pinning

Comparator for cntrdct's `clone-drift` detector. Spec:
[`docs/spec/sota-baselines-v0.md`](../../docs/spec/sota-baselines-v0.md).

## Upstream

- Project: SourcererCC, "Scaling Code Clone Detection to Big-Code"
- Authors: H. Sajnani, V. Saini, J. Svajlenko, C.K. Roy, C.V. Lopes
- Venue: ICSE 2016 (citation key `sajnani-icse-2016`)
- Repository: <https://github.com/Mondego/SourcererCC>
- License: GPL-2.0 (upstream); the wrapper Dockerfile and
  `entrypoint.sh` in this directory are MIT (matching cntrdct);
  the resulting Docker image is GPL-2.0 by virtue of bundling the
  upstream tool, and is distributed as a separate artefact from the
  cntrdct binary so the cntrdct binary itself stays MIT.

## Pinned commit

Pending Phase D — the precise upstream commit SHA, build flags, and
the resulting image digest are committed here when the v0.4.0 release
runs the live image for the first time. Until then this file
documents the contract:

- Upstream commit: `TBD` (the maintainer pins this before tagging
  v0.4.0).
- Build command: `docker build -t ghcr.io/ktrysmt/cntrdct-baselines/sourcerercc:v1.0 .`
- Push command: `docker push ghcr.io/ktrysmt/cntrdct-baselines/sourcerercc:v1.0`
- Verify digest: `docker inspect --format '{{index .RepoDigests 0}}'
  ghcr.io/ktrysmt/cntrdct-baselines/sourcerercc:v1.0` — the resulting
  `sha256:...` is copied into `src/baselines.rs::REGISTRY` and
  into the comparison report.

## Reproducibility checklist (pre-release)

1. Confirm the upstream repo commit in this file matches what is
   built into the image.
2. Run the image against `benchmarks/audit-corpus/files/` and the
   wild corpora; commit the resulting JSONL under
   `benchmarks/baselines/v<release>/sourcerercc.jsonl`.
3. Re-run `cntrdct eval --baseline sourcerercc --baselines-skip-run`
   to regenerate the comparison report from the committed JSONL.
4. Hand-update the README's "Latest baseline comparison" section
   from the regenerated report.

## Why a wrapper image instead of pulling upstream directly

SourcererCC ships as a multi-step pipeline (tokeniser → indexer →
clone-pair detector), not as a single CLI that takes a corpus
directory and emits findings. The wrapper image bundles the three
steps and the normalising shell entrypoint
(`baselines/sourcerercc/entrypoint.sh`) that maps SourcererCC's
clone-pair output into cntrdct's `NormalisedFinding` JSONL schema
(`docs/spec/sota-baselines-v0.md` §"Adapter contract"). Without the
wrapper the comparison harness would have to know about
SourcererCC's intermediate file layout, which would couple cntrdct
to the upstream's release cadence in a way the pinned image
explicitly avoids.

## Network posture

The image is built with all of its tooling and indices baked in,
and is run with `--network=none --read-only` by
`cntrdct::baselines::run_baseline_docker`. P3 is preserved by the
host side (cntrdct opens no sockets) and the comparator-side
network isolation is enforced by Docker. The
`network-isolation` CI job
(`.github/workflows/ci.yml`) does not exercise the live image —
CI runs against canned JSONL fixtures under
`tests/fixtures/baselines/` — but the spec's adapter contract
requires the offline posture for any baseline.
