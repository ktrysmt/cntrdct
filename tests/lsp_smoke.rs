//! T3-12 Phase 1.b smoke test: drives `cntrdct-lsp` over stdin/stdout
//! with a hand-rolled JSON-RPC client and asserts the
//! `Initialize` -> `didOpen` -> `publishDiagnostics` -> `Shutdown`
//! round-trip described in `docs/spec/lsp-v0.md` "Testing".
//!
//! The test runs only when the `lsp` Cargo feature is enabled (see
//! `Cargo.toml`'s `[[bin]] cntrdct-lsp` declaration). CI exercises it
//! through the dedicated `clippy (lsp feature)` and
//! `cargo test --features lsp` steps.

#![cfg(feature = "lsp")]

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

#[test]
fn initialize_did_open_publishes_diagnostics_then_shutdown() {
    // Spawn the actual `cntrdct-lsp` binary the same way an editor
    // would. `CARGO_BIN_EXE_cntrdct-lsp` is set by Cargo when the
    // feature-gated binary is built alongside the test.
    let mut child = Command::new(env!("CARGO_BIN_EXE_cntrdct-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cntrdct-lsp");

    let mut stdin = child.stdin.take().expect("stdin pipe");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout pipe"));

    // tempfile gives us a real `.rs` path so `Url::to_file_path()` on
    // the server side round-trips cleanly. The contents on disk are
    // irrelevant — `scan_buffer` consumes the text we send via
    // didOpen, not the file body.
    let tmp = tempfile::Builder::new()
        .suffix(".rs")
        .tempfile()
        .expect("tempfile");
    let uri = format!("file://{}", tmp.path().display());

    // 1. initialize
    write_request(&mut stdin, 1, "initialize", json!({"capabilities": {}}));
    let init = read_for_id(&mut stdout, 1);
    assert_eq!(
        init["result"]["serverInfo"]["name"], "cntrdct-lsp",
        "initialize must return serverInfo.name=cntrdct-lsp; got {}",
        init
    );
    assert_eq!(
        init["result"]["capabilities"]["textDocumentSync"], 1,
        "FULL sync == 1; got {}",
        init["result"]
    );

    // 2. initialized notification (no response expected)
    write_notification(&mut stdin, "initialized", json!({}));

    // 3. textDocument/didOpen — the source mirrors the t1 detector
    //    test under `tests/detector_unreachable_after_terminator.rs`
    //    and reliably produces exactly one
    //    `unreachable-after-terminator` finding.
    write_notification(
        &mut stdin,
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": "rust",
                "version": 1,
                "text": "fn f() { return; bar(); }"
            }
        }),
    );

    let diags = read_until_method(&mut stdout, "textDocument/publishDiagnostics");
    let params = &diags["params"];
    assert_eq!(params["uri"], uri, "publishDiagnostics URI must match");
    let arr = params["diagnostics"]
        .as_array()
        .expect("diagnostics must be a JSON array");
    assert!(
        !arr.is_empty(),
        "expected at least one diagnostic, got: {}",
        params
    );
    let d0 = &arr[0];
    assert_eq!(d0["source"], "cntrdct", "source must be 'cntrdct'");
    assert_eq!(
        d0["code"], "unreachable-after-terminator",
        "code must carry the detector id"
    );
    // DiagnosticSeverity::WARNING == 2 per LSP 3.17.
    assert_eq!(d0["severity"], 2, "Warning severity must serialize as 2");

    // 4. shutdown
    write_request(&mut stdin, 2, "shutdown", json!(null));
    let shut = read_for_id(&mut stdout, 2);
    assert!(
        shut["result"].is_null(),
        "shutdown returns null Ok; got: {}",
        shut
    );

    // 5. exit notification — tower-lsp's runtime tears the server
    //    down on this notification. Closing stdin is a belt-and-
    //    suspenders fallback for editors that crash mid-handshake.
    write_notification(&mut stdin, "exit", json!(null));
    drop(stdin);
    let _ = child.wait();
}

// ---------- JSON-RPC framing helpers (LSP wire format) ----------

fn write_request<W: Write>(w: &mut W, id: u64, method: &str, params: Value) {
    let body = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    write_frame(w, &body);
}

fn write_notification<W: Write>(w: &mut W, method: &str, params: Value) {
    let body = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    write_frame(w, &body);
}

fn write_frame<W: Write>(w: &mut W, body: &Value) {
    let s = body.to_string();
    write!(w, "Content-Length: {}\r\n\r\n{}", s.len(), s).expect("write frame");
    w.flush().expect("flush stdin");
}

fn read_for_id<R: BufRead + Read>(r: &mut R, id: u64) -> Value {
    loop {
        let v = read_frame(r);
        if v.get("id").and_then(Value::as_u64) == Some(id) {
            return v;
        }
    }
}

fn read_until_method<R: BufRead + Read>(r: &mut R, method: &str) -> Value {
    loop {
        let v = read_frame(r);
        if v.get("method").and_then(Value::as_str) == Some(method) {
            return v;
        }
    }
}

fn read_frame<R: BufRead + Read>(r: &mut R) -> Value {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = r.read_line(&mut line).expect("read LSP header line");
        if n == 0 {
            panic!("EOF reading LSP header (server died?)");
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                rest.trim()
                    .parse()
                    .expect("Content-Length value not numeric"),
            );
        }
    }
    let len = content_length.expect("Content-Length header missing");
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).expect("read LSP body");
    serde_json::from_slice(&body).expect("LSP body must be JSON")
}
