//! cntrdct-lsp v0 — Language Server Protocol implementation.
//!
//! Phases shipped (per `docs/spec/lsp-v0.md`):
//! - Phase 1 — lifecycle methods (`initialize`, `initialized`,
//!   `shutdown`).
//! - Phase 1.b — document events
//!   (`textDocument/{didOpen,didChange,didSave,didClose}`) plus the
//!   `Finding -> Diagnostic` mapping pushed back to the editor via
//!   `textDocument/publishDiagnostics`.
//! - Phase 1.c — per-URI debouncing on `didChange` so a burst of
//!   keystrokes does not stall the editor on multi-thousand-LOC
//!   buffers.
//! - Phase 1.c+ — per-URI monotonic generation counter that gates
//!   every `publish_diagnostics` call. `JoinHandle::abort()` cannot
//!   interrupt a `spawn_blocking` scan that has already started; the
//!   generation check ensures such a stale scan drops its publish
//!   when a newer event has bumped the counter while it was running.
//!
//! Phases 2 / 3 (VS Code extension scaffolding + Marketplace) are
//! still pending.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, Location as LspLocation, MessageType,
    NumberOrString, Position, Range, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer};

use crate::core::{Detector, Finding, Severity};
use crate::detectors::arg_swap::ArgSwap;
use crate::detectors::clone_drift::CloneDrift;
use crate::detectors::comment_code::CommentCode;
use crate::detectors::lang::rust_config_interaction::ConfigInteraction;
use crate::detectors::pr_miner::PrMinerDetector;
use crate::detectors::unreachable_after_terminator::UnreachableAfterTerminator;

/// Quiet window observed before a debounced `didChange` triggers a
/// scan. Long enough to swallow a typing burst (typical inter-stroke
/// gap ~100-200 ms), short enough to stay below perceptible delay.
/// Phase 1.c hardcodes this; configurability via `cntrdct.toml` is
/// deferred. See `docs/spec/lsp-v0.md` "Debouncing".
const DIDCHANGE_DEBOUNCE: Duration = Duration::from_millis(250);

/// Shared per-URI state map. Each `Url` carries the handle of any
/// debounced didChange task (Phase 1.c) plus a monotonic generation
/// counter (Phase 1.c+). The handle gets `abort()`-ed on the next
/// event for the same URI so a still-sleeping debounce does not fire;
/// the counter gates `publish_diagnostics` so an in-flight
/// `spawn_blocking` scan whose `abort()` arrived too late to stop its
/// blocking-pool thread cannot publish stale diagnostics over the
/// editor's problems pane.
type StateMap = Arc<Mutex<HashMap<Url, UriState>>>;

#[derive(Default)]
struct UriState {
    /// Most recent debounced didChange task scheduled for this URI, if
    /// any. `did_change` / `did_save` / `did_close` all `abort()` it
    /// before scheduling their own work; `abort()` only stops tasks
    /// that have not yet exited the `tokio::time::sleep` window.
    handle: Option<JoinHandle<()>>,
    /// Monotonic per-URI generation. Every event that produces a new
    /// scan (`did_open` / `did_change` / `did_save`) or invalidates
    /// pending work (`did_close`) bumps this. A scheduled scan
    /// captures the value at scheduling time and only publishes if
    /// the captured value still equals the latest. This is the Phase
    /// 1.c+ defense against a `spawn_blocking` scan that was already
    /// past the sleep when its `JoinHandle::abort()` ran and so
    /// continued executing on the blocking pool until completion.
    latest_generation: u64,
    /// Findings from the most recent scan whose `IrFile.parse_recovered`
    /// was false — i.e. the last buffer state tree-sitter parsed
    /// cleanly. Reused when a subsequent didChange produces a recovered
    /// (mid-keystroke) tree so the editor's problems pane does not
    /// blink between every keystroke. ir-v0.md §F5 LSP non-regression
    /// requirement; eviction happens on `did_close`.
    last_clean_findings: Option<Vec<Finding>>,
}

