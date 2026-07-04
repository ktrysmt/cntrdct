//! Lockstep guard between the GitHub Action's per-path language universe
//! (`prepare_config.py::KNOWN_LANGUAGES`) and the parser crate's
//! `Language::all()`.
//!
//! `docs/spec/multilang-v0.md` (Compatibility) requires the action's
//! hard-coded universe to be updated "in lockstep with `Language::all()`".
//! Before this test that lockstep was manual and drifted (the script sat
//! at `rust, python` long after TypeScript / Go / `.tsx` shipped). The
//! assertion below fails the build the moment the two disagree, so adding
//! a language forces the one-line script update in the same change.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use cntrdct::core::Language;

#[test]
fn action_language_universe_matches_all() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(".github/actions/scan/scripts/prepare_config.py");
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    // Isolate the `KNOWN_LANGUAGES: List[str] = [ ... ]` list literal.
    // Split on `=` first so the `List[str]` type annotation's brackets do
    // not get mistaken for the list literal.
    let after_name = src
        .split_once("KNOWN_LANGUAGES")
        .unwrap_or_else(|| panic!("KNOWN_LANGUAGES not found in {}", path.display()))
        .1;
    let after_eq = after_name
        .split_once('=')
        .expect("KNOWN_LANGUAGES assignment has no `=`")
        .1;
    let open = after_eq.find('[').expect("KNOWN_LANGUAGES has no `[`");
    let close = open
        + after_eq[open..]
            .find(']')
            .expect("KNOWN_LANGUAGES has no `]`");
    let list_body = &after_eq[open + 1..close];

    let script_langs: BTreeSet<String> = list_body
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let expected: BTreeSet<String> = Language::all()
        .iter()
        .map(|l| l.canonical_name().to_string())
        .collect();

    assert_eq!(
        script_langs,
        expected,
        "prepare_config.py KNOWN_LANGUAGES drifted from Language::all(); \
         update {} to {:?}",
        path.display(),
        expected
    );
}
