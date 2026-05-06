#!/usr/bin/env bash
# install.sh — download the latest cntrdct release matching the current
# platform and extract `cntrdct` (and `cargo-cntrdct`) into a PATH directory.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ktrysmt/cntrdct/main/scripts/install.sh | bash
#
# Optional env vars:
#   CNTRDCT_VERSION   pin to a specific tag (default: latest published release)
#   CNTRDCT_PREFIX    install destination directory (default: $HOME/.local/bin)
#   CNTRDCT_REPO      override the release source (default: ktrysmt/cntrdct)

set -euo pipefail

REPO="${CNTRDCT_REPO:-ktrysmt/cntrdct}"
PREFIX="${CNTRDCT_PREFIX:-$HOME/.local/bin}"
VERSION="${CNTRDCT_VERSION:-}"

err() { echo "install.sh: $*" >&2; exit 1; }

detect_target() {
    local os arch
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"
    case "${os}-${arch}" in
        linux-x86_64)        echo "x86_64-unknown-linux-gnu" ;;
        linux-aarch64|linux-arm64) echo "aarch64-unknown-linux-gnu" ;;
        darwin-x86_64)       echo "x86_64-apple-darwin" ;;
        darwin-arm64)        echo "aarch64-apple-darwin" ;;
        *)                   err "unsupported platform: ${os}-${arch}" ;;
    esac
}

resolve_latest() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep -m1 '"tag_name"' \
            | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep -m1 '"tag_name"' \
            | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        err "neither curl nor wget is installed"
    fi
}

download() {
    local url="$1" dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    else
        err "neither curl nor wget is installed"
    fi
}

verify_sha256() {
    local dir="$1" file="$2"
    if command -v shasum >/dev/null 2>&1; then
        ( cd "${dir}" && shasum -a 256 -c "${file}.sha256" )
    elif command -v sha256sum >/dev/null 2>&1; then
        ( cd "${dir}" && sha256sum -c "${file}.sha256" )
    else
        err "neither shasum nor sha256sum is installed"
    fi
}

main() {
    local target tag asset url tmpdir archive
    target="$(detect_target)"
    tag="${VERSION:-$(resolve_latest)}"
    if [[ -z "${tag}" ]]; then
        err "could not resolve latest release tag"
    fi
    case "${tag}" in
        v[0-9]*) ;;
        *) err "invalid tag format: ${tag}" ;;
    esac
    asset="cntrdct-${tag}-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${tag}/${asset}"

    tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/cntrdct-install.XXXXXX")"
    trap 'rm -rf "$tmpdir"' EXIT

    archive="${tmpdir}/${asset}"
    echo "downloading ${url}"
    download "${url}" "${archive}"
    echo "downloading ${url}.sha256"
    download "${url}.sha256" "${archive}.sha256"
    verify_sha256 "${tmpdir}" "${asset}"

    tar -C "${tmpdir}" --no-same-owner -xzf "${archive}"
    extracted="${tmpdir}/cntrdct-${tag}-${target}"

    mkdir -p "${PREFIX}"
    install -m 0755 "${extracted}/cntrdct" "${PREFIX}/cntrdct"
    install -m 0755 "${extracted}/cargo-cntrdct" "${PREFIX}/cargo-cntrdct"

    echo "installed cntrdct ${tag} to ${PREFIX}/cntrdct"
    case ":${PATH}:" in
        *":${PREFIX}:"*) ;;
        *) echo "warning: ${PREFIX} is not on PATH" ;;
    esac
}

main "$@"