/// The cntrdct LSP server.
///
/// Holds the tower-lsp [`Client`] handle so server methods can push
/// notifications (`window/logMessage`,
/// `textDocument/publishDiagnostics`) back to the editor, plus the
/// per-URI [`UriState`] map shared with debounced scan tasks
/// (Phase 1.c) and the generation counter (Phase 1.c+).
pub struct CntrdctLsp {
    /// tower-lsp client handle, populated by [`tower_lsp::LspService::new`].
    pub client: Client,
    /// Per-URI debounce handle + generation counter; see [`UriState`].
    state: StateMap,
}

impl CntrdctLsp {
    /// Construct a server bound to `client` with an empty per-URI state
    /// map. Used by `LspService::new` in `src/lsp_main.rs`.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for CntrdctLsp {
    async fn initialize(&self, _params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "cntrdct-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                // Full sync for v0 — the cntrdct detectors all parse
                // whole-files anyway, so incremental sync would not
                // save work. See lsp-v0.md "Lifecycle methods".
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cntrdct-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        // Bump the generation here too: a slow didOpen scan could
        // otherwise be overtaken by a fast didChange that fires before
        // the open-time scan completes, and we want the latest event
        // to win deterministically.
        let my_gen = bump_generation(&self.state, &uri).await;
        scan_and_publish_if_current(&self.client, uri, text, &self.state, my_gen).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // Under FULL sync clients send a single content change whose
        // `text` is the entire updated document. If the array is empty
        // there is nothing to do; if a client ever batches multiple
        // entries the last one is canonical (each entry replaces the
        // whole document under FULL sync).
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        self.schedule_debounced_scan(uri, change.text).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        // Cancel any pending debounced didChange scan for this URI and
        // bump the generation atomically. The bump invalidates any
        // in-flight `spawn_blocking` scan from a prior didChange whose
        // abort() arrived too late to stop the blocking-pool thread,
        // so the explicit save's publish is the one the editor sees.
        let my_gen = self.cancel_pending_and_bump(&uri).await;
        // The client may include the saved buffer text via
        // `save.includeText`; when it does we trust it. Otherwise we
        // re-read the file from disk so the diagnostics reflect the
        // newly-saved state. v0 keeps this synchronous; the spawn-
        // blocking inside `scan_and_publish_if_current` keeps the
        // event loop free.
        let text = match params.text {
            Some(t) => t,
            None => match uri.to_file_path().ok().and_then(read_file_sync) {
                Some(s) => s,
                None => return,
            },
        };
        scan_and_publish_if_current(&self.client, uri, text, &self.state, my_gen).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        // Cancel any pending debounced didChange scan and bump the
        // generation so a slow scan whose blocking-pool thread is
        // still running cannot land its publish after we clear the
        // editor's problems pane.
        self.cancel_pending_and_bump(&uri).await;
        // F5: evict the per-URI clean-findings cache too — the buffer
        // is gone, and the next `did_open` for the same URI starts a
        // fresh state.
        {
            let mut guard = self.state.lock().await;
            if let Some(entry) = guard.get_mut(&uri) {
                entry.last_clean_findings = None;
            }
        }
        // The empty publish here is unconditional — closing the buffer
        // always clears its diagnostics. Any in-flight stale scan has
        // already been invalidated by the generation bump above and
        // will drop its own publish before sending it.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }
}

impl CntrdctLsp {
    /// Schedule a debounced scan of `text` for `uri`. If a prior scan
    /// is already pending for the same URI it is aborted and replaced;
    /// only the most recent buffer state survives the quiet window
    /// (Phase 1.c). The generation counter is bumped atomically with
    /// the abort + spawn so any in-flight `spawn_blocking` scan that
    /// outran its abort is invalidated and will drop its publish
    /// (Phase 1.c+).
    async fn schedule_debounced_scan(&self, uri: Url, text: String) {
        // Hold the lock across abort() + spawn() so a parallel
        // `did_change` for the same URI cannot interleave between
        // bumping the generation and installing the new handle.
        let mut guard = self.state.lock().await;
        let entry = guard.entry(uri.clone()).or_default();
        entry.latest_generation += 1;
        let my_gen = entry.latest_generation;
        if let Some(prev) = entry.handle.take() {
            prev.abort();
        }
        let client = self.client.clone();
        let state = self.state.clone();
        let uri_for_task = uri.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(DIDCHANGE_DEBOUNCE).await;
            scan_and_publish_if_current(&client, uri_for_task, text, &state, my_gen).await;
        });
        entry.handle = Some(handle);
    }

    /// Drop any pending debounced scan for `uri` and bump the
    /// generation. Returns the new generation so the caller (e.g.
    /// `did_save`) can pass it to its own scan. Called from
    /// `did_save` and `did_close` so an explicit user action is not
    /// shadowed by a stale follow-up publish — both the still-sleeping
    /// debounce (via `abort()`) and an in-flight blocking scan (via
    /// the generation gate) are invalidated.
    async fn cancel_pending_and_bump(&self, uri: &Url) -> u64 {
        let mut guard = self.state.lock().await;
        let entry = guard.entry(uri.clone()).or_default();
        entry.latest_generation += 1;
        if let Some(prev) = entry.handle.take() {
            prev.abort();
        }
        entry.latest_generation
    }
}

