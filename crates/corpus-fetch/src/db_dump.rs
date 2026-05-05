//! crates.io daily DB-dump parser and downloader.
//!
//! The crates.io project publishes a daily PostgreSQL CSV dump at
//! `https://static.crates.io/db-dump.tar.gz`. The archive is a gzip'd tar
//! whose entries live under a single timestamped directory:
//!
//! ```text
//! 2026-04-15-000000/metadata.json
//! 2026-04-15-000000/schema.sql
//! 2026-04-15-000000/data/crates.csv
//! 2026-04-15-000000/data/crate_downloads.csv
//! ...
//! ```
//!
//! For the empirical study we only need two tables:
//! - `crates.csv` — `id, name, ...` (full schema varies by dump version, but
//!   `id` and `name` have been stable for years).
//! - `crate_downloads.csv` — `crate_id, downloads` (lifetime download count
//!   per crate).
//!
//! Joining the two on `id` yields the (name, downloads) ranking that drives
//! the `cntrdct rank` subcommand. The dump weighs about 1 GB compressed and
//! is read in a single linear pass; per-pass memory peaks at the size of the
//! two CSV tables (a few hundred MB), which is the simplest design that
//! still works on a developer laptop. If the dump ever outgrows RAM, the
//! join can move to an on-disk hash with `sled` or `redb` without affecting
//! callers.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::sparse_index::{backoff_delay, retry_after_seconds};
use crate::FetchError;

pub const DEFAULT_DB_DUMP_URL: &str = "https://static.crates.io/db-dump.tar.gz";

/// Default retry budget for the dump download. The dump is ~1 GB; a single
/// transient failure mid-stream costs minutes of wasted bandwidth, so the
/// default is set low enough that callers fail fast rather than hammering
/// the static endpoint, but high enough that a flaky link or a brief 503
/// doesn't abort the whole pipeline.
pub const DEFAULT_DUMP_MAX_RETRIES: u32 = 3;

/// One row of the (crate name, lifetime downloads) ranking produced by
/// joining `crates.csv` against `crate_downloads.csv`. The `license` field
/// is the SPDX expression of the crate's default version, joined through
/// `default_versions.csv` × `versions.csv`. It is `None` when the dump
/// omits one of those tables (older snapshots) or when the row's license
/// column is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateRanking {
    pub name: String,
    pub downloads: u64,
    pub license: Option<String>,
}

/// Provenance metadata pulled from the dump's top-level `metadata.json`.
///
/// Both fields are optional because the dump format has shifted over the
/// years and we want corpus-fetch to keep reading older snapshots that may
/// lack one or the other key. A `None` `commit_hash` falls back to the
/// `timestamp` for reproducibility — the timestamp is sufficient to refetch
/// the same dump from crates.io's snapshot history.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct DumpMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,
}

