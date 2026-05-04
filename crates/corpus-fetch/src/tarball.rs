//! `.crate` tarball download + SHA-256 verification.
//!
//! crates.io serves source tarballs at
//! `https://static.crates.io/crates/<name>/<name>-<version>.crate`. Each
//! archive is a gzip'd tar with a single top-level directory `<name>-<version>/`.
//!
//! The Sparse Index entry for each version carries a `cksum` field, defined
//! by Cargo as the SHA-256 of the `.crate` file's bytes. This module
//! downloads those bytes, hashes them, and rejects the response if the digest
//! does not match. Decompression happens in `extract.rs`; keeping verification
//! separate from extraction means corpus reproducibility — the manifest's
//! `sha256` column equals the input to `extract.rs`, not whatever the
//! filesystem ends up with after gzip + tar + filter.

use sha2::{Digest, Sha256};

use crate::sparse_index::HttpClient;
use crate::FetchError;

pub const DEFAULT_TARBALL_BASE: &str = "https://static.crates.io";

/// Hex-encoded SHA-256 of `bytes`. Hex is lowercase; the Sparse Index
/// `cksum` field is also lowercase hex, so equality is a plain string compare.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[derive(Debug, Clone)]
pub struct TarballClient<C: HttpClient> {
    client: C,
    base_url: String,
}

impl<C: HttpClient> TarballClient<C> {
    pub fn new(client: C) -> Self {
        Self {
            client,
            base_url: DEFAULT_TARBALL_BASE.to_string(),
        }
    }

    /// Override the tarball base URL (default: `https://static.crates.io`).
    /// Used by integration tests pointing at a fixture-backed mock.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Build the canonical tarball URL for `(name, version)`. Made public so
    /// callers writing custom orchestrators can stitch the same URL into a
    /// retry loop, error report, etc., without rebuilding the format string.
    pub fn tarball_url(&self, name: &str, version: &str) -> String {
        format!(
            "{}/crates/{}/{}-{}.crate",
            self.base_url.trim_end_matches('/'),
            name,
            name,
            version
        )
    }

    /// Download the `.crate` tarball and verify its SHA-256 matches
    /// `expected_cksum` (the value from the sparse-index record).
    ///
    /// Returns the raw bytes on success, [`FetchError::ChecksumMismatch`] if
    /// the digest disagrees, or any transport error from the underlying
    /// [`HttpClient`].
    pub fn fetch_verified(
        &self,
        name: &str,
        version: &str,
        expected_cksum: &str,
    ) -> Result<Vec<u8>, FetchError> {
        let url = self.tarball_url(name, version);
        let bytes = self.client.get_bytes(&url)?;
        let actual = sha256_hex(&bytes);
        if !actual.eq_ignore_ascii_case(expected_cksum) {
            return Err(FetchError::ChecksumMismatch {
                expected: expected_cksum.to_string(),
                actual,
            });
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// SHA-256 of the empty string, per RFC 6234 / NIST FIPS 180-4 test
    /// vectors. Hard-coded so the implementation can be sanity-checked against
    /// a known value rather than self-comparison.
    const SHA256_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    /// SHA-256 of the ASCII string "abc", another standard NIST vector.
    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sha256_hex_empty_input_matches_nist_vector() {
        assert_eq!(sha256_hex(b""), SHA256_EMPTY);
    }

    #[test]
    fn sha256_hex_abc_matches_nist_vector() {
        assert_eq!(sha256_hex(b"abc"), SHA256_ABC);
    }

    #[test]
    fn sha256_hex_returns_64_chars_for_arbitrary_input() {
        let hex = sha256_hex(b"some random payload");
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    type MockResponse = (String, Result<Vec<u8>, FetchError>);

    struct ByteMock {
        responses: Mutex<Vec<MockResponse>>,
        last_url: Mutex<Option<String>>,
    }

    impl ByteMock {
        fn new() -> Self {
            Self {
                responses: Mutex::new(Vec::new()),
                last_url: Mutex::new(None),
            }
        }
        fn expect(&self, url: &str, body: Vec<u8>) {
            self.responses
                .lock()
                .unwrap()
                .push((url.to_string(), Ok(body)));
        }
        fn expect_err(&self, url: &str, err: FetchError) {
            self.responses
                .lock()
                .unwrap()
                .push((url.to_string(), Err(err)));
        }
    }

    impl HttpClient for ByteMock {
        fn get_text(&self, _url: &str) -> Result<String, FetchError> {
            unimplemented!("tarball test mock serves bytes only")
        }
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            *self.last_url.lock().unwrap() = Some(url.to_string());
            let mut guard = self.responses.lock().unwrap();
            for (i, (u, _)) in guard.iter().enumerate() {
                if u == url {
                    let (_, r) = guard.remove(i);
                    return r;
                }
            }
            Err(FetchError::Http(format!("no mock for {url}")))
        }
    }

    #[test]
    fn tarball_url_uses_static_crates_layout() {
        let mock = ByteMock::new();
        let tc = TarballClient::new(mock).with_base_url("https://example.test");
        assert_eq!(
            tc.tarball_url("serde", "1.0.0"),
            "https://example.test/crates/serde/serde-1.0.0.crate"
        );
    }

    #[test]
    fn fetch_verified_returns_bytes_when_digest_matches() {
        let payload = b"fixture-payload".to_vec();
        let cksum = sha256_hex(&payload);

        let mock = ByteMock::new();
        mock.expect(
            "https://example.test/crates/foo/foo-0.1.0.crate",
            payload.clone(),
        );
        let tc = TarballClient::new(mock).with_base_url("https://example.test");
        let bytes = tc.fetch_verified("foo", "0.1.0", &cksum).unwrap();
        assert_eq!(bytes, payload);
    }

    #[test]
    fn fetch_verified_accepts_uppercase_expected_cksum() {
        let payload = b"another-payload".to_vec();
        let cksum_upper = sha256_hex(&payload).to_uppercase();

        let mock = ByteMock::new();
        mock.expect(
            "https://example.test/crates/foo/foo-0.1.0.crate",
            payload.clone(),
        );
        let tc = TarballClient::new(mock).with_base_url("https://example.test");
        let bytes = tc.fetch_verified("foo", "0.1.0", &cksum_upper).unwrap();
        assert_eq!(bytes, payload);
    }

    #[test]
    fn fetch_verified_rejects_mismatched_cksum() {
        let payload = b"fixture-payload".to_vec();
        let mock = ByteMock::new();
        mock.expect(
            "https://example.test/crates/foo/foo-0.1.0.crate",
            payload,
        );
        let tc = TarballClient::new(mock).with_base_url("https://example.test");
        let err = tc.fetch_verified("foo", "0.1.0", "deadbeef").unwrap_err();
        match err {
            FetchError::ChecksumMismatch { expected, actual } => {
                assert_eq!(expected, "deadbeef");
                assert_eq!(actual.len(), 64);
            }
            other => panic!("expected ChecksumMismatch, got {other:?}"),
        }
    }

    #[test]
    fn fetch_verified_propagates_not_found() {
        let mock = ByteMock::new();
        mock.expect_err(
            "https://example.test/crates/missing/missing-0.1.0.crate",
            FetchError::NotFound("missing".to_string()),
        );
        let tc = TarballClient::new(mock).with_base_url("https://example.test");
        let err = tc.fetch_verified("missing", "0.1.0", "doesnt-matter").unwrap_err();
        assert!(matches!(err, FetchError::NotFound(_)));
    }
}
