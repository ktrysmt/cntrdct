//! clone-drift detector — Type-3 clone groups with Type-2 partition drift signal.
//!
//! Spec: `cntrdct/docs/spec/clone-drift-v0.md`; Python via `multilang-v0.md` Pattern A.
//!
//! Algorithm (shared):
//! 1. Parse each IrFile for the target language with tree-sitter, collect top-level fns.
//! 2. Normalize each fn to a sequence of AST node kinds with identifiers and
//!    literals replaced by placeholder tokens.
//! 3. Build n-gram sets and compute pairwise Jaccard similarity.
//! 4. Cluster fns with similarity >= SIMILARITY_THRESHOLD via union-find.
//! 5. Within each cluster of size >= MIN_GROUP_SIZE, partition by exact normalized
//!    form. Emit Finding for any size-1 partition coexisting with a size>=2
//!    partition. Each language runs in isolation; Rust fns never mix with Python.
//!
//! Language-specific details:
//! - Rust: `function_item` only; normalization handles Rust token kinds.
//! - Python: `function_definition` (including `decorated_definition` wrappers);
//!   top-level only. Normalization handles Python token kinds.

use std::collections::{HashMap, HashSet};

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::IrFile;
use rayon::prelude::*;

pub const SIMILARITY_THRESHOLD: f64 = 0.5;
pub const NGRAM_SIZE: usize = 3;
pub const MIN_GROUP_SIZE: usize = 3;
/// Functions whose normalized AST token sequence is shorter than this
/// threshold are excluded from clustering. Drift signals on trivially
/// short bodies (single return, single pass) are too noisy to be
/// useful in practice; industrial NiCad / SourcererCC pipelines apply
/// equivalent minimum-size gates. Tunable per the same exposure model
/// as `SIMILARITY_THRESHOLD`.
pub const MIN_FN_TOKENS: usize = 22;
/// F5c-ii: a drifted-clone candidate must be a near-duplicate of the
/// dominant exemplar. Generic-family resemblance (Jaccard 0.5..0.7)
/// is not a drift signal; the bug pattern requires the singleton to
/// differ from the cluster's canonical form by only a small number
/// of tokens. 0.7 keeps small (1-3 token) drifts and rejects
/// structural variants that share only the surface n-gram skeleton.
/// Higher than `SIMILARITY_THRESHOLD` because cluster membership is a
/// transitive-chain property while the drift signal is a direct
/// near-clone property; empirically tuned so the Python pilot drift
/// fixture (Jaccard 0.78) clears it while structural variants such
/// as nom@1309 (0.53) and nom@1330 (0.66) do not. See
/// `docs/spec/clone-drift-v0.md` F5c-ii.
pub const NEAR_DUPLICATE_THRESHOLD: f64 = 0.7;
/// F5d-ii: when the dominant partition holds fewer than
/// `LENGTH_IMBALANCE_DOMINANT_FLOOR` functions (i.e. exactly 2, the
/// F5c-i strict-majority floor for a 3-fn cluster), the canonical-form
/// evidence is structurally weak — the 2 dominant members might
/// themselves be a designed sibling pair (`layer_is_none` /
/// `subscriber_is_none` in tracing-subscriber) rather than 2 copies of
/// one canonical form. In that weak-evidence regime we additionally
/// require small token-length symmetry between the drifted singleton
/// and the dominant exemplar; an asymmetry above this fraction
/// indicates structural divergence (extra struct definition, repeated
/// body block) and the gate suppresses. Tuned at 0.15 so the wild β
/// residuals (uuid `encode_*` at 0.242, tracing-subscriber `*_is_none`
/// twins at 0.186) are caught while genuine drift fixtures with
/// dominant size 2 (FN_VARIANT_A vs FN_VARIANT_B at ≈ 0.11) clear the
/// gate. Clusters with dominant size ≥ 3 are exempt; the textbook bug
/// pattern of "1 of N copies missed an update" (corpus_005 at length
/// imbalance 0.258 with dominant size 4) fires unaffected. See
/// `docs/spec/clone-drift-v0.md` F5d-ii.
pub const LENGTH_IMBALANCE_THRESHOLD: f64 = 0.15;
/// F5d-ii applies only when `largest.len() < LENGTH_IMBALANCE_DOMINANT_FLOOR`.
/// At 3 the gate triggers exactly when the dominant partition holds 2
/// functions — the F5c-i strict-majority floor for a 3-fn cluster.
/// Larger dominant partitions carry strong canonical-form evidence and
/// are exempt from the length-symmetry requirement.
pub const LENGTH_IMBALANCE_DOMINANT_FLOOR: usize = 3;
/// F5d-iii: a cluster at exactly `MIN_GROUP_SIZE` whose dominant
/// exemplar is also near the `MIN_FN_TOKENS` floor is at the
/// resolution limit of the detector — signature normalization
/// dominates the n-gram set, so any single-token shift in the
/// singleton looks like a drift even when the three siblings are
/// independently designed delegate wrappers. The +2 buffer admits
/// genuine drift fixtures (t5's dominant exemplar normalises to ≈ 35
/// tokens) while suppressing the syn parse-API family (dominant
/// `parse_str`/`parse2` body normalises to 22 tokens). See
/// `docs/spec/clone-drift-v0.md` F5d-iii.
pub const SMALL_CLUSTER_TOKEN_BUFFER: usize = 2;

