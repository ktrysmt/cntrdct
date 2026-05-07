//! `cntrdct-lsp` binary entry point.
//!
//! Compiled only when the `lsp` Cargo feature is enabled; gated by
//! `required-features = ["lsp"]` in the `[[bin]]` declaration so that
//! a default `cargo install cntrdct` does not pay the tokio + tower-lsp
//! build cost. See `docs/spec/lsp-v0.md`.

use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| cntrdct::lsp::CntrdctLsp { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
