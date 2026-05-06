//! Integration tests for SHA-256 verification of fetched tarballs.
//!
//! `cntrdct-research fetch` joins each crate's tarball download
//! against the `cksum` field carried by the Sparse Index version
//! record. A mismatch is a corruption / tamper / mirror-divergence
//! signal and must be loudly surfaced as a [`FetchError`]; the
//! orchestrator must NOT silently fall through to extraction. This
//! property is part of the corpus's reproducibility contract: the
//! manifest is meaningful only if every tarball whose hash was
//! recorded matches what was extracted.
//!
//! Inline unit tests in `src/fetcher.rs` exercise the basic
//! mismatch case against the private `DualMock`. This file pins the
//! same property at the integration level (public API only) plus
//! two adjacent contracts:
//!
//! - The mismatch path must not create the per-crate extraction
//!   directory; orphan directories from corrupt downloads break
//!   the manifest invariant in `manifest::append_row` (a row exists
//!   iff a directory exists).
//! - Comparison is case-insensitive against the index's `cksum`
//!   field, per `tarball.rs::TarballClient::fetch` using
//!   `eq_ignore_ascii_case`. Uppercase index payloads (some
//!   internal mirrors) must not produce spurious mismatches.

use cntrdct_corpus_fetch::{
    fetch_one, ExtractOptions, FetchError, SparseIndexClient, TarballClient,
    DEFAULT_LICENSE_ALLOWLIST,
};

mod common;
use common::{make_crate, sparse_record, DualFixtureClient, INDEX_BASE, STATIC_BASE};

#[test]
fn case_a_mismatch_returns_checksum_error_with_both_digests() {
    // The tarball is well-formed but the sparse-index `cksum` is a
    // wrong value. fetch_one must return FetchError::ChecksumMismatch
    // and the error must carry both the expected and the actual
    // digest so an operator can audit which side drifted.
    let lib_body: &[u8] = b"pub fn ok() {}";
    let (bytes, actual_digest) = make_crate("alpha-0.1.0", &[("src/lib.rs", lib_body)]);
    let bogus = "deadbeef0000000000000000000000000000000000000000000000000000000000";

    let idx = DualFixtureClient::new();
    idx.add_text(
        &format!("{INDEX_BASE}/al/ph/alpha"),
        sparse_record("alpha", "0.1.0", Some("MIT"), bogus, false),
    );
    let tar_mock = DualFixtureClient::new();
    tar_mock.add_bytes(
        &format!("{STATIC_BASE}/crates/alpha/alpha-0.1.0.crate"),
        bytes,
    );

    let sparse = SparseIndexClient::new(idx).with_base_url(INDEX_BASE);
    let tarball = TarballClient::new(tar_mock).with_base_url(STATIC_BASE);

    let tmp = tempfile::tempdir().unwrap();
    let err = fetch_one(
        &sparse,
        &tarball,
        "alpha",
        tmp.path(),
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        None,
    )
    .expect_err("checksum mismatch must surface as FetchError, not be skipped");

    match err {
        FetchError::ChecksumMismatch { expected, actual } => {
            assert_eq!(expected, bogus);
            assert_eq!(actual, actual_digest);
        }
        other => panic!("expected ChecksumMismatch, got {other:?}"),
    }
}

#[test]
fn case_b_mismatch_does_not_create_crate_dir() {
    // The integrity-bound side-effect contract: a checksum-mismatch
    // error must leave out_root pristine. The manifest invariant
    // (row exists iff directory exists) requires that errored
    // fetches leave neither.
    let lib_body: &[u8] = b"pub fn ok() {}";
    let (bytes, _good_digest) = make_crate("beta-0.2.0", &[("src/lib.rs", lib_body)]);

    let idx = DualFixtureClient::new();
    idx.add_text(
        &format!("{INDEX_BASE}/be/ta/beta"),
        sparse_record(
            "beta",
            "0.2.0",
            Some("MIT"),
            "deadbeef0000000000000000000000000000000000000000000000000000000000",
            false,
        ),
    );
    let tar_mock = DualFixtureClient::new();
    tar_mock.add_bytes(
        &format!("{STATIC_BASE}/crates/beta/beta-0.2.0.crate"),
        bytes,
    );

    let sparse = SparseIndexClient::new(idx).with_base_url(INDEX_BASE);
    let tarball = TarballClient::new(tar_mock).with_base_url(STATIC_BASE);

    let tmp = tempfile::tempdir().unwrap();
    let out_root = tmp.path();
    let err = fetch_one(
        &sparse,
        &tarball,
        "beta",
        out_root,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        None,
    )
    .expect_err("checksum mismatch must surface as FetchError");
    assert!(matches!(err, FetchError::ChecksumMismatch { .. }));

    assert!(
        !out_root.join("beta-0.2.0").exists(),
        "fetch_one must not create the crate dir on a checksum mismatch",
    );
    let entries: Vec<_> = std::fs::read_dir(out_root).unwrap().collect();
    assert!(
        entries.is_empty(),
        "out_root must be empty after a checksum mismatch; found {entries:?}",
    );
}

#[test]
fn case_c_uppercase_index_cksum_matches_lowercase_actual_digest() {
    // tarball.rs uses `eq_ignore_ascii_case` for the cksum compare.
    // An internal mirror that publishes uppercase hex on the index
    // side must not produce spurious mismatches against the
    // lowercase digest cntrdct computes locally.
    let lib_body: &[u8] = b"pub fn ok() {}";
    let (bytes, actual_lower) = make_crate("gamma-0.3.0", &[("src/lib.rs", lib_body)]);
    let actual_upper = actual_lower.to_ascii_uppercase();
    assert_ne!(
        actual_lower, actual_upper,
        "test corpus must produce a digest whose upper- and lowercase forms differ",
    );

    let idx = DualFixtureClient::new();
    idx.add_text(
        &format!("{INDEX_BASE}/ga/mm/gamma"),
        sparse_record("gamma", "0.3.0", Some("MIT"), &actual_upper, false),
    );
    let tar_mock = DualFixtureClient::new();
    tar_mock.add_bytes(
        &format!("{STATIC_BASE}/crates/gamma/gamma-0.3.0.crate"),
        bytes,
    );

    let sparse = SparseIndexClient::new(idx).with_base_url(INDEX_BASE);
    let tarball = TarballClient::new(tar_mock).with_base_url(STATIC_BASE);

    let tmp = tempfile::tempdir().unwrap();
    fetch_one(
        &sparse,
        &tarball,
        "gamma",
        tmp.path(),
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        None,
    )
    .expect("uppercase index cksum must not produce a mismatch");

    assert!(
        tmp.path().join("gamma-0.3.0").exists(),
        "the case-insensitive match path must complete extraction normally",
    );
}
