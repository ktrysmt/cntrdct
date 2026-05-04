//! crates.io Sparse Index client.
//!
//! The Sparse Index (`https://index.crates.io/`) serves one HTTP file per
//! crate. Each file is JSON Lines — one object per published version, in
//! chronological order. We fetch a single file per crate, parse each line
//! into [`CrateMeta`], and surface the most recent non-yanked record.
//!
//! Reference: <https://doc.rust-lang.org/cargo/reference/registry-index.html>
//! (sparse and git index share the same file format; the path-prefix scheme
//! is shared too).
//!
//! HTTP transport is abstracted behind [`HttpClient`] so tests can substitute
//! a mock that serves fixtures from disk; the production binding is
//! [`ReqwestClient`], a thin shim over `reqwest::blocking` with rustls.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::FetchError;

pub const DEFAULT_SPARSE_INDEX_BASE: &str = "https://index.crates.io";

/// One published version of a crate, as served by the Sparse Index.
///
/// Fields beyond the ones we use (`deps`, `features`, `links`, `v`,
/// `rust_version`) are silently ignored via serde's default policy. Only the
/// data we need to filter and reproduce a fetch is materialised: the version
/// string, the SPDX license expression, the yanked flag, and the SHA-256
/// checksum of the source tarball — `cksum` is what `cargo` itself verifies
/// downloads against, so recording it pins the corpus by content.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CrateMeta {
    pub name: String,
    #[serde(rename = "vers")]
    pub version: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub yanked: bool,
    pub cksum: String,
}

/// Compute the Sparse Index path for a crate name.
///
/// The index splits crates into directories by the first one or two
/// characters of the lowercased name so a single shard never holds the entire
/// corpus. The rules:
///
/// | length | path                          |
/// |--------|-------------------------------|
/// | 1      | `1/<name>`                    |
/// | 2      | `2/<name>`                    |
/// | 3      | `3/<first>/<name>`            |
/// | 4+     | `<first2>/<chars3-4>/<name>`  |
///
/// Names are validated as ASCII alphanumerics plus `-` and `_`. This is
/// stricter than `cargo`'s own rules but sufficient for any crate that
/// reached crates.io, and it eliminates path-traversal concerns from
/// untrusted input.
pub fn index_path(name: &str) -> Result<String, FetchError> {
    if name.is_empty() || name.len() > 64 {
        return Err(FetchError::InvalidName(name.to_string()));
    }
    let lc = name.to_ascii_lowercase();
    if !lc
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(FetchError::InvalidName(name.to_string()));
    }
    let path = match lc.len() {
        1 => format!("1/{lc}"),
        2 => format!("2/{lc}"),
        3 => format!("3/{}/{lc}", &lc[..1]),
        _ => format!("{}/{}/{lc}", &lc[..2], &lc[2..4]),
    };
    Ok(path)
}

/// HTTP transport seam.
///
/// Implementors must turn a GET into either a UTF-8 body (`get_text`, used
/// for Sparse Index JSONL responses) or raw bytes (`get_bytes`, used for
/// `.crate` tarball downloads). A 404 response must be reported as
/// [`FetchError::NotFound`] so the caller can distinguish "no such crate"
/// from a transport failure — that distinction becomes part of the corpus
/// manifest.
pub trait HttpClient: Send + Sync {
    fn get_text(&self, url: &str) -> Result<String, FetchError>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError>;
}

/// Default maximum number of retries for transient HTTP failures.
pub const DEFAULT_MAX_RETRIES: u32 = 5;
/// Default base delay used by the exponential-backoff schedule. The actual
/// delay for attempt `n` is `base_delay * 2^min(n, 6)`, so the schedule
/// caps at roughly `64 * base_delay`.
pub const DEFAULT_BASE_DELAY: Duration = Duration::from_secs(1);

/// Production HTTP client backed by `reqwest::blocking` with rustls.
///
/// Implements polite retry semantics around the GET path:
/// - 429 / 503 / 504 responses are retried up to `max_retries` times.
/// - The `Retry-After` header (seconds form, RFC 7231 §7.1.3) is honoured
///   when present.
/// - Transient `reqwest` connection / timeout errors retry on the same
///   exponential schedule.
/// - Non-retriable status codes (200 success, 404, other 4xx) and
///   non-transient errors return immediately.
#[derive(Debug)]
pub struct ReqwestClient {
    inner: reqwest::blocking::Client,
    max_retries: u32,
    base_delay: Duration,
}

