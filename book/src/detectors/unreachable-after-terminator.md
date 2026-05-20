# unreachable-after-terminator

Flags a statement that follows an unconditional terminator within the
same block. The contradiction is between the program's static
control-flow graph and the textual layout of the source.

| Property | Value |
|---|---|
| Detector ID | `unreachable-after-terminator` |
| Languages | Rust, Python |
| IEEE 1044-2009 class | Logic |
| Default severity | Warning |
| Spec | [`docs/spec/unreachable-after-terminator-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/unreachable-after-terminator-v0.md) |

## Terminator sets

| Language | Terminators |
|---|---|
| Rust | `return`, `break`, `continue`, plus macro terminators `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `abort!`, `exit!` |
| Python | `raise`, `sys.exit(...)`, `os._exit(...)`, `assert False`, trailing `return` |

## What it flags

```rust
fn lookup(name: &str) -> Option<u32> {
    return cache_lookup(name);
    log::info!("lookup {}", name);   // <- flagged
}
```

For each block, the detector classifies statements as terminators or
non-terminators, walks the block in order, and emits one Finding
pointing at the *first* statement after a terminator. Subsequent
statements in the same block are silent to avoid duplicate noise on
a single contradiction site (`evidence.raw.following_count` records
how many statements were rendered unreachable).

## What it does not flag (by design)

The wild β corpus exposed two systematic false-positive patterns that
the spec closes via F4b and F4c:

- F4b — cfg-gated terminators. The canonical cross-platform return
  idiom

  ```rust
  #[cfg(feature = "x")]
  return foo();
  #[cfg(not(feature = "x"))]
  return bar();
  ```

  has exactly one terminator active per cfg evaluation; treating
  them as sequential statements produced 10 / 10 false positives on
  the wild Rust β corpus. F4b suppresses the terminator side; a
  terminator followed by a cfg-gated *follower* still fires (T34).
- F4c — hoisted item suppression. `fn` / `const` / `static` / `use`
  / `struct` / `enum` / `type` / `mod` / `impl` / `trait` items are
  hoisted by the Rust compiler; their textual position carries no
  runtime ordering. They are filtered out of the block's statement
  list before terminator analysis runs. An executable statement
  *after* the items still fires (T37).

Additional non-goals:

- Inter-procedural reachability (a helper that always panics).
- Branch-merging analysis (`if cond { return 1 } else { return 2 }
  bar();` requires dataflow, out of v0 scope).
- `loop { ... }` without `break`.
- `match` arms whose every branch diverges.

## Language coverage

- Rust: grandfathered citations. Hovemeyer & Pugh OOPSLA 2004 defines
  the "UR — Unreachable code" bug pattern; Engler et al. SOSP 2001
  establishes the broader "Bugs as Deviant Behavior" framing.
- Python: unconfirmed citation per
  [`docs/surveys/unreachable-after-terminator-python-2026-05.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/surveys/unreachable-after-terminator-python-2026-05.md);
  P1 is satisfied by the two grandfathered Rust citations.

## Interaction with `#[allow(unreachable_code)]`

The detector treats `#[allow(unreachable_code)]` on any ancestor
function or block (and the `#![allow(unreachable_code)]` inner-
attribute form) as a suppression. The match is by textual substring
on the attribute source — robust attribute parsing is out of v0
scope and unnecessary for the high-precision common case.

## Configuration

```toml
[detectors.unreachable-after-terminator]
enabled = true
severity = "Warning"
```

In-source:

- Rust: `#[cntrdct::allow(unreachable-after-terminator)]` on the
  containing function, or the bare `#[allow(unreachable_code)]` form
  which the detector already respects.
- Python: `# cntrdct: allow(unreachable-after-terminator)` (Q-9).

## Related

- [config-interaction](./config-interaction.md) — the cfg attribute
  family interacts with both detectors from opposite sides.
- [Severity and anomaly classes (P5)](../concepts/severity.md) —
  Logic anomaly class.
