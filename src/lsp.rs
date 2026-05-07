//! cntrdct-lsp v0 — Language Server Protocol skeleton.
//!
//! Phase 1 scaffolding for ROADMAP T3-12. This module ships the
//! lifecycle methods (`initialize`, `initialized`, `shutdown`) so the
//! binary can be wired into editors today; the document-event side
//! (`didOpen` / `didChange` / `didSave` / `didClose`) and the
//! `Finding -> Diagnostic` mapping land in phase 1.b.
//!
//! See `docs/spec/lsp-v0.md` for the full v0 surface and the phase plan.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// The cntrdct LSP server.
///
/// Holds the tower-lsp `Client` handle so server methods can push
/// notifications (`window/logMessage`, `textDocument/publishDiagnostics`)
/// back to the editor.
pub struct CntrdctLsp {
    pub client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for CntrdctLsp {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
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

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
