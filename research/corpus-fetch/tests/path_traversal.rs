//! Integration tests for `extract_filtered` against malicious tarballs.
//!
//! The unit tests in `src/extract.rs` exercise `strip_top_component`
//! directly and rely on the `tar` crate's `Builder` for the integration
//! path. `Builder` refuses to construct entries with `..`, absolute
//! prefixes, or NUL-stuffed names, so a malicious archive served by a
//! hostile crates.io mirror cannot be reproduced through the high-level
//! builder API. These tests hand-craft raw tar + gzip bytes that bypass
//! `Builder`'s safety checks and assert that `extract_filtered` itself
//! refuses them at the integration level — a regression guard that
//! survives even if `Builder`'s safety properties drift across tar
//! crate versions.

use cntrdct_corpus_fetch::{extract_filtered, ExtractOptions};

const BLOCK: usize = 512;

const TYPEFLAG_REGULAR: u8 = b'0';
const TYPEFLAG_HARDLINK: u8 = b'1';
const TYPEFLAG_SYMLINK: u8 = b'2';

fn raw_entry(name: &[u8], content: &[u8], typeflag: u8) -> Vec<u8> {
    let mut header = [0u8; BLOCK];
    let n = name.len().min(100);
    header[..n].copy_from_slice(&name[..n]);
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    let size_field = format!("{:011o}\0", content.len());
    header[124..136].copy_from_slice(size_field.as_bytes());
    header[136..148].copy_from_slice(b"00000000000\0");
    header[148..156].copy_from_slice(b"        ");
    header[156] = typeflag;
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    let cksum: u32 = header.iter().map(|&b| b as u32).sum();
    let cksum_field = format!("{:06o}\0 ", cksum);
    header[148..156].copy_from_slice(cksum_field.as_bytes());

    let mut out = Vec::with_capacity(BLOCK + content.len() + BLOCK);
    out.extend_from_slice(&header);
    out.extend_from_slice(content);
    let pad = (BLOCK - content.len() % BLOCK) % BLOCK;
    out.extend(std::iter::repeat_n(0u8, pad));
    out
}

fn build_raw_archive(entries: &[(&[u8], &[u8], u8)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (name, content, typeflag) in entries {
        bytes.extend_from_slice(&raw_entry(name, content, *typeflag));
    }
    bytes.extend(std::iter::repeat_n(0u8, BLOCK * 2));
    bytes
}

fn gzip(plain: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(plain).unwrap();
    e.finish().unwrap()
}

#[test]
fn parent_dir_traversal_in_entry_name_is_rejected() {
    let plain = build_raw_archive(&[
        (
            b"foo-0.1.0/../etc/passwd",
            b"r:x:0:0:root:/root:/bin/sh\n",
            TYPEFLAG_REGULAR,
        ),
        (b"foo-0.1.0/src/lib.rs", b"pub fn ok() {}", TYPEFLAG_REGULAR),
    ]);
    let bytes = gzip(&plain);
    let tmp = tempfile::tempdir().unwrap();
    let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();

    assert_eq!(
        report.kept, 1,
        "only the legitimate src/lib.rs should be kept"
    );
    assert!(report.skipped_unsafe_path >= 1);
    assert!(tmp.path().join("src/lib.rs").exists());
    // Defensive: confirm no file with the malicious tail name landed
    // anywhere under the output directory.
    assert!(!tmp.path().join("etc/passwd").exists());
    assert!(!tmp.path().join("../etc/passwd").exists());
}

#[test]
fn absolute_path_in_entry_name_is_rejected() {
    let plain = build_raw_archive(&[
        (b"/abs/path/lib.rs", b"escape", TYPEFLAG_REGULAR),
        (
            b"foo-0.1.0/src/main.rs",
            b"pub fn main() {}",
            TYPEFLAG_REGULAR,
        ),
    ]);
    let bytes = gzip(&plain);
    let tmp = tempfile::tempdir().unwrap();
    let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();

    assert_eq!(report.kept, 1);
    assert!(report.skipped_unsafe_path >= 1);
    assert!(tmp.path().join("src/main.rs").exists());
}

#[test]
fn symlink_entry_is_rejected_as_non_regular() {
    let plain = build_raw_archive(&[
        (b"foo-0.1.0/escape.rs", b"", TYPEFLAG_SYMLINK),
        (b"foo-0.1.0/src/lib.rs", b"pub fn ok() {}", TYPEFLAG_REGULAR),
    ]);
    let bytes = gzip(&plain);
    let tmp = tempfile::tempdir().unwrap();
    let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();

    assert_eq!(report.kept, 1);
    assert!(report.skipped_non_regular >= 1);
    assert!(!tmp.path().join("escape.rs").exists());
    assert!(tmp.path().join("src/lib.rs").exists());
}

#[test]
fn hardlink_entry_is_rejected_as_non_regular() {
    let plain = build_raw_archive(&[
        (b"foo-0.1.0/hardlink.rs", b"", TYPEFLAG_HARDLINK),
        (b"foo-0.1.0/src/lib.rs", b"pub fn ok() {}", TYPEFLAG_REGULAR),
    ]);
    let bytes = gzip(&plain);
    let tmp = tempfile::tempdir().unwrap();
    let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();

    assert_eq!(report.kept, 1);
    assert!(report.skipped_non_regular >= 1);
    assert!(!tmp.path().join("hardlink.rs").exists());
}

#[test]
fn root_only_entry_is_rejected() {
    let plain = build_raw_archive(&[
        (b"foo-0.1.0/", b"", TYPEFLAG_REGULAR),
        (b"foo-0.1.0/src/lib.rs", b"pub fn ok() {}", TYPEFLAG_REGULAR),
    ]);
    let bytes = gzip(&plain);
    let tmp = tempfile::tempdir().unwrap();
    let report = extract_filtered(&bytes, tmp.path(), &ExtractOptions::default()).unwrap();

    assert_eq!(report.kept, 1);
    assert!(report.skipped_unsafe_path >= 1);
}
