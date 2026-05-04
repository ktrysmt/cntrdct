//! End-to-end fetch orchestrator.
//!
//! Given a crate name plus an output directory, [`fetch_one`] walks the
//! pipeline:
//!
//! 1. resolve the latest non-yanked version through the Sparse Index,
//! 2. apply the license filter,
//! 3. download and verify the `.crate` tarball,
//! 4. extract the filtered subset into `<out_root>/<name>-<version>/`,
//! 5. return either a [`ManifestRow`] for the manifest writer or a
//!    [`SkipReason`] explaining why the crate was excluded.
//!
//! The orchestrator never panics on per-crate skip conditions — the caller's
//! batch loop expects to log skipped crates and continue. Only true I/O or
//! checksum failures bubble up as [`FetchError`].

use std::path::Path;

use crate::extract::{extract_filtered, ExtractOptions, ExtractReport};
use crate::license::{license_decision, LicenseDecision};
use crate::manifest::ManifestRow;
use crate::sparse_index::{HttpClient, SparseIndexClient};
use crate::tarball::TarballClient;
use crate::FetchError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    LicenseRejected(String),
    LicenseMissing,
    NotFound,
    AllVersionsYanked,
}

#[derive(Debug, Clone)]
pub enum FetchOutcome {
    Fetched {
        row: ManifestRow,
        extract_report: ExtractReport,
    },
    Skipped {
        name: String,
        reason: SkipReason,
    },
}

