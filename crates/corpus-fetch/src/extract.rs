//! Filtered extraction of a `.crate` tarball into a directory tree.
//!
//! `.crate` archives are gzip'd tar files with a single top-level directory
//! named `<crate>-<version>/`. We unpack only the entries that match the
//! corpus filter rules — by default, regular files with the `.rs` extension,
//! sized at or below 50 KB, and not located under `tests/`, `target/`,
//! `examples/`, or `benches/` (these mirror the v0 guidance in
//! `projects/A_1000_crate/README.md`).
//!
//! The leading `<crate>-<version>/` component is stripped before resolving
//! the destination path, so callers pass the per-crate output directory
//! (e.g. `corpus/wild/serde-1.0.0/`) and end up with `src/lib.rs` directly
//! beneath it rather than a redundant nested `serde-1.0.0/` layer.

use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::FetchError;

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Skip any regular file whose tar-recorded size exceeds this many bytes.
    /// The 50 KB default tracks the README's heuristic for filtering
    /// auto-generated source.
    pub max_file_bytes: u64,
    /// Top-level directory names to skip outright. The match is on the first
    /// path component after stripping the archive's `<crate>-<version>/`
    /// prefix.
    pub exclude_dirs: Vec<String>,
    /// Lowercase extensions to keep (without the leading dot). Empty list
    /// means accept-all.
    pub include_extensions: Vec<String>,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 50 * 1024,
            exclude_dirs: vec![
                "tests".into(),
                "target".into(),
                "examples".into(),
                "benches".into(),
            ],
            include_extensions: vec!["rs".into()],
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractReport {
    pub kept: usize,
    pub skipped_size: usize,
    pub skipped_dir: usize,
    pub skipped_ext: usize,
    pub skipped_unsafe_path: usize,
    pub skipped_non_regular: usize,
}

/// Extract entries from a gzip'd tar archive into `out_dir`, applying the
/// corpus filters in [`ExtractOptions`]. Returns a [`ExtractReport`]
/// summarising how many entries each filter dropped, which the manifest
/// writer can use to log per-crate statistics.
pub fn extract_filtered(
    archive_bytes: &[u8],
    out_dir: &Path,
    opts: &ExtractOptions,
) -> Result<ExtractReport, FetchError> {
    let cursor = Cursor::new(archive_bytes);
    let gz = GzDecoder::new(cursor);
    let mut archive = Archive::new(gz);

    let mut report = ExtractReport::default();

    let entries = archive
        .entries()
        .map_err(|e| FetchError::Archive(e.to_string()))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| FetchError::Archive(e.to_string()))?;

        // Skip non-regular entries (directories, symlinks, hardlinks). We do
        // not want to honour symlinks in a corpus filter — they could escape
        // out_dir even after path-component validation.
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() {
            report.skipped_non_regular += 1;
            continue;
        }

        let raw_path = entry
            .path()
            .map_err(|e| FetchError::Archive(e.to_string()))?
            .into_owned();

        let rel = match strip_top_component(&raw_path) {
            Some(p) => p,
            None => {
                // Either the archive entry was the top-level directory
                // itself, or its path contained a `..` / absolute prefix that
                // we refuse to honour.
                report.skipped_unsafe_path += 1;
                continue;
            }
        };

        let top = rel
            .components()
            .next()
            .and_then(|c| match c {
                Component::Normal(s) => s.to_str(),
                _ => None,
            });
        if let Some(t) = top {
            if opts.exclude_dirs.iter().any(|d| d == t) {
                report.skipped_dir += 1;
                continue;
            }
        }

        if !opts.include_extensions.is_empty() {
            let ext = rel
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase());
            match ext {
                Some(ref e) if opts.include_extensions.iter().any(|x| x == e) => {}
                _ => {
                    report.skipped_ext += 1;
                    continue;
                }
            }
        }

        let size = entry.header().size().unwrap_or(u64::MAX);
        if size > opts.max_file_bytes {
            report.skipped_size += 1;
            continue;
        }

        let dest = out_dir.join(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::with_capacity(size as usize);
        entry.read_to_end(&mut buf)?;
        std::fs::write(&dest, &buf)?;
        report.kept += 1;
    }

    Ok(report)
}

