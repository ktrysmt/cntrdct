# Multi-language architecture v0 spec

Status: draft, awaiting approval before implementation. Owner of the
M-series ROADMAP track.

## Background

cntrdct shipped v0 as a Rust-only linter. Every detector hard-codes
tree-sitter-rust, every per-file loop checks `file.language == "rust"`,
and the CLI walker filters for `.rs` extensions. The core trait
surface (`Detector`, `ParsedFile`, `supported_languages`) was
designed to admit other languages but no concrete second language has
ever been validated.

The strategic pivot is documented in ROADMAP M-series: the
peer-reviewed-citation differentiator is language-agnostic, the
commercial market for a single-language linter is bounded, and the
academic Track A explicitly does not require Rust focus. This spec
describes the architecture that makes Python (and later TypeScript /
Go / Java) a first-class deployment target.

## Scope

In:

- A `Language` enum and parser-provider abstraction usable across all
  detectors, the CLI, and the corpus-fetch pipeline.
- A migration path from the current `String`-typed `language` field
  to canonical names without breaking existing tests.
- The detector dispatch pattern: cross-cutting concepts use one crate
  parameterised by `Language`; language-specific concepts stay in
  their own crates.
- The P1 extension that requires per-language citations (delegated to
  `citations-policy.md`).

Out:

- Concrete Python detector implementations (M-2 / M-3 own those).
- New languages beyond Python (the M-series sequences add languages
  one at a time after this spec lands).
- Replacing tree-sitter with a different parsing backend.
- IDE / LSP integration for non-Rust languages.

## Functional requirements

### F1 — Language enum

A new crate `cntrdct-parsers` defines:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
}
```

Variants are added one at a time as detectors land for each
language. The enum is `non_exhaustive` so downstream crates that
match on it must opt into a `_ => ...` arm — this protects against
silent breakage when a language is added.

### F2 — Extension mapping

`cntrdct-parsers::detect_language(path: &Path) -> Option<Language>`
maps file extensions to variants. v0 mappings:

- `.rs` → `Language::Rust`
- `.py`, `.pyi` → `Language::Python`

A `Path` with no recognised extension returns `None`; the caller
decides whether to skip silently (current scan behaviour) or warn.

Shebang detection (`#!/usr/bin/env python3` on extension-less files)
is out of scope for v0; revisit when Phase F users surface a need.

### F3 — Parser provider

```rust
pub trait ParserProvider: Send + Sync {
    fn language(&self) -> Language;
    fn ts_language(&self) -> tree_sitter::Language;
}

pub fn parser_for(lang: Language) -> Box<dyn ParserProvider>;
```

Concrete providers (`RustParserProvider`, `PythonParserProvider`)
own their tree-sitter language constructor. Detectors call
`parser_for(file.language).ts_language()` rather than depending on
`tree_sitter_rust` or `tree_sitter_python` directly. This means
detector crates lose their direct tree-sitter-<lang> dependency in
favour of a single dependency on `cntrdct-parsers`.

A detector can short-circuit by language at any time:

```rust
fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
    ctx.files
        .par_iter()
        .filter(|f| matches!(f.language, Language::Rust | Language::Python))
        .flat_map_iter(|f| self.scan_one(f))
        .collect()
}
```

### F4 — `ParsedFile.language` migration

Phase 4a: keep `ParsedFile.language: String` but document the
canonical name set in `cntrdct-parsers` (`"rust"`, `"python"`). All
existing string comparisons keep working.

Phase 4b (after M-2 lands): change `ParsedFile.language` to
`Language` enum. All `if file.language != "rust"` sites become
`matches!(file.language, Language::Rust | Language::Python)` (or
just `_` filters where the detector accepts every language it can
parse).

Phase 4b is breaking for any out-of-tree consumer of
`cntrdct-core`, but that surface is currently us-only. Bump
`cntrdct-core` to 0.2.0 in the same PR.

### F5 — CLI file walker

`crates/cli/src/lib.rs::collect_rust_files` becomes
`collect_supported_files(root: &Path, langs: &[Language])`. The
default `langs` set is `Language::all()` (every variant). The
walker uses `cntrdct-parsers::detect_language` to assign each
walked file a `Language`; files that map to `None` are silently
dropped, identical to current `.rs`-only behaviour.

### F6 — Detector dispatch pattern

Default: Pattern A — one detector crate parameterised by language.

```rust
impl Detector for ArgSwap {
    fn supported_languages(&self) -> &'static [Language] {
        &[Language::Rust, Language::Python]
    }
    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        ctx.files
            .par_iter()
            .filter(|f| self.supported_languages().contains(&f.language))
            .flat_map_iter(|f| match f.language {
                Language::Rust   => self.scan_rust(f),
                Language::Python => self.scan_python(f),
            })
            .collect()
    }
}
```

The per-language `scan_*` helpers are private to the detector crate.
Differences in AST node kinds, terminator sets, doc-comment syntax,
etc. are absorbed inside the dispatch arm; the public surface
(`Detector` trait, citation set, registration) stays one entity per
detector concept.

This applies to `unreachable-after-terminator` as well: the
divergent-terminator set (`return`/`panic!()`/`unreachable!()` in
Rust; `raise`/`sys.exit`/`os._exit`/`assert False`/`return` in
Python) is held in a per-language constant table inside the crate.
The walk + post-terminator-statement detection is shared.

Pattern B — separate detector crate — is reserved for the case where
the bug pattern itself is single-language by definition. Specifically:

- `config-interaction` is the canonical example: it detects
  contradictory pairs of Rust `#[cfg(...)]` attributes. The concept
  is `cfg`, not "build-time configuration in general". Its
  `supported_languages()` stays `&[Language::Rust]`.
- A future Go-build-tag detector would be a separate crate with its
  own `detector_id` (e.g. `build-tag-interaction-go`) — even though
  conceptually similar to `config-interaction`, the AST mechanism
  and the bug-pattern wording differ enough that conflating them
  would muddy the citation set.

The decision rule: "is this the same bug pattern in different
syntaxes (→ A), or a different bug pattern that happens to belong
to the same family (→ B)?"

### F7 — `supported_languages` return type

The trait method changes return type from `&'static [&'static str]`
to `&'static [Language]`. This is a `cntrdct-core` breaking change
landed together with F4 phase 4b.

### F8 — Detector registration

`register_detector` (P1 enforcement) gains a per-language citation
check delegated to `citations-policy.md` rules. See M-6.

## Migration sequence

1. M-1 ships F1, F2, F3, F4-phase-4a, F5. No detector behaviour
   changes; existing tests continue to pass.
2. M-6 ships citations-policy.md and the consistency test extension.
3. M-2 ships the first Python detector — `unreachable-after-terminator`
   extended via Pattern A. This is the spec's first stress-test for
   the dispatch model; if Pattern A produces unreasonable
   `match`-bloat the abstraction is revised, but Pattern B is not
   the fallback (a single large match is preferable to two crates
   that share a `detector_id`).
4. F4 phase 4b lands together with M-3's first cross-cutting Python
   detector (likely `comment-code`, since Python docstrings are
   well-structured and the iComment / aComment lineage maps cleanly).
5. M-3 finishes the remaining cross-cutting detectors.
6. M-4 adds the Python β corpus.
7. M-5 wires the action / config / SARIF surfaces.

## Non-goals

- Auto-discovery of supported languages by introspecting installed
  tree-sitter grammars. The `Language` enum is closed and intentional.
- Per-language severity defaults. Severity stays detector-defined
  with `cntrdct.toml` as the override seam.
- Translation of detector messages. English-only.

## Risks and open questions

R1. tree-sitter-python's grammar is more permissive about partial
parses than tree-sitter-rust. `if root.has_error()` may need
language-specific tolerance — currently every detector skips files
with parse errors silently. We may need a `recover_on_error: bool`
toggle per language.

R2. `cntrdct-parsers` becoming a transitive dependency of every
detector crate means a tree-sitter version bump touches more
packages than today. Pin tree-sitter at the workspace level (already
the case via `[workspace.dependencies]`) and treat the bump as a
single PR.

R3. Per-language citations may not exist for every cross-cutting
detector. iComment was C; aComment was Java; PR-Miner (Li & Zhou)
was C/C++; Rice et al. was Java/C++. None ran on Python natively.
M-6's policy is best-effort (see citations-policy.md): the survey
must happen and be recorded, but a missing per-language citation
does not block the language extension. The detector still ships
with its existing cross-cutting citation; the resulting metadata
flag (`language_citation_status: "unconfirmed"`) tells SARIF
consumers that this language's coverage is grounded indirectly.

R4. The `corpus-fetch` crate (currently Rust-specific via the
crates.io Sparse Index) does not generalise to PyPI directly. M-4
will likely stand up `corpus-fetch-python` rather than parameterise
the existing one — the source-of-truth APIs are too different.

## Compatibility

- `cntrdct-core` minor version bumps to 0.2.0 when F4 phase 4b
  lands.
- `cntrdct-parsers` ships at 0.1.0 alongside M-1.
- Detector crates do not need a major bump; their public API stays
  the same trait surface.
- `cntrdct.toml` schema gets an additive `[languages]` section in
  M-5; older configs continue to work. The section is keyed by
  canonical language name and supports two fields per entry:
  - `enabled: bool` — when `false`, the file walker skips files of
    that language at discovery time.
  - `suppress: [String]` — detector IDs whose findings are dropped
    when the finding's primary file is in this language. Equivalent
    to `[detectors.<id>] enabled = false` but scoped to one language.
- The GitHub Action wrapper (`.github/actions/scan/`) gains a
  multi-line `paths:` input where each line is `<path>` or
  `<path>:<lang_csv>`. The optional `:<lang_csv>` synthesises an
  ephemeral `cntrdct.toml` that enables only the listed languages
  for that path's scan. Mutually exclusive with the user-supplied
  `config:` input — the action errors out if both are set with a
  per-path hint, since the synthesised file would conflict with the
  user's. Default lang-universe is hard-coded in
  `prepare_config.py` and updated in lockstep with
  `cntrdct-parsers::Language::all()`.
- SARIF emitter is unchanged. Multi-path scans merge SARIF
  documents by concatenating the `runs[]` array
  (`merge_sarif.py`); SARIF natively supports multiple runs per
  document so no rule-table normalisation is required.

## Approval

Approved 2026-05-04 with the following decisions locked in:

- Pattern A is the default. Pattern B is reserved for detectors
  whose bug pattern is single-language by definition
  (`config-interaction` is the only current example).
- F4 phase 4b (the `ParsedFile.language: String → Language` change)
  ships together with M-3's first cross-cutting Python detector and
  bumps `cntrdct-core` to 0.2.0.
- R3 (per-language citation availability) is handled per
  `citations-policy.md`: best-effort survey with explicit
  `language_citation_status` metadata when a primary citation is
  unavailable, rather than blocking language support.

Implementation can begin with M-6, then M-1, per the M-series
sequencing in ROADMAP.
