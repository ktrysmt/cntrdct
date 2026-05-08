//! cntrdct-lsp v0 — Language Server Protocol implementation.
//!
//! Phases shipped (per `docs/spec/lsp-v0.md`):
//! - Phase 1 — lifecycle methods (`initialize`, `initialized`,
//!   `shutdown`).
//! - Phase 1.b — document events
//!   (`textDocument/{didOpen,didChange,didSave,didClose}`) plus the
//!   `Finding -> Diagnostic` mapping pushed back to the editor via
//!   `textDocument/publishDiagnostics`.
//!
//! Phase 1.c (debouncing on `didChange`) and Phases 2 / 3 (VS Code
//! extension scaffolding + Marketplace) are still pending.

use std::collections::HashMap;
use std::sync::OnceLock;

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
use crate::detectors::config_interaction::ConfigInteraction;
use crate::detectors::pr_miner::PrMinerDetector;
use crate::detectors::unreachable_after_terminator::UnreachableAfterTerminator;

/// The cntrdct LSP server.
///
/// Holds the tower-lsp [`Client`] handle so server methods can push
/// notifications (`window/logMessage`,
/// `textDocument/publishDiagnostics`) back to the editor.
pub struct CntrdctLsp {
    /// tower-lsp client handle, populated by [`tower_lsp::LspService::new`].
    pub client: Client,
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
        self.scan_and_publish(uri, text).await;
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
        self.scan_and_publish(uri, change.text).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        // The client may include the saved buffer text via
        // `save.includeText`; when it does we trust it. Otherwise we
        // re-read the file from disk so the diagnostics reflect the
        // newly-saved state. v0 keeps this synchronous; the spawn-
        // blocking inside `scan_and_publish` keeps the event loop free.
        let text = match params.text {
            Some(t) => t,
            None => match uri.to_file_path().ok().and_then(read_file_sync) {
                Some(s) => s,
                None => return,
            },
        };
        self.scan_and_publish(uri, text).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Clear the editor's problems pane for this URI. Without this
        // the diagnostics from the last `publishDiagnostics` would stay
        // pinned to a buffer the editor no longer has open.
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }
}

impl CntrdctLsp {
    /// Run the Layer 1 detector battery against `text` for the buffer
    /// identified by `uri`, then publish the resulting diagnostics.
    /// The scan itself is CPU-bound and synchronous, so it is offloaded
    /// to tokio's blocking pool to keep the LSP event loop responsive
    /// on multi-thousand-LOC buffers.
    async fn scan_and_publish(&self, uri: Url, text: String) {
        let Ok(path) = uri.to_file_path() else {
            // Non-file URIs (e.g. `untitled:` buffers) cannot be fed to
            // the detector battery, but publishing an empty diagnostic
            // vector keeps the editor consistent with the protocol.
            self.client.publish_diagnostics(uri, Vec::new(), None).await;
            return;
        };

        let scan_path = path.clone();
        let scan_result =
            tokio::task::spawn_blocking(move || crate::scan_buffer(&scan_path, text)).await;

        let findings = match scan_result {
            Ok(Ok((findings, _parsed))) => findings,
            Ok(Err(e)) => {
                self.client
                    .log_message(MessageType::ERROR, format!("cntrdct scan failed: {e}"))
                    .await;
                Vec::new()
            }
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, format!("cntrdct scan panicked: {e}"))
                    .await;
                Vec::new()
            }
        };

        let diagnostics: Vec<Diagnostic> = findings
            .iter()
            .filter(|f| f.primary.file == path)
            .map(|f| finding_to_diagnostic(&uri, f))
            .collect();
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
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
}
