//! Retry / Retry-After integration tests for `download_dump_streaming`.
//!
//! Mirrors the pattern used in `tests/retry.rs` for the sparse-index client:
//! mockito stands up a local HTTP server so the suite stays deterministic
//! and offline. `download_dump_streaming_with_retry` is configured with a
//! 10 ms base delay so the suite finishes well under a second even when
//! every retry triggers.

use std::time::Duration;

use cntrdct_corpus_fetch::{download_dump_streaming_with_retry, FetchError};

const TINY_DELAY: Duration = Duration::from_millis(10);

#[test]
fn retries_on_503_then_succeeds() {
    let mut server = mockito::Server::new();
    let m1 = server
        .mock("GET", "/dump.tar.gz")
        .with_status(503)
        .expect(1)
        .create();
    let m2 = server
        .mock("GET", "/dump.tar.gz")
        .with_status(200)
        .with_body(b"fake-tarball-bytes")
        .expect(1)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    let bytes = download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        3,
        TINY_DELAY,
    )
    .expect("retry should recover after one 503");
    assert_eq!(bytes, "fake-tarball-bytes".len() as u64);
    assert_eq!(std::fs::read(&out).unwrap(), b"fake-tarball-bytes");
    m1.assert();
    m2.assert();
}

#[test]
fn retries_on_504_then_succeeds() {
    let mut server = mockito::Server::new();
    let m1 = server
        .mock("GET", "/dump.tar.gz")
        .with_status(504)
        .expect(1)
        .create();
    let m2 = server
        .mock("GET", "/dump.tar.gz")
        .with_status(200)
        .with_body(b"ok")
        .expect(1)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        3,
        TINY_DELAY,
    )
    .expect("504 should retry");
    m1.assert();
    m2.assert();
}

#[test]
fn honours_retry_after_header_on_429() {
    let mut server = mockito::Server::new();
    let m1 = server
        .mock("GET", "/dump.tar.gz")
        .with_status(429)
        .with_header("Retry-After", "1")
        .expect(1)
        .create();
    let m2 = server
        .mock("GET", "/dump.tar.gz")
        .with_status(200)
        .with_body(b"ok")
        .expect(1)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    let started = std::time::Instant::now();
    download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        3,
        TINY_DELAY,
    )
    .expect("Retry-After should be honoured");
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_secs(1),
        "expected to wait >= 1s for Retry-After, got {elapsed:?}"
    );
    m1.assert();
    m2.assert();
}

#[test]
fn exhausts_retries_when_server_keeps_failing() {
    let mut server = mockito::Server::new();
    // 1 initial attempt + 2 retries = 3 calls total.
    let m = server
        .mock("GET", "/dump.tar.gz")
        .with_status(503)
        .expect(3)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    let err = download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        2,
        TINY_DELAY,
    )
    .expect_err("exhausted retries should error");
    let msg = format!("{err}");
    assert!(msg.contains("503"), "expected 503 in error, got: {msg}");
    // Partial output must not linger after a failed run.
    assert!(!out.exists(), "partial output should be cleaned up");
    m.assert();
}

#[test]
fn does_not_retry_on_404() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/dump.tar.gz")
        .with_status(404)
        .expect(1)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    let err = download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        5,
        TINY_DELAY,
    )
    .expect_err("404 should not retry");
    assert!(matches!(err, FetchError::NotFound(_)));
    m.assert();
}

#[test]
fn does_not_retry_on_400_class_other_than_429() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/dump.tar.gz")
        .with_status(400)
        .expect(1)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    let err = download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        5,
        TINY_DELAY,
    )
    .expect_err("400 should not retry");
    let msg = format!("{err}");
    assert!(msg.contains("400"));
    m.assert();
}

#[test]
fn success_on_first_attempt_does_not_retry() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/dump.tar.gz")
        .with_status(200)
        .with_body(b"hello-dump")
        .expect(1)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    let bytes = download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        5,
        TINY_DELAY,
    )
    .unwrap();
    assert_eq!(bytes, "hello-dump".len() as u64);
    assert_eq!(std::fs::read(&out).unwrap(), b"hello-dump");
    m.assert();
}

#[test]
fn max_retries_zero_disables_retry_and_matches_old_behaviour() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/dump.tar.gz")
        .with_status(503)
        .expect(1)
        .create();

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("dump.tar.gz");
    let err = download_dump_streaming_with_retry(
        &format!("{}/dump.tar.gz", server.url()),
        &out,
        0,
        TINY_DELAY,
    )
    .expect_err("max_retries=0 should error on the first transient failure");
    let msg = format!("{err}");
    assert!(msg.contains("503"), "got: {msg}");
    m.assert();
}
