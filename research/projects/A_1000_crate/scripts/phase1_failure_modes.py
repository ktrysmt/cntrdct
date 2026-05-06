"""Controlled vocabulary for Phase 1 FP failure modes.

Defined in ``failure-modes-v1.md`` section 3. Keys are detector_id
strings as they appear in ``phase1-labels.csv``; values are
frozensets of permitted ``failure_mode`` labels for that detector.

The literals ``"other"`` and ``""`` are always accepted by the
aggregator and are NOT included in this mapping; consumers should
short-circuit on those before consulting the vocabulary.

Cross-detector shared modes (currently only
``cross-file-context-resolved``, shared between ``clone-drift`` and
``arg-swap``) are listed under each detector that admits them. The
sharing is preserved at the data level rather than as a separate
"shared" set so per-detector validation stays a single membership
test.
"""

from __future__ import annotations

FAILURE_MODES_BY_DETECTOR: dict[str, frozenset[str]] = {
    "clone-drift": frozenset(
        {
            "boilerplate-shape-only",
            "type-or-cfg-justified-drift",
            "metadata-only-drift",
            "auto-generated-clone",
            "cross-file-context-resolved",
            "cross-crate-pool-mismatch",
        }
    ),
    "arg-swap": frozenset(
        {
            "type-distinct-positions",
            "commutative-callee",
            "builder-positional-convention",
            "cross-file-context-resolved",
        }
    ),
    "comment-code": frozenset(
        {
            "higher-abstract-intent",
            "future-work-marker",
            "doctest-divergence",
            "translation-ambiguity",
            "stale-but-harmless",
            "parameter-contract-misread",
        }
    ),
    "unreachable-after-terminator": frozenset(
        {
            "cfg-gated-divergence",
            "macro-internal-divergence",
            "non-divergent-loop",
            "wrong-control-flow-block",
            "runtime-conditional-divergence",
        }
    ),
    "config-interaction": frozenset(
        {
            "non-exclusive-on-tier1",
            "complementary-by-design",
            "build-script-resolved",
            "target-spec-mismatch-not-bug",
        }
    ),
}
