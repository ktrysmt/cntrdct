# Suppression v0 spec (T2-7)

Status: implemented. Owner of the T2-7 track. This document is the spec
referenced by `src/lib.rs::apply_suppression` and the `src/config.rs`
module docs.

## Background

cntrdct's detectors are deliberately many and independent, so users need
escape hatches to silence a finding they have judged intentional. Two
seams exist, both applied by `apply_suppression` /
`crate::config::apply` after Layer 1 produces `Vec<Finding>` and before
ranking:

1. Project-level: a `cntrdct.toml` at the scan root.
2. In-source: an annotation next to the flagged code.

Both are deterministic and run with no network / LLM (P3): suppression is
a pure filter over findings plus the parsed source tree.

## `cntrdct.toml`

Discovered at `<scan-root>/cntrdct.toml` (or supplied via `--config`); a
missing file is not an error. Relevant sections:

- `[paths]` — `include` / `exclude` globs. A finding whose primary file
  matches any `exclude` glob is dropped; when `include` is non-empty, a
  finding whose primary file matches no `include` glob is dropped.
  Exclusion wins over inclusion.
- `[detectors.<id>]` — `enabled = false` drops every finding from that
  detector; `severity = "<name>"` remaps `raw_severity`.
- `[languages.<canonical-name>]` — keyed by the canonical language name
  (`rust`, `python`, `typescript`, `tsx`, `go`). `enabled = false` makes
  the file walker skip files of that language at discovery time;
  `suppress = ["<id>", ...]` drops findings whose primary file is in that
  language and whose `detector_id` is listed. The canonical-name set is
  `Language::all()` mapped through `Language::canonical_name()`.

## In-source suppression

`collect_attribute_suppressions(file)` dispatches on `file.language` and
returns a set of `AttributeSuppression { detector_ids, start_line,
end_line }` line ranges. A finding is dropped when its primary
`start_line` falls within a suppression range whose `detector_ids` is
`None` (catch-all) or contains the finding's `detector_id`.

### Rust — attribute form

`#[cntrdct::allow(<id>, ...)]` on the item (function, struct, enum, impl,
trait, mod, const, static, type) containing the finding. Inner attributes
(`#![cntrdct::allow(...)]`) inside an item are also honoured. The empty
form `#[cntrdct::allow()]` suppresses every detector on the item.

### Python / TypeScript / `.tsx` / Go — comment form

- Python: `# cntrdct: allow(<id>, ...)`
- TypeScript / `.tsx` / Go: `// cntrdct: allow(<id>, ...)` (block
  `/* cntrdct: allow(...) */` also accepted)

The comment-based languages share one collector
(`collect_comment_suppressions`), differing only in the comment-marker
prefix stripped before the `cntrdct: allow(...)` payload is parsed
(`#` for Python, `//` / `/* */` for TS / `.tsx` / Go). Two placements:

- Trailing (`code()  // cntrdct: allow(<id>)`): the comment shares its
  line with code (detected by any non-whitespace byte before the comment
  on that line). Suppresses findings whose `start_line` equals the
  comment's line.
- Standalone (the comment occupies its own line): suppresses the next
  non-comment named sibling's full span, mirroring the Rust
  attribute-precedes-item shape. Intervening blank lines and additional
  allow lines stack onto the same target.

`allow()` with an empty argument list is the catch-all that suppresses
every detector on the range.

Directive comments that happen to start with the comment marker
(`//go:build ...`, `/// <reference ... />`) never parse as an allow
payload — the `cntrdct:` prefix check fails — so they stay inert.

## Non-goals

- Suppression by finding fingerprint / baseline (that is the Q-15
  baselines mechanism, a separate seam).
- Wildcards in `allow(...)` (e.g. `allow(clone-*)`). The argument list is
  an exact `detector_id` set.
- Block-level Rust suppression on arbitrary statements: Rust attaches to
  items only, matching the language's attribute grammar.
