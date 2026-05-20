//! config-interaction detector — flag items carrying a pair of cfg attributes
//! whose predicates are structurally negations of each other.
//!
//! Spec: `cntrdct/docs/spec/config-interaction-v0.md`.

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, ParsedFile, Severity,
};
use rayon::prelude::*;

static CITATIONS: &[Citation] = &[
    Citation {
        key: "tartler-eurosys-2011",
        authors: "B. Tartler, D. Lohmann, J. Sincero, W. Schröder-Preikschat",
        title:
            "Feature consistency in compile-time-configurable system software: facing the Linux 10,000 feature problem",
        venue: "EuroSys 2011",
        year: 2011,
        doi: Some("10.1145/1966445.1966451"),
        url: None,
        // Pattern B: config-interaction is Rust-specific by definition
        // (it scans `#[cfg(...)]` attributes). The cited Linux/C work
        // grounds the broader concept; per the grandfather clause we
        // mark it as Rust-grounded.
        languages: &[Language::Rust],
    },
    Citation {
        key: "nadi-icse-2014",
        authors: "S. Nadi, T. Berger, C. Kästner, K. Czarnecki",
        title: "Mining configuration constraints: Static analyses and empirical results",
        venue: "ICSE 2014",
        year: 2014,
        doi: Some("10.1145/2568225.2568283"),
        url: None,
        languages: &[Language::Rust],
    },
];

const ITEM_KINDS: &[&str] = &[
    "function_item",
    "struct_item",
    "enum_item",
    "mod_item",
    "impl_item",
    "trait_item",
    "static_item",
    "const_item",
    "type_item",
    "union_item",
];

#[derive(Debug, Default)]
pub struct ConfigInteraction;

impl ConfigInteraction {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for ConfigInteraction {
    fn id(&self) -> &'static str {
        "config-interaction"
    }

    fn name(&self) -> &'static str {
        "Config Interaction"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [Language] {
        &[Language::Rust]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = ctx
            .files
            .par_iter()
            .filter(|f| f.language == Language::Rust)
            .flat_map_iter(|file| {
                let mut local = Vec::new();
                scan_file(file, &mut local);
                local
            })
            .collect();
        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
                .then(a.primary.start_col.cmp(&b.primary.start_col))
        });
        Ok(findings)
    }
}

fn scan_file(file: &ParsedFile, findings: &mut Vec<Finding>) {
    let mut parser = tree_sitter::Parser::new();
    let lang = crate::parsers::parser_for(Language::Rust).ts_language();
    if parser.set_language(&lang).is_err() {
        return;
    }
    let tree = match parser.parse(&file.source, None) {
        Some(t) => t,
        None => return,
    };
    let root = tree.root_node();
    if root.has_error() {
        return;
    }
    walk(root, file, findings);
}

fn walk(node: tree_sitter::Node, file: &ParsedFile, findings: &mut Vec<Finding>) {
    if ITEM_KINDS.contains(&node.kind()) {
        if let Some(f) = analyze_item(node, file) {
            findings.push(f);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, file, findings);
    }
}

#[derive(Debug, Clone)]
struct CfgAttr {
    canonical: String,
    is_not: bool,
    inner_canonical: String,
    location: (u32, u32, u32, u32),
}

