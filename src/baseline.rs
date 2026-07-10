//! Scan baseline (ratchet) support.
//!
//! Spec: `docs/spec/baseline-v0.md` (B-1). A baseline file records
//! fingerprints of the findings a project has decided to tolerate for
//! now, so that later scans report only NEW findings. This is the
//! standard adoption path for large existing codebases: write a
//! baseline once, then ratchet — every scan afterwards is clean unless
//! a contradiction is introduced.
//!
//! Everything in this module is deterministic file I/O plus pure
//! filtering (design constraint P3: no LLM, no network). The baseline
//! is applied AFTER Layer 2 ranking and BEFORE Layer 3 adjudication,
//! so the opt-in LLM budget is spent on new findings only.
//!
//! # Fingerprint
//!
//! A finding's fingerprint is the triple
//! `(detector_id, relative file path, digit-normalized message)`:
//!
//! - The file path is made relative to the scan root and uses `/`
//!   separators, so a baseline written on one machine (or in CI)
//!   matches on another regardless of absolute checkout location or
//!   platform.
//! - The message has every ASCII digit run collapsed to `#` because
//!   some detectors embed line numbers ("preceded by return on line
//!   42") or volatile counts ("diverged from 3 similar siblings") in
//!   their messages. Without normalization, inserting an unrelated
//!   line above a known finding would resurrect it — the opposite of
//!   what a ratchet promises.
//!
//! Line and column numbers are deliberately NOT part of the
//! fingerprint (same rationale). Identical findings are disambiguated
//! by an occurrence `count` per fingerprint, PHPStan-baseline style:
//! if the baseline records 2 occurrences and a scan produces 3, the
//! first 2 (in ranked order) are suppressed and the third surfaces.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::{Finding, RankedFinding};

/// The baseline file format version this build reads and writes.
/// Bumped only on incompatible fingerprint or shape changes; a
/// mismatch is a hard error rather than a silent partial match.
pub const BASELINE_FORMAT_VERSION: u32 = 1;

/// Errors surfaced by baseline load / save.
#[derive(Debug, Error)]
pub enum BaselineError {
    /// Reading or writing the baseline file failed.
    #[error("io error on {path}: {source}")]
    Io {
        /// Path of the file being read or written.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The baseline file exists but is not valid baseline JSON.
    #[error("parse error in {path}: {source}")]
    Parse {
        /// Path of the malformed file.
        path: PathBuf,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// The baseline file declares a format version this build does
    /// not understand.
    #[error("unsupported baseline version {found} in {path} (this build supports {supported}); regenerate with `cntrdct scan <path> --write-baseline {path}`")]
    Version {
        /// Path of the incompatible file.
        path: PathBuf,
        /// Version declared by the file.
        found: u32,
        /// Version this build supports.
        supported: u32,
    },
}

/// One tolerated fingerprint plus its occurrence count.
///
/// The components are stored explicitly (not as an opaque hash) so a
/// baseline file stays reviewable in code review: each entry names
/// the detector, the file, and the (digit-normalized) message it
/// tolerates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineEntry {
    /// `Detector::id()` of the tolerated finding.
    pub detector_id: String,
    /// Scan-root-relative path with `/` separators.
    pub file: String,
    /// Digit-normalized finding message (see [`normalize_message`]).
    pub message: String,
    /// Number of identical-fingerprint occurrences tolerated.
    pub count: usize,
}

/// On-disk baseline document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineFile {
    /// Format version; see [`BASELINE_FORMAT_VERSION`].
    pub version: u32,
    /// Tolerated fingerprints, sorted by (detector_id, file, message)
    /// on write so regeneration produces byte-stable diffs.
    pub entries: Vec<BaselineEntry>,
}

/// Result of filtering ranked findings through a baseline.
#[derive(Debug)]
pub struct BaselineOutcome {
    /// Findings that survived (i.e. are NEW relative to the baseline),
    /// in their original ranked order.
    pub kept: Vec<RankedFinding>,
    /// Number of findings suppressed as already-known.
    pub suppressed: usize,
}

/// Collapse every ASCII digit run in `message` to a single `#`.
///
/// "statement is unreachable; preceded by return on line 42" and the
/// same message with line 43 normalize identically, which is what
/// makes the fingerprint tolerant to unrelated edits above the
/// finding.
pub fn normalize_message(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut in_digits = false;
    for c in message.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('#');
                in_digits = true;
            }
        } else {
            in_digits = false;
            out.push(c);
        }
    }
    out
}