static CITATIONS: &[Citation] = &[
    Citation {
        key: "cordy-roy-icpc-2008",
        authors: "J.R. Cordy, C.K. Roy",
        title: "The NiCad Clone Detector",
        venue: "ICPC 2008",
        year: 2008,
        doi: None,
        url: Some("https://research.cs.queensu.ca/home/cordy/Papers/NiCadICPC.pdf"),
        // NiCad's experimental subjects were Java and C/C++. The Rust
        // grandfather clause covers it under citations-policy.md (b).
        // Python coverage: confirmed via assi-tosem-2025 (NiCad applied
        // to nine Python DL frameworks). See
        // docs/surveys/clone-drift-python-2026-05.md.
        languages: &[Language::Rust, Language::Python],
    },
    Citation {
        key: "bettenburg-msr-2009",
        authors: "N. Bettenburg, W. Shang, W. Ibrahim, B. Adams, Y. Zou, A.E. Hassan",
        title: "An Empirical Study on Inconsistent Changes to Code Clones at the Release Level",
        venue: "MSR 2009",
        year: 2009,
        doi: None,
        url: None,
        languages: &[Language::Rust, Language::Python],
    },
    Citation {
        key: "krinke-icsm-2007",
        authors: "J. Krinke",
        title: "A Study of Consistent and Inconsistent Changes to Code Clones",
        venue: "ICSM 2007",
        year: 2007,
        doi: None,
        url: None,
        languages: &[Language::Rust, Language::Python],
    },
    Citation {
        key: "assi-tosem-2025",
        authors: "M. Assi, S. Hassan, Y. Zou",
        title: "Unraveling Code Clone Dynamics in Deep Learning Frameworks",
        venue: "ACM TOSEM",
        year: 2025,
        doi: Some("10.1145/3721125"),
        url: Some("https://dl.acm.org/doi/10.1145/3721125"),
        // Independent peer-reviewed application of NiCad to nine Python
        // DL frameworks (TensorFlow, Paddle, PyTorch, Aesara, Ray, MXNet,
        // Keras, Jax, BentoML); satisfies citations-policy.md clause (b)
        // for cordy-roy-icpc-2008 on Python and the inconsistent-change
        // framing on Python for bettenburg-msr-2009 / krinke-icsm-2007.
        // See docs/surveys/clone-drift-python-2026-05.md.
        languages: &[Language::Python],
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

    fn supported_languages(&self) -> &'static [Language] {
        &[Language::Rust, Language::Python]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = Vec::new();
        findings.extend(run_detect_for_language(
            ctx,
            Language::Rust,
            extract_rust_fns,
            LanguageCitationStatus::Confirmed,
            &[
                "cordy-roy-icpc-2008",
                "bettenburg-msr-2009",
                "krinke-icsm-2007",
            ],
        ));
        findings.extend(run_detect_for_language(
            ctx,
            Language::Python,
            extract_python_fns,
            LanguageCitationStatus::Confirmed,
            &[
                "cordy-roy-icpc-2008",
                "bettenburg-msr-2009",
                "krinke-icsm-2007",
                "assi-tosem-2025",
            ],
        ));
        // F2b: intra-fn if-then-else branch clone detection. The
        // function-level pipeline above operates on top-level `fn`
        // items only (granularity locked by F2). F2b runs in parallel
        // and surfaces if-expressions whose `consequence` and
        // `alternative` blocks normalise to identical token
        // sequences. This is the same Type-1 / Type-2 clone signal
        // applied at sub-function granularity — NiCad (Cordy-Roy
        // ICPC 2008) defines clone detection at "function or
        // fragment" granularity, so the citation set is unchanged.
        findings.extend(run_intra_fn_if_clones_rust(ctx));

        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then(a.primary.start_line.cmp(&b.primary.start_line))
        });

        Ok(findings)
    }
}

