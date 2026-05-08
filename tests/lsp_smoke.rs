//! T3-12 Phase 1.b + 1.c smoke tests: drive `cntrdct-lsp` over
//! stdin/stdout with a hand-rolled JSON-RPC client and assert the
//! lifecycle and document-event round-trips described in
//! `docs/spec/lsp-v0.md` "Testing".
//!
//! - Phase 1.b: `Initialize` -> `didOpen` -> `publishDiagnostics`
//!   -> `Shutdown`.
//! - Phase 1.c: a burst of `didChange` notifications inside the
//!   debounce window collapses to exactly one `publishDiagnostics`
//!   carrying the most recent buffer state.
//!
//! Both tests run only when the `lsp` Cargo feature is enabled (see
//! `Cargo.toml`'s `[[bin]] cntrdct-lsp` declaration). CI exercises
//! them through the dedicated `clippy (lsp feature)` and
//! `cargo test --features lsp` steps.

#![cfg(feature = "lsp")]

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

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

#[test]
fn did_change_debounces_rapid_bursts_to_one_publish() {
    // Phase 1.c: three didChange notifications fired well inside the
    // 250 ms debounce window must collapse to exactly one
    // publishDiagnostics carrying the *last* buffer state. If the
    // debouncer were missing we would see three publishes (one per
    // didChange).
    let mut child = Command::new(env!("CARGO_BIN_EXE_cntrdct-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cntrdct-lsp");

    let mut stdin = child.stdin.take().expect("stdin pipe");
    let stdout = child.stdout.take().expect("stdout pipe");
    let rx = spawn_reader(stdout);

    let tmp = tempfile::Builder::new()
        .suffix(".rs")
        .tempfile()
        .expect("tempfile");
    let uri = format!("file://{}", tmp.path().display());

    // 1. initialize / initialized
    write_request(&mut stdin, 1, "initialize", json!({"capabilities": {}}));
    recv_for_id(&rx, 1, Duration::from_secs(5));
    write_notification(&mut stdin, "initialized", json!({}));

    // 2. didOpen — the initial scan is *not* debounced (Phase 1.c
    //    rationale: first impression must be immediate). Drain its
    //    publish so it does not count against the burst assertion.
    write_notification(
        &mut stdin,
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": "rust",
                "version": 1,
                "text": "fn unchanged() {}"
            }
        }),
    );
    let open_pub = recv_for_method(
        &rx,
        "textDocument/publishDiagnostics",
        Duration::from_secs(5),
    )
    .expect("didOpen must publish diagnostics");
    assert_eq!(open_pub["params"]["uri"], uri);

    // 3. Burst of three didChange notifications — only the third has
    //    text that produces a finding. Sent without intervening sleeps
    //    so they all land within the debounce window.
    let burst = [
        (2u64, "fn a() {}"),
        (3, "fn b() {}"),
        (4, "fn f() { return; bar(); }"),
    ];
    for (version, text) in burst {
        write_notification(
            &mut stdin,
            "textDocument/didChange",
            json!({
                "textDocument": {"uri": uri, "version": version},
                "contentChanges": [{"text": text}]
            }),
        );
    }

    // 4. Wait long enough for the debouncer to fire on the last
    //    didChange and for the spawned scan to publish, plus slack
    //    for any superseded scan to (incorrectly) publish if the
    //    debouncer were broken.
    thread::sleep(Duration::from_millis(700));

    let publishes = drain_method(&rx, "textDocument/publishDiagnostics");
    assert_eq!(
        publishes.len(),
        1,
        "expected exactly 1 publishDiagnostics after a 3-notification burst, got {}: {:#?}",
        publishes.len(),
        publishes
    );
    let arr = publishes[0]["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics must be an array");
    assert_eq!(
        arr.len(),
        1,
        "the surviving publish must reflect the *last* didChange (1 finding), got: {:#?}",
        publishes[0]
    );
    assert_eq!(arr[0]["code"], "unreachable-after-terminator");

    // 5. shutdown / exit
    write_request(&mut stdin, 99, "shutdown", json!(null));
    recv_for_id(&rx, 99, Duration::from_secs(5));
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

// ---------- Reader-thread + channel helpers (Phase 1.c) ----------
//
// The Phase 1.b helpers above block on a single stdout reader; that
// shape cannot answer "did *no* further frame arrive in the next
// 700 ms". The Phase 1.c debounce assertion needs that, so we run a
// reader thread that pumps every frame into an mpsc channel and let
// the test thread pull from the channel with timeouts.

fn spawn_reader<R: Read + Send + 'static>(r: R) -> Receiver<Value> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut br = BufReader::new(r);
        loop {
            match read_frame_or_eof(&mut br) {
                Some(v) => {
                    if tx.send(v).is_err() {
                        return;
                    }
                }
                None => return,
            }
        }
    });
    rx
}

fn read_frame_or_eof<R: BufRead + Read>(r: &mut R) -> Option<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        match r.read_line(&mut line) {
            Ok(0) | Err(_) => return None,
            Ok(_) => {}
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(rest) = line.trim().strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok();
        }
    }
    let len = content_length?;
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn recv_for_id(rx: &Receiver<Value>, id: u64, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        let v = rx
            .recv_timeout(remaining)
            .unwrap_or_else(|_| panic!("timed out waiting for response id={id}"));
        if v.get("id").and_then(Value::as_u64) == Some(id) {
            return v;
        }
    }
}

fn recv_for_method(rx: &Receiver<Value>, method: &str, timeout: Duration) -> Option<Value> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.checked_duration_since(Instant::now())?;
        let v = rx.recv_timeout(remaining).ok()?;
        if v.get("method").and_then(Value::as_str) == Some(method) {
            return Some(v);
        }
    }
}

fn drain_method(rx: &Receiver<Value>, method: &str) -> Vec<Value> {
    let mut out = Vec::new();
    while let Ok(v) = rx.try_recv() {
        if v.get("method").and_then(Value::as_str) == Some(method) {
            out.push(v);
        }
    }
    out
}