/// Normalize a finding's file path into its baseline key: relative to
/// `scan_root`, `/`-separated. Falls back to the path as given when it
/// is not under `scan_root` (should not happen for walker-produced
/// findings), and to the bare file name when the scan root IS the file
/// (`cntrdct scan src/lib.rs`).
pub fn file_key(file: &Path, scan_root: &Path) -> String {
    let rel = file.strip_prefix(scan_root).unwrap_or(file);
    let s = rel.to_string_lossy().replace('\\', "/");
    if s.is_empty() {
        // strip_prefix of the root itself yields "": single-file scan.
        file.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        s
    }
}

/// Compute the fingerprint triple for one finding.
fn fingerprint(finding: &Finding, scan_root: &Path) -> (String, String, String) {
    (
        finding.detector_id.clone(),
        file_key(&finding.primary.file, scan_root),
        normalize_message(&finding.message),
    )
}

/// Build a baseline document from the current ranked finding set.
pub fn build(ranked: &[RankedFinding], scan_root: &Path) -> BaselineFile {
    let mut counts: HashMap<(String, String, String), usize> = HashMap::new();
    for rf in ranked {
        *counts
            .entry(fingerprint(&rf.finding, scan_root))
            .or_insert(0) += 1;
    }
    let mut entries: Vec<BaselineEntry> = counts
        .into_iter()
        .map(|((detector_id, file, message), count)| BaselineEntry {
            detector_id,
            file,
            message,
            count,
        })
        .collect();
    entries.sort_by(|a, b| {
        a.detector_id
            .cmp(&b.detector_id)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.message.cmp(&b.message))
    });
    BaselineFile {
        version: BASELINE_FORMAT_VERSION,
        entries,
    }
}

/// Drop every finding whose fingerprint has remaining budget in
/// `baseline`, preserving ranked order for the survivors. Duplicate
/// entries for one fingerprint have their counts summed.
pub fn filter_ranked(
    baseline: &BaselineFile,
    ranked: Vec<RankedFinding>,
    scan_root: &Path,
) -> BaselineOutcome {
    let mut budget: HashMap<(String, String, String), usize> = HashMap::new();
    for e in &baseline.entries {
        *budget
            .entry((e.detector_id.clone(), e.file.clone(), e.message.clone()))
            .or_insert(0) += e.count;
    }
    let mut kept = Vec::with_capacity(ranked.len());
    let mut suppressed = 0usize;
    for rf in ranked {
        let key = fingerprint(&rf.finding, scan_root);
        match budget.get_mut(&key) {
            Some(b) if *b > 0 => {
                *b -= 1;
                suppressed += 1;
            }
            _ => kept.push(rf),
        }
    }
    BaselineOutcome { kept, suppressed }
}