fn run_detect_for_language(
    ctx: &DetectContext,
    lang: Language,
    extract_fns_fn: fn(&IrFile) -> Option<Vec<FnInfo>>,
    citation_status: LanguageCitationStatus,
    citation_keys: &'static [&'static str],
) -> Vec<Finding> {
    // F5b: extract FnInfos paired with their per-file scope key so
    // clustering can run independently per scope. Per-file parsing
    // dominates runtime; the parallel collect preserves source order
    // because rayon's collect from an indexed parallel iterator is
    // deterministic.
    let per_file: Vec<(String, Vec<FnInfo>)> = ctx
        .files
        .par_iter()
        .filter(|f| f.language == lang)
        .filter_map(|file| {
            extract_fns_fn(file).map(|fns| {
                let filtered: Vec<FnInfo> = fns
                    .into_iter()
                    .filter(|info| info.normalized.len() >= MIN_FN_TOKENS)
                    .collect();
                (scope_id(file), filtered)
            })
        })
        .collect();

    // Bucket by scope. BTreeMap for deterministic iteration order so
    // findings come out in stable order across runs.
    let mut buckets: std::collections::BTreeMap<String, Vec<FnInfo>> =
        std::collections::BTreeMap::new();
    for (scope, fns) in per_file {
        buckets.entry(scope).or_default().extend(fns);
    }

    let mut findings: Vec<Finding> = Vec::new();
    for fns in buckets.values() {
        if fns.len() < MIN_GROUP_SIZE {
            continue;
        }
        findings.extend(emit_findings_for_scope(fns, citation_status, citation_keys));
    }

    findings
}

