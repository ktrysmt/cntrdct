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

pub mod error;
pub mod license;
pub mod sparse_index;

pub use error::FetchError;
pub use license::{
    license_decision, license_acceptable, LicenseDecision, DEFAULT_LICENSE_ALLOWLIST,
};
pub use sparse_index::{
    index_path, CrateMeta, HttpClient, ReqwestClient, SparseIndexClient, DEFAULT_SPARSE_INDEX_BASE,
};
