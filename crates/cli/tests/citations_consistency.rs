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
use cntrdct_core::{
    register_detector, Citation, DetectContext, Detector, DetectorError, Finding, Language,
};
use cntrdct_detector_arg_swap::ArgSwap;
use cntrdct_detector_clone_drift::CloneDrift;
use cntrdct_detector_comment_code::CommentCode;
use cntrdct_detector_config_interaction::ConfigInteraction;
use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;

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
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

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
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

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
        Box::new(UnreachableAfterTerminator::new()),
        Box::new(ConfigInteraction::new()),
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

// ---------- M-6: per-language citation policy ----------
//
// Per `docs/spec/citations-policy.md`:
//
// - P1 itself is unchanged: every detector has at least one citation.
//   `register_detector` enforces this and `no_detector_has_empty_citations`
//   above asserts it.
//
// - The per-language citation requirement is SHOULD, not MUST. A detector
//   may declare a language without any citation grounded in that language;
//   the gap surfaces in `LanguageCitationStatus::Unconfirmed` on each
//   emitted finding rather than blocking registration.
//
// The tests below document this contract in code: an under-cited fixture
// detector still registers cleanly, a `supported_languages()` entry that
// is not a recognised canonical name fails, and citations on a single
// detector cannot share a `key` (catches retro-fit copy-paste).

#[test]
fn no_detector_has_duplicate_citation_keys() {
    for d in registered_detectors() {
        let keys: Vec<&'static str> = d.citations().iter().map(|c| c.key).collect();
        let mut seen: BTreeSet<&'static str> = BTreeSet::new();
        for key in &keys {
            assert!(
                seen.insert(*key),
                "Detector `{}` lists citation key `{}` more than once",
                d.id(),
                key
            );
        }
    }
}

// `every_supported_language_is_a_known_canonical_name` was retired in
// F4-4b: `supported_languages()` now returns `&[Language]`, so every
// entry is by construction a valid variant. The runtime invariant the
// test enforced (string → enum mapping) no longer exists at the trait
// boundary; the type system provides the same guarantee unconditionally.

/// Fixture: a detector declaring Python support but no Python-grounded
/// citation. Per `citations-policy.md`, this is an acceptable state: the
/// detector continues to register cleanly because P1 (≥1 citation overall)
/// is satisfied. Per-language coverage is best-effort and surfaces via
/// `LanguageCitationStatus::Unconfirmed` at finding emission time.
struct UnderCitedFixture;

static UNDER_CITED_FIXTURE_CITATIONS: &[Citation] = &[Citation {
    key: "fixture-citation-2026",
    authors: "Fixture",
    title: "Fixture",
    venue: "Fixture",
    year: 2026,
    doi: None,
    url: None,
    languages: &[Language::Rust], // grounded in Rust only
}];

impl Detector for UnderCitedFixture {
    fn id(&self) -> &'static str {
        "fixture-under-cited"
    }
    fn name(&self) -> &'static str {
        "Under-cited fixture"
    }
    fn citations(&self) -> &'static [Citation] {
        UNDER_CITED_FIXTURE_CITATIONS
    }
    fn supported_languages(&self) -> &'static [Language] {
        // Python is declared but no citation in this set is grounded in
        // Python. The policy permits this; the detector should still
        // register and emit findings with Unconfirmed status.
        &[Language::Rust, Language::Python]
    }
    fn detect(&self, _: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        Ok(vec![])
    }
}

#[test]
fn under_cited_fixture_still_registers_per_should_policy() {
    // P1 gate is satisfied (one citation present). Per-language gap is
    // intentionally tolerated by the policy.
    register_detector(&UnderCitedFixture)
        .expect("under-cited fixture must still register under SHOULD policy");
}

#[test]
fn under_cited_fixture_has_python_without_python_citation() {
    // Document the test scenario structurally so a future change that
    // accidentally tightens the policy (or accidentally adds Python
    // grounding to the fixture) breaks this assertion deliberately and
    // the spec rationale is re-examined.
    let langs = UnderCitedFixture.supported_languages();
    assert!(langs.contains(&Language::Python));
    let any_python_citation = UNDER_CITED_FIXTURE_CITATIONS
        .iter()
        .any(|c| c.languages.contains(&Language::Python));
    assert!(
        !any_python_citation,
        "fixture must lack a Python-grounded citation to model the SHOULD scenario"
    );
}