fn emit_findings_for_scope(
    fns: &[FnInfo],
    citation_status: LanguageCitationStatus,
    citation_keys: &'static [&'static str],
) -> Vec<Finding> {
    let groups = cluster(fns);
    let mut findings: Vec<Finding> = Vec::new();

    for group in &groups {
        if group.len() < MIN_GROUP_SIZE {
            continue;
        }
        let parts = partition(group, fns);
        let largest = parts
            .iter()
            .max_by_key(|p| p.len())
            .expect("non-empty parts");
        // F5c-i: a drifted-clone signal requires the dominant exact-form
        // partition to cover a strict majority of the cluster. Without
        // this guard, parser-combinator-style libraries (e.g. nom) where
        // a loosely-Jaccard-similar group of 100+ functions sub-partitions
        // into many small variants emit one finding per singleton; none
        // are bugs, all are intentional family members. A strict majority
        // (`2 * largest > group`) is the textbook clone-with-drift shape
        // (Bettenburg et al., MSR 2009; Krinke, ICSM 2007).
        if largest.len() * 2 <= group.len() {
            continue;
        }
        if largest.len() < 2 {
            continue;
        }

        // F5d-i: multi-singleton family. A cluster carrying two or more
        // size-1 partitions is the structural signature of a designed
        // family of N parallel variants (e.g. charset_normalizer's
        // `is_<script>` siblings each searching for a different
        // substring), not the textbook "one of N copies missed an
        // update" drift shape. Suppress the entire group's singleton
        // emission once the multi-singleton signal fires. See
        // `docs/spec/clone-drift-v0.md` F5d-i.
        let partition_sizes: Vec<usize> = parts.iter().map(|x| x.len()).collect();
        let singleton_count = partition_sizes.iter().filter(|&&n| n == 1).count();
        if singleton_count >= 2 {
            continue;
        }

        let dominant_exemplar_idx = largest[0];
        let dominant_exemplar = &fns[dominant_exemplar_idx];
        let dominant_len = dominant_exemplar.normalized.len();

        // F5d-iii: small-cluster floor. A cluster at exactly
        // MIN_GROUP_SIZE whose dominant exemplar normalises to within
        // SMALL_CLUSTER_TOKEN_BUFFER of MIN_FN_TOKENS is at the
        // detector's resolution limit; signature normalization (params,
        // type bounds, where clauses) dominates the n-gram set, so any
        // single-token body shift in the singleton looks like a drift
        // even when the three siblings are independently designed
        // delegate wrappers. See `docs/spec/clone-drift-v0.md` F5d-iii.
        if group.len() == MIN_GROUP_SIZE
            && dominant_len <= MIN_FN_TOKENS + SMALL_CLUSTER_TOKEN_BUFFER
        {
            continue;
        }

        for p in &parts {
            if p.len() != 1 {
                continue;
            }
            let drifted_idx = p[0];
            let drifted = &fns[drifted_idx];
            // F5c-ii: drift candidate must be a near-duplicate of the
            // dominant exemplar. Cluster membership only requires
            // pairwise Jaccard >= SIMILARITY_THRESHOLD (0.5) with at
            // least one other group member, which is too loose: a
            // structurally different function can be transitively
            // pulled into the cluster. The bug pattern (one of N
            // copies missed an update) requires the singleton to
            // differ from the canonical form by a small number of
            // tokens, i.e. Jaccard(singleton, dominant) very high.
            let dominant_jaccard = jaccard(&drifted.ngrams, &dominant_exemplar.ngrams);
            if dominant_jaccard < NEAR_DUPLICATE_THRESHOLD {
                continue;
            }
            // F5d-ii: length-imbalance gate, conditioned on weak
            // dominant-form evidence. With a dominant partition of
            // exactly 2 (the F5c-i strict-majority floor for a 3-fn
            // cluster) the "two members happen to share a normalised
            // form" interpretation is just as plausible as "the two
            // members are 2 copies of one canonical form"; we
            // therefore additionally require that the drifted
            // singleton matches the dominant exemplar in token-length.
            // A high-Jaccard / high-length-imbalance pair under that
            // weak-evidence cluster is the family-of-variants shape
            // (uuid `encode_braced` adds a nested struct, etc.).
            // Clusters with dominant size ≥ 3 are exempt: the
            // textbook bug pattern of "1 of N copies missed an
            // update" (corpus_005 with N = 4 at imbalance 0.258)
            // fires unaffected.
            let drift_len = drifted.normalized.len();
            let max_len = drift_len.max(dominant_len) as f64;
            let length_imbalance = if max_len > 0.0 {
                (drift_len as f64 - dominant_len as f64).abs() / max_len
            } else {
                0.0
            };
            if largest.len() < LENGTH_IMBALANCE_DOMINANT_FLOOR
                && length_imbalance > LENGTH_IMBALANCE_THRESHOLD
            {
                continue;
            }

            let related: Vec<Location> = largest
                .iter()
                .filter(|&&i| i != drifted_idx)
                .map(|&i| fns[i].location.clone())
                .collect();
            let related_count = related.len();
            findings.push(Finding {
                detector_id: "clone-drift".to_string(),
                primary: drifted.location.clone(),
                related,
                message: format!("function diverged from {} similar siblings", related_count),
                raw_severity: Severity::Warning,
                anomaly_class: AnomalyClass::Logic,
                evidence: Evidence {
                    citation_keys: citation_keys.to_vec(),
                    raw: serde_json::json!({
                        "similarity_threshold": SIMILARITY_THRESHOLD,
                        "near_duplicate_threshold": NEAR_DUPLICATE_THRESHOLD,
                        "length_imbalance_threshold": LENGTH_IMBALANCE_THRESHOLD,
                        "group_size": group.len(),
                        "partition_sizes": partition_sizes,
                        "singleton_count": singleton_count,
                        "dominant_jaccard": dominant_jaccard,
                        "drifted_len": drift_len,
                        "dominant_len": dominant_len,
                        "length_imbalance": length_imbalance,
                    }),
                    language_citation_status: citation_status,
                },
            });
        }
    }

    findings
}