/// Run the full fetch pipeline for a single crate. See module docs.
pub fn fetch_one<C1: HttpClient, C2: HttpClient>(
    sparse: &SparseIndexClient<C1>,
    tarball: &TarballClient<C2>,
    crate_name: &str,
    out_root: &Path,
    allowlist: &[&str],
    extract_opts: &ExtractOptions,
) -> Result<FetchOutcome, FetchError> {
    let meta = match sparse.fetch_latest_non_yanked(crate_name) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Ok(FetchOutcome::Skipped {
                name: crate_name.to_string(),
                reason: SkipReason::AllVersionsYanked,
            });
        }
        Err(FetchError::NotFound(_)) => {
            return Ok(FetchOutcome::Skipped {
                name: crate_name.to_string(),
                reason: SkipReason::NotFound,
            });
        }
        Err(e) => return Err(e),
    };

    match license_decision(meta.license.as_deref(), allowlist) {
        LicenseDecision::Accepted => {}
        LicenseDecision::Rejected => {
            return Ok(FetchOutcome::Skipped {
                name: crate_name.to_string(),
                reason: SkipReason::LicenseRejected(meta.license.unwrap_or_default()),
            });
        }
        LicenseDecision::Missing => {
            return Ok(FetchOutcome::Skipped {
                name: crate_name.to_string(),
                reason: SkipReason::LicenseMissing,
            });
        }
    }

    let bytes = tarball.fetch_verified(&meta.name, &meta.version, &meta.cksum)?;

    let crate_dir = out_root.join(format!("{}-{}", meta.name, meta.version));
    std::fs::create_dir_all(&crate_dir)?;
    let extract_report = extract_filtered(&bytes, &crate_dir, extract_opts)?;

    Ok(FetchOutcome::Fetched {
        row: ManifestRow {
            name: meta.name,
            version: meta.version,
            license: meta.license.unwrap_or_default(),
            downloads: None,
            sha256: meta.cksum,
        },
        extract_report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license::DEFAULT_LICENSE_ALLOWLIST;
    use crate::tarball::sha256_hex;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::sync::Mutex;
    use tar::{Builder, Header};

    /// Build a fake `.crate` tarball plus its SHA-256 digest.
    fn make_crate(top_dir: &str, files: &[(&str, &[u8])]) -> (Vec<u8>, String) {
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

    /// Mock that serves both text (for sparse index) and bytes (for tarball)
    /// from one URL → response map.
    struct DualMock {
        text: Mutex<Vec<(String, String)>>,
        bytes: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl DualMock {
        fn new() -> Self {
            Self {
                text: Mutex::new(Vec::new()),
                bytes: Mutex::new(Vec::new()),
            }
        }
        fn expect_text(&self, url: &str, body: &str) {
            self.text
                .lock()
                .unwrap()
                .push((url.to_string(), body.to_string()));
        }
        fn expect_bytes(&self, url: &str, body: Vec<u8>) {
            self.bytes
                .lock()
                .unwrap()
                .push((url.to_string(), body));
        }
    }

    impl HttpClient for DualMock {
        fn get_text(&self, url: &str) -> Result<String, FetchError> {
            let mut g = self.text.lock().unwrap();
            for (i, (u, _)) in g.iter().enumerate() {
                if u == url {
                    return Ok(g.remove(i).1);
                }
            }
            Err(FetchError::NotFound(url.to_string()))
        }
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            let mut g = self.bytes.lock().unwrap();
            for (i, (u, _)) in g.iter().enumerate() {
                if u == url {
                    return Ok(g.remove(i).1);
                }
            }
            Err(FetchError::NotFound(url.to_string()))
        }
    }

    const INDEX_BASE: &str = "https://idx.test";
    const STATIC_BASE: &str = "https://static.test";

    fn build_clients(
        mock_idx: DualMock,
        mock_tar: DualMock,
    ) -> (SparseIndexClient<DualMock>, TarballClient<DualMock>) {
        (
            SparseIndexClient::new(mock_idx).with_base_url(INDEX_BASE),
            TarballClient::new(mock_tar).with_base_url(STATIC_BASE),
        )
    }

    fn sparse_record(name: &str, version: &str, license: Option<&str>, cksum: &str, yanked: bool) -> String {
        let lic = match license {
            Some(s) => format!(",\"license\":\"{s}\""),
            None => String::new(),
        };
        format!(
            "{{\"name\":\"{name}\",\"vers\":\"{version}\",\"deps\":[],\"cksum\":\"{cksum}\",\"features\":{{}},\"yanked\":{yanked}{lic}}}"
        )
    }

    #[test]
    fn happy_path_fetched_outcome_carries_row_and_extract_report() {
        let (bytes, digest) = make_crate(
            "demo-0.1.0",
            &[
                ("src/lib.rs", b"pub fn ok() {}"),
                ("src/util.rs", b"// helper"),
                ("tests/it.rs", b"// dropped"),
                ("README.md", b"dropped"),
            ],
        );

        let idx = DualMock::new();
        idx.expect_text(
            &format!("{INDEX_BASE}/de/mo/demo"),
            &sparse_record("demo", "0.1.0", Some("MIT OR Apache-2.0"), &digest, false),
        );
        let tar = DualMock::new();
        tar.expect_bytes(
            &format!("{STATIC_BASE}/crates/demo/demo-0.1.0.crate"),
            bytes,
        );

        let (sparse, tarball) = build_clients(idx, tar);
        let tmp = tempfile::tempdir().unwrap();
        let result = fetch_one(
            &sparse,
            &tarball,
            "demo",
            tmp.path(),
            DEFAULT_LICENSE_ALLOWLIST,
            &ExtractOptions::default(),
        )
        .unwrap();

        match result {
            FetchOutcome::Fetched {
                row,
                extract_report,
            } => {
                assert_eq!(row.name, "demo");
                assert_eq!(row.version, "0.1.0");
                assert_eq!(row.license, "MIT OR Apache-2.0");
                assert_eq!(row.sha256, digest);
                assert_eq!(row.downloads, None);
                assert_eq!(extract_report.kept, 2); // 2 .rs under src
                assert_eq!(extract_report.skipped_dir, 1); // tests/
                assert_eq!(extract_report.skipped_ext, 1); // README.md
                assert!(tmp.path().join("demo-0.1.0/src/lib.rs").exists());
            }
            other => panic!("expected Fetched, got {other:?}"),
        }
    }

    #[test]
    fn license_rejected_skips_without_downloading_tarball() {
        let idx = DualMock::new();
        idx.expect_text(
            &format!("{INDEX_BASE}/3/g/gpl"),
            &sparse_record("gpl", "0.1.0", Some("GPL-3.0"), "deadbeef", false),
        );
        let tar = DualMock::new();
        // Intentionally no tarball expectation; if the orchestrator hit the
        // network path it would receive NotFound and bubble up the error.

        let (sparse, tarball) = build_clients(idx, tar);
        let tmp = tempfile::tempdir().unwrap();
        let result = fetch_one(
            &sparse,
            &tarball,
            "gpl",
            tmp.path(),
            DEFAULT_LICENSE_ALLOWLIST,
            &ExtractOptions::default(),
        )
        .unwrap();
        match result {
            FetchOutcome::Skipped {
                name,
                reason: SkipReason::LicenseRejected(spdx),
            } => {
                assert_eq!(name, "gpl");
                assert_eq!(spdx, "GPL-3.0");
            }
            other => panic!("expected LicenseRejected, got {other:?}"),
        }
    }

    #[test]
    fn license_missing_skip_distinct_from_rejected() {
        let idx = DualMock::new();
        idx.expect_text(
            &format!("{INDEX_BASE}/no/li/nolicense"),
            &sparse_record("nolicense", "0.1.0", None, "deadbeef", false),
        );
        let tar = DualMock::new();

        let (sparse, tarball) = build_clients(idx, tar);
        let tmp = tempfile::tempdir().unwrap();
        let result = fetch_one(
            &sparse,
            &tarball,
            "nolicense",
            tmp.path(),
            DEFAULT_LICENSE_ALLOWLIST,
            &ExtractOptions::default(),
        )
        .unwrap();
        assert!(matches!(
            result,
            FetchOutcome::Skipped {
                reason: SkipReason::LicenseMissing,
                ..
            }
        ));
    }

    #[test]
    fn missing_crate_returns_not_found_skip() {
        let idx = DualMock::new();
        // No expectation - DualMock::get_text returns NotFound for unknown URLs.
        let tar = DualMock::new();

        let (sparse, tarball) = build_clients(idx, tar);
        let tmp = tempfile::tempdir().unwrap();
        let result = fetch_one(
            &sparse,
            &tarball,
            "nonexistent",
            tmp.path(),
            DEFAULT_LICENSE_ALLOWLIST,
            &ExtractOptions::default(),
        )
        .unwrap();
        assert!(matches!(
            result,
            FetchOutcome::Skipped {
                reason: SkipReason::NotFound,
                ..
            }
        ));
    }

    #[test]
    fn all_versions_yanked_returns_skip() {
        let idx = DualMock::new();
        idx.expect_text(
            &format!("{INDEX_BASE}/3/y/yan"),
            &sparse_record("yan", "0.1.0", Some("MIT"), "abc", true),
        );
        let tar = DualMock::new();

        let (sparse, tarball) = build_clients(idx, tar);
        let tmp = tempfile::tempdir().unwrap();
        let result = fetch_one(
            &sparse,
            &tarball,
            "yan",
            tmp.path(),
            DEFAULT_LICENSE_ALLOWLIST,
            &ExtractOptions::default(),
        )
        .unwrap();
        assert!(matches!(
            result,
            FetchOutcome::Skipped {
                reason: SkipReason::AllVersionsYanked,
                ..
            }
        ));
    }

    #[test]
    fn checksum_mismatch_propagates_as_error() {
        let (bytes, _good_digest) = make_crate("bad-0.1.0", &[("src/lib.rs", b"x")]);
        let idx = DualMock::new();
        idx.expect_text(
            &format!("{INDEX_BASE}/3/b/bad"),
            &sparse_record("bad", "0.1.0", Some("MIT"), "deadbeefbad", false),
        );
        let tar = DualMock::new();
        tar.expect_bytes(&format!("{STATIC_BASE}/crates/bad/bad-0.1.0.crate"), bytes);

        let (sparse, tarball) = build_clients(idx, tar);
        let tmp = tempfile::tempdir().unwrap();
        let err = fetch_one(
            &sparse,
            &tarball,
            "bad",
            tmp.path(),
            DEFAULT_LICENSE_ALLOWLIST,
            &ExtractOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(err, FetchError::ChecksumMismatch { .. }));
    }
}
