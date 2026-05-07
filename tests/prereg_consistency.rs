//! Consistency guard between the OSF preregistration draft and the live
//! detector set.
//!
//! Per design constraint P1 every detector cites peer-reviewed prior art.
//! The OSF preregistration commits — in advance of running the eval harness
//! — to which detectors are being measured and which citations they rest on.
//! This test enforces that the most recent prereg file under `prereg/`
//!
//! 1. exists at all,
//! 2. carries the canonical OSF section headings (Hypotheses, Design Plan,
//!    Sampling Plan, Variables, Analysis Plan, References),
//! 3. links to the eval harness spec, and
//! 4. names every Layer 1 citation key currently returned by a registered
//!    detector.
//!
//! Adding a new detector therefore REQUIRES a fresh, dated prereg (or an
//! amendment) before the test suite turns green again — which is the whole
//! point of preregistration.
//!
//! Sibling artefacts (labelling rubrics, prereg addenda) live alongside the
//! formal prereg in the same directory but follow different schemas — they
//! reference the parent rather than restate it. They are skipped by name
//! pattern so the consistency check stays focused on full preregistrations.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use cntrdct::core::Detector;
use cntrdct::detectors::arg_swap::ArgSwap;
use cntrdct::detectors::clone_drift::CloneDrift;
use cntrdct::detectors::comment_code::CommentCode;
use cntrdct::detectors::config_interaction::ConfigInteraction;
use cntrdct::detectors::pr_miner::PrMinerDetector;
use cntrdct::detectors::unreachable_after_terminator::UnreachableAfterTerminator;

const REQUIRED_SECTIONS: &[&str] = &[
    "## Hypotheses",
    "## Design Plan",
    "## Sampling Plan",
    "## Variables",
    "## Analysis Plan",
    "## References",
];

const EVAL_SPEC_REF: &str = "docs/spec/eval-v0.md";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn registered_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(CloneDrift::new()),
        Box::new(ArgSwap::new()),
        Box::new(CommentCode::new()),
        Box::new(UnreachableAfterTerminator::new()),
        Box::new(ConfigInteraction::new()),
        Box::new(PrMinerDetector::new()),
    ]
}

#[test]
fn registered_detectors_match_canonical_id_set() {
    let mut got: Vec<String> = registered_detectors()
        .iter()
        .map(|d| d.id().to_string())
        .collect();
    got.sort();
    let mut want: Vec<String> = cntrdct::ALL_DETECTOR_IDS
        .iter()
        .map(|s| s.to_string())
        .collect();
    want.sort();
    assert_eq!(
        got, want,
        "tests/prereg_consistency.rs::registered_detectors() and \
         cntrdct::ALL_DETECTOR_IDS disagree; update both together"
    );
}

// ---------- Q-8: preregistration deviation log ----------
//
// Per `docs/spec/citations-policy.md` and `ROADMAP.md` Q-8, every
// preregistration revision (a `prereg/<date>-osf-prereg.md` carrying
// a `Supersedes:` header) must be accompanied by an entry under
// `prereg/deviations/<date>-<topic>.md` enumerating which sections of
// the prior frozen prereg changed and why. This is the operational
// half of the preregistration discipline: the deviation log is what
// turns supersession into an audit trail rather than silent revision.
//
// Evidence: van den Akker et al. (2024) Psychological Methods,
// doi:10.1037/met0000687, reports that a large majority of
// deviations from preregistered analyses go undocumented. The
// matching deviation file is the smallest mechanical guard against
// that failure mode for cntrdct's preregistration cadence.

const DEVIATION_HEADERS: &[&str] = &["Prereg:", "Supersedes:", "Author:", "Date:"];

fn deviations_dir() -> PathBuf {
    prereg_dir().join("deviations")
}

fn list_deviation_logs() -> Vec<PathBuf> {
    let dir = deviations_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(OsStr::to_str) == Some("md"))
        .collect();
    files.sort();
    files
}

/// First 10 characters of a file stem, expected to be a `YYYY-MM-DD`
/// ISO date. The prereg file naming convention pins this position so
/// matching against deviation log filenames is a cheap prefix check.
fn date_prefix(path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem().and_then(OsStr::to_str)?;
    if stem.len() < 10 {
        return None;
    }
    Some(stem[..10].to_string())
}

