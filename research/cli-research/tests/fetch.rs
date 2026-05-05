//! Integration tests for `cntrdct fetch`.
//!
//! These exercise the parts of the orchestration that do not require
//! network access: empty / comment-only crate lists, output-directory
//! creation, and CSV header initialisation. The full happy-path with
//! tarball download is covered by the orchestrator tests in
//! `cntrdct-corpus-fetch::fetcher::tests`, which use mock HTTP clients.

use cntrdct_research::{run_fetch, run_rank, FetchProgress};
use cntrdct_corpus_fetch::{ExtractOptions, DEFAULT_LICENSE_ALLOWLIST};

#[test]
fn empty_crate_list_returns_zero_summary_and_creates_out_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let list = tmp.path().join("crates.txt");
    std::fs::write(&list, "").unwrap();
    let out = tmp.path().join("corpus/wild");

    let summary = run_fetch(
        &list,
        &out,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        1,
        false,
        FetchProgress::Text,
    )
    .expect("empty list should not error");

    assert_eq!(summary.fetched, 0);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.errors, 0);
    assert!(out.exists(), "out dir should be created up front");
    // Manifest is created lazily on the first fetched row, so it should not
    // exist yet for an empty input.
    assert!(!out.join("manifest.csv").exists());
}

#[test]
fn comments_and_blank_lines_are_skipped_in_crate_list() {
    let tmp = tempfile::tempdir().unwrap();
    let list = tmp.path().join("crates.txt");
    std::fs::write(
        &list,
        "# this is a comment\n\
         \n\
         # another comment with whitespace below\n\
         \n",
    )
    .unwrap();
    let out = tmp.path().join("corpus");

    let summary = run_fetch(
        &list,
        &out,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        1,
        false,
        FetchProgress::Text,
    )
    .expect("comment-only list should not error");
    assert_eq!(summary.fetched, 0);
    assert_eq!(summary.skipped, 0);
}

#[test]
fn missing_crate_list_path_surfaces_read_error() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("does-not-exist.txt");
    let out = tmp.path().join("corpus");

    let err = run_fetch(
        &missing,
        &out,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        1,
        false,
        FetchProgress::Text,
    )
    .expect_err("missing list should error");
    let msg = format!("{err}");
    assert!(
        msg.contains("could not read crates list"),
        "expected ReadList error, got: {msg}"
    );
}

#[test]
fn resume_filters_out_crates_already_in_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("corpus");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join("manifest.csv"),
        "crate,version,license,downloads,sha256\n\
         serde,1.0.2,MIT OR Apache-2.0,,abc\n\
         log,0.4.20,MIT OR Apache-2.0,,def\n",
    )
    .unwrap();
    let list = tmp.path().join("crates.txt");
    // Both entries are already in the manifest, so resume should drop them
    // both before any network call is made.
    std::fs::write(&list, "serde\nlog\n").unwrap();

    let summary = run_fetch(
        &list,
        &out,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        1,
        true,
        FetchProgress::Text,
    )
    .unwrap();
    assert_eq!(summary.fetched, 0);
    assert_eq!(summary.resume_skipped, 2);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.errors, 0);
}

#[test]
fn ndjson_progress_emits_parseable_json_lines_on_resume_path() {
    // Spawn the binary so we exercise the real stderr stream. Using the
    // resume-only code path keeps the test offline: every input crate is
    // already in the manifest, so the binary emits one `resume_skip`
    // event per entry and never touches the network.
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("corpus");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join("manifest.csv"),
        "crate,version,license,downloads,sha256\n\
         serde,1.0.2,MIT,,abc\n\
         log,0.4.20,MIT,,def\n",
    )
    .unwrap();
    let list = tmp.path().join("crates.txt");
    std::fs::write(&list, "serde\nlog\n").unwrap();

    let bin = env!("CARGO_BIN_EXE_cntrdct-research");
    let output = std::process::Command::new(bin)
        .args([
            "fetch",
            list.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--resume",
            "--progress",
            "ndjson",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "cntrdct-research fetch exited non-zero");

    let stderr = String::from_utf8(output.stderr).unwrap();
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "expected one event per input, got: {stderr}"
    );
    for (line, expected_name) in lines.iter().zip(["serde", "log"]) {
        let parsed: serde_json::Value =
            serde_json::from_str(line).expect("each progress line must be valid JSON");
        assert_eq!(parsed["event"], "resume_skip");
        assert_eq!(parsed["name"], expected_name);
    }
}

