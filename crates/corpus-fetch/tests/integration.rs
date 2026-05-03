//! Fixture-driven integration tests for the corpus-fetch crate.
//!
//! These exercise the full pipeline (path computation → HTTP fetch → JSONL
//! parse → license decision) against canned Sparse Index responses on disk.
//! They never touch the network.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use cntrdct_corpus_fetch::{
    license_decision, FetchError, HttpClient, LicenseDecision, SparseIndexClient,
    DEFAULT_LICENSE_ALLOWLIST,
};

const BASE: &str = "https://example.test";

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(name: &str) -> String {
    std::fs::read_to_string(fixture_dir().join(name))
        .unwrap_or_else(|e| panic!("missing fixture {name}: {e}"))
}

struct FixtureClient {
    map: Mutex<HashMap<String, String>>,
}

impl FixtureClient {
    fn new() -> Self {
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert(format!("{BASE}/se/rd/serde"), load("serde.jsonl"));
        map.insert(format!("{BASE}/3/l/log"), load("log.jsonl"));
        map.insert(format!("{BASE}/ri/ng/ring"), load("ring.jsonl"));
        map.insert(format!("{BASE}/gp/l-/gpl-only"), load("gpl-only.jsonl"));
        map.insert(format!("{BASE}/no/li/nolicense"), load("nolicense.jsonl"));
        Self {
            map: Mutex::new(map),
        }
    }
}

impl HttpClient for FixtureClient {
    fn get_text(&self, url: &str) -> Result<String, FetchError> {
        match self.map.lock().unwrap().get(url) {
            Some(body) => Ok(body.clone()),
            None => Err(FetchError::NotFound(url.to_string())),
        }
    }
}

fn client() -> SparseIndexClient<FixtureClient> {
    SparseIndexClient::new(FixtureClient::new()).with_base_url(BASE)
}

#[test]
fn serde_latest_non_yanked_is_accepted() {
    let c = client();
    let latest = c.fetch_latest_non_yanked("serde").unwrap().unwrap();
    assert_eq!(latest.version, "1.0.2");
    assert_eq!(latest.license.as_deref(), Some("MIT OR Apache-2.0"));
    assert_eq!(
        license_decision(latest.license.as_deref(), DEFAULT_LICENSE_ALLOWLIST),
        LicenseDecision::Accepted
    );
}

#[test]
fn log_is_accepted() {
    let c = client();
    let latest = c.fetch_latest_non_yanked("log").unwrap().unwrap();
    assert_eq!(
        license_decision(latest.license.as_deref(), DEFAULT_LICENSE_ALLOWLIST),
        LicenseDecision::Accepted
    );
}

#[test]
fn ring_with_clause_is_accepted() {
    let c = client();
    let latest = c.fetch_latest_non_yanked("ring").unwrap().unwrap();
    assert_eq!(
        latest.license.as_deref(),
        Some("Apache-2.0 WITH LLVM-exception")
    );
    assert_eq!(
        license_decision(latest.license.as_deref(), DEFAULT_LICENSE_ALLOWLIST),
        LicenseDecision::Accepted
    );
}

#[test]
fn gpl_only_is_rejected() {
    let c = client();
    let latest = c.fetch_latest_non_yanked("gpl-only").unwrap().unwrap();
    assert_eq!(
        license_decision(latest.license.as_deref(), DEFAULT_LICENSE_ALLOWLIST),
        LicenseDecision::Rejected
    );
}

#[test]
fn missing_license_is_reported_as_missing() {
    let c = client();
    let latest = c.fetch_latest_non_yanked("nolicense").unwrap().unwrap();
    assert_eq!(latest.license, None);
    assert_eq!(
        license_decision(latest.license.as_deref(), DEFAULT_LICENSE_ALLOWLIST),
        LicenseDecision::Missing
    );
}

#[test]
fn unknown_crate_returns_not_found() {
    let c = client();
    let err = c.fetch_versions("does-not-exist").unwrap_err();
    assert!(matches!(err, FetchError::NotFound(_)));
}

#[test]
fn serde_yanked_row_is_present_in_full_listing() {
    let c = client();
    let all = c.fetch_versions("serde").unwrap();
    assert_eq!(all.len(), 3);
    assert!(all.iter().any(|v| v.yanked && v.version == "1.0.1"));
}