/// Stream the dump archive at `archive_path`, parse the two relevant CSV
/// tables, and return the top `n` crates by lifetime downloads.
///
/// Ties on the downloads count are broken by ascending crate name so the
/// output is deterministic across runs.
pub fn read_top_n_from_archive(
    archive_path: &Path,
    n: usize,
) -> Result<Vec<CrateRanking>, FetchError> {
    let file = File::open(archive_path)?;
    let gz = GzDecoder::new(file);
    let mut archive = Archive::new(gz);

    let mut id_to_name: HashMap<u64, String> = HashMap::new();
    let mut id_to_downloads: HashMap<u64, u64> = HashMap::new();
    let mut crate_id_to_default_version: HashMap<u64, u64> = HashMap::new();
    let mut version_id_to_license: HashMap<u64, Option<String>> = HashMap::new();

    let entries = archive
        .entries()
        .map_err(|e| FetchError::Archive(e.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| FetchError::Archive(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| FetchError::Archive(e.to_string()))?
            .into_owned();
        let path_str = path.to_string_lossy();

        if path_str.ends_with("/data/crates.csv") || path_str == "data/crates.csv" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            parse_crates_csv(&buf, &mut id_to_name)?;
        } else if path_str.ends_with("/data/crate_downloads.csv")
            || path_str == "data/crate_downloads.csv"
        {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            parse_crate_downloads_csv(&buf, &mut id_to_downloads)?;
        } else if path_str.ends_with("/data/default_versions.csv")
            || path_str == "data/default_versions.csv"
        {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            parse_default_versions_csv(&buf, &mut crate_id_to_default_version)?;
        } else if path_str.ends_with("/data/versions.csv") || path_str == "data/versions.csv" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            parse_versions_licenses_csv(&buf, &mut version_id_to_license)?;
        }
    }

    if id_to_name.is_empty() {
        return Err(FetchError::Malformed(
            "db dump missing data/crates.csv".into(),
        ));
    }
    if id_to_downloads.is_empty() {
        return Err(FetchError::Malformed(
            "db dump missing data/crate_downloads.csv".into(),
        ));
    }

    // License is best-effort: an older dump or a hand-crafted fixture may
    // omit either of the version tables. In that case `license` stays None.
    let license_for = |crate_id: u64| -> Option<String> {
        crate_id_to_default_version
            .get(&crate_id)
            .and_then(|vid| version_id_to_license.get(vid).cloned())
            .flatten()
    };

    let mut combined: Vec<CrateRanking> = id_to_downloads
        .into_iter()
        .filter_map(|(id, downloads)| {
            id_to_name.remove(&id).map(|name| CrateRanking {
                name,
                downloads,
                license: license_for(id),
            })
        })
        .collect();
    combined.sort_by(|a, b| b.downloads.cmp(&a.downloads).then(a.name.cmp(&b.name)));
    combined.truncate(n);
    Ok(combined)
}

/// Read the dump's `metadata.json` (if any) and surface the recorded
/// timestamp and crates.io commit hash for provenance pinning.
///
/// Returns a default `DumpMetadata` (both fields `None`) when the archive
/// has no `metadata.json` entry; older dumps shipped without one. Schema
/// drift is handled by trying each of `crates_io_commit_hash`,
/// `commit_hash`, and `git_hash` in that order.
pub fn read_metadata_from_archive(archive_path: &Path) -> Result<DumpMetadata, FetchError> {
    let file = File::open(archive_path)?;
    let gz = GzDecoder::new(file);
    let mut archive = Archive::new(gz);

    let entries = archive
        .entries()
        .map_err(|e| FetchError::Archive(e.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| FetchError::Archive(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| FetchError::Archive(e.to_string()))?
            .into_owned();
        let path_str = path.to_string_lossy();
        if path_str.ends_with("/metadata.json") || path_str == "metadata.json" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return parse_metadata_json(&buf);
        }
    }
    Ok(DumpMetadata::default())
}

fn parse_metadata_json(bytes: &[u8]) -> Result<DumpMetadata, FetchError> {
    let v: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| FetchError::Malformed(format!("metadata.json: {e}")))?;
    let timestamp = v
        .get("timestamp")
        .and_then(|s| s.as_str())
        .map(String::from);
    let commit_hash = ["crates_io_commit_hash", "commit_hash", "git_hash"]
        .iter()
        .find_map(|k| v.get(*k).and_then(|s| s.as_str()).map(String::from));
    Ok(DumpMetadata {
        timestamp,
        commit_hash,
    })
}

fn parse_crates_csv(bytes: &[u8], out: &mut HashMap<u64, String>) -> Result<(), FetchError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|e| FetchError::Malformed(format!("crates.csv: {e}")))?
        .clone();
    let id_col = column_index(&headers, "id")?;
    let name_col = column_index(&headers, "name")?;
    for record in reader.records() {
        let record = record.map_err(|e| FetchError::Malformed(format!("crates.csv row: {e}")))?;
        let id_raw = record
            .get(id_col)
            .ok_or_else(|| FetchError::Malformed("crates.csv: short row at id".into()))?;
        let id: u64 = id_raw.parse().map_err(|_| {
            FetchError::Malformed(format!("crates.csv: id `{id_raw}` is not a u64"))
        })?;
        let name = record
            .get(name_col)
            .ok_or_else(|| FetchError::Malformed("crates.csv: short row at name".into()))?
            .to_string();
        out.insert(id, name);
    }
    Ok(())
}