#[test]
fn provenance_json_records_rank_source_when_metadata_present() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("corpus");
    std::fs::create_dir_all(&out).unwrap();
    let list = tmp.path().join("crates.txt");
    // Synthesise the crate list as if `cntrdct rank` had emitted it,
    // including the comment-line provenance pin.
    std::fs::write(
        &list,
        "# generated by `cntrdct rank`\n\
         # dump-timestamp: 2026-04-15T00:00:00Z\n\
         # dump-commit-hash: abc123def\n\
         # columns: name downloads\n",
    )
    .unwrap();

    let summary = run_fetch(
        &list,
        &out,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        1,
        false,
        FetchProgress::Text,
    )
    .unwrap();
    assert_eq!(summary.fetched, 0);

    let body = std::fs::read_to_string(out.join("provenance.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["rank_source"]["dump_timestamp"], "2026-04-15T00:00:00Z");
    assert_eq!(v["rank_source"]["dump_commit_hash"], "abc123def");
    assert!(v["fetched_at_unix"].is_number());
    assert!(v["cntrdct_corpus_fetch_version"].is_string());
    assert_eq!(v["fetch_summary"]["fetched"], 0);
}

#[test]
fn provenance_json_omits_rank_source_when_no_metadata_in_list() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("corpus");
    std::fs::create_dir_all(&out).unwrap();
    let list = tmp.path().join("crates.txt");
    std::fs::write(&list, "# manually authored list\n").unwrap();

    run_fetch(
        &list,
        &out,
        DEFAULT_LICENSE_ALLOWLIST,
        &ExtractOptions::default(),
        1,
        false,
        FetchProgress::Text,
    )
    .unwrap();

    let body = std::fs::read_to_string(out.join("provenance.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(
        v.get("rank_source").is_none(),
        "rank_source should be absent"
    );
}

#[test]
fn run_rank_writes_crate_list_to_output_file() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::{Builder, Header};

    fn append(tar: &mut Builder<GzEncoder<Vec<u8>>>, path: &str, body: &str) {
        let mut h = Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, path, body.as_bytes()).unwrap();
    }

    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = Builder::new(gz);
    append(
        &mut tar,
        "2026-04-15-000000/metadata.json",
        "{\"timestamp\":\"2026-04-15T00:00:00Z\",\"crates_io_commit_hash\":\"abc123\"}",
    );
    append(
        &mut tar,
        "2026-04-15-000000/data/crates.csv",
        "id,name,description\n1,serde,\n2,log,\n3,rand,\n",
    );
    append(
        &mut tar,
        "2026-04-15-000000/data/crate_downloads.csv",
        "crate_id,downloads\n1,80000000\n2,30000000\n3,50000000\n",
    );
    let archive = tar.into_inner().unwrap().finish().unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let dump_path = tmp.path().join("db-dump.tar.gz");
    std::fs::write(&dump_path, &archive).unwrap();
    let output = tmp.path().join("crates.txt");

    run_rank(&dump_path, 2, Some(&output)).unwrap();
    let body = std::fs::read_to_string(&output).unwrap();
    // Metadata comment lines must be present for downstream provenance.
    assert!(
        body.contains("# dump-timestamp: 2026-04-15T00:00:00Z"),
        "missing dump-timestamp: {body}"
    );
    assert!(
        body.contains("# dump-commit-hash: abc123"),
        "missing dump-commit-hash: {body}"
    );
    let lines: Vec<&str> = body.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "serde 80000000");
    assert_eq!(lines[1], "rand 50000000");
}

#[test]
fn run_rank_writes_sidecar_licenses_tsv_alongside_output() {
    // Real crates.io sparse index does not expose a license field, so
    // `cntrdct rank` is the only chance to capture licenses up-front from
    // the dump. The fetcher reads the sibling `<output>.licenses.tsv` to
    // inject the values it could not get from the index.
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::{Builder, Header};

    fn append(tar: &mut Builder<GzEncoder<Vec<u8>>>, path: &str, body: &str) {
        let mut h = Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, path, body.as_bytes()).unwrap();
    }

    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = Builder::new(gz);
    append(
        &mut tar,
        "2026-05-04-000000/data/crates.csv",
        "id,name,description\n1,serde,\n2,log,\n",
    );
    append(
        &mut tar,
        "2026-05-04-000000/data/crate_downloads.csv",
        "crate_id,downloads\n1,80\n2,30\n",
    );
    append(
        &mut tar,
        "2026-05-04-000000/data/default_versions.csv",
        "crate_id,num_versions,version_id\n1,1,101\n2,1,102\n",
    );
    append(
        &mut tar,
        "2026-05-04-000000/data/versions.csv",
        "id,num,license\n101,1.0.0,\"MIT OR Apache-2.0\"\n102,0.4.0,\"MIT\"\n",
    );
    let archive = tar.into_inner().unwrap().finish().unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let dump_path = tmp.path().join("db-dump.tar.gz");
    std::fs::write(&dump_path, &archive).unwrap();
    let output = tmp.path().join("crates.txt");

    run_rank(&dump_path, 5, Some(&output)).unwrap();

    let sidecar = tmp.path().join("crates.txt.licenses.tsv");
    assert!(sidecar.exists(), "sidecar should be written next to output");
    let body = std::fs::read_to_string(&sidecar).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines[0], "name\tlicense", "header expected");
    let rest: std::collections::HashSet<&str> = lines[1..].iter().copied().collect();
    assert!(rest.contains("serde\tMIT OR Apache-2.0"), "got: {rest:?}");
    assert!(rest.contains("log\tMIT"), "got: {rest:?}");
    assert_eq!(rest.len(), 2);
}

#[test]
fn run_rank_writes_empty_sidecar_when_dump_lacks_version_tables() {
    // Older snapshots ship only crates.csv + crate_downloads.csv; the rank
    // path stays usable but the sidecar is just the header. This locks in
    // the contract so a future change does not skip writing the file.
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::{Builder, Header};

    fn append(tar: &mut Builder<GzEncoder<Vec<u8>>>, path: &str, body: &str) {
        let mut h = Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, path, body.as_bytes()).unwrap();
    }

    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = Builder::new(gz);
    append(
        &mut tar,
        "2026-04-15-000000/data/crates.csv",
        "id,name,description\n1,serde,\n",
    );
    append(
        &mut tar,
        "2026-04-15-000000/data/crate_downloads.csv",
        "crate_id,downloads\n1,80\n",
    );
    let archive = tar.into_inner().unwrap().finish().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let dump_path = tmp.path().join("db-dump.tar.gz");
    std::fs::write(&dump_path, &archive).unwrap();
    let output = tmp.path().join("crates.txt");

    run_rank(&dump_path, 5, Some(&output)).unwrap();
    let sidecar = tmp.path().join("crates.txt.licenses.tsv");
    let body = std::fs::read_to_string(&sidecar).unwrap();
    assert_eq!(body, "name\tlicense\n");
}
