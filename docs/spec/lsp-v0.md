# cntrdct-lsp v0 — Language Server for the cntrdct linter

Status: phase 1 + phase 1.b + phase 1.c + phase 1.c+ shipped
(lifecycle methods, document events, the Finding -> Diagnostic
mapping, per-URI didChange debouncing, and the per-URI generation
counter that gates `publish_diagnostics` against in-flight
`spawn_blocking` scans all landed under feature flag `lsp`); phases
2 / 3 still pending.

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

## Document events (phase 1.b — landed)

| LSP method | v0.b behaviour |
| --- | --- |
| `textDocument/didOpen` | Run [`crate::scan_buffer`] on the opened buffer; publish `textDocument/publishDiagnostics` immediately (not debounced). |
| `textDocument/didChange` | Schedule a debounced scan (Phase 1.c, see below). Each new change for the same URI within the quiet window aborts the prior scheduled scan. |
| `textDocument/didSave` | Cancel any pending debounced didChange scan for this URI, then re-scan and republish. When the client omits the saved text the server reads from disk via `Url::to_file_path()`. |
| `textDocument/didClose` | Cancel any pending debounced didChange scan for this URI, then publish an empty diagnostic vector to clear stale findings. |

Implementation note: the buffer scan does not call
`scan_full_with_config` (which walks the disk and reads file contents).
Instead, [`crate::scan_buffer`] feeds the editor-supplied text into a
one-file `DetectContext` and runs the same Layer 1 detector battery
that disk-walking callers use, sharing the registration list through
`run_detectors_on`. This keeps the LSP path independent of disk state
on `didOpen` / `didChange` (where the editor's buffer is the source
of truth) while still letting `didSave` fall back to disk when the
client elects not to include the saved text. Detection itself is
CPU-bound, so `scan_and_publish` runs the scan on `tokio::task::spawn_blocking`
to keep the event loop free.

## Debouncing (phase 1.c — landed)

`textDocument/didChange` does not scan inline. The handler clones the
client, captures the new buffer text, and `tokio::spawn`s a task that
sleeps for the quiet window (`DIDCHANGE_DEBOUNCE = 250 ms` in
`src/lsp.rs`) before invoking the same `scan_and_publish` path that
`didOpen` and `didSave` use. The handle is stored in a per-URI
`Arc<tokio::sync::Mutex<HashMap<Url, JoinHandle<()>>>>`; a follow-up
`didChange` for the same URI calls `JoinHandle::abort()` on the prior
handle and replaces it, so only the most recent buffer state survives
the window.

The window value (250 ms) is hardcoded for v0:

- Long enough to swallow a typing burst (typical inter-keystroke gap
  ~100-200 ms).
- Short enough to stay below perceptible diagnostic-update delay.
- Configurability via `cntrdct.toml` is deferred until external usage
  signals a need.

`didOpen` is *not* debounced: the first scan must be immediate so the
editor's problems pane reflects the opened buffer without a perceptible
gap. `didSave` and `didClose` actively drain the pending map for their
URI before acting, so a stale scan scheduled by an earlier `didChange`
cannot land its publish *after* the explicit user action.

## Generation counter (phase 1.c+ — landed)

`JoinHandle::abort()` cannot interrupt a `spawn_blocking` task that has
already started executing the detector pass; the OS thread keeps
running until it returns, and its publish (if reached) would otherwise
land after the replacement scan's publish. Phase 1.c+ closes this race
by extending the per-URI map from `JoinHandle<()>` to a `UriState`
record carrying both the handle and a monotonic `latest_generation:
u64` counter (`src/lsp.rs`). Every event that produces a new scan
(`didOpen` / `didChange` / `didSave`) or invalidates pending work
(`didClose`) bumps the counter, and each scheduled scan captures the
value at scheduling time. After `spawn_blocking` returns, the scan
re-locks the state, compares its captured generation against the
latest, and drops its publish silently if they no longer match.

The cancel + bump in `did_save` and `did_close` is atomic (a single
locked critical section) so there is no window in which an in-flight
stale scan can read its own generation as still-current. Error logs
(`window/logMessage`) still fire from a stale scan because they
describe a real scan failure that the user wants to see regardless of
freshness; only the diagnostic publish is gated.

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
   Landed.
