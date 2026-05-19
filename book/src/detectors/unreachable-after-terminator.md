# unreachable-after-terminator

Flags a statement that follows an unconditional terminator within the
same block:

- Rust: `return`, `panic!()`, `unreachable!()`, `todo!()`, `break`,
  `continue`.
- Python: `raise`, `sys.exit()`, `os._exit()`, `assert False`,
  trailing `return`.

The detector cfg-gates the terminator suppression (F4b) so an item
hoisted into a `#[cfg(...)]` block does not trigger a spurious finding
for the matching unreachable branch (F4c).

- **Rust citations:** Aho, Lam, Sethi, Ullman "Compilers" (Ch. 9 dead
  code) + Engler, Chen, Hallem, Chou, Chelf OSDI 2001
  (grandfathered).
- **Python citation:** Unconfirmed
  (`docs/surveys/unreachable-after-terminator-python-2026-05.md`);
  P1 satisfied by the two grandfathered Rust citations.
- **IEEE 1044-2009 class:** Logic.
- **Default severity:** Warning.

Spec:
[`docs/spec/unreachable-after-terminator-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/unreachable-after-terminator-v0.md).
