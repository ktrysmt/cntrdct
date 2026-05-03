//! unreachable-after-terminator detector — flag statements that follow a
//! divergent statement inside the same block.
//!
//! Spec: `cntrdct/docs/spec/unreachable-after-terminator-v0.md`.

use cntrdct_core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Location,
    ParsedFile, Severity,
};

pub const TERMINATOR_MACROS: &[&str] = &[
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "abort",
    "exit",
];

pub const SUPPRESSION_TOKEN: &str = "unreachable_code";

static CITATIONS: &[Citation] = &[
    Citation {
        key: "hovemeyer-pugh-oopsla-2004",
        authors: "D. Hovemeyer, W. Pugh",
        title: "Finding Bugs is Easy",
        venue: "OOPSLA 2004",
        year: 2004,
        doi: Some("10.1145/1052883.1052895"),
        url: None,
    },
    Citation {
        key: "engler-sosp-2001",
        authors: "D. Engler, D.Y. Chen, S. Hallem, A. Chou, B. Chelf",
        title:
            "Bugs as Deviant Behavior: A General Approach to Inferring Errors in Systems Code",
        venue: "SOSP 2001",
        year: 2001,
        doi: Some("10.1145/502034.502041"),
        url: None,
    },
];

#[derive(Debug, Default)]
pub struct UnreachableAfterTerminator;

impl UnreachableAfterTerminator {
    pub fn new() -> Self {
        Self
    }
}

impl Detector for UnreachableAfterTerminator {
    fn id(&self) -> &'static str {
        "unreachable-after-terminator"
    }

    fn name(&self) -> &'static str {
        "Unreachable After Terminator"
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
            scan_file(file, &mut findings);
        }
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
    let lang = tree_sitter_rust::language();
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
    if node.kind() == "block" {
        analyze_block(node, file, findings);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, file, findings);
    }
}

fn analyze_block(block: tree_sitter::Node, file: &ParsedFile, findings: &mut Vec<Finding>) {
    if is_suppressed(block, &file.source) {
        return;
    }

    let stmts: Vec<tree_sitter::Node> = {
        let mut cursor = block.walk();
        block
            .children(&mut cursor)
            .filter(|c| is_block_statement(*c))
            .collect()
    };

    for (i, stmt) in stmts.iter().enumerate() {
        if let Some(kind) = terminator_kind(*stmt, &file.source) {
            let following = stmts.len() - i - 1;
            if following == 0 {
                return;
            }
            let follower = stmts[i + 1];
            findings.push(build_finding(file, follower, *stmt, kind, following));
            return;
        }
    }
}

fn is_block_statement(node: tree_sitter::Node) -> bool {
    if !node.is_named() {
        return false;
    }
    !matches!(
        node.kind(),
        "inner_attribute_item" | "attribute_item" | "line_comment" | "block_comment"
    )
}

fn terminator_kind(stmt: tree_sitter::Node, source: &str) -> Option<&'static str> {
    if stmt.kind() != "expression_statement" {
        return None;
    }
    let mut cursor = stmt.walk();
    let inner = stmt.children(&mut cursor).find(|c| c.is_named())?;
    match inner.kind() {
        "return_expression" => Some("return"),
        "break_expression" => Some("break"),
        "continue_expression" => Some("continue"),
        "macro_invocation" => macro_terminator_name(inner, source),
        _ => None,
    }
}

fn macro_terminator_name(call: tree_sitter::Node, source: &str) -> Option<&'static str> {
    let macro_node = call.child_by_field_name("macro")?;
    let text = macro_node.utf8_text(source.as_bytes()).ok()?;
    let last = text.rsplit("::").next().unwrap_or(text);
    TERMINATOR_MACROS.iter().copied().find(|&m| m == last)
}

fn is_suppressed(node: tree_sitter::Node, source: &str) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            if matches!(child.kind(), "attribute_item" | "inner_attribute_item")
                && attribute_contains(child, source, SUPPRESSION_TOKEN)
            {
                return true;
            }
        }
        let mut sib = n.prev_named_sibling();
        while let Some(s) = sib {
            if s.kind() == "attribute_item" {
                if attribute_contains(s, source, SUPPRESSION_TOKEN) {
                    return true;
                }
                sib = s.prev_named_sibling();
            } else {
                break;
            }
        }
        current = n.parent();
    }
    false
}

fn attribute_contains(attr: tree_sitter::Node, source: &str, token: &str) -> bool {
    attr.utf8_text(source.as_bytes())
        .map(|t| t.contains(token))
        .unwrap_or(false)
}

fn build_finding(
    file: &ParsedFile,
    follower: tree_sitter::Node,
    terminator: tree_sitter::Node,
    kind: &'static str,
    following_count: usize,
) -> Finding {
    let primary = node_location(file, follower);
    let related = vec![node_location(file, terminator)];
    let terminator_line = related[0].start_line;
    Finding {
        detector_id: "unreachable-after-terminator".to_string(),
        primary,
        related,
        message: format!(
            "statement is unreachable; preceded by {} on line {}",
            kind, terminator_line
        ),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["hovemeyer-pugh-oopsla-2004", "engler-sosp-2001"],
            raw: serde_json::json!({
                "terminator_kind": kind,
                "terminator_line": terminator_line,
                "following_count": following_count,
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
