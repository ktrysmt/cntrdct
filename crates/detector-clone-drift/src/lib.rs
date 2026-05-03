//! clone-drift detector — Type-3 clone groups with Type-2 partition drift signal.
//!
//! Spec: `cntrdct/docs/spec/clone-drift-v0.md`.
//!
//! Algorithm:
//! 1. Parse each ParsedFile (rust only) with tree-sitter, collect top-level fns.
//! 2. Normalize each fn to a sequence of AST node kinds with identifiers and
//!    literals replaced by placeholder tokens.
//! 3. Build n-gram sets and compute pairwise Jaccard similarity.
//! 4. Cluster fns with similarity >= SIMILARITY_THRESHOLD via union-find.
//! 5. Within each cluster of size >= MIN_GROUP_SIZE, partition by exact normalized
//!    form. Emit Finding for any size-1 partition coexisting with a size>=2
//!    partition.

use std::collections::{HashMap, HashSet};

use cntrdct_core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Location,
    ParsedFile, Severity,
};
use rayon::prelude::*;

pub const SIMILARITY_THRESHOLD: f64 = 0.5;
pub const NGRAM_SIZE: usize = 3;
pub const MIN_GROUP_SIZE: usize = 3;

static CITATIONS: &[Citation] = &[
    Citation {
        key: "cordy-roy-icpc-2008",
        authors: "J.R. Cordy, C.K. Roy",
        title: "The NiCad Clone Detector",
        venue: "ICPC 2008",
        year: 2008,
        doi: None,
        url: Some("https://research.cs.queensu.ca/home/cordy/Papers/NiCadICPC.pdf"),
    },
    Citation {
        key: "bettenburg-msr-2009",
        authors: "N. Bettenburg, W. Shang, W. Ibrahim, B. Adams, Y. Zou, A.E. Hassan",
        title: "An Empirical Study on Inconsistent Changes to Code Clones at the Release Level",
        venue: "MSR 2009",
        year: 2009,
        doi: None,
        url: None,
    },
    Citation {
        key: "krinke-icsm-2007",
        authors: "J. Krinke",
        title: "A Study of Consistent and Inconsistent Changes to Code Clones",
        venue: "ICSM 2007",
        year: 2007,
        doi: None,
        url: None,
    },
];

#[derive(Debug, Default)]
pub struct CloneDrift;

impl CloneDrift {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct FnInfo {
    location: Location,
    normalized: Vec<String>,
    ngrams: HashSet<Vec<String>>,
}

impl Detector for CloneDrift {
    fn id(&self) -> &'static str {
        "clone-drift"
    }

    fn name(&self) -> &'static str {
        "Clone Drift"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &["rust"]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        // Per-file parsing dominates this detector's runtime; clustering and
        // partitioning that follow are intrinsically cross-file and stay
        // serial. The parallel collect preserves source order because rayon's
        // collect from an indexed parallel iterator is deterministic.
        let all_fns: Vec<FnInfo> = ctx
            .files
            .par_iter()
            .filter(|f| f.language == "rust")
            .filter_map(extract_fns)
            .flatten()
            .collect();

        if all_fns.len() < MIN_GROUP_SIZE {
            return Ok(vec![]);
        }

        let groups = cluster(&all_fns);
        let mut findings: Vec<Finding> = Vec::new();

        for group in &groups {
            if group.len() < MIN_GROUP_SIZE {
                continue;
            }
            let parts = partition(group, &all_fns);
            let has_majority = parts.iter().any(|p| p.len() >= 2);
            if !has_majority {
                continue;
            }
            let largest = parts
                .iter()
                .max_by_key(|p| p.len())
                .expect("non-empty parts");

            for p in &parts {
                if p.len() != 1 {
                    continue;
                }
                let drifted_idx = p[0];
                let drifted = &all_fns[drifted_idx];
                let related: Vec<Location> = largest
                    .iter()
                    .filter(|&&i| i != drifted_idx)
                    .map(|&i| all_fns[i].location.clone())
                    .collect();
                let related_count = related.len();
                let partition_sizes: Vec<usize> = parts.iter().map(|x| x.len()).collect();
                findings.push(Finding {
                    detector_id: "clone-drift".to_string(),
                    primary: drifted.location.clone(),
                    related,
                    message: format!("function diverged from {} similar siblings", related_count),
                    raw_severity: Severity::Warning,
                    anomaly_class: AnomalyClass::Logic,
                    evidence: Evidence {
                        citation_keys: vec![
                            "cordy-roy-icpc-2008",
                            "bettenburg-msr-2009",
                            "krinke-icsm-2007",
                        ],
                        raw: serde_json::json!({
                            "similarity_threshold": SIMILARITY_THRESHOLD,
                            "group_size": group.len(),
                            "partition_sizes": partition_sizes,
                        }),
                    },
                });
            }
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

fn extract_fns(file: &ParsedFile) -> Option<Vec<FnInfo>> {
    let mut parser = tree_sitter::Parser::new();
    let lang = tree_sitter_rust::language();
    parser.set_language(&lang).ok()?;
    let tree = parser.parse(&file.source, None)?;
    let root = tree.root_node();

    if root.has_error() {
        return None;
    }

    let mut fns = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_item" {
            let normalized = normalize(child);
            let ngrams = build_ngrams(&normalized, NGRAM_SIZE);
            let location = node_location(file, child);
            fns.push(FnInfo {
                location,
                normalized,
                ngrams,
            });
        }
    }
    Some(fns)
}