fn parse_crate_downloads_csv(bytes: &[u8], out: &mut HashMap<u64, u64>) -> Result<(), FetchError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|e| FetchError::Malformed(format!("crate_downloads.csv: {e}")))?
        .clone();
    let id_col = column_index(&headers, "crate_id")?;
    let downloads_col = column_index(&headers, "downloads")?;
    for record in reader.records() {
        let record =
            record.map_err(|e| FetchError::Malformed(format!("crate_downloads.csv row: {e}")))?;
        let id_raw = record.get(id_col).ok_or_else(|| {
            FetchError::Malformed("crate_downloads.csv: short row at crate_id".into())
        })?;
        let id: u64 = id_raw.parse().map_err(|_| {
            FetchError::Malformed(format!(
                "crate_downloads.csv: crate_id `{id_raw}` is not a u64"
            ))
        })?;
        let dl_raw = record.get(downloads_col).ok_or_else(|| {
            FetchError::Malformed("crate_downloads.csv: short row at downloads".into())
        })?;
        let downloads: u64 = dl_raw.parse().map_err(|_| {
            FetchError::Malformed(format!(
                "crate_downloads.csv: downloads `{dl_raw}` is not a u64"
            ))
        })?;
        out.insert(id, downloads);
    }
    Ok(())
}

fn parse_default_versions_csv(bytes: &[u8], out: &mut HashMap<u64, u64>) -> Result<(), FetchError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|e| FetchError::Malformed(format!("default_versions.csv: {e}")))?
        .clone();
    let crate_id_col = column_index(&headers, "crate_id")?;
    let version_id_col = column_index(&headers, "version_id")?;
    for record in reader.records() {
        let record =
            record.map_err(|e| FetchError::Malformed(format!("default_versions.csv row: {e}")))?;
        let crate_id_raw = record.get(crate_id_col).ok_or_else(|| {
            FetchError::Malformed("default_versions.csv: short row at crate_id".into())
        })?;
        let crate_id: u64 = crate_id_raw.parse().map_err(|_| {
            FetchError::Malformed(format!(
                "default_versions.csv: crate_id `{crate_id_raw}` is not a u64"
            ))
        })?;
        let version_id_raw = record.get(version_id_col).ok_or_else(|| {
            FetchError::Malformed("default_versions.csv: short row at version_id".into())
        })?;
        let version_id: u64 = version_id_raw.parse().map_err(|_| {
            FetchError::Malformed(format!(
                "default_versions.csv: version_id `{version_id_raw}` is not a u64"
            ))
        })?;
        out.insert(crate_id, version_id);
    }
    Ok(())
}

fn parse_versions_licenses_csv(
    bytes: &[u8],
    out: &mut HashMap<u64, Option<String>>,
) -> Result<(), FetchError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|e| FetchError::Malformed(format!("versions.csv: {e}")))?
        .clone();
    let id_col = column_index(&headers, "id")?;
    let license_col = column_index(&headers, "license")?;
    for record in reader.records() {
        let record = record.map_err(|e| FetchError::Malformed(format!("versions.csv row: {e}")))?;
        let id_raw = record
            .get(id_col)
            .ok_or_else(|| FetchError::Malformed("versions.csv: short row at id".into()))?;
        let id: u64 = id_raw.parse().map_err(|_| {
            FetchError::Malformed(format!("versions.csv: id `{id_raw}` is not a u64"))
        })?;
        let lic_raw = record
            .get(license_col)
            .ok_or_else(|| FetchError::Malformed("versions.csv: short row at license".into()))?;
        let license = if lic_raw.trim().is_empty() {
            None
        } else {
            Some(lic_raw.to_string())
        };
        out.insert(id, license);
    }
    Ok(())
}

fn column_index(headers: &csv::StringRecord, name: &str) -> Result<usize, FetchError> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| FetchError::Malformed(format!("missing column: {name}")))
}

