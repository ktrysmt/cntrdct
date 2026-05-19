# Citation policy (P1)

Every detector ships with at least one peer-reviewed citation. The
constraint is enforced structurally:

- `core::register_detector` rejects any detector whose `citations()`
  returns empty.
- `tests/citations_consistency.rs` asserts that every citation key
  resolves to an entry in
  [CITATIONS.md](https://github.com/ktrysmt/cntrdct/blob/master/CITATIONS.md).
- For multi-language detectors, each supported language requires its
  own citation grounded in empirical work on that target language
  (`docs/spec/citations-policy.md`). The existing Rust citation does
  not transfer automatically.

The CITATIONS.md file is grouped by layer (Layer 1 detectors,
Layer 2 ranker, Layer 3 adjudicator) and carries DOIs and venue
metadata so the Q-6 retraction monitor can cross-check against
Retraction Watch and Crossref Works on every CI run.

See also:
[`docs/spec/citations-policy.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/citations-policy.md).