fn analyze_item(item: tree_sitter::Node, file: &ParsedFile) -> Option<Finding> {
    let attrs = collect_cfg_attrs(item, &file.source);
    if attrs.len() < 2 {
        return None;
    }

    let mut pairs: Vec<(usize, usize, ContradictionKind)> = Vec::new();
    for i in 0..attrs.len() {
        for j in (i + 1)..attrs.len() {
            if let Some(kind) = classify_contradiction(&attrs[i], &attrs[j]) {
                pairs.push((i, j, kind));
            }
        }
    }
    if pairs.is_empty() {
        return None;
    }

    let (i0, j0, kind) = pairs[0].clone();
    let a = &attrs[i0];
    let b = &attrs[j0];

    let inner_predicate = match &kind {
        ContradictionKind::NotPair => {
            if a.is_not {
                a.inner_canonical.clone()
            } else {
                b.inner_canonical.clone()
            }
        }
        // F5b: `cfg(true)` and `cfg(false)` are atomic primitive
        // contradictions — there is no shared inner predicate to
        // report, so emit a literal description for downstream
        // consumers and the SARIF message.
        ContradictionKind::TrueFalse => "true vs false".to_string(),
    };

    let primary = node_location(file, item);
    let related = vec![
        location_from_tuple(file, a.location),
        location_from_tuple(file, b.location),
    ];

    let attribute_lines = vec![a.location.0, b.location.0];
    let additional_pairs = pairs.len() - 1;

    let message = match &kind {
        ContradictionKind::NotPair => format!(
            "item carries cfg pair `cfg({pred})` and `cfg(not({pred}))` — unsatisfiable under any configuration",
            pred = inner_predicate
        ),
        ContradictionKind::TrueFalse => "item carries cfg pair `cfg(true)` and `cfg(false)` — unsatisfiable under any configuration".to_string(),
    };

    Some(Finding {
        detector_id: "config-interaction".to_string(),
        primary,
        related,
        message,
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["tartler-eurosys-2011", "nadi-icse-2014"],
            raw: serde_json::json!({
                "inner_predicate": inner_predicate,
                "contradiction_kind": kind.as_str(),
                "attribute_lines": attribute_lines,
                "additional_pairs": additional_pairs,
            }),
            language_citation_status: LanguageCitationStatus::Confirmed,
        },
    })
}

fn collect_cfg_attrs(item: tree_sitter::Node, source: &str) -> Vec<CfgAttr> {
    let mut attrs: Vec<CfgAttr> = Vec::new();

    // Outer attrs: walk preceding siblings of the item that are attribute_item nodes
    let mut sib = item.prev_named_sibling();
    let mut outer: Vec<tree_sitter::Node> = Vec::new();
    while let Some(s) = sib {
        if s.kind() == "attribute_item" {
            outer.push(s);
            sib = s.prev_named_sibling();
        } else {
            break;
        }
    }
    outer.reverse();
    for a in outer {
        if let Some(parsed) = parse_cfg_attribute(a, source) {
            attrs.push(parsed);
        }
    }

    // Inner attrs: children of item that are inner_attribute_item or attribute_item
    // (some grammars place outer attrs as children of the item too)
    let mut cursor = item.walk();
    for child in item.children(&mut cursor) {
        if matches!(child.kind(), "inner_attribute_item" | "attribute_item") {
            if let Some(parsed) = parse_cfg_attribute(child, source) {
                attrs.push(parsed);
            }
        }
    }

    attrs
}

/// Parse an attribute_item (or inner_attribute_item) node. Returns Some only
/// when the attribute is a top-level `cfg(...)` invocation (not `cfg_attr`,
/// not a different attribute name).
fn parse_cfg_attribute(attr: tree_sitter::Node, source: &str) -> Option<CfgAttr> {
    let text = attr.utf8_text(source.as_bytes()).ok()?;
    let inside = strip_attribute_brackets(text)?;
    let trimmed = inside.trim();

    let after_cfg = strip_cfg_call(trimmed)?;
    let canonical = canonicalize(after_cfg);

    let is_not;
    let inner_canonical;
    if let Some(inner) = strip_not_call(&canonical) {
        is_not = true;
        inner_canonical = inner;
    } else {
        is_not = false;
        inner_canonical = canonical.clone();
    }

    let start = attr.start_position();
    let end = attr.end_position();
    Some(CfgAttr {
        canonical,
        is_not,
        inner_canonical,
        location: (
            start.row as u32 + 1,
            start.column as u32 + 1,
            end.row as u32 + 1,
            end.column as u32 + 1,
        ),
    })
}