/// Stream the dump tarball from `url` to `out_path` without buffering it in
/// memory. The dump is ~1 GB; reading it whole would be wasteful even when
/// it fits.
///
/// Retries on transient HTTP failures (429, 503, 504, connect / timeout
/// errors, mid-stream body interruptions) with exponential backoff.
/// `Retry-After` headers are honoured. Permanent failures (404, other 4xx)
/// short-circuit. Each retry deletes the partial output file before
/// restarting from byte zero — `Range`-based resume is a deliberate
/// non-goal until corpus runs hit the multi-attempt threshold often enough
/// to justify the complexity.
///
/// Lives outside the [`crate::HttpClient`] trait on purpose — adding a
/// streaming method to the trait would force every mock in the suite to
/// grow a method none of the other tests need. The CLI calls this helper
/// directly with a `reqwest::blocking::Client`; tests skip it and instead
/// drive [`read_top_n_from_archive`] against a fixture archive on disk.
pub fn download_dump_streaming(url: &str, out_path: &Path) -> Result<u64, FetchError> {
    download_dump_streaming_with_retry(
        url,
        out_path,
        DEFAULT_DUMP_MAX_RETRIES,
        crate::sparse_index::DEFAULT_BASE_DELAY,
    )
}

/// Variant of [`download_dump_streaming`] with a configurable retry policy.
///
/// Tests pass a sub-second `base_delay` to keep the suite fast; production
/// uses the 1-second default to stay polite on the static endpoint. Setting
/// `max_retries == 0` disables retry and matches the previous single-shot
/// behaviour.
pub fn download_dump_streaming_with_retry(
    url: &str,
    out_path: &Path,
    max_retries: u32,
    base_delay: Duration,
) -> Result<u64, FetchError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60 * 30))
        .user_agent(concat!(
            "cntrdct-corpus-fetch/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/ktrysmt/cntrdct)"
        ))
        .build()
        .map_err(|e| FetchError::Http(e.to_string()))?;

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut attempt: u32 = 0;
    loop {
        match try_download_once(&client, url, out_path) {
            Ok(bytes) => return Ok(bytes),
            Err(DumpAttemptError::Permanent(err)) => return Err(err),
            Err(DumpAttemptError::Transient { error, retry_after }) => {
                if attempt >= max_retries {
                    return Err(error);
                }
                // Drop any partial output so the next attempt starts clean.
                // Ignore the unlink result: a missing file (the request
                // failed before File::create succeeded) is the success case.
                let _ = std::fs::remove_file(out_path);
                let delay = retry_after
                    .map(Duration::from_secs)
                    .unwrap_or_else(|| backoff_delay(attempt, base_delay));
                std::thread::sleep(delay);
                attempt += 1;
            }
        }
    }
}

enum DumpAttemptError {
    Permanent(FetchError),
    Transient {
        error: FetchError,
        retry_after: Option<u64>,
    },
}