/// Drop the first path component (the archive's `<crate>-<version>/`
/// prefix) and return the remainder. Refuses any path containing `..` or
/// non-Normal components, returning `None` so the caller can count it as a
/// dropped unsafe entry.
fn strip_top_component(path: &Path) -> Option<PathBuf> {
    let mut comps = path.components();
    let first = comps.next()?;
    if !matches!(first, Component::Normal(_)) {
        return None;
    }
    let mut out = PathBuf::new();
    for c in comps {
        match c {
            Component::Normal(s) => out.push(s),
            Component::CurDir => continue,
            // ParentDir, RootDir, Prefix → reject as path traversal.
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        // The entry was the root directory itself (e.g. "serde-1.0.0/").
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::{Builder, Header};

    /// Helper: build a synthetic .crate-shaped gzip'd tar in memory from a
    /// list of (path, contents) pairs, all rooted under `top_dir/`.
    fn make_crate(top_dir: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
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
        let gz = tar.into_inner().unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn extracts_rs_files_under_src() {
        let bytes = make_crate(
            "foo-0.1.0",
            &[
                ("src/lib.rs", b"pub fn answer() -> u32 { 42 }"),
                ("src/util.rs", b"// helper"),
                ("Cargo.toml", b"[package]\nname=\"foo\""),
            ],
        );
        let tmp = tempfile::tempdir().unwrap();
        let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();
        assert_eq!(report.kept, 2);
        assert!(tmp.path().join("src/lib.rs").exists());
        assert!(tmp.path().join("src/util.rs").exists());
        assert!(!tmp.path().join("Cargo.toml").exists());
    }

    #[test]
    fn excludes_default_directories() {
        let bytes = make_crate(
            "foo-0.1.0",
            &[
                ("src/lib.rs", b"keep"),
                ("tests/it.rs", b"drop"),
                ("examples/demo.rs", b"drop"),
                ("benches/bench.rs", b"drop"),
                ("target/leftover.rs", b"drop"),
            ],
        );
        let tmp = tempfile::tempdir().unwrap();
        let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();
        assert_eq!(report.kept, 1);
        assert_eq!(report.skipped_dir, 4);
        assert!(tmp.path().join("src/lib.rs").exists());
    }

    #[test]
    fn skips_files_above_size_limit() {
        let big = vec![b'a'; 100 * 1024];
        let small = b"small".to_vec();
        let bytes = make_crate(
            "foo-0.1.0",
            &[("src/big.rs", &big), ("src/small.rs", &small)],
        );
        let tmp = tempfile::tempdir().unwrap();
        let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();
        assert_eq!(report.kept, 1);
        assert_eq!(report.skipped_size, 1);
        assert!(tmp.path().join("src/small.rs").exists());
        assert!(!tmp.path().join("src/big.rs").exists());
    }

    #[test]
    fn extension_filter_rejects_non_rs_when_default() {
        let bytes = make_crate(
            "foo-0.1.0",
            &[
                ("src/lib.rs", b"keep"),
                ("src/data.json", b"drop"),
                ("README.md", b"drop"),
            ],
        );
        let tmp = tempfile::tempdir().unwrap();
        let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();
        assert_eq!(report.kept, 1);
        assert_eq!(report.skipped_ext, 2);
    }

    #[test]
    fn empty_extension_list_keeps_everything_size_permitting() {
        let bytes = make_crate(
            "foo-0.1.0",
            &[("Cargo.toml", b"[package]"), ("src/lib.rs", b"x")],
        );
        let opts = ExtractOptions {
            include_extensions: Vec::new(),
            ..ExtractOptions::default()
        };
        let tmp = tempfile::tempdir().unwrap();
        let report = extract_filtered(&bytes, tmp.path(), &opts).unwrap();
        assert_eq!(report.kept, 2);
    }

    // Note: the tar 0.4 Builder rejects paths containing `..` when archives
    // are being built, so we cannot construct a malicious-path tarball
    // through the high-level API. The defensive check in `strip_top_component`
    // is still required for any third-party-built archive that bypasses the
    // tar crate's safety checks, and is exercised directly below via
    // `strip_top_component_rejects_parent_dir` and friends.

    #[test]
    fn strip_top_component_drops_first_part() {
        assert_eq!(
            strip_top_component(Path::new("foo-0.1.0/src/lib.rs")),
            Some(PathBuf::from("src/lib.rs"))
        );
        assert_eq!(
            strip_top_component(Path::new("foo-0.1.0/Cargo.toml")),
            Some(PathBuf::from("Cargo.toml"))
        );
    }

    #[test]
    fn strip_top_component_returns_none_for_root_only() {
        assert_eq!(strip_top_component(Path::new("foo-0.1.0/")), None);
    }

    #[test]
    fn strip_top_component_rejects_parent_dir() {
        assert_eq!(strip_top_component(Path::new("foo-0.1.0/../etc")), None);
    }

    #[test]
    fn strip_top_component_rejects_absolute() {
        assert_eq!(strip_top_component(Path::new("/abs/path")), None);
    }
}