/// Bump the per-URI generation counter and return the new value. Used
/// by `did_open` directly and by [`CntrdctLsp::schedule_debounced_scan`] /
/// [`CntrdctLsp::cancel_pending_and_bump`] indirectly through their own
/// guarded paths. The function is a thin wrapper so the unit tests can
/// drive the counter without standing up a `tower_lsp::Client`.
async fn bump_generation(state: &StateMap, uri: &Url) -> u64 {
    let mut guard = state.lock().await;
    let entry = guard.entry(uri.clone()).or_default();
    entry.latest_generation += 1;
    entry.latest_generation
}

/// Phase 1.c+ generation gate: returns true iff `uri`'s latest
/// generation in `state` still equals `my_gen`. Returns false for
/// unknown URIs (the `did_close` path removes nothing today, so this
/// is mostly a defensive default — any future entry-cleanup code that
/// drops a URI from the map effectively invalidates all in-flight
/// scans for it, which is the conservative outcome).
async fn is_current(state: &StateMap, uri: &Url, my_gen: u64) -> bool {
    state
        .lock()
        .await
        .get(uri)
        .is_some_and(|s| s.latest_generation == my_gen)
}

/// Run the Layer 1 detector battery against `text` for the buffer
/// identified by `uri`, then publish the resulting diagnostics — but
/// only if the per-URI generation counter still matches `my_gen` at
/// publish time. The scan itself is CPU-bound and synchronous, so it
/// is offloaded to tokio's blocking pool to keep the LSP event loop
/// responsive on multi-thousand-LOC buffers. Free function (not a
/// method) so the debouncer in `schedule_debounced_scan` can invoke
/// it from a spawned task that does not hold `&self`.
///
/// `my_gen` is captured at scheduling time and re-checked after the
/// blocking scan returns. A scan whose generation has been overtaken
/// by a fresher event (didChange / didSave / didClose) drops its
/// publish silently — error logs still fire because they describe a
/// real failure that the user wants to see regardless of staleness.
async fn scan_and_publish_if_current(
    client: &Client,
    uri: Url,
    text: String,
    state: &StateMap,
    my_gen: u64,
) {
    let Ok(path) = uri.to_file_path() else {
        // Non-file URIs (e.g. `untitled:` buffers) cannot be fed to
        // the detector battery. Publish an empty diagnostic vector to
        // keep the editor consistent with the protocol, but only if
        // we are still the current generation — otherwise a fresher
        // event has already supplanted us.
        if is_current(state, &uri, my_gen).await {
            client.publish_diagnostics(uri, Vec::new(), None).await;
        }
        return;
    };

    let scan_path = path.clone();
    let scan_result =
        tokio::task::spawn_blocking(move || crate::scan_buffer(&scan_path, text)).await;

    // F5 LSP non-regression (ir-v0.md): inspect the returned IR vector
    // to discover whether the buffer state was parse-clean. A
    // recovered tree (`parse_recovered = true`) means tree-sitter saw
    // syntax errors and every cross-cutting detector early-returned;
    // we substitute the cached findings from the last clean parse
    // rather than blanking out the editor's problems pane.
    let (findings, parse_recovered) = match scan_result {
        Ok(Ok((findings, files))) => {
            let recovered = files.iter().any(|f| f.parse_recovered);
            (findings, recovered)
        }
        Ok(Err(e)) => {
            client
                .log_message(MessageType::ERROR, format!("cntrdct scan failed: {e}"))
                .await;
            (Vec::new(), false)
        }
        Err(e) => {
            client
                .log_message(MessageType::ERROR, format!("cntrdct scan panicked: {e}"))
                .await;
            (Vec::new(), false)
        }
    };

    if !is_current(state, &uri, my_gen).await {
        return;
    }

    let publishable = if parse_recovered {
        // Mid-keystroke: keep the prior clean findings visible.
        cached_or_empty(state, &uri).await
    } else {
        // Clean parse: store the new findings as the cache for any
        // subsequent recovered tree to fall back on.
        cache_clean_findings(state, &uri, findings.clone()).await;
        findings
    };

    let diagnostics: Vec<Diagnostic> = publishable
        .iter()
        .filter(|f| f.primary.file == path)
        .map(|f| finding_to_diagnostic(&uri, f))
        .collect();
    client.publish_diagnostics(uri, diagnostics, None).await;
}

