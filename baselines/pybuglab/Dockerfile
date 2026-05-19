# PyBugLab wrapper for cntrdct's Q-15 baseline harness.
#
# Spec: docs/spec/sota-baselines-v0.md §"Adapter contract".
#
# Comparator for cntrdct's `arg-swap` detector. PyBugLab
# (Allamanis, Jackson-Flux, Brockschmidt, NeurIPS 2021) is a learned
# Python bug detector; this wrapper image bakes the upstream repo,
# its pre-trained weights, and the inference dependencies, then ships
# an entrypoint that walks /corpus for .py files, runs PyBugLab in
# inference mode, and emits one NormalisedFinding JSONL row per
# detected arg-swap site to /out/findings.jsonl.
#
# Run with `--network=none --read-only` per spec F4. Build with
# `docker build -t ghcr.io/ktrysmt/cntrdct-baselines/pybuglab:v1.0 .`
# from this directory; push and pin the resulting digest into
# baselines/pybuglab/UPSTREAM.md and src/baselines.rs::REGISTRY
# before tagging the cntrdct release.
#
# Determinism (spec F6): PyBugLab is a learned model, so the wrapper
# pins the inference seed at build time. Re-running the image with
# the same seed against the same corpus produces byte-identical JSONL.

FROM python:3.11-slim-bookworm

# Inference-time dependencies. Versions are pinned via the upstream
# repo's requirements file at build time (see the `pip install` step
# below); listing the runtime packages here keeps the layer cache
# stable when only the seed / commit pins change.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        git \
        jq \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Upstream commit pin. Replace `TBD` with the exact upstream SHA when
# the live image is being prepared; the wrapper image must be rebuilt
# and re-pushed whenever this changes, with the new digest propagated
# to UPSTREAM.md and src/baselines.rs::REGISTRY.
ARG PYBUGLAB_COMMIT=TBD

# Inference seed pinned at build time per spec F6. Burning the seed
# into the image (rather than reading it from the runtime mount) keeps
# the determinism contract local to the image identity: a given image
# digest implies a single seed value.
ARG PYBUGLAB_SEED=0
ENV PYBUGLAB_SEED=${PYBUGLAB_SEED}

WORKDIR /opt
RUN git clone https://github.com/microsoft/neurips21-self-supervised-bug-detection-and-repair.git pybuglab \
    && cd pybuglab \
    && git checkout "${PYBUGLAB_COMMIT}" \
    && git rev-parse HEAD > /opt/pybuglab_commit.txt

# Install Python dependencies and download the pre-trained weights.
# The placeholder keeps the image build skeleton auditable; the real
# commands are filled in once the upstream commit is pinned and the
# weights URL is committed under baselines/pybuglab/UPSTREAM.md. The
# build is a no-op until PYBUGLAB_COMMIT is real.
RUN test "${PYBUGLAB_COMMIT}" = "TBD" || ( \
    cd /opt/pybuglab \
    && pip install --no-cache-dir -r requirements.txt \
    && echo "TODO: download pre-trained weights per UPSTREAM.md once commit is pinned" \
)

# Entrypoint owns the corpus walk, the PyBugLab invocation, and the
# JSONL normalisation. Mounted /corpus is readonly; /out is the only
# writable surface.
COPY entrypoint.sh /usr/local/bin/cntrdct-pybuglab
RUN chmod +x /usr/local/bin/cntrdct-pybuglab

ENTRYPOINT ["/usr/local/bin/cntrdct-pybuglab"]