/// F5b scope key for `file`, computed without filesystem I/O.
///
/// Rule order, first match wins:
///
/// 1. Provenance header (`// Source: ...` or `# Source: ...`) in the
///    file's first ~512 bytes. The full URL becomes the scope key
///    so two files extracted from the same tarball / .crate share
///    a key automatically.
/// 2. Cargo project layout. Path components like `<crate>/src/...`,
///    `<crate>/tests/...`, `<crate>/examples/...` split into
///    per-crate scopes. The substring up to the matched separator
///    is the scope key. A leading `src/` / `tests/` / `examples/`
///    (no parent component) yields the empty scope, which is the
///    correct shared key for a single-crate scan.
/// 3. Filename `__` separator (the wild β corpus's secondary
///    convention when provenance is missing).
/// 4. Parent directory of the file as a string. Bare filenames with
///    no parent yield the empty scope (preserves backward-compat
///    with existing test fixtures using bare names).
fn scope_id(file: &IrFile) -> String {
    if let Some(s) = scope_from_provenance(&file.source) {
        return s;
    }
    if let Some(s) = scope_from_cargo_layout(&file.path) {
        return s;
    }
    if let Some(s) = scope_from_underscore_basename(&file.path) {
        return s;
    }
    file.path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn scope_from_provenance(source: &str) -> Option<String> {
    let head: String = source.chars().take(512).collect();
    for line in head.lines() {
        let t = line.trim_start();
        if let Some(rest) = t
            .strip_prefix("// Source: ")
            .or_else(|| t.strip_prefix("# Source: "))
        {
            let rest = rest.trim();
            // F5b convention: the provenance value carries a URL
            // (`https://static.crates.io/...`, `https://files.pythonhosted.org/...`,
            // or a project-specific URL for synthetic fixtures). A
            // descriptive line like `// Source: shape adapted from` is
            // NOT a scope key — accepting it would collapse every
            // fixture sharing that note into one super-scope, which
            // empirically suppresses real drift findings under F5c-i /
            // F5d-i. Require a scheme-shaped prefix
            // (`<letters>://<value>`); any non-URL value falls through
            // to the next scope rule.
            if is_url_shaped(rest) {
                return Some(format!("provenance::{rest}"));
            }
        }
    }
    None
}

/// True iff `s` looks like `<scheme>://<value>` with a non-empty,
/// alphanumeric scheme prefix and a non-empty value. Cheap structural
/// check — no full URL parser, no allocation.
fn is_url_shaped(s: &str) -> bool {
    let Some(scheme_end) = s.find("://") else {
        return false;
    };
    if scheme_end == 0 {
        return false;
    }
    let scheme = &s[..scheme_end];
    if !scheme
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    {
        return false;
    }
    let rest = &s[scheme_end + 3..];
    !rest.is_empty()
}

fn scope_from_cargo_layout(path: &std::path::Path) -> Option<String> {
    let s = path.to_string_lossy();
    for sep in ["/src/", "/tests/", "/examples/"] {
        if let Some(idx) = s.find(sep) {
            return Some(s[..idx].to_string());
        }
    }
    for prefix in ["src/", "tests/", "examples/"] {
        if s.starts_with(prefix) {
            return Some(String::new());
        }
    }
    None
}

fn scope_from_underscore_basename(path: &std::path::Path) -> Option<String> {
    let basename = path.file_name()?.to_string_lossy();
    let (prefix, rest) = basename.split_once("__")?;
    if prefix.is_empty() || rest.is_empty() {
        return None;
    }
    Some(format!("underscore::{prefix}"))
}

fn extract_rust_fns(file: &IrFile) -> Option<Vec<FnInfo>> {
    if file.parse_recovered {
        return None;
    }
    let root = file.raw_tree.root_node();

    let mut fns = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_item" {
            let normalized = normalize_rust(child);
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

fn normalize_rust(node: tree_sitter::Node) -> Vec<String> {
    let mut out = Vec::new();
    walk_normalize_rust(node, &mut out);
    out
}

fn walk_normalize_rust(node: tree_sitter::Node, out: &mut Vec<String>) {
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
        walk_normalize_rust(child, out);
    }
}

fn normalize_python(node: tree_sitter::Node) -> Vec<String> {
    let mut out = Vec::new();
    walk_normalize_python(node, &mut out);
    out
}

fn walk_normalize_python(node: tree_sitter::Node, out: &mut Vec<String>) {
    if !node.is_named() {
        return;
    }
    let kind = node.kind();
    if kind == "comment" {
        return;
    }

    let leaf_token = match kind {
        "identifier" => Some("IDENT"),
        "integer" => Some("LIT_INT"),
        "float" => Some("LIT_FLOAT"),
        "string" => Some("LIT_STR"),
        "true" | "false" => Some("LIT_BOOL"),
        _ => None,
    };

    if let Some(rep) = leaf_token {
        out.push(rep.to_string());
        return;
    }

    out.push(kind.to_string());
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_normalize_python(child, out);
    }
}