impl ReqwestClient {
    pub fn new() -> Result<Self, FetchError> {
        Self::with_retry_policy(DEFAULT_MAX_RETRIES, DEFAULT_BASE_DELAY)
    }

    /// Build a client with a custom retry policy. Tests pass a sub-second
    /// `base_delay` so the suite finishes quickly; production keeps the
    /// 1-second default so we stay polite on crates.io's static endpoint.
    pub fn with_retry_policy(
        max_retries: u32,
        base_delay: Duration,
    ) -> Result<Self, FetchError> {
        let inner = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!(
                "cntrdct-corpus-fetch/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/ktrysmt/cntrdct)"
            ))
            .build()
            .map_err(|e| FetchError::Http(e.to_string()))?;
        Ok(Self {
            inner,
            max_retries,
            base_delay,
        })
    }

    /// Send a GET with retry. Returns the raw `reqwest::Response` so the
    /// caller picks how to consume the body (text / bytes). Status filtering
    /// (NotFound vs other failures) lives in `get_text` / `get_bytes`.
    fn send_with_retry(&self, url: &str) -> Result<reqwest::blocking::Response, FetchError> {
        let mut attempt: u32 = 0;
        loop {
            let result = self.inner.get(url).send();
            match result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let is_retriable = matches!(status, 429 | 503 | 504);
                    if is_retriable && attempt < self.max_retries {
                        let delay = retry_after_seconds(&resp)
                            .map(Duration::from_secs)
                            .unwrap_or_else(|| backoff_delay(attempt, self.base_delay));
                        drop(resp);
                        std::thread::sleep(delay);
                        attempt += 1;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) if (e.is_timeout() || e.is_connect()) && attempt < self.max_retries => {
                    let delay = backoff_delay(attempt, self.base_delay);
                    std::thread::sleep(delay);
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(FetchError::Http(e.to_string())),
            }
        }
    }
}

/// Parse `Retry-After` as seconds. The HTTP-date form is rare on
/// machine-to-machine APIs and we deliberately do not handle it; callers
/// fall back to the exponential-backoff schedule when this returns `None`.
fn retry_after_seconds(resp: &reqwest::blocking::Response) -> Option<u64> {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()
}

/// `base * 2^min(attempt, 6)` — caps the multiplier so the delay never
/// outgrows the timeout window. With the 1s default base, attempt 0 sleeps
/// 1s, attempt 1 sleeps 2s, …, attempt 6+ sleeps 64s.
fn backoff_delay(attempt: u32, base: Duration) -> Duration {
    let exp = attempt.min(6);
    base.saturating_mul(1u32 << exp)
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new().expect("reqwest blocking client builds with default settings")
    }
}

impl HttpClient for ReqwestClient {
    fn get_text(&self, url: &str) -> Result<String, FetchError> {
        let resp = self.send_with_retry(url)?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(FetchError::NotFound(url.to_string()));
        }
        if !status.is_success() {
            return Err(FetchError::Http(format!("status {status} from {url}")));
        }
        resp.text().map_err(|e| FetchError::Http(e.to_string()))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let resp = self.send_with_retry(url)?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(FetchError::NotFound(url.to_string()));
        }
        if !status.is_success() {
            return Err(FetchError::Http(format!("status {status} from {url}")));
        }
        let bytes = resp
            .bytes()
            .map_err(|e| FetchError::Http(e.to_string()))?;
        Ok(bytes.to_vec())
    }
}

/// Sparse Index client.
#[derive(Debug, Clone)]
pub struct SparseIndexClient<C: HttpClient> {
    client: C,
    base_url: String,
}

impl<C: HttpClient> SparseIndexClient<C> {
    pub fn new(client: C) -> Self {
        Self {
            client,
            base_url: DEFAULT_SPARSE_INDEX_BASE.to_string(),
        }
    }

    /// Override the index base URL. Used by integration tests pointing at a
    /// local fixture server; production callers leave this at the default.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Fetch every published version of `crate_name`, including yanked rows,
    /// in the order the index serves them (chronological).
    pub fn fetch_versions(&self, crate_name: &str) -> Result<Vec<CrateMeta>, FetchError> {
        let path = index_path(crate_name)?;
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let body = self.client.get_text(&url)?;
        parse_jsonl(&body)
    }

