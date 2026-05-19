# comment-code

Flags a doc comment that claims a behaviour the implementation does
not exhibit. Three patterns are covered: Result-shape (`Returns Err`
claims without a matching error return), panic-shape (`panics if`
claims without a panic site), and deprecated-shape
(`# Deprecated` / `.. deprecated::` without the implementation actually
being deprecated or removed).

- **Rust citation:** Tan, Yuan, Krasich, Zhou SOSP 2007 (the iComment
  Pattern A / B / C taxonomy).
- **Python citation:** Unconfirmed
  (`docs/surveys/comment-code-python-2026-05.md`); P1 satisfied by the
  Rust citation as a grandfather clause for cross-language extension.
- **IEEE 1044-2009 class:** Documentation.
- **Default severity:** Warning.

The Q-14 recall audit closed at 34 / 0 / 1.00 on `comment-code`,
saturating all three Tan SOSP 2007 patterns across twenty-three
permissive-licensed upstreams.

Spec:
[`docs/spec/comment-code-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/comment-code-v0.md).
