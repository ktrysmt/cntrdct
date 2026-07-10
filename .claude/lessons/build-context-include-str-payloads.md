# Compile-time embedded payloads live outside src/ — keep them in every build context

The cntrdct build embeds three files at compile time via `include_str!`:

- `benchmarks/priors-default.json` (src/lib.rs)
- `benchmarks/llm-calibration/platt-default.json` (src/lib.rs)
- `data/python-builtin-exceptions.json` (src/detectors/lang/python_unreachable_except.rs)

Any mechanism that filters the build context (`.dockerignore`, `Cargo.toml`
`exclude`, CI sparse checkouts) MUST keep all three, or `cargo build` fails
with "couldn't read ..." at the `include_str!` site.

- VERIFIED (2026-07-10): a `.dockerignore` excluding `data/` broke the Docker
  build exactly this way; fixed with `data` + `!data/python-builtin-exceptions.json`.
- To re-derive the current list: `grep -rn "include_str!\|include_bytes!" src/`.

Related pitfall: validating `docker build` (or any command) through a pipe
(`docker build ... | tail`) masks the exit code — check `PIPESTATUS[0]` or
redirect to a file and test `$?` directly.