/// Load and validate a baseline file. A missing file is an error (not
/// a silent empty baseline): in the ratchet workflow a missing file
/// means a misconfigured CI job, and reporting every finding as "new"
/// against a typo'd path would be noisy in the wrong direction.
pub fn load(path: &Path) -> Result<BaselineFile, BaselineError> {
    let body = fs::read_to_string(path).map_err(|e| BaselineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let parsed: BaselineFile = serde_json::from_str(&body).map_err(|e| BaselineError::Parse {
        path: path.to_path_buf(),
        source: e,
    })?;
    if parsed.version != BASELINE_FORMAT_VERSION {
        return Err(BaselineError::Version {
            path: path.to_path_buf(),
            found: parsed.version,
            supported: BASELINE_FORMAT_VERSION,
        });
    }
    Ok(parsed)
}

/// Write `baseline` as pretty JSON (trailing newline included, so the
/// file is friendly to line-oriented diff tooling). Creates parent
/// directories as needed.
pub fn save(path: &Path, baseline: &BaselineFile) -> Result<(), BaselineError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| BaselineError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
    }
    let mut body = serde_json::to_string_pretty(baseline).expect("BaselineFile serializes cleanly");
    body.push('\n');
    fs::write(path, body).map_err(|e| BaselineError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{AnomalyClass, Evidence, LanguageCitationStatus, Location, Severity};

    fn ranked(detector_id: &str, file: &str, line: u32, message: &str) -> RankedFinding {
        RankedFinding {
            finding: Finding {
                detector_id: detector_id.to_string(),
                primary: Location {
                    file: PathBuf::from(file),
                    start_line: line,
                    start_col: 1,
                    end_line: line,
                    end_col: 2,
                },
                related: vec![],
                message: message.to_string(),
                raw_severity: Severity::Warning,
                anomaly_class: AnomalyClass::Logic,
                evidence: Evidence {
                    citation_keys: vec!["test-2026"],
                    raw: serde_json::Value::Null,
                    language_citation_status: LanguageCitationStatus::Confirmed,
                },
                origin: Default::default(),
            },
            posterior_tp: None,
            wilson_lower: None,
            prior_method: None,
            rank_score: 1.0,
            adjudication: None,
        }
    }

    #[test]
    fn normalize_collapses_digit_runs() {
        assert_eq!(
            normalize_message("preceded by return on line 42"),
            "preceded by return on line #"
        );
        assert_eq!(
            normalize_message("diverged from 12 similar siblings"),
            "diverged from # similar siblings"
        );
        assert_eq!(normalize_message("no digits here"), "no digits here");
        assert_eq!(normalize_message("v1a2b33"), "v#a#b#");
    }

    #[test]
    fn file_key_strips_scan_root_and_uses_forward_slashes() {
        assert_eq!(
            file_key(Path::new("/repo/src/a.rs"), Path::new("/repo")),
            "src/a.rs"
        );
        // Path not under the root falls back to the path as given.
        assert_eq!(
            file_key(Path::new("/elsewhere/a.rs"), Path::new("/repo")),
            "/elsewhere/a.rs"
        );
        // Single-file scan: the root IS the file.
        assert_eq!(
            file_key(Path::new("/repo/src/a.rs"), Path::new("/repo/src/a.rs")),
            "a.rs"
        );
    }

    #[test]
    fn line_shift_keeps_fingerprint_stable() {
        let root = Path::new("/repo");
        let before = ranked(
            "unreachable-after-terminator",
            "/repo/a.rs",
            10,
            "statement is unreachable; preceded by return on line 9",
        );
        let after = ranked(
            "unreachable-after-terminator",
            "/repo/a.rs",
            14,
            "statement is unreachable; preceded by return on line 13",
        );
        let baseline = build(std::slice::from_ref(&before), root);
        let outcome = filter_ranked(&baseline, vec![after], root);
        assert_eq!(outcome.suppressed, 1);
        assert!(outcome.kept.is_empty());
    }

    #[test]
    fn count_budget_surfaces_the_excess_occurrence() {
        let root = Path::new("/repo");
        let mk = || ranked("clone-drift", "/repo/a.rs", 1, "diverged from 3 siblings");
        let baseline = build(&[mk(), mk()], root);
        assert_eq!(baseline.entries.len(), 1);
        assert_eq!(baseline.entries[0].count, 2);
        let outcome = filter_ranked(&baseline, vec![mk(), mk(), mk()], root);
        assert_eq!(outcome.suppressed, 2);
        assert_eq!(outcome.kept.len(), 1, "third occurrence must surface");
    }

    #[test]
    fn different_detector_or_file_is_not_suppressed() {
        let root = Path::new("/repo");
        let known = ranked("arg-swap", "/repo/a.rs", 1, "swapped");
        let baseline = build(std::slice::from_ref(&known), root);
        let other_file = ranked("arg-swap", "/repo/b.rs", 1, "swapped");
        let other_detector = ranked("clone-drift", "/repo/a.rs", 1, "swapped");
        let outcome = filter_ranked(&baseline, vec![other_file, other_detector], root);
        assert_eq!(outcome.suppressed, 0);
        assert_eq!(outcome.kept.len(), 2);
    }

    #[test]
    fn save_load_round_trips_and_rejects_future_versions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("baseline.json");
        let root = Path::new("/repo");
        let baseline = build(
            &[ranked("arg-swap", "/repo/a.rs", 1, "swapped 2 args")],
            root,
        );
        save(&path, &baseline).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.version, BASELINE_FORMAT_VERSION);
        assert_eq!(loaded.entries, baseline.entries);

        let future = path.with_file_name("future.json");
        fs::write(&future, r#"{"version": 999, "entries": []}"#).unwrap();
        match load(&future) {
            Err(BaselineError::Version { found, .. }) => assert_eq!(found, 999),
            other => panic!("expected Version error, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn missing_file_is_an_io_error() {
        match load(Path::new("/nonexistent/cntrdct-baseline.json")) {
            Err(BaselineError::Io { .. }) => {}
            other => panic!("expected Io error, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn entries_are_sorted_for_stable_diffs() {
        let root = Path::new("/repo");
        let baseline = build(
            &[
                ranked("clone-drift", "/repo/b.rs", 1, "m"),
                ranked("arg-swap", "/repo/a.rs", 1, "m"),
                ranked("clone-drift", "/repo/a.rs", 1, "m"),
            ],
            root,
        );
        let keys: Vec<(&str, &str)> = baseline
            .entries
            .iter()
            .map(|e| (e.detector_id.as_str(), e.file.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("arg-swap", "a.rs"),
                ("clone-drift", "a.rs"),
                ("clone-drift", "b.rs"),
            ]
        );
    }
}
