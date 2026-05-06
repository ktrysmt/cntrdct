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
    ]
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
            !stem.contains("-rubric-") && !stem.contains("-addendum")
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
