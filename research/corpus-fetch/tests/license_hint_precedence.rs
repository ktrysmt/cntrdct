//! Regression test for license-hint precedence in `fetch_one`.
//!
//! The precedence rule is: a license value that the Sparse Index
//! already carries on the version record wins over any caller-supplied
//! hint; the hint is consulted only when the index value is absent.
//! Both absent yields a `LicenseMissing` skip without downloading the
//! tarball or creating an extraction directory.
//!
//! Inline unit tests in `src/fetcher.rs` already exercise this rule
//! against the private DualMock, but the precedence is part of the
//! crate's public contract — `cntrdct-research fetch` joins a
//! db-dump-derived licenses sidecar against the Sparse Index value and
//! relies on this rule for every crate it fetches. This file pins the
//! contract at the integration level so future refactors of the
//! fetcher's internals cannot quietly change the ordering without a
//! public-API test failure.

use cntrdct_corpus_fetch::{
    fetch_one, ExtractOptions, FetchOutcome, SkipReason, SparseIndexClient, TarballClient,
    DEFAULT_LICENSE_ALLOWLIST,
};

mod common;
use common::{make_crate, sparse_record, DualFixtureClient, INDEX_BASE, STATIC_BASE};

#[test]
fn case_a_index_license_wins_over_contradicting_hint() {
    // Index carries license = "MIT"; caller supplies a contradicting
    // hint of "Apache-2.0". The recorded license must come from the
    // index, never from the hint. (Older mirrors and internal
    // registries do publish license fields; the hint is fallback only.)
    let lib_body: &[u8] = b"pub fn ok() {}";
    let (bytes, digest) = make_crate("alpha-0.1.0", &[("src/lib.rs", lib_body)]);

    let idx = DualFixtureClient::new();
    idx.add_text(
        &format!("{INDEX_BASE}/al/ph/alpha"),
        sparse_record("alpha", "0.1.0", Some("MIT"), &digest, false),
    );
    let tar_mock = DualFixtureClient::new();
    tar_mock.add_bytes(
        &format!("{STATIC_BASE}/crates/alpha/alpha-0.1.0.crate"),
        bytes,
    );

    let sparse = SparseIndexClient::new(idx).with_base_url(INDEX_BASE);
    let tarball = TarballClient::new(tar_mock).with_base_url(STATIC_BASE);

    let tmp = tempfile::tempdir().unwrap();
    let result = fetch_one(
        &sparse,
        &tarball,
        "alpha",
        tmp.path(),
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        Some("Apache-2.0"),
    )
    .unwrap();

    match result {
        FetchOutcome::Fetched { row, .. } => {
            assert_eq!(
                row.license, "MIT",
                "index value must win; hint must not override",
            );
        }
        other => panic!("expected Fetched, got {other:?}"),
    }
}

#[test]
fn case_b_hint_substitutes_when_index_omits_license() {
    // Index has no license field — the steady-state for the real
    // crates.io sparse index. Caller supplies the SPDX expression as a
    // hint pulled from the db-dump-derived licenses sidecar. The
    // recorded license is the hint value.
    let lib_body: &[u8] = b"pub fn ok() {}";
    let (bytes, digest) = make_crate("beta-0.2.0", &[("src/lib.rs", lib_body)]);

    let idx = DualFixtureClient::new();
    idx.add_text(
        &format!("{INDEX_BASE}/be/ta/beta"),
        sparse_record("beta", "0.2.0", None, &digest, false),
    );
    let tar_mock = DualFixtureClient::new();
    tar_mock.add_bytes(
        &format!("{STATIC_BASE}/crates/beta/beta-0.2.0.crate"),
        bytes,
    );

    let sparse = SparseIndexClient::new(idx).with_base_url(INDEX_BASE);
    let tarball = TarballClient::new(tar_mock).with_base_url(STATIC_BASE);

    let tmp = tempfile::tempdir().unwrap();
    let result = fetch_one(
        &sparse,
        &tarball,
        "beta",
        tmp.path(),
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        Some("MIT OR Apache-2.0"),
    )
    .unwrap();

    match result {
        FetchOutcome::Fetched { row, .. } => {
            assert_eq!(
                row.license, "MIT OR Apache-2.0",
                "hint fills the index license gap",
            );
        }
        other => panic!("expected Fetched, got {other:?}"),
    }
}

#[test]
fn case_c_no_index_no_hint_skips_with_license_missing_and_no_side_effects() {
    // Index has no license field AND caller supplies no hint. fetch_one
    // must return Skipped { reason: LicenseMissing }, must not extract
    // the tarball, and must not create the crate directory under
    // out_root.
    //
    // The tarball mock is left empty on purpose: if the orchestrator's
    // skip path ever regressed and tried to download anyway, the
    // request would surface FetchError::NotFound and the test would
    // fail loudly via the `unwrap()` below — failing for the right
    // reason rather than papering over the regression.
    let idx = DualFixtureClient::new();
    idx.add_text(
        &format!("{INDEX_BASE}/ga/mm/gamma"),
        sparse_record(
            "gamma",
            "0.3.0",
            None,
            "deadbeef0000000000000000000000000000000000000000000000000000000000",
            false,
        ),
    );
    let tar_mock = DualFixtureClient::new();

    let sparse = SparseIndexClient::new(idx).with_base_url(INDEX_BASE);
    let tarball = TarballClient::new(tar_mock).with_base_url(STATIC_BASE);

    let tmp = tempfile::tempdir().unwrap();
    let out_root = tmp.path();
    let result = fetch_one(
        &sparse,
        &tarball,
        "gamma",
        out_root,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        None,
    )
    .expect("LicenseMissing must be a Skip outcome, not an error");

    match result {
        FetchOutcome::Skipped {
            name,
            reason: SkipReason::LicenseMissing,
        } => {
            assert_eq!(name, "gamma");
        }
        other => panic!("expected LicenseMissing skip, got {other:?}"),
    }

    // The skip path must not have created an extracted crate directory:
    // the manifest contract is that a row exists iff the directory
    // exists, and a LicenseMissing skip writes neither.
    assert!(
        !out_root.join("gamma-0.3.0").exists(),
        "fetch_one must not create the crate dir on a LicenseMissing skip",
    );
}
