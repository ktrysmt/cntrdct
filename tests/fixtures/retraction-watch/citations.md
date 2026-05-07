# Retraction-monitor fixture (Q-6)

This file is a deliberately tainted CITATIONS.md analogue used by the
`citations.yml` workflow's smoke test. It cites one synthetic DOI that
the sibling `cache.csv` lists as retracted; the workflow asserts that
`scripts/check_retractions.py` exits non-zero on this fixture, pinning
the failure path against future regressions.

The DOI prefix `10.99999/` is unassigned, so the synthetic record
cannot collide with a real publication.

## Layer 1 (Deterministic detectors)

### fixture-retracted

- `fixture-retracted-2026` — Synthetic Authors, "Fixture Retracted Paper",
  FIXTURE 2026.
  DOI 10.99999/cntrdct-q6-retracted-fixture
  Languages: (general; fixture only)
