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

Two patterns coexist:

Pattern A — parameterised by language (cross-cutting concept):

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

Pattern B — separate detector crate per language (language-specific
concept):

`config-interaction` (Rust `#[cfg(...)]`) stays its own crate. A
hypothetical Go-build-tag detector would be a new crate with its own
`detector_id` (e.g. `build-tag-interaction-go`).

Pattern A vs B is decided per detector at design time by asking:
"is the bug pattern the same idea expressed in different syntaxes,
or is it a structurally different anomaly?" The former → A, the
latter → B.

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
3. M-2 ships the first Python detector (Pattern A applied to
   `unreachable-after-terminator` is borderline — the divergent
   terminator set differs significantly between Rust and Python, so
   we may decide here to use Pattern B instead and create a separate
   `unreachable-after-terminator-python` crate). This is the spec's
   first stress-test; M-1's abstraction is revised if it doesn't
   accommodate.
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

R1. Pattern A vs B for `unreachable-after-terminator`: the divergent
terminator set in Python (`raise`, `sys.exit`, `os._exit`,
`assert False`) overlaps semantically with Rust's set but the AST
node names are different and the detection logic touches different
tree-sitter node kinds. If the Pattern A implementation forces
unreasonable code-sharing, fall back to Pattern B and document the
decision in M-2's spec addendum.

R2. tree-sitter-python's grammar is more permissive about partial
parses than tree-sitter-rust. `if root.has_error()` may need
language-specific tolerance — currently every detector skips files
with parse errors silently. We may need a `recover_on_error: bool`
toggle per language.

R3. `cntrdct-parsers` becoming a transitive dependency of every
detector crate means a tree-sitter version bump touches more
packages than today. Pin tree-sitter at the workspace level (already
the case via `[workspace.dependencies]`) and treat the bump as a
single PR.

R4. Per-language citations may not exist for every cross-cutting
detector. iComment was C; aComment was Java; PR-Miner (Li & Zhou)
was C/C++; Rice et al. was Java/C++. None ran on Python natively.
M-6's policy will require a specific Python-grounded citation; some
detectors may need contemporary references (Pradel-Sen 2018 etc.)
or domain-specific Python static-analysis surveys. Expect a survey
budget per language.

R5. The `corpus-fetch` crate (currently Rust-specific via the
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
  M-5; older configs continue to work.
- SARIF emitter is unchanged.

## Approval

This spec is approved when:

1. Pattern A vs B decision is recorded for each shipping detector.
2. R4 (per-language citation feasibility) has been pre-surveyed for
   the cross-cutting trio (`clone-drift`, `arg-swap`, `comment-code`)
   and has at least one candidate citation each.
3. ROADMAP M-1 has a green-light date.
