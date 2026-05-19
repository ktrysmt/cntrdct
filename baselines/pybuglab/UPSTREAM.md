# PyBugLab baseline — upstream pinning

Comparator for cntrdct's `arg-swap` detector. Spec:
[`docs/spec/sota-baselines-v0.md`](../../docs/spec/sota-baselines-v0.md).

## Upstream

- Project: PyBugLab, "Self-Supervised Bug Detection and Repair"
- Authors: M. Allamanis, H. Jackson-Flux, M. Brockschmidt
- Venue: NeurIPS 2021 (citation key `allamanis-neurips-2021`)
- Repository: <https://github.com/microsoft/neurips21-self-supervised-bug-detection-and-repair>
- License: MIT (upstream); the wrapper Dockerfile and
  `entrypoint.sh` in this directory are MIT (matching cntrdct);
  the resulting Docker image is MIT by virtue of bundling the
  upstream tool, and is distributed as a separate artefact from the
  cntrdct binary.

## Pinned commit + weights + seed

Pending live wiring — the precise upstream commit SHA, pre-trained
weights URL + SHA-256, inference seed, and the resulting image digest
are committed here when the live image is built for the first time.
Until then this file documents the contract:

- Upstream commit: `TBD` (the maintainer pins this before tagging
  the next release that ships live PyBugLab numbers).
- Pre-trained weights: `TBD` (URL + SHA-256 committed at the same
  time as the commit pin; the weights are downloaded at image build
  time and frozen into the image so runtime is offline).
- Inference seed (`PYBUGLAB_SEED` build-arg): `0` by default. A given
  image digest implies a single seed value per spec F6; bumping the
  seed requires rebuilding and re-pushing the image with a new tag.
- Build command:
  `docker build \
    --build-arg PYBUGLAB_COMMIT=<sha> \
    --build-arg PYBUGLAB_SEED=0 \
    -t ghcr.io/ktrysmt/cntrdct-baselines/pybuglab:v1.0 .`
- Push command:
  `docker push ghcr.io/ktrysmt/cntrdct-baselines/pybuglab:v1.0`
- Verify digest: `docker inspect --format '{{index .RepoDigests 0}}'
  ghcr.io/ktrysmt/cntrdct-baselines/pybuglab:v1.0` — the resulting
  `sha256:...` is copied into `src/baselines.rs::REGISTRY` and into
  the comparison report.

## Reproducibility checklist (pre-release)

1. Confirm the upstream repo commit and the pre-trained weights
   SHA-256 in this file match what is built into the image.
2. Confirm `PYBUGLAB_SEED` matches the value pinned in this file.
3. Run the image against `benchmarks/wild-corpus-python/` (the
   audit-corpus's Python slice is empty for arg-swap by construction
   — see `docs/spec/recall-audit-v0.md` for the v0 scope); commit
   the resulting JSONL under
   `benchmarks/baselines/v<release>/pybuglab.jsonl`.
4. Re-run `cntrdct eval --baseline pybuglab --baselines-skip-run`
   to regenerate the comparison report from the committed JSONL.
5. Hand-update the README's "Baseline comparison" section from the
   regenerated report.

## Why a wrapper image instead of pulling upstream directly

PyBugLab ships as a research codebase with separate scripts for
training, evaluation, and inference, and assumes a CUDA-enabled
environment by default. The wrapper image pins the inference path
on CPU (so a maintainer's workstation without a GPU can reproduce the
comparison), bakes the pre-trained weights, and ships a normalising
shell entrypoint (`baselines/pybuglab/entrypoint.sh`) that maps
PyBugLab's per-buggy-site output into cntrdct's `NormalisedFinding`
JSONL schema (`docs/spec/sota-baselines-v0.md` §"Adapter contract").
Without the wrapper the comparison harness would have to know about
PyBugLab's intermediate file layout, which would couple cntrdct to
the upstream's release cadence in a way the pinned image explicitly
avoids.

The `arg-swap` mapping is direct: PyBugLab's "wrong argument" /
"argument swap" bug class corresponds 1:1 to cntrdct's `arg-swap`
detector. Bug classes PyBugLab predicts that cntrdct does not yet
ship (e.g. `wrong-operator`, `wrong-comparison`) are filtered out by
the entrypoint before normalisation; they would surface as
`DetectorIdMismatch` adapter errors on the harness side otherwise.

## Network posture

The image is built with all of its tooling, dependencies, and
pre-trained weights baked in, and is run with `--network=none
--read-only` by `cntrdct::baselines::run_baseline_docker`. P3 is
preserved by the host side (cntrdct opens no sockets) and the
comparator-side network isolation is enforced by Docker. The
`network-isolation` CI job (`.github/workflows/ci.yml`) does not
exercise the live image — CI runs against canned JSONL fixtures
under `tests/fixtures/baselines/` — but the spec's adapter contract
requires the offline posture for any baseline.
