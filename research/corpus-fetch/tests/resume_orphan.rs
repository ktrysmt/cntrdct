//! Integration tests for orphan recovery: when a previous `fetch_one`
//! run crashed between extracting the tarball and the caller's manifest
//! append, the next run against the same `out_dir` must complete
//! cleanly. The caller can then append the missing manifest row,
//! recovering the audit-trail entry that the crash dropped.
//!
//! `manifest::append_row`'s docstring promises that the manifest stays
//! well-formed even if the run aborts midway. The dual property — that
//! `fetch_one` itself can re-extract over a populated `<crate>-<version>/`
//! directory without erroring — is asserted here so a regression in
//! either `fetch_one` or `extract_filtered` does not silently leave
//! orphan extraction directories unrecoverable.

use cntrdct_corpus_fetch::{
    append_row, fetch_one, read_manifest_names, ExtractOptions, FetchOutcome, SparseIndexClient,
    TarballClient, DEFAULT_LICENSE_ALLOWLIST,
};

mod common;
use common::{make_crate, sparse_record, DualFixtureClient, INDEX_BASE, STATIC_BASE};

#[test]
fn orphan_extract_dir_recovers_cleanly_and_appends_manifest_row() {
    let new_lib: &[u8] = b"pub fn answer() -> u32 { 42 }";
    let new_file: &[u8] = b"// new file shipped this version";
    let (bytes, digest) = make_crate(
        "foo-0.1.0",
        &[("src/lib.rs", new_lib), ("src/new_file.rs", new_file)],
    );

    let idx = DualFixtureClient::new();
    idx.add_text(
        &format!("{INDEX_BASE}/3/f/foo"),
        sparse_record("foo", "0.1.0", Some("MIT"), &digest, false),
    );
    let tar_mock = DualFixtureClient::new();
    tar_mock.add_bytes(&format!("{STATIC_BASE}/crates/foo/foo-0.1.0.crate"), bytes);

    let sparse = SparseIndexClient::new(idx).with_base_url(INDEX_BASE);
    let tarball = TarballClient::new(tar_mock).with_base_url(STATIC_BASE);

    // Simulate the "process crashed after extract, before manifest
    // append" state: the crate directory exists with a stale lib.rs and
    // an orphan file from a hypothetical earlier extraction of `foo`.
    let tmp = tempfile::tempdir().unwrap();
    let out_root = tmp.path();
    let crate_dir = out_root.join("foo-0.1.0");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::write(
        crate_dir.join("src/lib.rs"),
        b"pub fn answer() -> u32 { 0 } // stale".as_slice(),
    )
    .unwrap();
    let orphan_body: &[u8] = b"// shipped in a previous extraction, not in this tarball";
    std::fs::write(crate_dir.join("src/orphan.rs"), orphan_body).unwrap();

    // Manifest is empty: the row that would have documented foo-0.1.0
    // was lost when the previous run crashed.
    let manifest_path = out_root.join("manifest.csv");
    assert!(read_manifest_names(&manifest_path).unwrap().is_empty());

    // (a) Action: re-run fetch_one. It must complete cleanly even
    // though the crate directory already has files.
    let result = fetch_one(
        &sparse,
        &tarball,
        "foo",
        out_root,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        None,
    )
    .expect("orphan dir must not cause fetch_one to error");

    let row = match result {
        FetchOutcome::Fetched {
            row,
            extract_report,
        } => {
            assert_eq!(row.name, "foo");
            assert_eq!(row.version, "0.1.0");
            assert_eq!(row.sha256, digest);
            assert_eq!(extract_report.kept, 2, "tarball ships 2 .rs files");
            row
        }
        other => panic!("expected Fetched, got {other:?}"),
    };

    // (c) Pre-existing files matching tarball entries are overwritten
    // with the new contents — std::fs::write replaces, never appends.
    assert_eq!(
        std::fs::read(crate_dir.join("src/lib.rs")).unwrap(),
        new_lib
    );
    assert_eq!(
        std::fs::read(crate_dir.join("src/new_file.rs")).unwrap(),
        new_file
    );

    // The orphan file from the previous extraction is left on disk;
    // current semantics of `extract_filtered` do not prune. Asserted
    // explicitly so a future change (e.g. a `--clean-orphans` flag)
    // forces revisit of this test rather than silently changing the
    // recovery contract.
    assert_eq!(
        std::fs::read(crate_dir.join("src/orphan.rs")).unwrap(),
        orphan_body,
        "extract_filtered does not prune orphan files in v0",
    );

    // (b) The caller appends the manifest row recovered from this
    // pass; the audit-trail entry the crash dropped is now present.
    append_row(&manifest_path, &row).unwrap();
    let names = read_manifest_names(&manifest_path).unwrap();
    assert!(
        names.contains("foo"),
        "manifest must contain the recovered crate"
    );
}

#[test]
fn fetch_one_is_idempotent_when_run_twice_against_same_out_dir() {
    let lib_body: &[u8] = b"pub fn ok() {}";
    let (bytes, digest) = make_crate("bar-0.2.0", &[("src/lib.rs", lib_body)]);

    let make_clients = || {
        let idx = DualFixtureClient::new();
        idx.add_text(
            &format!("{INDEX_BASE}/3/b/bar"),
            sparse_record("bar", "0.2.0", Some("MIT"), &digest, false),
        );
        let tar_mock = DualFixtureClient::new();
        tar_mock.add_bytes(
            &format!("{STATIC_BASE}/crates/bar/bar-0.2.0.crate"),
            bytes.clone(),
        );
        (
            SparseIndexClient::new(idx).with_base_url(INDEX_BASE),
            TarballClient::new(tar_mock).with_base_url(STATIC_BASE),
        )
    };

    let tmp = tempfile::tempdir().unwrap();
    let out_root = tmp.path();

    let (sparse, tarball) = make_clients();
    let r1 = fetch_one(
        &sparse,
        &tarball,
        "bar",
        out_root,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        None,
    )
    .unwrap();
    assert!(matches!(r1, FetchOutcome::Fetched { .. }));
    let lib_first = std::fs::read(out_root.join("bar-0.2.0/src/lib.rs")).unwrap();
    assert_eq!(lib_first, lib_body);
    let dir_count_first = std::fs::read_dir(out_root.join("bar-0.2.0/src"))
        .unwrap()
        .count();

    let (sparse2, tarball2) = make_clients();
    let r2 = fetch_one(
        &sparse2,
        &tarball2,
        "bar",
        out_root,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        None,
    )
    .unwrap();
    assert!(matches!(r2, FetchOutcome::Fetched { .. }));
    let lib_second = std::fs::read(out_root.join("bar-0.2.0/src/lib.rs")).unwrap();
    assert_eq!(lib_second, lib_body);
    assert_eq!(
        lib_first.len(),
        lib_second.len(),
        "size unchanged: no duplicate or appended bytes",
    );

    let dir_count_second = std::fs::read_dir(out_root.join("bar-0.2.0/src"))
        .unwrap()
        .count();
    assert_eq!(
        dir_count_first, dir_count_second,
        "no extra files should appear on second run",
    );
}
