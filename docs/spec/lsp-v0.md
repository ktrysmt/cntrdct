# cntrdct-lsp v0 — Language Server for the cntrdct linter

Status: phase 1 scaffolding (skeleton landing under feature flag `lsp`).

This spec scopes the v0 surface of `cntrdct-lsp`, a Language Server Protocol
implementation that exposes cntrdct's findings to LSP-speaking editors
(VS Code, Helix, Neovim, JetBrains via the LSP4IJ plugin, …). It pairs with
ROADMAP T3-12 and is the upstream contract that any client (e.g. the
forthcoming `vscode-cntrdct` extension) renders against.

## Wire format

JSON-RPC over stdio, per
[LSP 3.17](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/).
The server is a single binary `cntrdct-lsp`, gated by the `lsp` Cargo
feature so users who only want the CLI do not pay the tokio + tower-lsp
build cost.

## Lifecycle methods

| LSP method | v0 behaviour |
| --- | --- |
| `initialize` | Returns `ServerInfo { name: "cntrdct-lsp", version: <crate version> }` and `ServerCapabilities { text_document_sync: Full }`. No workspace-folder support yet. |
| `initialized` | Logs "cntrdct-lsp ready" via `window/logMessage` at `INFO`. |
| `shutdown` | Returns `Ok(())`. Idempotent. |
| `exit` | Provided by tower-lsp's runtime; no override. |

`textDocumentSync = Full` is deliberate for v0: incremental sync requires
re-tokenising change ranges, and the cntrdct detectors all parse
whole-files anyway. We can graduate to `Incremental` if profiling shows
the full-document re-parse is a hot path.

## Document events (deferred to phase 1.b)

The following methods are part of phase 1 but land in a follow-up commit
once the `Finding -> Diagnostic` mapping (below) is wired in:

| LSP method | v0.b behaviour |
| --- | --- |
| `textDocument/didOpen` | Run `scan_full_with_config` on the opened buffer; publish `textDocument/publishDiagnostics`. |
| `textDocument/didChange` | Re-scan on every change for v0; debouncing is Phase 1.c. |
| `textDocument/didSave` | Re-scan and republish (idempotent with didChange). |
| `textDocument/didClose` | Publish an empty diagnostic vector to clear stale findings. |

## Finding -> Diagnostic mapping

| cntrdct field | LSP `Diagnostic` field | Notes |
| --- | --- | --- |
| `Finding.severity` | `severity` | `Error -> Error`, `Warning -> Warning`, `Note -> Information`, `Info -> Hint`. |
| `Finding.detector_id` | `code` (string variant) | Matches the SARIF `ruleId`. |
| `Finding.message` | `message` | Plain text; multi-line allowed. |
| `Finding.primary.location` | `range` | LSP `Position` is 0-indexed; cntrdct's `Location` is 1-indexed. The 1-based-to-0-based conversion is the only conversion needed. |
| `Finding.evidence.citation_keys` | `relatedInformation[]` | One related info entry per citation, with the citation URL as the location URI placeholder. (Phase 1.b — link-to-paper UX is a stretch goal.) |
| `Finding.evidence.raw` | `data` | JSON value, opaque to the editor. Useful for code-action providers (Phase 2). |

`source` on every `Diagnostic` is the literal string `"cntrdct"` so users
can filter the editor's problems pane.

## Out of scope (v0)

These are explicitly deferred so v0 has a clean shipping boundary; each
will get its own follow-up entry once v0 is in users' hands.

- `textDocument/codeAction` — suggested fixes (e.g. "remove the
  unreachable statement"). Requires a per-detector edit generator.
- `textDocument/hover` — the citation tooltip pattern. Useful but adds a
  whole separate "given a position, find the nearest finding" loop.
- `initializationOptions` and `workspace/configuration` — reading
  `cntrdct.toml` from initialization params. v0 uses the on-disk config
  in the workspace root, same path as `cntrdct scan`.
- `workspace/didChangeWatchedFiles` — re-scan on dependency changes.
- Per-language edition of the file system walker. v0 only inspects the
  buffer the editor sent us, not its imports.

## Compatibility envelope

- LSP protocol 3.17 (`tower-lsp` 0.20 series).
- Tokio multi-threaded runtime; opted-in via the `lsp` Cargo feature.
- Editor expectations: any editor that ships an LSP client capable of
  Full-text-sync diagnostics. VS Code, Helix, Neovim's built-in LSP, and
  Emacs lsp-mode all qualify.

## Phase plan (T3-12)

1. Phase 1 — server scaffolding behind `lsp` feature flag (this spec +
   skeleton implementation: Initialize / Initialized / Shutdown).
2. Phase 1.b — document events + Finding -> Diagnostic mapping.
3. Phase 1.c — debouncing on didChange.
4. Phase 2 — `vscode-cntrdct` extension scaffolding (TypeScript / pnpm,
   bundled with the LSP binary auto-downloaded from GitHub Releases).
5. Phase 3 — Marketplace listing.

Phases 1 and 1.b together are the minimum to advertise the LSP as
"ships diagnostics inline"; phases 1.c and 2 deliver the
production-quality VS Code experience; phase 3 is the public release.

## Testing

- `tests/lsp_smoke.rs` (lands with phase 1.b) drives the server's
  stdin/stdout with a small JSON-RPC client and asserts the
  `Initialize` -> `Initialized` -> `Shutdown` round-trip.
- The skeleton in phase 1 ships without tests; the smoke test is the
  acceptance gate for the next commit.