fn try_download_once(
    client: &reqwest::blocking::Client,
    url: &str,
    out_path: &Path,
) -> Result<u64, DumpAttemptError> {
    let send_result = client.get(url).send();
    let mut resp = match send_result {
        Ok(resp) => resp,
        Err(e) if e.is_timeout() || e.is_connect() => {
            return Err(DumpAttemptError::Transient {
                error: FetchError::Http(e.to_string()),
                retry_after: None,
            });
        }
        Err(e) => {
            return Err(DumpAttemptError::Permanent(FetchError::Http(e.to_string())));
        }
    };

    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(DumpAttemptError::Permanent(FetchError::NotFound(
            url.to_string(),
        )));
    }
    let status_code = status.as_u16();
    if matches!(status_code, 429 | 503 | 504) {
        let retry_after = retry_after_seconds(&resp);
        return Err(DumpAttemptError::Transient {
            error: FetchError::Http(format!("status {status} from {url}")),
            retry_after,
        });
    }
    if !status.is_success() {
        return Err(DumpAttemptError::Permanent(FetchError::Http(format!(
            "status {status} from {url}"
        ))));
    }

    let mut file = match File::create(out_path) {
        Ok(f) => f,
        Err(e) => return Err(DumpAttemptError::Permanent(FetchError::from(e))),
    };
    match resp.copy_to(&mut file) {
        Ok(bytes) => Ok(bytes),
        // A mid-stream interruption surfaces here; treat as transient so the
        // retry loop can clean up the partial file and try again.
        Err(e) => Err(DumpAttemptError::Transient {
            error: FetchError::Http(e.to_string()),
            retry_after: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::{Builder, Header};

    /// Build a fake db-dump tar.gz with the two relevant CSVs under a fixed
    /// timestamped prefix. `crates_rows` is `(id, name)`,
    /// `downloads_rows` is `(crate_id, downloads)`.
    fn make_dump_archive(
        prefix: &str,
        crates_rows: &[(u64, &str)],
        downloads_rows: &[(u64, u64)],
    ) -> Vec<u8> {
        let mut crates_csv = String::from("id,name,description\n");
        for (id, name) in crates_rows {
            crates_csv.push_str(&format!("{id},{name},\n"));
        }
        let mut downloads_csv = String::from("crate_id,downloads\n");
        for (id, dl) in downloads_rows {
            downloads_csv.push_str(&format!("{id},{dl}\n"));
        }

        let buf = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut tar = Builder::new(gz);

        let crates_path = format!("{prefix}/data/crates.csv");
        let mut h = Header::new_gnu();
        h.set_size(crates_csv.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, &crates_path, crates_csv.as_bytes())
            .unwrap();

        let downloads_path = format!("{prefix}/data/crate_downloads.csv");
        let mut h = Header::new_gnu();
        h.set_size(downloads_csv.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, &downloads_path, downloads_csv.as_bytes())
            .unwrap();

        tar.into_inner().unwrap().finish().unwrap()
    }

    fn write_archive_to_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), bytes).unwrap();
        tmp
    }

    #[test]
    fn top_n_returns_crates_in_descending_downloads_order() {
        let archive = make_dump_archive(
            "2026-04-15-000000",
            &[(1, "serde"), (2, "log"), (3, "rand"), (4, "tokio")],
            &[
                (1, 80_000_000),
                (2, 50_000_000),
                (3, 30_000_000),
                (4, 100_000_000),
            ],
        );
        let tmp = write_archive_to_temp(&archive);

        let top = read_top_n_from_archive(tmp.path(), 3).unwrap();
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].name, "tokio");
        assert_eq!(top[0].downloads, 100_000_000);
        assert_eq!(top[1].name, "serde");
        assert_eq!(top[2].name, "log");
    }

    #[test]
    fn top_n_smaller_than_n_returns_everything_sorted() {
        let archive = make_dump_archive(
            "2026-04-15-000000",
            &[(1, "a"), (2, "b")],
            &[(1, 10), (2, 20)],
        );
        let tmp = write_archive_to_temp(&archive);

        let top = read_top_n_from_archive(tmp.path(), 100).unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "b");
        assert_eq!(top[1].name, "a");
    }

    #[test]
    fn top_n_breaks_ties_by_name_ascending() {
        let archive = make_dump_archive(
            "2026-04-15-000000",
            &[(1, "zzz"), (2, "aaa"), (3, "mmm")],
            &[(1, 50), (2, 50), (3, 50)],
        );
        let tmp = write_archive_to_temp(&archive);

        let top = read_top_n_from_archive(tmp.path(), 3).unwrap();
        assert_eq!(top[0].name, "aaa");
        assert_eq!(top[1].name, "mmm");
        assert_eq!(top[2].name, "zzz");
    }

    #[test]
    fn top_n_skips_crates_missing_a_downloads_row() {
        let archive = make_dump_archive(
            "2026-04-15-000000",
            &[(1, "has_dl"), (2, "no_dl")],
            &[(1, 100)],
        );
        let tmp = write_archive_to_temp(&archive);

        let top = read_top_n_from_archive(tmp.path(), 10).unwrap();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].name, "has_dl");
    }

    /// Build a fake db-dump archive that *also* includes `default_versions.csv`
    /// and `versions.csv`, so the license-join path is exercised end-to-end.
    fn make_dump_archive_with_licenses(
        prefix: &str,
        crates_rows: &[(u64, &str)],
        downloads_rows: &[(u64, u64)],
        // (crate_id, version_id)
        default_versions: &[(u64, u64)],
        // (version_id, license_str)
        versions_licenses: &[(u64, &str)],
    ) -> Vec<u8> {
        let mut crates_csv = String::from("id,name,description\n");
        for (id, name) in crates_rows {
            crates_csv.push_str(&format!("{id},{name},\n"));
        }
        let mut downloads_csv = String::from("crate_id,downloads\n");
        for (id, dl) in downloads_rows {
            downloads_csv.push_str(&format!("{id},{dl}\n"));
        }
        let mut default_csv = String::from("crate_id,num_versions,version_id\n");
        for (cid, vid) in default_versions {
            default_csv.push_str(&format!("{cid},1,{vid}\n"));
        }
        // Real versions.csv is wide; we only need `id` and `license` columns
        // to be parseable, so emit them with a dummy `num` between.
        let mut versions_csv = String::from("id,num,license\n");
        for (vid, lic) in versions_licenses {
            versions_csv.push_str(&format!("{vid},0.0.0,\"{lic}\"\n"));
        }

        let buf = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut tar = Builder::new(gz);
        for (path_suffix, body) in [
            ("data/crates.csv", crates_csv.as_bytes()),
            ("data/crate_downloads.csv", downloads_csv.as_bytes()),
            ("data/default_versions.csv", default_csv.as_bytes()),
            ("data/versions.csv", versions_csv.as_bytes()),
        ] {
            let mut h = Header::new_gnu();
            h.set_size(body.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            tar.append_data(&mut h, format!("{prefix}/{path_suffix}"), body)
                .unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn license_is_joined_via_default_versions_and_versions_csv() {
        let archive = make_dump_archive_with_licenses(
            "2026-05-04-000000",
            &[(1, "syn"), (2, "log"), (3, "tokio"), (4, "no_default")],
            &[(1, 30), (2, 20), (3, 40), (4, 10)],
            // crate 4 has no default_version row -> license stays None
            &[(1, 101), (2, 102), (3, 103)],
            // Empty license string for log -> None despite a default version row
            &[(101, "MIT OR Apache-2.0"), (102, ""), (103, "MIT")],
        );
        let tmp = write_archive_to_temp(&archive);

        let top = read_top_n_from_archive(tmp.path(), 10).unwrap();
        let by_name: HashMap<&str, &CrateRanking> =
            top.iter().map(|r| (r.name.as_str(), r)).collect();
        assert_eq!(by_name["syn"].license.as_deref(), Some("MIT OR Apache-2.0"));
        assert_eq!(by_name["tokio"].license.as_deref(), Some("MIT"));
        assert_eq!(by_name["log"].license, None, "empty string -> None");
        assert_eq!(
            by_name["no_default"].license, None,
            "no default_versions row -> None"
        );
    }

    #[test]
    fn license_stays_none_when_version_tables_are_absent() {
        // Original `make_dump_archive` only ships crates.csv + crate_downloads.csv
        // (older snapshot shape). License must be None across the board.
        let archive = make_dump_archive(
            "2026-04-15-000000",
            &[(1, "syn"), (2, "log")],
            &[(1, 100), (2, 50)],
        );
        let tmp = write_archive_to_temp(&archive);
        let top = read_top_n_from_archive(tmp.path(), 5).unwrap();
        assert!(top.iter().all(|r| r.license.is_none()));
    }

    #[test]
    fn missing_crates_csv_is_a_malformed_error() {
        let buf = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut tar = Builder::new(gz);
        let body = "crate_id,downloads\n1,10\n";
        let mut h = Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(
            &mut h,
            "2026-04-15-000000/data/crate_downloads.csv",
            body.as_bytes(),
        )
        .unwrap();
        let archive = tar.into_inner().unwrap().finish().unwrap();

        let tmp = write_archive_to_temp(&archive);
        let err = read_top_n_from_archive(tmp.path(), 5).unwrap_err();
        match err {
            FetchError::Malformed(m) => assert!(m.contains("crates.csv"), "got: {m}"),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn missing_crate_downloads_csv_is_a_malformed_error() {
        let buf = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut tar = Builder::new(gz);
        let body = "id,name\n1,foo\n";
        let mut h = Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, "2026-04-15-000000/data/crates.csv", body.as_bytes())
            .unwrap();
        let archive = tar.into_inner().unwrap().finish().unwrap();

        let tmp = write_archive_to_temp(&archive);
        let err = read_top_n_from_archive(tmp.path(), 5).unwrap_err();
        match err {
            FetchError::Malformed(m) => assert!(m.contains("crate_downloads.csv"), "got: {m}"),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn missing_required_column_in_crates_csv_is_an_error() {
        let buf = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut tar = Builder::new(gz);
        // Header lacks `name`.
        let body = "id,description\n1,foo\n";
        let mut h = Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, "2026-04-15-000000/data/crates.csv", body.as_bytes())
            .unwrap();
        let dl_body = "crate_id,downloads\n1,10\n";
        let mut h = Header::new_gnu();
        h.set_size(dl_body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(
            &mut h,
            "2026-04-15-000000/data/crate_downloads.csv",
            dl_body.as_bytes(),
        )
        .unwrap();
        let archive = tar.into_inner().unwrap().finish().unwrap();

        let tmp = write_archive_to_temp(&archive);
        let err = read_top_n_from_archive(tmp.path(), 5).unwrap_err();
        match err {
            FetchError::Malformed(m) => assert!(m.contains("missing column: name"), "got: {m}"),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    fn append_to_archive(tar: &mut Builder<GzEncoder<Vec<u8>>>, path: &str, body: &[u8]) {
        let mut h = Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append_data(&mut h, path, body).unwrap();
    }

    #[test]
    fn metadata_returns_timestamp_and_commit_hash() {
        let buf = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut tar = Builder::new(gz);
        append_to_archive(
            &mut tar,
            "2026-04-15-000000/metadata.json",
            br#"{"timestamp":"2026-04-15T00:00:00Z","crates_io_commit_hash":"abc123def"}"#,
        );
        let archive = tar.into_inner().unwrap().finish().unwrap();
        let tmp = write_archive_to_temp(&archive);
        let meta = read_metadata_from_archive(tmp.path()).unwrap();
        assert_eq!(meta.timestamp.as_deref(), Some("2026-04-15T00:00:00Z"));
        assert_eq!(meta.commit_hash.as_deref(), Some("abc123def"));
    }

    #[test]
    fn metadata_falls_back_through_alternative_commit_keys() {
        for key in ["commit_hash", "git_hash"] {
            let buf = Vec::new();
            let gz = GzEncoder::new(buf, Compression::default());
            let mut tar = Builder::new(gz);
            let body = format!("{{\"timestamp\":\"t\",\"{key}\":\"deadbeef\"}}");
            append_to_archive(&mut tar, "2026-04-15-000000/metadata.json", body.as_bytes());
            let archive = tar.into_inner().unwrap().finish().unwrap();
            let tmp = write_archive_to_temp(&archive);
            let meta = read_metadata_from_archive(tmp.path()).unwrap();
            assert_eq!(meta.commit_hash.as_deref(), Some("deadbeef"), "key={key}");
        }
    }

    #[test]
    fn metadata_returns_default_when_archive_has_no_metadata_json() {
        let buf = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut tar = Builder::new(gz);
        append_to_archive(
            &mut tar,
            "2026-04-15-000000/data/crates.csv",
            b"id,name\n1,foo\n",
        );
        let archive = tar.into_inner().unwrap().finish().unwrap();
        let tmp = write_archive_to_temp(&archive);
        let meta = read_metadata_from_archive(tmp.path()).unwrap();
        assert!(meta.timestamp.is_none());
        assert!(meta.commit_hash.is_none());
    }

    #[test]
    fn parse_crates_csv_handles_csv_quoting_in_descriptions() {
        let csv = "id,name,description\n\
                   1,serde,\"Serialization framework, with derive macros\"\n\
                   2,log,\"A lightweight logging facade\"\n";
        let mut out = HashMap::new();
        parse_crates_csv(csv.as_bytes(), &mut out).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out.get(&1).map(String::as_str), Some("serde"));
        assert_eq!(out.get(&2).map(String::as_str), Some("log"));
    }
}
