# Detector false-positive backlog

Status: backlog (candidate precision improvements, not yet implemented).

Provenance: harvested from the research-track false-positive failure-mode
taxonomy (`research/projects/A_1000_crate/failure-modes-v1.md` §3, plus
the 2026-05-06 wild-corpus dry-run) before that research workspace was
retired. Each entry is a known false-positive class for a Layer 1
detector and the suppression it suggests. None of these are wired into
the detectors yet (verified against `src/detectors/*` at harvest time);
they are candidate improvements, not contracts. Real-world examples are
preserved so the lead can be reproduced.

Each candidate, if implemented, must still satisfy P1 (cite peer-reviewed
prior art) and the existing per-detector spec under `docs/spec/`.

## arg-swap (`src/detectors/arg_swap.rs`, `docs/spec/arg-swap-v0.md`)

- `commutative-callee` — suppress the swap finding when the callee is
  commutative over the two arguments (`min`, `max`, `std::cmp::Ord::cmp`,
  set union, addition, ...). Swapping the operands does not change
  meaning. Example: `cmp::min(local_min, candidate)` where the two local
  names partially match the callee's `(a, b)` parameters but `min` is
  commutative. Not currently present (the only `.cmp` uses in the
  detector are for result sorting).
- `type-distinct-positions` — suppress when the two argument types differ
  between call site and definition, so a positional swap is impossible
  under the type system.
- `builder-positional-convention` — suppress for builder / coordinate
  constructors (`Point::new(x, y)`, `Range { start, end }`) where the
  name match is incidental and the call convention is established.

## clone-drift (`src/detectors/clone_drift.rs`, `docs/spec/clone-drift-v0.md`)

- `cross-crate-pool-mismatch` — a detector-scope bug, not a per-finding
  one: the similarity pool spans crate boundaries, so two independent
  implementations of the same conceptual role (parser combinator,
  iterator combinator, container API) in different crates land in the
  same pool and fire a spurious "divergence". Example: nom's `tag`
  combinator and winnow's parser helper co-occur in the shape-similarity
  pool. The fix is in the pool boundary / `MAX_RELATED` scoping, not in
  judging individual findings. No cross-crate / pool-boundary handling
  exists in the detector today.
- `cross-file-context-resolved` — drift that is a legitimate intentional
  difference once the caller / trait definition / test expectations are
  read (e.g. one setter in a group adds validation because a builder API
  elsewhere makes that validation mandatory before the call).
- Already-known v0 classes for reference: `boilerplate-shape-only`,
  `type-or-cfg-justified-drift`, `metadata-only-drift`,
  `auto-generated-clone`.

## comment-code (`src/detectors/comment_code.rs`, `docs/spec/comment-code-v0.md`)

- `parameter-contract-misread` — the comment states a behavioural
  constraint on a *callback / closure argument*, not on the function
  body, but the detector attributes the function-level claim (panic /
  deprecated / Result) to the body. Example: parking_lot's
  `parking_lot.rs:736` doc "the validate callback must not panic" — the
  constraint is on the caller-supplied closure; the function body does
  not panic.
- `future-work-marker` — skip TODO / FIXME / NOTE / XXX comments; they
  are future-work notes, not claims about current behaviour. (The
  detector's existing `"todo!"` handling targets the `todo!()`
  terminator macro, not prose TODO comments.)
- `doctest-divergence` — skip fenced doctest blocks (```` ```rust ````);
  an apparent mismatch there is between example code and the body, which
  run separately.
- Other known classes: `higher-abstract-intent`, `translation-ambiguity`
  (non-English comments), `stale-but-harmless`.

## unreachable-after-terminator (`src/detectors/unreachable_after_terminator.rs`, `docs/spec/unreachable-after-terminator-v0.md`)

- `runtime-conditional-divergence` — the terminator is guarded by a
  runtime condition (`if std::env::var("DEBUG").is_ok() { return; }`,
  feature-flag load, CLI flag), so the following code is reachable in the
  normal configuration; the detector treats the guard as always taken.
- Other known classes: `cfg-gated-divergence`, `macro-internal-divergence`,
  `non-divergent-loop`, `wrong-control-flow-block`.

## config-interaction (`src/detectors/config_interaction.rs`, `docs/spec/config-interaction-v0.md`)

- `non-exclusive-on-tier1` — predicates that look mutually exclusive but
  both hold on at least one tier-1 target triple (e.g. treating
  `target_os = "linux"` and `target_arch = "x86_64"` as exclusive).
- `complementary-by-design` — genuinely exclusive predicates where each
  side ships an implementation on a separate module / feature path, so no
  runtime configuration is left without an implementation (e.g.
  `#[cfg(unix)]` / `#[cfg(windows)]` for a crate that only targets unix
  and windows per `Cargo.toml`).
- `build-script-resolved` / `target-spec-mismatch-not-bug` — the cfg
  combination is resolved at build time by `build.rs`, or the predicates
  provide per-architecture implementations with no tier-1 gap.
