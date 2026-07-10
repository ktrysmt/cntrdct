# cntrdct container image.
#
# Build:  docker build -t cntrdct .
# Run:    docker run --rm -v "$PWD:/work" cntrdct scan .
#
# The image carries only the `cntrdct` binary (not `cargo-cntrdct` or
# the LSP server): the cargo-subcommand shim is pointless without
# cargo, and editor integrations run the binary on the host. `scan`
# is fully offline (P3), so the runtime stage needs no network tooling;
# ca-certificates is included only for the opt-in
# `--adjudicate-via=anthropic` HTTPS path.

FROM rust:1-slim-bookworm AS build
WORKDIR /build
COPY . .
# --locked: build exactly what Cargo.lock pins, fail on drift.
RUN cargo build --release --locked --bin cntrdct

FROM debian:bookworm-slim
LABEL org.opencontainers.image.source="https://github.com/ktrysmt/cntrdct"
LABEL org.opencontainers.image.description="Evidence-based contradiction linter for Rust, Python, TypeScript, and Go."
LABEL org.opencontainers.image.licenses="MIT"
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/cntrdct /usr/local/bin/cntrdct
# Mount the code to scan at /work (see the `docker run` line above).
WORKDIR /work
ENTRYPOINT ["cntrdct"]
CMD ["--help"]
