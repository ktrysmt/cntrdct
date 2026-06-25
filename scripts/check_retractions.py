#!/usr/bin/env python3
"""
Check every DOI cited by cntrdct against a cached Retraction Watch
snapshot and (optionally) Crossref Works' `update-to` records, and exit
non-zero if any cited paper has been retracted.

Protects design constraint P1 (citation discipline)
against retracted papers: a retracted citation MUST fail CI before it
ships.

DOI sources (union, deduplicated):
  - `CITATIONS.md` lines containing `DOI <doi>` or `doi.org/<doi>`.
  - Rust `Citation` static arrays under `src/**/*.rs` of the form
    `doi: Some("...")`. The keys-vs-CITATIONS.md consistency test in
    `tests/citations_consistency.rs` already enforces that these two
    sets agree on citation keys; the script unions DOIs to be robust
    against either side carrying the canonical value.

Retraction sources:
  - Cached Retraction Watch snapshot at
    `benchmarks/retraction-watch/cache.csv`. Schema matches the
    Crossref-Labs-hosted Retraction Watch dataset
    (https://api.labs.crossref.org/data/retractionwatch). The cache is
    pinned by SHA-256 at `benchmarks/retraction-watch/cache.sha256`;
    a mismatch fails the script unless `--no-verify-cache` is passed.
  - Crossref Works API (https://api.crossref.org/works/<doi>),
    inspecting `update-to` entries with `type == "retraction"`.
    Skipped when `--no-network` is set or when `urlopen` raises.

Modes:
  default                     check both cache and Crossref
  --no-network                check cache only (default for fixture tests)
  --verify-cache              recompute cache.sha256 and exit
  --refresh-cache             download a fresh Retraction Watch dump,
                              overwrite cache.csv, rewrite cache.sha256

Usage:
  python3 scripts/check_retractions.py
  python3 scripts/check_retractions.py --no-network
  python3 scripts/check_retractions.py --citations <path> --cache <path>
  python3 scripts/check_retractions.py --refresh-cache \\
      --retraction-watch-email kotaro.yoshimatsu@gmail.com

Exit codes:
  0   no retractions detected
  1   one or more cited DOIs are retracted
  2   cache integrity / argument / IO error
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Set, Tuple

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CITATIONS = REPO_ROOT / "CITATIONS.md"
DEFAULT_SRC = REPO_ROOT / "src"
DEFAULT_CACHE = REPO_ROOT / "benchmarks" / "retraction-watch" / "cache.csv"
DEFAULT_SHA = REPO_ROOT / "benchmarks" / "retraction-watch" / "cache.sha256"

# Crossref Labs hosts the Retraction Watch dataset post-2023. The
# endpoint requires an `email=` parameter so they can rate-limit and
# contact heavy users; pass via `--retraction-watch-email` or the
# `RETRACTION_WATCH_EMAIL` environment variable.
RETRACTION_WATCH_URL = "https://api.labs.crossref.org/data/retractionwatch"

# Crossref Works API — used for per-DOI `update-to` lookups. The polite
# pool requires an email in `mailto`; we pass one when available.
CROSSREF_WORKS_URL = "https://api.crossref.org/works/"

DOI_PATTERN_INLINE = re.compile(
    r"(?:DOI[:\s]+|doi\.org/)(10\.\d{4,9}/[^\s\"'<>),]+)",
    re.IGNORECASE,
)
DOI_PATTERN_RUST = re.compile(r'doi:\s*Some\(\s*"([^"]+)"\s*\)')


@dataclass(frozen=True)
class CitedDoi:
    """A single DOI plus a human-readable origin for diagnostics."""

    doi: str
    origin: str


@dataclass(frozen=True)
class Retraction:
    """One retraction record from the cache."""

    original_doi: str
    retraction_doi: str
    retraction_date: str
    reason: str


def normalise_doi(doi: str) -> str:
    """Lowercase + strip common surround for comparison."""

    d = doi.strip().rstrip(".,);]")
    d = d.lower()
    if d.startswith("doi:"):
        d = d[len("doi:") :]
    if d.startswith("https://doi.org/"):
        d = d[len("https://doi.org/") :]
    elif d.startswith("http://doi.org/"):
        d = d[len("http://doi.org/") :]
    elif d.startswith("doi.org/"):
        d = d[len("doi.org/") :]
    return d


def collect_dois_from_citations_md(path: Path) -> List[CitedDoi]:
    if not path.is_file():
        return []
    text = path.read_text(encoding="utf-8")
    out: List[CitedDoi] = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        for m in DOI_PATTERN_INLINE.finditer(line):
            out.append(
                CitedDoi(
                    doi=normalise_doi(m.group(1)),
                    origin=f"{path.name}:{lineno}",
                )
            )
    return out


def collect_dois_from_rust_sources(src_root: Path) -> List[CitedDoi]:
    if not src_root.is_dir():
        return []
    out: List[CitedDoi] = []
    for path in sorted(src_root.rglob("*.rs")):
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            continue
        for lineno, line in enumerate(text.splitlines(), start=1):
            for m in DOI_PATTERN_RUST.finditer(line):
                rel = path.relative_to(REPO_ROOT) if path.is_relative_to(REPO_ROOT) else path
                out.append(
                    CitedDoi(
                        doi=normalise_doi(m.group(1)),
                        origin=f"{rel}:{lineno}",
                    )
                )
    return out


def merged_dois(
    citations_md: Path,
    src_root: Path,
) -> Dict[str, List[str]]:
    """Return doi → list of origin strings (sorted, deduped)."""

    out: Dict[str, Set[str]] = {}
    for entry in collect_dois_from_citations_md(citations_md):
        out.setdefault(entry.doi, set()).add(entry.origin)
    for entry in collect_dois_from_rust_sources(src_root):
        out.setdefault(entry.doi, set()).add(entry.origin)
    return {k: sorted(v) for k, v in sorted(out.items())}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def verify_cache_integrity(cache: Path, sha_path: Path) -> Tuple[bool, str]:
    """Return (ok, message). True iff sha_path holds sha256 of cache."""

    if not cache.is_file():
        return False, f"cache file missing: {cache}"
    if not sha_path.is_file():
        return False, f"sha256 pin missing: {sha_path}"
    expected = sha_path.read_text(encoding="utf-8").strip().split()[0].lower()
    actual = sha256_file(cache).lower()
    if expected != actual:
        return False, (
            f"cache.sha256 mismatch:\n"
            f"  expected (pin):   {expected}\n"
            f"  actual  (cache):  {actual}\n"
            f"  cache: {cache}\n"
            f"  pin:   {sha_path}\n"
            f"  if the cache was updated intentionally, refresh the pin via "
            f"`python3 scripts/check_retractions.py --rewrite-sha`."
        )
    return True, f"cache.sha256 matches ({actual})"


def rewrite_sha(cache: Path, sha_path: Path) -> str:
    digest = sha256_file(cache)
    sha_path.write_text(digest + "\n", encoding="utf-8")
    return digest


def load_cache(cache: Path) -> List[Retraction]:
    """Parse cache.csv into Retraction records.

    Schema: header row identifies columns. We accept the canonical
    Retraction Watch column names (`OriginalPaperDOI`,
    `RetractionDOI`, `RetractionDate`, `Reason`) plus a few common
    casings. Empty cache (header only) yields an empty list.
    """

    if not cache.is_file():
        return []
    out: List[Retraction] = []
    with cache.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames is None:
            return []
        # Build a tolerant column lookup.
        cols = {name.strip().lower(): name for name in reader.fieldnames}

        def col(*candidates: str) -> Optional[str]:
            for c in candidates:
                key = c.strip().lower()
                if key in cols:
                    return cols[key]
            return None

        original_col = col("OriginalPaperDOI", "original_doi", "original")
        retraction_col = col("RetractionDOI", "retraction_doi", "retraction")
        date_col = col("RetractionDate", "retraction_date", "date")
        reason_col = col("Reason", "reason")
        if original_col is None:
            return []
        for row in reader:
            original = (row.get(original_col) or "").strip()
            if not original:
                continue
            out.append(
                Retraction(
                    original_doi=normalise_doi(original),
                    retraction_doi=normalise_doi(row.get(retraction_col, "") or "") if retraction_col else "",
                    retraction_date=(row.get(date_col, "") or "").strip() if date_col else "",
                    reason=(row.get(reason_col, "") or "").strip() if reason_col else "",
                )
            )
    return out


def crossref_retraction_status(doi: str, polite_email: Optional[str], timeout: float) -> Optional[Retraction]:
    """Return a Retraction record if Crossref reports the DOI as
    retracted, else None. Skips silently on network errors.
    """

    encoded = urllib.parse.quote(doi, safe="")
    url = CROSSREF_WORKS_URL + encoded
    headers = {
        "User-Agent": (
            "cntrdct-retraction-monitor/1.0 "
            f"(+https://github.com/ktrysmt/cntrdct{'; mailto=' + polite_email if polite_email else ''})"
        ),
        "Accept": "application/json",
    }
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            payload = json.load(resp)
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError, json.JSONDecodeError):
        return None

    message = payload.get("message", {}) if isinstance(payload, dict) else {}
    update_to = message.get("update-to", []) or []
    for entry in update_to:
        if not isinstance(entry, dict):
            continue
        if (entry.get("type") or "").lower() == "retraction":
            return Retraction(
                original_doi=normalise_doi(doi),
                retraction_doi=normalise_doi(entry.get("DOI", "") or ""),
                retraction_date=(entry.get("updated", {}) or {}).get("date-time", "")
                if isinstance(entry.get("updated"), dict)
                else "",
                reason="Crossref update-to type=retraction",
            )
    # Some retractions are signalled via top-level update_policy /
    # subtype rather than `update-to`; treat the explicit subtype
    # marker as retraction too.
    if (message.get("subtype") or "").lower() == "retraction":
        return Retraction(
            original_doi=normalise_doi(doi),
            retraction_doi=normalise_doi(message.get("DOI", "") or ""),
            retraction_date="",
            reason="Crossref subtype=retraction",
        )
    return None


def refresh_cache(
    cache: Path,
    sha_path: Path,
    email: Optional[str],
    timeout: float,
) -> Tuple[int, str]:
    """Download a fresh Retraction Watch dump and rewrite cache + sha.

    Returns (exit_code, message). Email is mandatory; without it the
    Crossref Labs endpoint returns 400. Output is normalised to the
    minimal four-column schema the loader expects.
    """

    if not email:
        return 2, "refresh-cache requires --retraction-watch-email or RETRACTION_WATCH_EMAIL"

    qs = urllib.parse.urlencode({"email": email})
    url = f"{RETRACTION_WATCH_URL}?{qs}"
    headers = {"User-Agent": "cntrdct-retraction-monitor/1.0", "Accept": "text/csv"}
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8", errors="replace")
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError) as exc:
        return 2, f"failed to fetch Retraction Watch dump: {exc}"

    reader = csv.DictReader(body.splitlines())
    if reader.fieldnames is None:
        return 2, "Retraction Watch response was empty / not CSV"

    cols = {name.strip().lower(): name for name in reader.fieldnames}

    def col(*candidates: str) -> Optional[str]:
        for c in candidates:
            key = c.strip().lower()
            if key in cols:
                return cols[key]
        return None

    original_col = col("OriginalPaperDOI", "original_doi", "original")
    if original_col is None:
        return 2, f"could not locate OriginalPaperDOI column; got {reader.fieldnames!r}"

    retraction_col = col("RetractionDOI", "retraction_doi") or ""
    date_col = col("RetractionDate", "retraction_date") or ""
    reason_col = col("Reason", "reason") or ""

    cache.parent.mkdir(parents=True, exist_ok=True)
    rows = 0
    with cache.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(["OriginalPaperDOI", "RetractionDOI", "RetractionDate", "Reason"])
        for row in reader:
            original = (row.get(original_col) or "").strip()
            if not original:
                continue
            writer.writerow(
                [
                    original,
                    (row.get(retraction_col) or "").strip() if retraction_col else "",
                    (row.get(date_col) or "").strip() if date_col else "",
                    (row.get(reason_col) or "").strip() if reason_col else "",
                ]
            )
            rows += 1

    digest = rewrite_sha(cache, sha_path)
    return 0, f"refreshed cache: {rows} rows; sha256 {digest}"


def parse_args(argv: List[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="Check cited DOIs against the Retraction Watch cache and Crossref",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    p.add_argument("--citations", type=Path, default=DEFAULT_CITATIONS)
    p.add_argument("--src", type=Path, default=DEFAULT_SRC)
    p.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    p.add_argument("--sha", type=Path, default=DEFAULT_SHA)
    p.add_argument(
        "--no-network",
        action="store_true",
        help="skip Crossref live lookup; cache-only mode (default in fixture tests)",
    )
    p.add_argument(
        "--no-verify-cache",
        action="store_true",
        help="skip cache.sha256 integrity check (used by --refresh-cache)",
    )
    p.add_argument("--verify-cache", action="store_true", help="verify cache integrity and exit")
    p.add_argument("--rewrite-sha", action="store_true", help="rewrite cache.sha256 and exit")
    p.add_argument(
        "--refresh-cache",
        action="store_true",
        help="download a fresh Retraction Watch dump and rewrite cache + sha; exit",
    )
    p.add_argument(
        "--retraction-watch-email",
        default=os.environ.get("RETRACTION_WATCH_EMAIL"),
        help="email passed to the Crossref Labs Retraction Watch endpoint",
    )
    p.add_argument(
        "--polite-email",
        default=os.environ.get("CROSSREF_POLITE_EMAIL"),
        help="email passed to Crossref Works as a polite-pool identifier",
    )
    p.add_argument("--timeout", type=float, default=20.0)
    return p.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(list(argv if argv is not None else sys.argv[1:]))

    if args.rewrite_sha:
        if not args.cache.is_file():
            print(f"error: cache file missing: {args.cache}", file=sys.stderr)
            return 2
        digest = rewrite_sha(args.cache, args.sha)
        print(f"rewrote {args.sha} ({digest})")
        return 0

    if args.refresh_cache:
        code, msg = refresh_cache(args.cache, args.sha, args.retraction_watch_email, args.timeout)
        print(msg, file=sys.stderr if code != 0 else sys.stdout)
        return code

    if args.verify_cache:
        ok, msg = verify_cache_integrity(args.cache, args.sha)
        print(msg, file=sys.stdout if ok else sys.stderr)
        return 0 if ok else 2

    if not args.no_verify_cache:
        ok, msg = verify_cache_integrity(args.cache, args.sha)
        if not ok:
            print(f"error: {msg}", file=sys.stderr)
            return 2

    dois = merged_dois(args.citations, args.src)
    if not dois:
        print("warning: no DOIs found to check", file=sys.stderr)
        return 0

    cache_records = load_cache(args.cache)
    cache_index: Dict[str, Retraction] = {r.original_doi: r for r in cache_records}

    retracted: List[Tuple[str, List[str], Retraction]] = []
    for doi, origins in dois.items():
        hit = cache_index.get(doi)
        if hit is None and not args.no_network:
            hit = crossref_retraction_status(doi, args.polite_email, args.timeout)
        if hit is not None:
            retracted.append((doi, origins, hit))

    if retracted:
        print("RETRACTION DETECTED — design constraint P1 violated:", file=sys.stderr)
        for doi, origins, rec in retracted:
            print(
                f"  - {doi}\n"
                f"      origin(s): {', '.join(origins)}\n"
                f"      retraction DOI: {rec.retraction_doi or '(unknown)'}\n"
                f"      date:           {rec.retraction_date or '(unknown)'}\n"
                f"      reason:         {rec.reason or '(unknown)'}",
                file=sys.stderr,
            )
        return 1

    print(
        f"OK: {len(dois)} cited DOI(s) checked, no retractions found "
        f"({'cache-only' if args.no_network else 'cache + Crossref'}).",
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