fn extract_python_fns(file: &IrFile) -> Option<Vec<FnInfo>> {
    if file.parse_recovered {
        return None;
    }
    let root = file.raw_tree.root_node();

    let mut fns = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                let normalized = normalize_python(child);
                let ngrams = build_ngrams(&normalized, NGRAM_SIZE);
                let location = node_location(file, child);
                fns.push(FnInfo {
                    location,
                    normalized,
                    ngrams,
                });
            }
            "decorated_definition" => {
                let mut dcursor = child.walk();
                let kids: Vec<tree_sitter::Node> = child.children(&mut dcursor).collect();
                if let Some(fn_def) = kids.iter().find(|c| c.kind() == "function_definition") {
                    let normalized = normalize_python(*fn_def);
                    let ngrams = build_ngrams(&normalized, NGRAM_SIZE);
                    let location = node_location(file, *fn_def);
                    fns.push(FnInfo {
                        location,
                        normalized,
                        ngrams,
                    });
                }
            }
            _ => {}
        }
    }
    Some(fns)
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

fn node_location(file: &IrFile, node: tree_sitter::Node) -> Location {
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

// ---------- F2b intra-fn if-branch clone detection (added 2026-05-21) ----------

/// Minimum normalised-token count for one branch of an if/else before
/// F2b will emit. Bodies smaller than this are too noisy to act on
/// — `if c { 0 } else { 0 }` is a stylistic placeholder, not a copy-
/// paste duplicate. The 22-token floor matches the function-level
/// `MIN_FN_TOKENS` so trivially-small block expressions are filtered
/// out under both pipelines.
pub const INTRA_FN_IF_MIN_TOKENS: usize = 22;

fn run_intra_fn_if_clones_rust(ctx: &DetectContext) -> Vec<Finding> {
    ctx.files
        .par_iter()
        .filter(|f| f.language == Language::Rust)
        .flat_map_iter(|file| {
            let mut local = Vec::new();
            scan_rust_for_if_branch_clones(file, &mut local);
            local
        })
        .collect()
}

fn scan_rust_for_if_branch_clones(file: &IrFile, findings: &mut Vec<Finding>) {
    if file.parse_recovered {
        return;
    }
    let root = file.raw_tree.root_node();
    walk_rust_for_if_branches(root, file, findings);
}

fn walk_rust_for_if_branches(node: tree_sitter::Node, file: &IrFile, findings: &mut Vec<Finding>) {
    if node.kind() == "if_expression" {
        if let Some(f) = analyze_if_branches_rust(node, file) {
            findings.push(f);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_rust_for_if_branches(child, file, findings);
    }
}

fn analyze_if_branches_rust(if_expr: tree_sitter::Node, file: &IrFile) -> Option<Finding> {
    let consequence = if_expr.child_by_field_name("consequence")?;
    let alternative = if_expr.child_by_field_name("alternative")?;
    // F2b only fires on a flat if / else — chained `else if` (where
    // alternative is another `if_expression`) is out of scope for v0.
    // The walker recurses into the inner if so a clone-pair within
    // that nested branch still surfaces.
    let alt_block = find_else_block_rust(alternative)?;
    if consequence.kind() != "block" || alt_block.kind() != "block" {
        return None;
    }

    // Size gate uses the function-level normalised-token count so the
    // floor lines up with `MIN_FN_TOKENS` / `INTRA_FN_IF_MIN_TOKENS`.
    let conseq_normalized = normalize_rust(consequence);
    if conseq_normalized.len() < INTRA_FN_IF_MIN_TOKENS {
        return None;
    }

    // Equality gate uses source-text comparison after whitespace and
    // comment normalisation rather than normalised-token equality.
    // Type-2 clones (same AST shape, different identifiers) are
    // intentional in real code: `if c { foo(self.i) } else {
    // foo(self.j) }` is the canonical fan-out-by-argument pattern,
    // not a copy-paste bug. clippy's `if_same_then_else` agrees that
    // identifiers must match, and the wild-corpus β scan with token-
    // normalised comparison produced 20 such intentional-fan-out FPs
    // (itertools, regex_syntax, object, ...). Strict source equality
    // collapses those FPs to zero while still flagging the clippy
    // ui-test trigger sites (audit-corpus
    // clippy_ui_if_same_then_else.rs#L25 = line 29).
    let conseq_src = normalize_block_source(consequence, &file.source)?;
    let alt_src = normalize_block_source(alt_block, &file.source)?;
    if conseq_src != alt_src {
        return None;
    }

    let primary = node_location(file, if_expr);
    let related = vec![
        node_location(file, consequence),
        node_location(file, alt_block),
    ];

    Some(Finding {
        detector_id: "clone-drift".to_string(),
        primary,
        related,
        message: format!(
            "if-then-else branches contain identical source ({} tokens) — likely a copy-paste duplicate",
            conseq_normalized.len()
        ),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec![
                "cordy-roy-icpc-2008",
                "bettenburg-msr-2009",
                "krinke-icsm-2007",
            ],
            raw: serde_json::json!({
                "kind": "intra-fn-if-same-then-else",
                "branch_token_count": conseq_normalized.len(),
                "intra_fn_if_min_tokens": INTRA_FN_IF_MIN_TOKENS,
            }),
            language_citation_status: LanguageCitationStatus::Confirmed,
        },
    })
}

/// Source-text normalisation used by F2b: strip line and block
/// comments, collapse internal whitespace runs to a single space,
/// trim leading and trailing whitespace. Two blocks normalising to
/// the same string are byte-for-byte identical Rust source modulo
/// formatting and commentary.
fn normalize_block_source(block: tree_sitter::Node, source: &str) -> Option<String> {
    let text = block.utf8_text(source.as_bytes()).ok()?;
    let mut out = String::with_capacity(text.len());
    let mut prev_space = true;
    let mut iter = text.chars().peekable();
    while let Some(c) = iter.next() {
        if c == '/' && iter.peek() == Some(&'/') {
            // Line comment — consume to end of line.
            while let Some(&n) = iter.peek() {
                if n == '\n' {
                    break;
                }
                iter.next();
            }
            continue;
        }
        if c == '/' && iter.peek() == Some(&'*') {
            // Block comment — consume until matching `*/` (single
            // nesting level; nested block comments are vanishingly
            // rare and out of scope for v0).
            iter.next();
            while let Some(n) = iter.next() {
                if n == '*' && iter.peek() == Some(&'/') {
                    iter.next();
                    break;
                }
            }
            continue;
        }
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    let trimmed = out.trim().to_string();
    Some(trimmed)
}

fn find_else_block_rust(alternative: tree_sitter::Node) -> Option<tree_sitter::Node> {
    // `alternative` is an `else_clause` wrapping either a `block` (the
    // `else { ... }` shape) or an `if_expression` (`else if ...`).
    let mut cursor = alternative.walk();
    for child in alternative.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if child.kind() == "block" {
            return Some(child);
        }
        // else-if recurses through the outer walker; do NOT treat the
        // nested if as the alt body for the surrounding pair.
        if child.kind() == "if_expression" {
            return None;
        }
    }
    None
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