2. Phase 1.b — document events + Finding -> Diagnostic mapping.
   Landed.
3. Phase 1.c — debouncing on didChange. Landed.
4. Phase 1.c+ — per-URI generation counter gating `publish_diagnostics`
   against late-arriving stale `spawn_blocking` scans. Landed.
5. Phase 2 prereq — `cntrdct-lsp` binary shipped in the release
   artefacts. Landed: `.github/workflows/release.yml`'s `build` matrix
   now compiles each target with `--features lsp` so the
   `required-features = ["lsp"]` gate on the third binary is
   satisfied, and the per-target tar.gz / zip stage directory carries
   `cntrdct-lsp` (or `cntrdct-lsp.exe`) alongside the existing
   `cntrdct` and `cargo-cntrdct`. The lsp module itself stays
   `#[cfg(feature = "lsp")]`-gated so the regular CLI binaries are
   unaffected by the feature toggle. Cross-compilation under
   `cross build` for `aarch64-unknown-linux-gnu` is exercised in the
   same step as the native build; first end-to-end confirmation
   arrives with the next tag push.
6. Phase 2 — `vscode-cntrdct` extension scaffolding (TypeScript / pnpm,
   bundled with the LSP binary auto-downloaded from GitHub Releases).
   Pending.
7. Phase 3 — Marketplace listing. Pending.

Phases 1 + 1.b + 1.c + 1.c+ together are the minimum to advertise the
LSP as "ships diagnostics inline without choking on rapid typing and
without races on cancellation"; phase 2 delivers the production-
quality VS Code experience; phase 3 is the public release.

## Testing

- `tests/lsp_smoke.rs` spawns the actual `cntrdct-lsp` binary
  (`CARGO_BIN_EXE_cntrdct-lsp`) and drives its stdin/stdout with a
  hand-rolled JSON-RPC framing client. It asserts the
  `Initialize` -> `initialized` -> `didOpen` -> `publishDiagnostics`
  -> `Shutdown` round-trip, including that the published diagnostic
  carries `source = "cntrdct"`, `code = "<detector_id>"`, and
  severity 2 (Warning) for the unreachable-after-terminator finding.
- The `lsp::tests` unit module under `src/lsp.rs` covers the
  Finding -> Diagnostic mapping table directly: severity mapping for
  every `Severity` variant, 1-based-to-0-based range conversion,
  detector_id flowing into `code`, `source = "cntrdct"`,
  `evidence.raw` round-tripping into `data`, and the
  `relatedInformation` shape (one entry per citation key, URL
  fallout from the static citation registry, fallback to buffer URI
  for unknown keys).
- Phase 1.c adds a second smoke test
  `did_change_debounces_rapid_bursts_to_one_publish` to the same
  file. It fires three `didChange` notifications inside the 250 ms
  quiet window (only the last carries finding-producing text), waits
  700 ms, then drains the LSP wire and asserts exactly one
  `publishDiagnostics` survived — and that it carries the *last*
  buffer state (one `unreachable-after-terminator` finding). A reader
  thread + mpsc channel underpins the assertion so the test can ask
  "did *no* further frame arrive in the next 700 ms", which the
  Phase 1.b sync-read helpers cannot answer.
- Phase 1.c+ adds four unit tests in `lsp::tests` covering the
  generation counter primitives directly (no `tower_lsp::Client`
  required): `bump_generation` starts at 1 and increases monotonically,
  is per-URI, `is_current` matches only the latest generation, and
  returns false for unknown URIs. The race the counter defends against
  (a `spawn_blocking` scan whose abort lost to its blocking-pool
  thread) is awkward to force deterministically without a test seam,
  so the unit-level proof of the gate is the load-bearing assertion;
  the smoke test continues to validate end-to-end debounce behaviour.
- Both smoke and unit suites run only with `--features lsp` enabled;
  CI wires `cargo test --features lsp --test lsp_smoke` and
  `cargo test --features lsp --lib lsp::tests` as separate steps so
  a future regression in either path fails CI.
