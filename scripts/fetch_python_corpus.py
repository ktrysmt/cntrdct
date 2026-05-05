#!/usr/bin/env python3
"""
Fetch a curated set of pure-Python source files from PyPI and lay them
out under benchmarks/wild-corpus-python/ for the M-4 wild corpus.

Each entry in CORPUS pins (package, version, license_spdx, [files]).
For each entry the script:
  1. Downloads the package's sdist (tar.gz) from PyPI's JSON API.
  2. Verifies the SHA-256 of the tarball against the value PyPI itself
     reports for that release (catches redirected mirrors).
  3. Extracts the listed `files` paths into
     benchmarks/wild-corpus-python/files/<flat_name>.py.
  4. Prepends a 3-line provenance comment block (`# Source:`,
     `# License:`, `# Note:`) so the corpus is auditable.
  5. Emits / refreshes a manifest skeleton with `source`, `license`,
     `sha256` filled in. The maintainer fills in `expected: [...]`
     afterwards by running `cntrdct scan` and triaging findings.

Re-running the script is idempotent: the file content (header + body)
and recorded SHA-256 are deterministic for a pinned (package, version,
file_path) triple. CI can therefore call this script and `git diff`
to detect drift.

Usage:
  python3 scripts/fetch_python_corpus.py [--out benchmarks/wild-corpus-python]

Environment:
  Network access to https://pypi.org. No third-party Python deps; uses
  stdlib `urllib`, `tarfile`, `hashlib`, `json`.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import shutil
import sys
import tarfile
import tempfile
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import List, Tuple


@dataclass(frozen=True)
class CorpusEntry:
    """A single (package, version) bundle and the files we vendor from it."""

    package: str
    version: str
    license: str
    # (relative_path_inside_sdist, flat_name_for_corpus)
    files: Tuple[Tuple[str, str], ...]


# Allowlist. Pinned versions; permissive licenses only.
#
# Selection criteria:
# - License compatible with redistribution (MIT / BSD / Apache-2.0).
# - Pure Python (no compiled extensions).
# - Files chosen for diversity: control flow, exception handling, doc
#   comments — all the patterns the Python detectors target.
CORPUS: Tuple[CorpusEntry, ...] = (
    CorpusEntry(
        package="six",
        version="1.16.0",
        license="MIT",
        files=(
            ("six-1.16.0/six.py", "six_main.py"),
        ),
    ),
    CorpusEntry(
        package="attrs",
        version="22.2.0",
        license="MIT",
        files=(
            ("attrs-22.2.0/src/attr/_make.py", "attrs_make.py"),
            ("attrs-22.2.0/src/attr/converters.py", "attrs_converters.py"),
            ("attrs-22.2.0/src/attr/validators.py", "attrs_validators.py"),
        ),
    ),
    CorpusEntry(
        package="click",
        version="8.1.7",
        license="BSD-3-Clause",
        files=(
            ("click-8.1.7/src/click/decorators.py", "click_decorators.py"),
            ("click-8.1.7/src/click/exceptions.py", "click_exceptions.py"),
            ("click-8.1.7/src/click/utils.py", "click_utils.py"),
        ),
    ),
    CorpusEntry(
        package="idna",
        version="3.6",
        license="BSD-3-Clause",
        # uts46data.py is generated lookup-table data (not code) and
        # produces dozens of clone-drift false positives that swamp the
        # corpus signal. Excluded.
        files=(
            ("idna-3.6/idna/core.py", "idna_core.py"),
            ("idna-3.6/idna/intranges.py", "idna_intranges.py"),
        ),
    ),
    CorpusEntry(
        package="charset-normalizer",
        version="3.3.2",
        license="MIT",
        files=(
            ("charset-normalizer-3.3.2/charset_normalizer/api.py", "charset_normalizer_api.py"),
            ("charset-normalizer-3.3.2/charset_normalizer/utils.py", "charset_normalizer_utils.py"),
        ),
    ),
)

PYPI_JSON = "https://pypi.org/pypi/{package}/{version}/json"
USER_AGENT = "cntrdct-corpus-fetch/0.1 (https://github.com/ktrysmt/cntrdct)"


def http_get(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=30) as r:  # noqa: S310 — fixed-host
        return r.read()


def fetch_sdist(entry: CorpusEntry) -> Tuple[bytes, str]:
    """Return (raw_tarball_bytes, declared_sha256_from_pypi)."""
    meta = json.loads(
        http_get(PYPI_JSON.format(package=entry.package, version=entry.version))
    )
    sdist = next(
        (
            u
            for u in meta["urls"]
            if u["packagetype"] == "sdist" and u["url"].endswith(".tar.gz")
        ),
        None,
    )
    if sdist is None:
        raise RuntimeError(f"no sdist .tar.gz for {entry.package}=={entry.version}")
    body = http_get(sdist["url"])
    declared = sdist["digests"]["sha256"]
    actual = hashlib.sha256(body).hexdigest()
    if actual != declared:
        raise RuntimeError(
            f"sdist hash mismatch for {entry.package}=={entry.version}: "
            f"declared {declared}, got {actual}"
        )
    return body, sdist["url"]


def extract_file(tarball: bytes, member_path: str) -> bytes:
    with tarfile.open(fileobj=io.BytesIO(tarball), mode="r:gz") as tf:
        try:
            f = tf.extractfile(member_path)
        except KeyError:
            f = None
        if f is None:
            raise RuntimeError(f"member not found in tarball: {member_path}")
        return f.read()


def write_with_header(
    out_path: Path, body: bytes, source_url: str, license_spdx: str
) -> str:
    """Prepend a provenance header to `body` and write to `out_path`.
    Returns the SHA-256 of the written file."""
    header = (
        f"# Source: {source_url}\n"
        f"# License: {license_spdx}\n"
        f"# Note: verbatim extract from upstream sdist\n"
        f"\n"
    )
    composed = header.encode("utf-8") + body
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(composed)
    return hashlib.sha256(composed).hexdigest()


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out",
        default="benchmarks/wild-corpus-python",
        help="output directory (default: benchmarks/wild-corpus-python)",
    )
    parser.add_argument(
        "--manifest-skeleton",
        action="store_true",
        help=(
            "in addition to fetching files, write a manifest skeleton "
            "to <out>/manifest.skeleton.jsonl with empty `expected` arrays. "
            "Maintainer triages cntrdct findings into the real "
            "manifest.jsonl by hand."
        ),
    )
    args = parser.parse_args(argv[1:])

    out_root = Path(args.out)
    files_dir = out_root / "files"
    files_dir.mkdir(parents=True, exist_ok=True)

    skeleton: List[dict] = []
    for entry in CORPUS:
        print(f"fetching {entry.package}=={entry.version}", file=sys.stderr)
        tarball, sdist_url = fetch_sdist(entry)
        for member_path, flat_name in entry.files:
            try:
                body = extract_file(tarball, member_path)
            except RuntimeError as e:
                print(f"  skipping {flat_name}: {e}", file=sys.stderr)
                continue
            out_path = files_dir / flat_name
            sha = write_with_header(out_path, body, sdist_url, entry.license)
            print(f"  wrote files/{flat_name} sha256={sha[:12]}…", file=sys.stderr)
            skeleton.append(
                {
                    "file": f"files/{flat_name}",
                    "expected": [],
                    "source": sdist_url,
                    "license": entry.license,
                    "sha256": sha,
                }
            )

    if args.manifest_skeleton:
        skel_path = out_root / "manifest.skeleton.jsonl"
        with skel_path.open("w") as f:
            f.write(
                "// Manifest skeleton for the M-4 Python wild corpus.\n"
                "// Generated by scripts/fetch_python_corpus.py.\n"
                "// Maintainer fills in `expected: [...]` per file by\n"
                "// running `cntrdct scan` and triaging findings.\n"
            )
            for row in skeleton:
                f.write(json.dumps(row, sort_keys=True) + "\n")
        print(f"wrote {skel_path}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
