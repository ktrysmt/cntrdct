# SourcererCC wrapper for cntrdct's Q-15 baseline harness.
#
# Spec: docs/spec/sota-baselines-v0.md §"Adapter contract".
# Prereg: prereg/2026-05-19-osf-prereg.md §H6.1.
#
# The image bakes the upstream SourcererCC tool plus its Java + Python
# dependencies, then ships an entrypoint that walks /corpus, runs the
# SourcererCC pipeline, and emits one NormalisedFinding JSONL row per
# detected clone-pair to /out/findings.jsonl.
#
# Run with `--network=none --read-only` per spec F4. Build with
# `docker build -t ghcr.io/ktrysmt/cntrdct-baselines/sourcerercc:v1.0 .`
# from this directory; push and pin the resulting digest into
# baselines/sourcerercc/UPSTREAM.md and src/baselines.rs::REGISTRY
# before tagging the cntrdct release.

FROM eclipse-temurin:17-jdk-jammy

# Python is required for the upstream tokeniser's helper scripts and
# for the JSONL post-processing in entrypoint.sh.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        git \
        python3 \
        python3-pip \
        jq \
    && rm -rf /var/lib/apt/lists/*

# Upstream commit pin. Replace `TBD` with the exact upstream SHA when
# the v0.4.0 release is being prepared; the wrapper image must be
# rebuilt and re-pushed whenever this changes, with the new digest
# propagated to UPSTREAM.md.
ARG SOURCERERCC_COMMIT=TBD

WORKDIR /opt
RUN git clone https://github.com/Mondego/SourcererCC.git sourcerercc \
    && cd sourcerercc \
    && git checkout "${SOURCERERCC_COMMIT}" \
    && git rev-parse HEAD > /opt/sourcerercc_commit.txt

# Pre-build whatever upstream artefacts SourcererCC needs (e.g. the
# tokeniser jar). Real commands are filled in once the upstream
# commit is pinned; the placeholder keeps the image build skeleton
# auditable. The build is a no-op until SOURCERERCC_COMMIT is real.
RUN test "${SOURCERERCC_COMMIT}" = "TBD" || ( \
    cd /opt/sourcerercc \
    && echo "TODO: invoke upstream build per UPSTREAM.md once commit is pinned" \
)

# Entrypoint owns the corpus walk, the SourcererCC invocation, and the
# JSONL normalisation. Mounted /corpus is readonly; /out is the only
# writable surface.
COPY entrypoint.sh /usr/local/bin/cntrdct-sourcerercc
RUN chmod +x /usr/local/bin/cntrdct-sourcerercc

ENTRYPOINT ["/usr/local/bin/cntrdct-sourcerercc"]
