# Retraction Watch cache

Snapshot of the [Retraction Watch dataset](https://retractionwatch.com/),
hosted by [Crossref Labs](https://api.labs.crossref.org/data/retractionwatch)
since 2023, used by `scripts/check_retractions.py` to fail CI when any
DOI cited by a shipped detector has been retracted.

This is the cached half of the Q-6 retraction monitor (`ROADMAP.md`).
The Crossref Works `update-to` lookup is the live half; together they
catch retractions whether or not the bibliographic record has caught
up to Retraction Watch (or vice versa).

## Files

- `cache.csv` — the snapshot itself. Schema mirrors the Crossref Labs
  endpoint's canonical column names so the refresh job is a thin
  copy. Header line is required; row order is not significant.
  - `OriginalPaperDOI` — DOI of the retracted work (the one a
    detector might cite). Lowercased on read.
  - `RetractionDOI` — DOI of the retraction notice itself, where
    available.
  - `RetractionDate` — ISO-8601 date the retraction was issued.
  - `Reason` — free-text Retraction Watch summary
    (e.g. `+Plagiarism;+Falsification of Data`).
- `cache.sha256` — hex SHA-256 of `cache.csv`. Pinned and verified by
  the script on every run; an out-of-band edit fails CI until the pin
  is rewritten via `python3 scripts/check_retractions.py --rewrite-sha`.

## Refresh procedure

The repository ships an empty cache (header only). The
`.github/workflows/citations.yml` `refresh-cache` job runs weekly and
opens a PR with the latest snapshot. Manually:

```sh
RETRACTION_WATCH_EMAIL=you@example.com \
    python3 scripts/check_retractions.py --refresh-cache
git add benchmarks/retraction-watch/{cache.csv,cache.sha256}
git commit -m "chore(citations): refresh Retraction Watch cache"
```

The Crossref Labs endpoint requires the email parameter so they can
contact heavy users — see
<https://api.labs.crossref.org/swagger-ui/index.html#/data/retractionwatch>.

## Why a cache?

CI must fail closed on retractions, but the Crossref Works API has
rate-limit and uptime variability. Caching the Retraction Watch
dataset gives every PR run a deterministic floor of detected
retractions; the live Crossref lookup is opportunistic on top.
