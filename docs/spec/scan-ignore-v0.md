# Scan ignore v0 spec (I-1)

Status: implemented. Owner of the I-1 track. This document is the spec
referenced by `src/lib.rs::collect_supported_files` and
`src/main.rs` (`scan --no-ignore`). It supersedes the traversal parts
of `cli-v0.md` ("walk recursively and collect all `*.rs` files",
`walkdir` in the dependency list); the extension-to-language routing of
`multilang-v0.md` F5 is unchanged.

## Background

Before I-1 the file walker (`walkdir`) visited EVERY file under the
scan root: `target/`, `node_modules/`, vendored trees. Measured on this
repository, `cntrdct scan .` reported 371 findings of which 244 (~66%)
came from gitignored directories (`target/` 204, `data/` 40). The same
shape reproduces in every real Rust repo (`target/`) and JS repo
(`node_modules/`), making it the single biggest out-of-the-box noise
source and a direct adoption blocker.

## Behaviour

`cntrdct scan <dir>` discovers files with ripgrep's `ignore` crate
(`ignore::WalkBuilder`) instead of `walkdir`, keeping the crate's
standard filters (the ripgrep conventions users already know):

- `.gitignore`, `.ignore`, `.git/info/exclude`, and the global git
  excludes file (`core.excludesFile`) are respected. Git-derived rules
  apply only inside a git repository (`require_git` default); `.ignore`
  files apply everywhere.
- Ignore files in parent directories of the scan root are respected
  (`parents`), so scanning a subdirectory of a repo still honours the
  repo root's `.gitignore`.
- Hidden files and directories (dotfiles, including `.git/`) are
  skipped.

`scan --no-ignore` disables the ignore-FILE semantics (`.gitignore`,
`.ignore`, global git excludes, `.git/info/exclude`, parent discovery)
but keeps the hidden filter, so `.git/` internals are never scanned in
either mode. This mirrors ripgrep, where `--no-ignore` and `--hidden`
are independent axes; v0 ships only the former.

Unchanged:

- A single-FILE path argument (`cntrdct scan foo.rs`) bypasses the
  walker and is always scanned, ignored or not — same as naming a file
  explicitly on ripgrep's command line. A DIRECTORY argument is still
  walked with ignore rules (parent-directory discovery included), so
  `cntrdct scan target` on a gitignored `target/` yields zero files;
  use `--no-ignore` to scan an ignored directory.
- `scan_buffer` (the LSP path) never touches the walker.
- Extension-to-language routing (`multilang-v0.md` F5) and the
  deterministic per-directory file-name sort the ranker and snapshot
  tests rely on (`sort_by_file_name`).
- `eval` and `calibrate --audit-recall` walk their corpus directories
  with `no_ignore` semantics, preserving their pre-I-1 reports
  byte-for-byte. A corpus directory is an explicit input — and scratch
  corpora are commonly gitignored — so letting ignore rules empty the
  walk would silently score every manifest entry as a miss.

## API

- `WalkOptions { no_ignore: bool }`, `Default` = ignore files
  respected.
- `scan_full_with_options(path, config, walk)` — new entry point
  carrying the walk options.
- `scan_full_with_config(path, config)` — signature unchanged;
  delegates with `WalkOptions::default()`, so existing library callers
  pick up the new default without a code change.

## Design decisions

- Default ON, opt-out flag (not the reverse): the noisy default was
  the adoption blocker; a `--respect-gitignore` opt-in would leave
  first-run output dominated by build artifacts.
- `require_git` stays at its default (true). A `.gitignore` outside a
  git repository is inert, which keeps tempdir-based test fixtures
  (and unpacked archives) scanning every file unless they opt in with
  `.ignore` or a real `.git` marker.
- The hidden filter stays on under `--no-ignore` so the acceptance
  guarantee ".git/ is never scanned" holds unconditionally. A file
  that is BOTH hidden and wanted has no v0 escape hatch besides
  naming it explicitly (see Non-goals).
- Corpus-sensitive detectors (clone-drift sibling majorities,
  pr-miner frequent itemsets) see a smaller file universe under the
  new default, so per-file findings can legitimately differ between
  the two modes beyond the ignored files themselves — on a repo-root
  scan of this repo, findings attributed to `benchmarks/` files
  number 161 by default vs 118 under `--no-ignore` (a direct
  `cntrdct scan benchmarks` is unaffected by the mode). This is
  inherent to statistical detectors, not a walker defect.
- P3: the `ignore` crate is filesystem-only; `scan` still opens no
  socket and the `network-isolation` netns CI gate covers the new
  path unchanged.

## Non-goals (v0)

- A `--hidden` flag to scan dotfiles. Cut when someone asks; the
  single-file bypass covers the rare hidden-file scan today.
- A `cntrdct.toml` toggle for ignore semantics. `[paths]` globs
  already handle repo-specific exclusions (e.g. a planted-corpus
  directory like `benchmarks/`); duplicating walker policy in config
  would create two sources of truth.
- Ignore semantics in `scan_buffer` / LSP diagnostics: editors hand
  us buffers for files the user deliberately opened.

## Tests

`tests/scan_ignore.rs`:

- gitignored file skipped by default, restored by `--no-ignore`
  (file counts pinned via the S-1 "in N file(s)" stderr summary).
- `.git/` contents never scanned, with and without `--no-ignore`.
- `.gitignore` without a `.git` marker is inert (`require_git` pin).
- `.ignore` applies outside git repositories.
- An explicit single-file argument bypasses ignore rules.
