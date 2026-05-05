//! Retry / Retry-After integration tests for `ReqwestClient`.
//!
//! Uses mockito to stand up a local HTTP server so the suite stays
//! deterministic and offline. `with_retry_policy` is configured with a
//! 10 ms base delay to keep total test time under a second even when
//! every retry triggers.

use std::time::Duration;

use cntrdct_corpus_fetch::{FetchError, HttpClient, ReqwestClient};

#[test]
fn retries_on_503_then_succeeds() {
    let mut server = mockito::Server::new();
    let m1 = server
        .mock("GET", "/foo")
        .with_status(503)
        .expect(1)
        .create();
    let m2 = server
        .mock("GET", "/foo")
        .with_status(200)
        .with_body("ok")
        .expect(1)
        .create();

    let client = ReqwestClient::with_retry_policy(3, Duration::from_millis(10)).unwrap();
    let body = client
        .get_text(&format!("{}/foo", server.url()))
        .expect("retry should recover after one 503");
    assert_eq!(body, "ok");
    m1.assert();
    m2.assert();
}

#[test]
fn retries_on_429_with_retry_after_header() {
    let mut server = mockito::Server::new();
    let m1 = server
        .mock("GET", "/foo")
        .with_status(429)
        .with_header("Retry-After", "1")
        .expect(1)
        .create();
    let m2 = server
        .mock("GET", "/foo")
        .with_status(200)
        .with_body("ok")
        .expect(1)
        .create();

    // base_delay is 10ms but Retry-After says 1 second; the client should
    // honour the header rather than its own schedule.
    let client = ReqwestClient::with_retry_policy(3, Duration::from_millis(10)).unwrap();
    let started = std::time::Instant::now();
    let body = client
        .get_text(&format!("{}/foo", server.url()))
        .expect("Retry-After should be honoured");
    let elapsed = started.elapsed();
    assert_eq!(body, "ok");
    assert!(
        elapsed >= Duration::from_secs(1),
        "expected to wait >= 1s for Retry-After, got {elapsed:?}",
    );
    m1.assert();
    m2.assert();
}

#[test]
fn exhausts_retries_when_server_keeps_failing() {
    let mut server = mockito::Server::new();
    // 1 initial attempt + 2 retries = 3 calls total.
    let m = server
        .mock("GET", "/foo")
        .with_status(503)
        .expect(3)
        .create();

    let client = ReqwestClient::with_retry_policy(2, Duration::from_millis(10)).unwrap();
    let err = client
        .get_text(&format!("{}/foo", server.url()))
        .expect_err("exhausted retries should error");
    let msg = format!("{err}");
    assert!(msg.contains("503"), "expected 503 in error, got: {msg}");
    m.assert();
}

#[test]
fn does_not_retry_on_404() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/foo")
        .with_status(404)
        .expect(1)
        .create();

    let client = ReqwestClient::with_retry_policy(5, Duration::from_millis(10)).unwrap();
    let err = client
        .get_text(&format!("{}/foo", server.url()))
        .expect_err("404 should not retry");
    assert!(matches!(err, FetchError::NotFound(_)));
    m.assert();
}

#[test]
fn does_not_retry_on_400_class_other_than_429() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/foo")
        .with_status(400)
        .expect(1)
        .create();

    let client = ReqwestClient::with_retry_policy(5, Duration::from_millis(10)).unwrap();
    let err = client
        .get_text(&format!("{}/foo", server.url()))
        .expect_err("400 should not retry");
    let msg = format!("{err}");
    assert!(msg.contains("400"));
    m.assert();
}

#[test]
fn success_on_first_attempt_does_not_retry() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/foo")
        .with_status(200)
        .with_body("hello")
        .expect(1)
        .create();

    let client = ReqwestClient::with_retry_policy(5, Duration::from_millis(10)).unwrap();
    let body = client.get_text(&format!("{}/foo", server.url())).unwrap();
    assert_eq!(body, "hello");
    m.assert();
}
