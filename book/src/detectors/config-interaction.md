# config-interaction

Flags a top-level Rust item that bears two `#[cfg(...)]` attributes
whose predicates are structurally negations of each other (so the item
is unreachable under any feature configuration).

- **Citation:** Medeiros, Ribeiro, Gheyi, Apel, Kästner, Ferreira,
  Carvalho, Fonseca ICSE 2018 (configuration-interaction faults in
  C preprocessor systems).
- **IEEE 1044-2009 class:** Logic.
- **Default severity:** Warning.

Rust-only at v0. Python's `if sys.version_info` / typing-style guards
are out of scope (no semantic analogue to `#[cfg]`).

Spec:
[`docs/spec/config-interaction-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/config-interaction-v0.md).
