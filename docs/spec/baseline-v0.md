# Baseline (ratchet) v0 spec (B-1)

Status: implemented. Owner of the B-1 track. This document is the spec
referenced by `src/baseline.rs` and the `--baseline` /
`--write-baseline` / `--fail-on` flags on `cntrdct scan`.

## Background

Adopting a linter in an existing codebase fails on the first run when
the tool reports every pre-existing finding as equally urgent: the
team cannot fix hundreds of findings at once, starts ignoring the
output, and eventually removes the tool. The standard mitigation
(PHPStan baseline, detekt baseline, Semgrep `--baseline-commit`) is a
ratchet: record the current findings once, then report only NEW
findings on every subsequent scan.

B-1 is the cntrdct version of that ratchet. It is a pure,
deterministic output filter — no LLM, no network (P3), no detector
changes (P1 untouched), no effect on SARIF severity mapping (P5; the
filter runs before emission).

## Baseline file

JSON, pretty-printed, trailing newline, entries sorted by
`(detector_id, file, message)` so regeneration produces stable diffs:

```json
{
  "version": 1,
  "entries": [
    {
      "detector_id": "unreachable-after-terminator",
      "file": "src/dead.rs",
      "message": "statement is unreachable; preceded by return on line #",
      "count": 1
    }
  ]
}
```

- `version` — format version; a mismatch is a hard error with a
  regenerate hint (never a silent partial match).
- Fingerprint components are stored explicitly (not as an opaque
  hash) so a baseline diff is reviewable: each entry names what is
  being tolerated.
- The file location is user-chosen (`--baseline <path>` /
  `--write-baseline <path>`); the docs use `cntrdct-baseline.json` at
  the repo root by convention.

## Fingerprint

A finding's fingerprint is the triple:

1. `detector_id` — verbatim.
2. `file` — the primary location's path, relative to the scan root,
   `/`-separated (`\` normalized) so baselines are portable across
   machines, checkouts, and platforms. When the scan root IS the file
   (single-file scan) the bare file name is used. Consequence: a
   baseline is tied to its scan root — compare with the same root it
   was written with (`scan src` vs `scan .` produce different keys).
3. `message` — with every ASCII digit run collapsed to `#`.

Line/column numbers are deliberately excluded, and messages are
digit-normalized, because two detectors embed line numbers in their
messages (`unreachable-after-terminator`, `python-unreachable-except`)
and one embeds a volatile sibling count (`clone-drift`). Without
normalization, inserting an unrelated line above a known finding would
resurrect it — the opposite of what a ratchet promises. The cost is
coarser identity: two findings in one file differing only by numbers
share a fingerprint. That is handled by `count`.

### Count semantics

Each entry carries the number of identical-fingerprint occurrences
tolerated (PHPStan style). At filter time the counts form a budget:
findings are walked in ranked order, each match decrements its
fingerprint's budget, and once the budget is exhausted further
occurrences surface as new. Recording 2 and scanning 3 reports exactly
1 finding.

## CLI surface

- `--write-baseline <FILE>` — record the current finding set and exit
  0. Runs at the END of the pipeline (post-suppression,
  post-adjudication) so the baseline captures exactly what the scan
  would have reported. The findings are still printed. `--fail-on` is
  NOT enforced on this run (recording IS accepting), with a stderr
  note when a non-`never` policy was requested. Mutually exclusive
  with `--baseline`; to update a baseline, regenerate it.
- `--baseline <FILE>` — filter the output through the baseline. Runs
  AFTER Layer 2 ranking and BEFORE Layer 3 adjudication, so the
  opt-in LLM budget (`--adjudicate-top`) is spent on new findings
  only. Applies to both `--format json` and `--format sarif`. A
  missing or malformed file is a hard error (exit 1): in CI a typo'd
  path must fail loudly rather than report everything as new. The
  scan summary reports `baseline: N known finding(s) suppressed; M
  new` on stderr so an empty output cannot be misread as a clean
  codebase.
- `--fail-on {error,warning,never}` — exit-code policy, default
  `never` (preserves pre-B-1 behaviour: scan always exits 0 on
  success). `error` exits 3 when any REPORTED finding has
  `raw_severity` Error; `warning` also counts Warning. Applied after
  baseline filtering — with `--baseline`, only new findings can fail
  the run. The vocabulary mirrors the `fail-on` input of the GitHub
  Action (`.github/actions/scan`); the Action's own enforcement is
  unchanged and composes (the action can keep `fail-on: never` at the
  CLI level and enforce via `enforce.py`, or vice versa).

## Exit codes

| code | meaning |
|---|---|
| 0 | scan succeeded; no finding at/above the `--fail-on` threshold |
| 1 | operational error (bad path, unreadable baseline, config error) |
| 2 | CLI usage error (clap) |
| 3 | `--fail-on` threshold met by at least one reported finding |

3 is a new, distinct code so scripts can tell "scan crashed" from
"scan found problems".

## Ratchet workflow

```sh
# once, at adoption time
cntrdct scan . --write-baseline cntrdct-baseline.json
git add cntrdct-baseline.json

# every run afterwards (local, pre-commit, CI)
cntrdct scan . --baseline cntrdct-baseline.json --fail-on warning

# after fixing old findings (or accepting new ones), re-ratchet
cntrdct scan . --write-baseline cntrdct-baseline.json
```

## Non-goals (v0)

- Git-diff-based baselines (Semgrep `--baseline-commit`): requires
  invoking git and materializing a second working tree; revisit if the
  fingerprint ratchet proves too coarse.
- Automatic baseline shrinking (removing entries whose findings no
  longer occur): regeneration covers it.
- Combining `--baseline` with `--write-baseline` in one run.
- Baseline input plumbing for the GitHub Action (follow-up; the CLI
  flags already work inside any workflow step).

## Tests

- Unit: `src/baseline.rs` (`normalize_message`, `file_key`, count
  budget, sorted output, version rejection).
- Integration: `tests/scan_baseline.rs` (write→rescan→empty,
  line-shift tolerance, new-finding surfacing, SARIF filtering,
  missing-file error, flag conflict, `--fail-on` matrix).