#[test]
fn every_supersession_has_a_matching_deviation_log() {
    let logs = list_deviation_logs();
    for path in list_prereg_markdown() {
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let carries_supersedes = text
            .lines()
            .any(|l| l.trim_start().starts_with("Supersedes:"));
        if !carries_supersedes {
            // Initial preregistration. No prior frozen prereg exists,
            // so no deviation log is required.
            continue;
        }
        let prefix = date_prefix(&path).unwrap_or_else(|| {
            panic!(
                "prereg {} stem must start with a YYYY-MM-DD date",
                path.display()
            )
        });
        let matching: Vec<&PathBuf> = logs
            .iter()
            .filter(|p| {
                p.file_name()
                    .and_then(OsStr::to_str)
                    .map(|n| n.starts_with(&prefix))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            !matching.is_empty(),
            "prereg {} carries `Supersedes:` but no matching deviation log \
             under prereg/deviations/{}-*.md exists. Add one before merging \
             — silent supersession is the documented Q-8 failure mode.",
            path.display(),
            prefix
        );
    }
}

#[test]
fn deviation_logs_carry_required_headers() {
    for path in list_deviation_logs() {
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let missing: Vec<&&str> = DEVIATION_HEADERS
            .iter()
            .filter(|h| !text.contains(**h))
            .collect();
        assert!(
            missing.is_empty(),
            "deviation log {} is missing required headers: {:?}",
            path.display(),
            missing
        );
    }
}

#[test]
fn deviation_log_supersedes_resolves_to_a_real_prereg_file() {
    for path in list_deviation_logs() {
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        // `Supersedes:` line in the deviation front matter points at
        // the prereg file that was frozen at the time of the
        // deviation. We accept either a backtick-quoted path or a
        // bare path; both forms appear historically.
        let line = text
            .lines()
            .find(|l| l.trim_start().starts_with("Supersedes:"))
            .unwrap_or_else(|| panic!("{} has no `Supersedes:` line", path.display()));
        let value = line.trim_start().trim_start_matches("Supersedes:").trim();
        let cleaned = value.trim_matches('`');
        let candidate = workspace_root().join(cleaned);
        assert!(
            candidate.is_file(),
            "deviation log {} references missing prereg file `{}`",
            path.display(),
            cleaned
        );
    }
}

fn current_citation_keys() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for d in registered_detectors() {
        for c in d.citations() {
            out.insert(c.key.to_string());
        }
    }
    out
}

fn prereg_dir() -> PathBuf {
    workspace_root().join("prereg")
}

fn list_prereg_markdown() -> Vec<PathBuf> {
    let dir = prereg_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|p| p.extension().and_then(OsStr::to_str) == Some("md"))
        .filter(|p| {
            let stem = p.file_stem().and_then(OsStr::to_str).unwrap_or_default();
            // Sibling artefacts live next to the formal prereg but follow a
            // different schema; skip them so `latest_prereg()` keeps pointing
            // at the most recent full preregistration.
            !stem.contains("-rubric-")
                && !stem.contains("-addendum")
                && !stem.contains("-failure-modes-")
        })
        .collect();
    files.sort();
    files
}

fn latest_prereg() -> PathBuf {
    let files = list_prereg_markdown();
    assert!(
        !files.is_empty(),
        "expected at least one *.md under {}/",
        prereg_dir().display()
    );
    files.into_iter().last().unwrap()
}

#[test]
fn prereg_directory_contains_at_least_one_markdown_file() {
    let dir = prereg_dir();
    assert!(dir.is_dir(), "expected directory at {}", dir.display());
    assert!(
        !list_prereg_markdown().is_empty(),
        "no *.md files under {}",
        dir.display()
    );
}

#[test]
fn latest_prereg_contains_required_sections() {
    let path = latest_prereg();
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let missing: Vec<&str> = REQUIRED_SECTIONS
        .iter()
        .copied()
        .filter(|h| !text.contains(h))
        .collect();
    assert!(
        missing.is_empty(),
        "{} is missing required headings: {:?}",
        path.display(),
        missing
    );
}

#[test]
fn latest_prereg_references_eval_spec() {
    let path = latest_prereg();
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    assert!(
        text.contains(EVAL_SPEC_REF),
        "{} does not reference {}",
        path.display(),
        EVAL_SPEC_REF
    );
}

#[test]
fn latest_prereg_cites_every_registered_layer1_key() {
    let path = latest_prereg();
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let needed = current_citation_keys();
    let missing: Vec<&String> = needed
        .iter()
        .filter(|k| !text.contains(k.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "{} does not cite the following Layer 1 citation keys: {:?}",
        path.display(),
        missing
    );
}
