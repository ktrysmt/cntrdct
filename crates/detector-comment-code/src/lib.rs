//! comment-code detector — pattern-based comment/implementation mismatch.
//!
//! Spec: `cntrdct/docs/spec/comment-code-v0.md`.
//!
//! Algorithm:
//! 1. Parse each Rust file with tree-sitter; skip files with parse errors.
//! 2. For each top-level `function_item`, gather the immediately preceding
//!    `///` line-comment block into a single rendered doc string.
//! 3. Apply three hardcoded checks (Pattern A/B/C) against the rendered text,
//!    the function's return type text, body source, and attribute set.
//! 4. Emit one Finding per match.

use cntrdct_core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding,
    Location, ParsedFile, Severity,
};

static CITATIONS: &[Citation] = &[
    Citation {
        key: "tan-sosp-2007",
        authors: "L. Tan, D. Yuan, G. Krishna, Y. Zhou",
        title: "/*iComment: Bugs or Bad Comments?*/",
        venue: "SOSP 2007",
        year: 2007,
        doi: None,
        url: None,
    },
    Citation {
        key: "tan-pldi-2011",
        authors: "L. Tan, Y. Zhou, Y. Padioleau",
        title: "aComment: Mining Annotations from Comments and Code to Detect Interrupt-related Concurrency Bugs",
        venue: "PLDI 2011",
        year: 2011,
        doi: None,
        url: None,
    },
];

const PATTERN_A_TRIGGERS: &[&str] = &[
    "returns err",
    "returns result",
    "may fail",
    "fallible",
    "returns option",
    "may return none",
];

const PATTERN_B_BODY_MARKERS: &[&str] = &[
    "panic!",
    "unwrap",
    "expect(",
    "unreachable!",
    "assert!",
    "assert_eq!",
    "assert_ne!",
    "todo!",
    "unimplemented!",
    "debug_assert",
];

#[derive(Debug, Default)]
pub struct CommentCode;

impl CommentCode {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for CommentCode {
    fn id(&self) -> &'static str {
        "comment-code"
    }

    fn name(&self) -> &'static str {
        "Comment/Code Mismatch"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &["rust"]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = Vec::new();
        for file in ctx.files {
            if file.language != "rust" {
                continue;
            }
            collect_findings_in_file(file, &mut findings);
        }

        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
        });

        Ok(findings)
    }
}

fn collect_findings_in_file(file: &ParsedFile, out: &mut Vec<Finding>) {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_rust::language()).is_err() {
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

    let mut cursor = root.walk();
    let children: Vec<tree_sitter::Node> = root.children(&mut cursor).collect();
    for (idx, child) in children.iter().enumerate() {
        if child.kind() != "function_item" {
            continue;
        }
        let doc = collect_preceding_doc(&children, idx, &file.source);
        if doc.is_empty() {
            continue;
        }
        let doc_lc = doc.to_lowercase();

        if let Some(trigger) = pattern_a_match(*child, &file.source, &doc_lc) {
            out.push(make_finding(file, *child, "A", trigger));
        }
        if let Some(trigger) = pattern_b_match(*child, &file.source, &doc_lc) {
            out.push(make_finding(file, *child, "B", trigger));
        }
        if let Some(trigger) =
            pattern_c_match(&children, idx, &file.source, &doc_lc)
        {
            out.push(make_finding(file, *child, "C", trigger));
        }
    }
}

/// Walk preceding siblings of `children[idx]` upward as long as they are
/// `///` line comments. Returns the rendered doc text (lines joined with
/// `\n`, prefix stripped). `//!` and plain `//` comments are ignored.
fn collect_preceding_doc(
    children: &[tree_sitter::Node],
    idx: usize,
    source: &str,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let node = children[i];
        if node.kind() != "line_comment" {
            // Allow attribute items to sit between the doc block and the fn
            // (e.g. `/// docs\n#[inline]\nfn ...`). Skip them and keep walking.
            if node.kind() == "attribute_item" {
                continue;
            }
            break;
        }
        let text = &source[node.byte_range()];
        if let Some(rest) = text.strip_prefix("///") {
            let rendered = rest.strip_prefix(' ').unwrap_or(rest);
            lines.push(rendered.to_string());
        } else {
            break;
        }
    }
    lines.reverse();
    lines.join("\n")
}

fn pattern_a_match(
    node: tree_sitter::Node,
    source: &str,
    doc_lc: &str,
) -> Option<&'static str> {
    let trigger = PATTERN_A_TRIGGERS
        .iter()
        .find(|p| doc_lc.contains(*p))
        .copied()?;

    let return_type_text = match node.child_by_field_name("return_type") {
        Some(rt) => source[rt.byte_range()].to_string(),
        None => String::new(),
    };
    if return_type_text.contains("Result") || return_type_text.contains("Option") {
        return None;
    }
    Some(trigger)
}

fn pattern_b_match(
    node: tree_sitter::Node,
    source: &str,
    doc_lc: &str,
) -> Option<&'static str> {
    if !doc_lc.contains("panic") {
        return None;
    }
    let body = node.child_by_field_name("body")?;
    let body_text = &source[body.byte_range()];
    for marker in PATTERN_B_BODY_MARKERS {
        if body_text.contains(marker) {
            return None;
        }
    }
    Some("panic")
}

fn pattern_c_match(
    children: &[tree_sitter::Node],
    idx: usize,
    source: &str,
    doc_lc: &str,
) -> Option<&'static str> {
    if !doc_lc.contains("deprecated") {
        return None;
    }
    if preceding_siblings_have_deprecated(children, idx, source) {
        return None;
    }
    Some("deprecated")
}

/// Walk preceding siblings of `children[idx]` and look for `attribute_item`
/// nodes whose source text (after `#[`) begins with the identifier
/// `deprecated`. This catches `#[deprecated]`, `#[deprecated(note = "...")]`,
/// etc., while rejecting unrelated attributes like `#[inline]`. Stops at the
/// first non-comment, non-attribute sibling so we don't pick up attributes
/// that belong to a different item.
fn preceding_siblings_have_deprecated(
    children: &[tree_sitter::Node],
    idx: usize,
    source: &str,
) -> bool {
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let node = children[i];
        match node.kind() {
            "line_comment" => continue,
            "attribute_item" => {
                if attribute_item_is_deprecated(node, source) {
                    return true;
                }
            }
            _ => break,
        }
    }
    false
}

fn attribute_item_is_deprecated(node: tree_sitter::Node, source: &str) -> bool {
    let raw = &source[node.byte_range()];
    // Strip leading `#[` (or `#![` for inner attrs) and whitespace, then
    // check whether the first identifier is exactly `deprecated`.
    let after_open = raw
        .trim_start()
        .strip_prefix("#![")
        .or_else(|| raw.trim_start().strip_prefix("#["));
    let Some(after_open) = after_open else {
        return false;
    };
    let trimmed = after_open.trim_start();
    let first_ident: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    first_ident == "deprecated"
}

fn make_finding(
    file: &ParsedFile,
    node: tree_sitter::Node,
    pattern: &'static str,
    trigger: &'static str,
) -> Finding {
    Finding {
        detector_id: "comment-code".to_string(),
        primary: node_location(file, node),
        related: Vec::new(),
        message: format!(
            "doc comment claims '{}' but implementation does not match",
            trigger
        ),
        raw_severity: Severity::Note,
        anomaly_class: AnomalyClass::Documentation,
        evidence: Evidence {
            citation_keys: vec!["tan-sosp-2007", "tan-pldi-2011"],
            raw: serde_json::json!({
                "pattern": pattern,
                "trigger": trigger,
            }),
        },
    }
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