/// `#[ ... ]` or `#![ ... ]` → ` ... `. Returns None on shape mismatch.
fn strip_attribute_brackets(text: &str) -> Option<&str> {
    let t = text.trim();
    let after_hash = t.strip_prefix('#')?;
    let after_bang_or_bracket = after_hash.strip_prefix('!').unwrap_or(after_hash);
    let body = after_bang_or_bracket.strip_prefix('[')?.strip_suffix(']')?;
    Some(body)
}

/// `cfg(<pred>)` → `<pred>`. Returns None when the path is not exactly `cfg`,
/// or when the call shape does not match. Rejects `cfg_attr` explicitly.
fn strip_cfg_call(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    // Must start with "cfg" followed by `(`. Reject "cfg_attr".
    let rest = trimmed.strip_prefix("cfg")?;
    let rest = rest.trim_start();
    if rest.starts_with('_') {
        return None;
    }
    let inside = rest.strip_prefix('(')?.strip_suffix(')')?;
    Some(inside)
}

/// `not(<pred>)` → Some(canonicalized `<pred>`). None on shape mismatch.
fn strip_not_call(canonical: &str) -> Option<String> {
    let trimmed = canonical.trim();
    let rest = trimmed.strip_prefix("not")?;
    let rest = rest.trim_start();
    let inside = rest.strip_prefix('(')?.strip_suffix(')')?;
    Some(canonicalize(inside))
}

fn canonicalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(ch);
            last_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

/// F5 classification: kind of structural contradiction between two
/// cfg attributes attached to the same item. `NotPair` is the v0
/// `not(X)` vs `X` shape (F5a). `TrueFalse` is the F5b primitive-
/// constant pair: `cfg(true)` and `cfg(false)` are unsatisfiable
/// together because no configuration can make a literal `false`
/// hold while a literal `true` also holds (the AND is the literal
/// `false`).
#[derive(Debug, Clone)]
enum ContradictionKind {
    NotPair,
    TrueFalse,
}

impl ContradictionKind {
    fn as_str(&self) -> &'static str {
        match self {
            ContradictionKind::NotPair => "not-pair",
            ContradictionKind::TrueFalse => "true-false",
        }
    }
}

fn classify_contradiction(a: &CfgAttr, b: &CfgAttr) -> Option<ContradictionKind> {
    // F5a (original): one predicate is `not(X)` and the other is
    // structurally equal to `X`.
    if a.is_not && !b.is_not && a.inner_canonical == b.canonical {
        return Some(ContradictionKind::NotPair);
    }
    if b.is_not && !a.is_not && b.inner_canonical == a.canonical {
        return Some(ContradictionKind::NotPair);
    }
    // F5b (added 2026-05-21): primitive `true` paired with `false`.
    // rustc's `cfg.attr.duplicates` reference behaviour treats two
    // cfg attributes as conjunctive — `#[cfg(true)] #[cfg(false)]`
    // means `cfg(true) AND cfg(false)`, which simplifies to
    // `cfg(false)` and disables the item under every configuration.
    // The shape is structurally NOT a `not(X) / X` pair, so the v0
    // F5a check missed it (audit-corpus rustc_ui_both_true_false.rs
    // lines 11 and 15). The primitive atoms are syntactically
    // contradictory; no further reasoning is needed.
    let primitive_pair = matches!(
        (a.canonical.as_str(), b.canonical.as_str()),
        ("true", "false") | ("false", "true")
    );
    if primitive_pair {
        return Some(ContradictionKind::TrueFalse);
    }
    None
}

fn node_location(file: &ParsedFile, node: tree_sitter::Node) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        file: file.path.clone(),
        start_line: start.row as u32 + 1,
        start_col: start.column as u32 + 1,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32 + 1,
    }
}

fn location_from_tuple(file: &ParsedFile, t: (u32, u32, u32, u32)) -> Location {
    Location {
        file: file.path.clone(),
        start_line: t.0,
        start_col: t.1,
        end_line: t.2,
        end_col: t.3,
    }
}