fn normalize(node: tree_sitter::Node) -> Vec<String> {
    let mut out = Vec::new();
    walk_normalize(node, &mut out);
    out
}

fn walk_normalize(node: tree_sitter::Node, out: &mut Vec<String>) {
    if !node.is_named() {
        return;
    }
    let kind = node.kind();
    if kind == "line_comment" || kind == "block_comment" {
        return;
    }

    let leaf_token = match kind {
        "identifier"
        | "type_identifier"
        | "field_identifier"
        | "shorthand_field_identifier"
        | "scoped_identifier"
        | "scoped_type_identifier" => Some("IDENT"),
        "integer_literal" => Some("LIT_INT"),
        "float_literal" => Some("LIT_FLOAT"),
        "string_literal" | "raw_string_literal" => Some("LIT_STR"),
        "char_literal" => Some("LIT_CHAR"),
        "boolean_literal" => Some("LIT_BOOL"),
        _ => None,
    };

    if let Some(rep) = leaf_token {
        out.push(rep.to_string());
        return;
    }

    out.push(kind.to_string());
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_normalize(child, out);
    }
}

fn build_ngrams(seq: &[String], n: usize) -> HashSet<Vec<String>> {
    if seq.len() < n {
        return HashSet::new();
    }
    seq.windows(n).map(|w| w.to_vec()).collect()
}

fn jaccard(a: &HashSet<Vec<String>>, b: &HashSet<Vec<String>>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    inter as f64 / union as f64
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

/// Path-compressing root lookup. Used during the union-find merge phase so a
/// subsequent find on the same node is O(1). Cluster equivalence is unchanged
/// (path compression is a representation optimisation only).
fn find_root_compress(parent: &mut [usize], x: usize) -> usize {
    let mut root = x;
    while parent[root] != root {
        root = parent[root];
    }
    let mut cur = x;
    while parent[cur] != cur {
        let next = parent[cur];
        parent[cur] = root;
        cur = next;
    }
    root
}

fn cluster(fns: &[FnInfo]) -> Vec<Vec<usize>> {
    let n = fns.len();

    // Pairwise Jaccard scan dominates this detector's runtime. Compute the
    // O(n²) similarity test in parallel, materialise the qualifying pairs,
    // then merge into a single union-find serially. The merge is cheap
    // relative to the comparison phase, and keeping it serial avoids a lock
    // around `parent`.
    let pairs: Vec<(usize, usize)> = (0..n)
        .into_par_iter()
        .flat_map_iter(|i| {
            ((i + 1)..n)
                .filter(move |&j| jaccard(&fns[i].ngrams, &fns[j].ngrams) >= SIMILARITY_THRESHOLD)
                .map(move |j| (i, j))
        })
        .collect();

    let mut parent: Vec<usize> = (0..n).collect();
    for (i, j) in pairs {
        let ri = find_root_compress(&mut parent, i);
        let rj = find_root_compress(&mut parent, j);
        if ri != rj {
            parent[ri] = rj;
        }
    }

    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        groups
            .entry(find_root_compress(&mut parent, i))
            .or_default()
            .push(i);
    }

    let mut result: Vec<Vec<usize>> = groups.into_values().collect();
    for g in &mut result {
        g.sort();
    }
    result.sort();
    result
}

fn partition(group: &[usize], fns: &[FnInfo]) -> Vec<Vec<usize>> {
    let mut by_form: HashMap<&Vec<String>, Vec<usize>> = HashMap::new();
    for &idx in group {
        by_form.entry(&fns[idx].normalized).or_default().push(idx);
    }
    let mut parts: Vec<Vec<usize>> = by_form.into_values().collect();
    for p in &mut parts {
        p.sort();
    }
    parts.sort();
    parts
}