/// F5 cache lookup: return the URI's `last_clean_findings` if any,
/// else empty. Used when the current buffer state failed to parse
/// cleanly — we want the editor to keep showing the diagnostics from
/// the last good parse rather than flicker every keystroke.
async fn cached_or_empty(state: &StateMap, uri: &Url) -> Vec<Finding> {
    state
        .lock()
        .await
        .get(uri)
        .and_then(|s| s.last_clean_findings.clone())
        .unwrap_or_default()
}

/// F5 cache write: store `findings` as the URI's `last_clean_findings`
/// so a subsequent recovered-parse event can fall back to them.
async fn cache_clean_findings(state: &StateMap, uri: &Url, findings: Vec<Finding>) {
    let mut guard = state.lock().await;
    let entry = guard.entry(uri.clone()).or_default();
    entry.last_clean_findings = Some(findings);
}

fn read_file_sync(path: std::path::PathBuf) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Convert a [`Finding`] to a tower-lsp [`Diagnostic`] per the Phase
/// 1.b mapping in `docs/spec/lsp-v0.md`. The 1-based-to-0-based
/// coordinate transform is the only conversion required; cntrdct's
/// `Location` carries inclusive 1-based line/column pairs and LSP
/// `Position` is 0-based.
fn finding_to_diagnostic(buffer_uri: &Url, finding: &Finding) -> Diagnostic {
    let range = Range {
        start: Position {
            line: finding.primary.start_line.saturating_sub(1),
            character: finding.primary.start_col.saturating_sub(1),
        },
        end: Position {
            line: finding.primary.end_line.saturating_sub(1),
            character: finding.primary.end_col.saturating_sub(1),
        },
    };

    let severity = Some(match finding.raw_severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note => DiagnosticSeverity::INFORMATION,
        Severity::Info => DiagnosticSeverity::HINT,
    });

    let related_information = if finding.evidence.citation_keys.is_empty() {
        None
    } else {
        Some(
            finding
                .evidence
                .citation_keys
                .iter()
                .map(|key| DiagnosticRelatedInformation {
                    location: LspLocation {
                        uri: citation_url_for_key(key)
                            .and_then(|u| Url::parse(u).ok())
                            .unwrap_or_else(|| buffer_uri.clone()),
                        range,
                    },
                    message: format!("citation: {key}"),
                })
                .collect(),
        )
    };

    Diagnostic {
        range,
        severity,
        code: Some(NumberOrString::String(finding.detector_id.clone())),
        code_description: None,
        source: Some("cntrdct".to_string()),
        message: finding.message.clone(),
        related_information,
        tags: None,
        data: Some(finding.evidence.raw.clone()),
    }
}