    /// Fetch the most recent non-yanked version, if any.
    ///
    /// "Latest" here means latest-by-publish-order, not latest-by-semver. The
    /// distinction matters for crates that publish patch releases on older
    /// branches; for the empirical study we want whatever a fresh
    /// `cargo add <crate>` would pick today, and `cargo` resolves that the
    /// same way (newest publish that satisfies the requested req).
    pub fn fetch_latest_non_yanked(
        &self,
        crate_name: &str,
    ) -> Result<Option<CrateMeta>, FetchError> {
        let versions = self.fetch_versions(crate_name)?;
        Ok(versions.into_iter().rev().find(|v| !v.yanked))
    }
}

fn parse_jsonl(body: &str) -> Result<Vec<CrateMeta>, FetchError> {
    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let meta: CrateMeta =
            serde_json::from_str(line).map_err(|e| FetchError::Malformed(e.to_string()))?;
        out.push(meta);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn index_path_one_letter() {
        assert_eq!(index_path("a").unwrap(), "1/a");
    }

    #[test]
    fn index_path_two_letter() {
        assert_eq!(index_path("ab").unwrap(), "2/ab");
    }

    #[test]
    fn index_path_three_letter() {
        assert_eq!(index_path("log").unwrap(), "3/l/log");
        assert_eq!(index_path("syn").unwrap(), "3/s/syn");
    }

    #[test]
    fn index_path_four_letter() {
        assert_eq!(index_path("rand").unwrap(), "ra/nd/rand");
    }

    #[test]
    fn index_path_long_name() {
        assert_eq!(index_path("serde").unwrap(), "se/rd/serde");
        assert_eq!(index_path("tokio").unwrap(), "to/ki/tokio");
    }

    #[test]
    fn index_path_lowercases_input() {
        assert_eq!(index_path("Serde").unwrap(), "se/rd/serde");
        assert_eq!(index_path("LOG").unwrap(), "3/l/log");
    }

    #[test]
    fn index_path_accepts_dash_and_underscore() {
        assert_eq!(index_path("hyper-tls").unwrap(), "hy/pe/hyper-tls");
        assert_eq!(index_path("foo_bar").unwrap(), "fo/o_/foo_bar");
    }

    #[test]
    fn index_path_rejects_path_traversal() {
        assert!(matches!(index_path(""), Err(FetchError::InvalidName(_))));
        assert!(matches!(index_path("../etc"), Err(FetchError::InvalidName(_))));
        assert!(matches!(index_path("a/b"), Err(FetchError::InvalidName(_))));
        assert!(matches!(index_path("a b"), Err(FetchError::InvalidName(_))));
        assert!(matches!(index_path("a.b"), Err(FetchError::InvalidName(_))));
    }

    #[test]
    fn parse_jsonl_round_trips_a_single_record() {
        let body = r#"{"name":"serde","vers":"1.0.0","deps":[],"cksum":"abc","features":{},"yanked":false,"license":"MIT OR Apache-2.0"}"#;
        let recs = parse_jsonl(body).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].name, "serde");
        assert_eq!(recs[0].version, "1.0.0");
        assert_eq!(recs[0].license.as_deref(), Some("MIT OR Apache-2.0"));
        assert!(!recs[0].yanked);
        assert_eq!(recs[0].cksum, "abc");
    }

    #[test]
    fn parse_jsonl_skips_blank_lines() {
        let body = "\n{\"name\":\"a\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\"x\",\"features\":{},\"yanked\":false}\n\n";
        let recs = parse_jsonl(body).unwrap();
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn parse_jsonl_handles_missing_optional_license() {
        let body = r#"{"name":"a","vers":"0.1.0","deps":[],"cksum":"x","features":{},"yanked":false}"#;
        let recs = parse_jsonl(body).unwrap();
        assert_eq!(recs[0].license, None);
    }

    #[test]
    fn parse_jsonl_returns_records_in_file_order() {
        let body = "{\"name\":\"a\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\"x\",\"features\":{},\"yanked\":false}\n{\"name\":\"a\",\"vers\":\"0.2.0\",\"deps\":[],\"cksum\":\"y\",\"features\":{},\"yanked\":true}\n{\"name\":\"a\",\"vers\":\"0.3.0\",\"deps\":[],\"cksum\":\"z\",\"features\":{},\"yanked\":false}";
        let recs = parse_jsonl(body).unwrap();
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].version, "0.1.0");
        assert_eq!(recs[2].version, "0.3.0");
    }

    #[test]
    fn parse_jsonl_malformed_errs() {
        let body = "{not json";
        assert!(matches!(parse_jsonl(body), Err(FetchError::Malformed(_))));
    }

    // ---- MockClient driving SparseIndexClient ----

    struct MockClient {
        // url -> Result<body, FetchError>
        responses: Mutex<Vec<(String, Result<String, FetchError>)>>,
        last_url: Mutex<Option<String>>,
    }

    impl MockClient {
        fn new() -> Self {
            Self {
                responses: Mutex::new(Vec::new()),
                last_url: Mutex::new(None),
            }
        }
        fn expect(&self, url: &str, body: &str) {
            self.responses
                .lock()
                .unwrap()
                .push((url.to_string(), Ok(body.to_string())));
        }
        fn expect_err(&self, url: &str, err: FetchError) {
            self.responses
                .lock()
                .unwrap()
                .push((url.to_string(), Err(err)));
        }
    }

    impl HttpClient for MockClient {
        fn get_text(&self, url: &str) -> Result<String, FetchError> {
            *self.last_url.lock().unwrap() = Some(url.to_string());
            let mut guard = self.responses.lock().unwrap();
            for (i, (u, _)) in guard.iter().enumerate() {
                if u == url {
                    let (_, r) = guard.remove(i);
                    return r;
                }
            }
            Err(FetchError::Http(format!("no mock for {url}")))
        }

        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
            // Sparse index tests never call get_bytes; tarball tests use a
            // dedicated mock in `tarball.rs`.
            unimplemented!("MockClient does not serve bytes")
        }
    }

    #[test]
    fn fetch_versions_builds_correct_url() {
        let mock = MockClient::new();
        mock.expect(
            "https://example.test/se/rd/serde",
            "{\"name\":\"serde\",\"vers\":\"1.0.0\",\"deps\":[],\"cksum\":\"x\",\"features\":{},\"yanked\":false,\"license\":\"MIT OR Apache-2.0\"}",
        );
        let client =
            SparseIndexClient::new(mock).with_base_url("https://example.test");
        let versions = client.fetch_versions("serde").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].name, "serde");
    }

    #[test]
    fn fetch_versions_strips_trailing_slash_from_base_url() {
        let mock = MockClient::new();
        mock.expect(
            "https://example.test/3/l/log",
            "{\"name\":\"log\",\"vers\":\"0.4.0\",\"deps\":[],\"cksum\":\"x\",\"features\":{},\"yanked\":false,\"license\":\"MIT OR Apache-2.0\"}",
        );
        let client =
            SparseIndexClient::new(mock).with_base_url("https://example.test/");
        let versions = client.fetch_versions("log").unwrap();
        assert_eq!(versions.len(), 1);
    }

    #[test]
    fn fetch_versions_propagates_not_found() {
        let mock = MockClient::new();
        mock.expect_err(
            "https://example.test/no/pe/nope-crate",
            FetchError::NotFound("https://example.test/no/pe/nope-crate".to_string()),
        );
        let client =
            SparseIndexClient::new(mock).with_base_url("https://example.test");
        let err = client.fetch_versions("nope-crate").unwrap_err();
        assert!(matches!(err, FetchError::NotFound(_)));
    }

    #[test]
    fn fetch_latest_non_yanked_skips_yanked_tail() {
        let mock = MockClient::new();
        mock.expect(
            "https://example.test/3/a/abc",
            "{\"name\":\"abc\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\"x\",\"features\":{},\"yanked\":false}\n\
             {\"name\":\"abc\",\"vers\":\"0.2.0\",\"deps\":[],\"cksum\":\"y\",\"features\":{},\"yanked\":false}\n\
             {\"name\":\"abc\",\"vers\":\"0.3.0\",\"deps\":[],\"cksum\":\"z\",\"features\":{},\"yanked\":true}",
        );
        let client =
            SparseIndexClient::new(mock).with_base_url("https://example.test");
        let latest = client.fetch_latest_non_yanked("abc").unwrap().unwrap();
        assert_eq!(latest.version, "0.2.0");
    }

    #[test]
    fn fetch_latest_non_yanked_returns_none_when_all_yanked() {
        let mock = MockClient::new();
        mock.expect(
            "https://example.test/3/a/abc",
            "{\"name\":\"abc\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\"x\",\"features\":{},\"yanked\":true}",
        );
        let client =
            SparseIndexClient::new(mock).with_base_url("https://example.test");
        let latest = client.fetch_latest_non_yanked("abc").unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn fetch_versions_invalid_name_returns_invalid_name_error() {
        let mock = MockClient::new();
        let client =
            SparseIndexClient::new(mock).with_base_url("https://example.test");
        let err = client.fetch_versions("../etc/passwd").unwrap_err();
        assert!(matches!(err, FetchError::InvalidName(_)));
    }
}
