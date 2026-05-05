//! crates.io Sparse Index client and license filter.
//!
//! This crate is the first piece of the Track A empirical-study fetcher
//! (see `projects/A_1000_crate/README.md`). Scope of this module: given a
//! crate name, fetch the per-version metadata that the Sparse Index serves,
//! and decide whether a given SPDX license expression is acceptable for the
//! analysis corpus. Tarball download and on-disk extraction are handled by
//! a sibling module added in a later change.
//!
//! The HTTP seam follows the same pattern as `cntrdct-adjudicator-llm`:
//! production binds to `ReqwestClient`, tests bind to a hand-rolled mock so
//! the suite never reaches the network.

pub mod db_dump;
pub mod error;
pub mod extract;
pub mod fetcher;
pub mod license;
pub mod manifest;
pub mod sparse_index;
pub mod tarball;

pub use db_dump::{
    download_dump_streaming, download_dump_streaming_with_retry, read_metadata_from_archive,
    read_top_n_from_archive, CrateRanking, DumpMetadata, DEFAULT_DB_DUMP_URL,
    DEFAULT_DUMP_MAX_RETRIES,
};
pub use error::FetchError;
pub use extract::{extract_filtered, ExtractOptions, ExtractReport};
pub use fetcher::{fetch_one, FetchOutcome, SkipReason};
pub use license::{
    license_decision, license_acceptable, LicenseDecision, DEFAULT_LICENSE_ALLOWLIST,
};
pub use manifest::{
    append_row, read_manifest_names, read_manifest_rows, write_header, write_row, ManifestRow,
    MANIFEST_HEADER,
};
pub use sparse_index::{
    index_path, CrateMeta, HttpClient, ReqwestClient, SparseIndexClient, DEFAULT_SPARSE_INDEX_BASE,
};
pub use tarball::{sha256_hex, TarballClient, DEFAULT_TARBALL_BASE};