/// Resolve a citation key to its registered URL, if any. Built once on
/// first call by walking every detector's static citation array; the
/// LSP-side related-info path uses this so peer-reviewed sources show
/// up as the diagnostic's link target rather than the buffer URI.
/// Citations without a `url` (DOI-only, IEEE 1044-2009, etc.) fall
/// through to the buffer URI in `finding_to_diagnostic`.
fn citation_url_for_key(key: &str) -> Option<&'static str> {
    static REGISTRY: OnceLock<HashMap<&'static str, Option<&'static str>>> = OnceLock::new();
    let map = REGISTRY.get_or_init(|| {
        let detectors: Vec<Box<dyn Detector>> = vec![
            Box::new(CloneDrift::new()),
            Box::new(ArgSwap::new()),
            Box::new(CommentCode::new()),
            Box::new(ConfigInteraction::new()),
            Box::new(UnreachableAfterTerminator::new()),
            Box::new(PrMinerDetector::new()),
        ];
        let mut out: HashMap<&'static str, Option<&'static str>> = HashMap::new();
        for d in detectors {
            for cite in d.citations() {
                out.entry(cite.key).or_insert(cite.url);
            }
        }
        out
    });
    map.get(key).copied().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        AnomalyClass, Evidence, Finding, LanguageCitationStatus, Location as CntrdctLocation,
        Severity,
    };
    use std::path::PathBuf;

    fn make_finding(detector_id: &str, severity: Severity) -> Finding {
        Finding {
            detector_id: detector_id.to_string(),
            primary: CntrdctLocation {
                file: PathBuf::from("/tmp/a.rs"),
                start_line: 3,
                start_col: 5,
                end_line: 3,
                end_col: 17,
            },
            related: vec![],
            message: "drifted clone of sibling".to_string(),
            raw_severity: severity,
            anomaly_class: AnomalyClass::Logic,
            evidence: Evidence {
                citation_keys: vec!["cordy-roy-icpc-2008"],
                raw: serde_json::json!({"distance": 0.42}),
                language_citation_status: LanguageCitationStatus::Confirmed,
            },
        }
    }

    fn buffer_uri() -> Url {
        Url::parse("file:///tmp/a.rs").expect("parse file url")
    }

    #[test]
    fn one_based_to_zero_based_range_conversion() {
        let d = finding_to_diagnostic(
            &buffer_uri(),
            &make_finding("clone-drift", Severity::Warning),
        );
        assert_eq!(d.range.start.line, 2, "line 3 (1-based) -> 2 (0-based)");
        assert_eq!(d.range.start.character, 4, "col 5 (1-based) -> 4 (0-based)");
        assert_eq!(d.range.end.line, 2);
        assert_eq!(d.range.end.character, 16);
    }

    #[test]
    fn severity_mapping_covers_every_variant() {
        let cases = [
            (Severity::Error, DiagnosticSeverity::ERROR),
            (Severity::Warning, DiagnosticSeverity::WARNING),
            (Severity::Note, DiagnosticSeverity::INFORMATION),
            (Severity::Info, DiagnosticSeverity::HINT),
        ];
        for (raw, expected) in cases {
            let d = finding_to_diagnostic(&buffer_uri(), &make_finding("d", raw));
            assert_eq!(d.severity, Some(expected));
        }
    }

    #[test]
    fn detector_id_lands_in_code_and_source_is_cntrdct() {
        let d = finding_to_diagnostic(
            &buffer_uri(),
            &make_finding("clone-drift", Severity::Warning),
        );
        assert_eq!(
            d.code,
            Some(NumberOrString::String("clone-drift".to_string()))
        );
        assert_eq!(d.source.as_deref(), Some("cntrdct"));
    }

    #[test]
    fn evidence_raw_round_trips_into_data() {
        let d = finding_to_diagnostic(
            &buffer_uri(),
            &make_finding("clone-drift", Severity::Warning),
        );
        assert_eq!(d.data, Some(serde_json::json!({"distance": 0.42})));
    }

    #[test]
    fn related_information_is_one_per_citation_key() {
        let mut f = make_finding("clone-drift", Severity::Warning);
        f.evidence.citation_keys = vec!["cordy-roy-icpc-2008", "ieee-1044-2009"];
        let d = finding_to_diagnostic(&buffer_uri(), &f);
        let related = d.related_information.expect("related info present");
        assert_eq!(related.len(), 2);
        assert!(related[0].message.contains("cordy-roy-icpc-2008"));
        assert!(related[1].message.contains("ieee-1044-2009"));
    }

    #[test]
    fn related_information_resolves_known_citation_url_when_available() {
        // `cordy-roy-icpc-2008` is registered with a URL on
        // `CloneDrift::citations()`; the related-info location should
        // therefore point at the citation URL rather than the buffer
        // URI. If the key were unknown, the location URI would fall
        // back to the buffer URI.
        let d = finding_to_diagnostic(
            &buffer_uri(),
            &make_finding("clone-drift", Severity::Warning),
        );
        let related = d.related_information.expect("related info present");
        assert_eq!(related.len(), 1);
        assert_ne!(
            related[0].location.uri,
            buffer_uri(),
            "expected URL fallout from citation registry, not buffer URI"
        );
    }

    #[test]
    fn related_information_falls_back_to_buffer_uri_for_unknown_key() {
        let mut f = make_finding("clone-drift", Severity::Warning);
        f.evidence.citation_keys = vec!["zz-totally-not-registered-2099"];
        let d = finding_to_diagnostic(&buffer_uri(), &f);
        let related = d.related_information.expect("related info present");
        assert_eq!(related[0].location.uri, buffer_uri());
    }

    // ---------- Phase 1.c+ generation counter unit tests ----------

    fn empty_state() -> StateMap {
        Arc::new(Mutex::new(HashMap::new()))
    }

    #[tokio::test]
    async fn bump_generation_starts_at_one_and_increases_monotonically() {
        let state = empty_state();
        let uri = buffer_uri();
        assert_eq!(bump_generation(&state, &uri).await, 1);
        assert_eq!(bump_generation(&state, &uri).await, 2);
        assert_eq!(bump_generation(&state, &uri).await, 3);
    }

    #[tokio::test]
    async fn bump_generation_is_per_uri() {
        let state = empty_state();
        let a = Url::parse("file:///tmp/a.rs").unwrap();
        let b = Url::parse("file:///tmp/b.rs").unwrap();
        assert_eq!(bump_generation(&state, &a).await, 1);
        assert_eq!(bump_generation(&state, &b).await, 1);
        assert_eq!(bump_generation(&state, &a).await, 2);
        assert_eq!(bump_generation(&state, &b).await, 2);
    }

    #[tokio::test]
    async fn is_current_matches_only_the_latest_generation() {
        let state = empty_state();
        let uri = buffer_uri();
        let g1 = bump_generation(&state, &uri).await;
        assert!(
            is_current(&state, &uri, g1).await,
            "freshly-issued generation must be current"
        );
        let g2 = bump_generation(&state, &uri).await;
        assert!(
            !is_current(&state, &uri, g1).await,
            "stale generation must not be current after a fresh bump"
        );
        assert!(
            is_current(&state, &uri, g2).await,
            "newest generation must be current"
        );
    }

    #[tokio::test]
    async fn is_current_returns_false_for_unknown_uri() {
        let state = empty_state();
        let uri = Url::parse("file:///tmp/never-touched.rs").unwrap();
        assert!(!is_current(&state, &uri, 0).await);
        assert!(!is_current(&state, &uri, 42).await);
    }

    // ---------- F5 last-clean-findings cache ----------

    #[tokio::test]
    async fn cache_clean_findings_round_trips_per_uri() {
        let state = empty_state();
        let uri = buffer_uri();
        // Empty cache => no findings.
        assert!(cached_or_empty(&state, &uri).await.is_empty());

        let f = vec![make_finding("clone-drift", Severity::Warning)];
        cache_clean_findings(&state, &uri, f.clone()).await;

        let got = cached_or_empty(&state, &uri).await;
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].detector_id, "clone-drift");
    }

    #[tokio::test]
    async fn cache_clean_findings_is_per_uri() {
        let state = empty_state();
        let a = Url::parse("file:///tmp/a.rs").unwrap();
        let b = Url::parse("file:///tmp/b.rs").unwrap();

        cache_clean_findings(
            &state,
            &a,
            vec![make_finding("arg-swap", Severity::Warning)],
        )
        .await;
        // b URI has no cache entry; lookup must not see a's findings.
        assert!(cached_or_empty(&state, &b).await.is_empty());
        // a's cache is intact.
        let got_a = cached_or_empty(&state, &a).await;
        assert_eq!(got_a.len(), 1);
        assert_eq!(got_a[0].detector_id, "arg-swap");
    }
}
