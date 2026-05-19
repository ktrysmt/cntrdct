# Language Server (LSP)

`cntrdct-lsp` exposes findings to IDEs via the Language Server
Protocol. The binary ships behind the optional `lsp` Cargo feature:

```sh
cargo install cntrdct --features lsp
```

## Phase status (T3-12)

- Phase 1 — server scaffolding, lifecycle methods, log routing.
  Landed 2026-05-08.
- Phase 1.b — `textDocument/{didOpen,didChange,didSave,didClose}` →
  `textDocument/publishDiagnostics`. Severity, code, source, message,
  range, `relatedInformation`, and `data` mapping per the spec.
  Landed 2026-05-08.
- Phase 1.c — per-URI 250 ms debouncing on `didChange`. Landed
  2026-05-09.
- Phase 1.c+ — per-URI generation counter so a fresher event overtaking
  a still-running scan suppresses the stale publish. Landed 2026-05-09.
- Phase 2 — `vscode-cntrdct` extension scaffolding (TypeScript / pnpm),
  bundling the LSP binary auto-downloaded from GitHub Releases. Pending.
- Phase 3 — VS Code Marketplace listing + announcement. Pending.

Spec:
[`docs/spec/lsp-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/lsp-v0.md).
