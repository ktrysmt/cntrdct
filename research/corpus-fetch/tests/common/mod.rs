//! Shared test fixtures for corpus-fetch integration tests.
//!
//! Each integration test under `tests/` is its own compilation unit.
//! Per Rust convention this module is included via `mod common;` at
//! the top of each consumer file so Cargo does not treat the file as
//! a separate test binary.
//!
//! The fixtures originally lived inline in `resume_orphan.rs` and
//! `license_hint_precedence.rs` (each-file-self-contained); they were
//! centralised here when `sha256_mismatch.rs` landed as the third
//! consumer. This crossing of the three-files threshold matches the
//! refactor trigger documented in `CLAUDE.md` PITFALL-6.

use std::collections::HashMap;
use std::sync::Mutex;

use cntrdct_corpus_fetch::{sha256_hex, FetchError, HttpClient};
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::{Builder, Header};

pub const INDEX_BASE: &str = "https://idx.test";
pub const STATIC_BASE: &str = "https://static.test";

/// HashMap-backed mock that serves both text (sparse index) and bytes
/// (tarball) and returns the same response on repeated calls.
///
/// `fetcher::tests::DualMock` (private to `fetcher.rs`) uses Vec-based
/// one-shot semantics; integration scenarios that re-invoke the
/// orchestrator over the same crate need an idempotent mock, hence
/// the HashMap backing.
pub struct DualFixtureClient {
    text: Mutex<HashMap<String, String>>,
    bytes: Mutex<HashMap<String, Vec<u8>>>,
}

impl DualFixtureClient {
    pub fn new() -> Self {
        Self {
            text: Mutex::new(HashMap::new()),
            bytes: Mutex::new(HashMap::new()),
        }
    }
    pub fn add_text(&self, url: &str, body: String) {
        self.text.lock().unwrap().insert(url.to_string(), body);
    }
    pub fn add_bytes(&self, url: &str, body: Vec<u8>) {
        self.bytes.lock().unwrap().insert(url.to_string(), body);
    }
}

impl HttpClient for DualFixtureClient {
    fn get_text(&self, url: &str) -> Result<String, FetchError> {
        match self.text.lock().unwrap().get(url) {
            Some(body) => Ok(body.clone()),
            None => Err(FetchError::NotFound(url.to_string())),
        }
    }
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        match self.bytes.lock().unwrap().get(url) {
            Some(body) => Ok(body.clone()),
            None => Err(FetchError::NotFound(url.to_string())),
        }
    }
}

/// Build a synthetic `.crate` tarball whose top-level directory is
/// `top_dir` and whose member files are `(path, contents)` pairs.
/// Returns the gzipped bytes plus their lowercase-hex SHA-256 so the
/// caller can pin the digest in a synthetic sparse-index entry.
pub fn make_crate(top_dir: &str, files: &[(&str, &[u8])]) -> (Vec<u8>, String) {
    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = Builder::new(gz);
    for (path, contents) in files {
        let full = format!("{top_dir}/{path}");
        let mut header = Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, &full, *contents).unwrap();
    }
    let bytes = tar.into_inner().unwrap().finish().unwrap();
    let digest = sha256_hex(&bytes);
    (bytes, digest)
}

/// Render a single sparse-index version line (one JSON object per
/// line, the format `cargo` itself reads). `license` is `None` to
/// emit no `license` field (the steady state for crates.io's real
/// sparse-index payload). `cksum` is whatever the caller wants to
/// pin; `make_crate` returns the matching digest for the happy path,
/// but tests can pass a deliberately-wrong value to exercise error
/// paths.
pub fn sparse_record(
    name: &str,
    version: &str,
    license: Option<&str>,
    cksum: &str,
    yanked: bool,
) -> String {
    let lic = match license {
        Some(s) => format!(",\"license\":\"{s}\""),
        None => String::new(),
    };
    format!(
        "{{\"name\":\"{name}\",\"vers\":\"{version}\",\"deps\":[],\"cksum\":\"{cksum}\",\"features\":{{}},\"yanked\":{yanked}{lic}}}"
    )
}
