//! Consistency guard between `CITATIONS.md` and live `Detector::citations()`.
//!
//! Spec: per design constraint P1, every detector cites peer-reviewed prior art.
//! `CITATIONS.md` is the user-facing index of those citations. This test enforces
//! that, for each detector registered by the CLI (`clone-drift`, `arg-swap`),
//! the keys listed under the matching `### <detector-id>` subsection of
//! `## Layer 1` agree exactly with `Detector::citations()`.
//!
//! It catches two failure modes:
//! 1. A detector references a citation key that is missing from its subsection
//!    in `CITATIONS.md`.
//! 2. A `CITATIONS.md` subsection advertises a key that the live detector does
//!    not return from `citations()`.
//!
//! Future, unimplemented Layer 1 detectors (e.g. `comment-code`) may have their
//! own subsections without breaking this test — only registered detectors are
//! enforced.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use cntrdct_adjudicator_llm::ADJUDICATOR_CITATIONS;
use cntrdct_core::Detector;
use cntrdct_detector_arg_swap::ArgSwap;
use cntrdct_detector_clone_drift::CloneDrift;
use cntrdct_detector_comment_code::CommentCode;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/cli; the workspace root is two levels up.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above crates/cli")
        .to_path_buf()
}

/// Parse `CITATIONS.md` and return the set of citation keys listed under the
/// `## Layer <n>` heading (no `### <id>` subsections required).
///
/// Used by Layer 3 (LLM adjudicator), which is a single component rather than
/// a family of detectors and so does not need a per-id subsection.
fn layer_keys(layer_heading_prefix: &str) -> BTreeSet<String> {
    let path = workspace_root().join("CITATIONS.md");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    let mut in_target = false;
    let mut keys: BTreeSet<String> = BTreeSet::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("## ") {
            in_target = rest.starts_with(layer_heading_prefix);
            continue;
        }
        if !in_target {
            continue;
        }
        let bullet = match trimmed.strip_prefix("- ") {
            Some(b) => b,
            None => continue,
        };
        let after_open_tick = match bullet.strip_prefix('`') {
            Some(s) => s,
            None => continue,
        };
        if let Some(close) = after_open_tick.find('`') {
            let key = &after_open_tick[..close];
            if !key.is_empty() {
                keys.insert(key.to_string());
            }
        }
    }
    keys
}

/// Parse `CITATIONS.md` and return a map from detector id (the `### <id>`
/// heading text under `## Layer 1`) to the set of citation keys listed under
/// that subsection. Keys appear as bullet lines of the form `` - `<key>` — ...``.
fn layer1_subsections() -> BTreeMap<String, BTreeSet<String>> {
    let path = workspace_root().join("CITATIONS.md");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    let mut in_layer1 = false;
    let mut current: Option<String> = None;
    let mut sections: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for line in text.lines() {
        let trimmed = line.trim_start();

        if let Some(rest) = trimmed.strip_prefix("## ") {
            in_layer1 = rest.starts_with("Layer 1");
            current = None;
            continue;
        }

        if !in_layer1 {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("### ") {
            current = Some(rest.trim().to_string());
            sections.entry(current.clone().unwrap()).or_default();
            continue;
        }

        let section_id = match &current {
            Some(s) => s,
            None => continue,
        };

        let bullet = match trimmed.strip_prefix("- ") {
            Some(b) => b,
            None => continue,
        };
        let after_open_tick = match bullet.strip_prefix('`') {
            Some(s) => s,
            None => continue,
        };
        let close = match after_open_tick.find('`') {
            Some(idx) => idx,
            None => continue,
        };
        let key = &after_open_tick[..close];
        if !key.is_empty() {
            sections
                .entry(section_id.clone())
                .or_default()
                .insert(key.to_string());
        }
    }
    sections
}

fn registered_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(CloneDrift::new()),
        Box::new(ArgSwap::new()),
        Box::new(CommentCode::new()),
    ]
}

fn detector_keys(d: &dyn Detector) -> BTreeSet<String> {
    d.citations().iter().map(|c| c.key.to_string()).collect()
}

#[test]
fn every_registered_detector_has_a_layer1_subsection() {
    let sections = layer1_subsections();
    for d in registered_detectors() {
        assert!(
            sections.contains_key(d.id()),
            "Detector `{}` has no `### {}` subsection under `## Layer 1` in CITATIONS.md",
            d.id(),
            d.id()
        );
    }
}

#[test]
fn detector_keys_match_layer1_subsection_exactly() {
    let sections = layer1_subsections();
    for d in registered_detectors() {
        let det_keys = detector_keys(&*d);
        let md_keys = sections
            .get(d.id())
            .unwrap_or_else(|| panic!("missing `### {}` subsection", d.id()))
            .clone();

        let only_in_det: Vec<&String> = det_keys.difference(&md_keys).collect();
        assert!(
            only_in_det.is_empty(),
            "Detector `{}` cites keys not listed under its CITATIONS.md subsection: {:?}",
            d.id(),
            only_in_det
        );

        let only_in_md: Vec<&String> = md_keys.difference(&det_keys).collect();
        assert!(
            only_in_md.is_empty(),
            "CITATIONS.md `### {}` advertises keys the detector does not return: {:?}",
            d.id(),
            only_in_md
        );
    }
}

#[test]
fn no_detector_has_empty_citations() {
    for d in registered_detectors() {
        assert!(
            !detector_keys(&*d).is_empty(),
            "Detector `{}` returned no citations (P1 violation)",
            d.id()
        );
    }
}

// ---------- Layer 3 (Adjudicator) consistency ----------
//
// The adjudicator is a single component, not a family of detectors, so
// CITATIONS.md uses bullet lines under `## Layer 3 (LLM adjudicator)`
// instead of `### <id>` subsections. Mirror Layer 1's contract: every key
// surfaced from `cntrdct_adjudicator_llm::ADJUDICATOR_CITATIONS` MUST
// appear under that section, and vice versa.

fn adjudicator_keys() -> BTreeSet<String> {
    ADJUDICATOR_CITATIONS
        .iter()
        .map(|c| c.key.to_string())
        .collect()
}

#[test]
fn adjudicator_citations_match_layer3_section_exactly() {
    let md_keys = layer_keys("Layer 3");
    let code_keys = adjudicator_keys();

    let only_in_code: Vec<&String> = code_keys.difference(&md_keys).collect();
    assert!(
        only_in_code.is_empty(),
        "Adjudicator cites keys not listed under `## Layer 3` in CITATIONS.md: {:?}",
        only_in_code
    );

    let only_in_md: Vec<&String> = md_keys.difference(&code_keys).collect();
    assert!(
        only_in_md.is_empty(),
        "CITATIONS.md `## Layer 3` advertises keys the adjudicator does not return: {:?}",
        only_in_md
    );
}

#[test]
fn adjudicator_has_at_least_one_citation() {
    assert!(
        !adjudicator_keys().is_empty(),
        "Adjudicator returned no citations (Layer 3 P1 analogue violation)"
    );
}
