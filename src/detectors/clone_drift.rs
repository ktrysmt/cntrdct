//! clone-drift detector — Type-3 clone groups with Type-2 partition drift signal.
//!
//! Spec: `cntrdct/docs/spec/clone-drift-v0.md`; Python via `multilang-v0.md` Pattern A.
//!
//! Algorithm (shared):
//! 1. Read each [`IrFile`]'s top-level functions (`!is_method`); the
//!    converter has already normalised every function once into
//!    [`crate::ir::IrFn::normalised_tokens`] (function-item-rooted).
//! 2. Build n-gram sets and compute pairwise Jaccard similarity.
//! 3. Cluster fns with similarity >= SIMILARITY_THRESHOLD via union-find.
//! 4. Within each cluster of size >= MIN_GROUP_SIZE, partition by exact normalized
//!    form. Emit Finding for any size-1 partition coexisting with a size>=2
//!    partition. Each language runs in isolation; Rust fns never mix with Python.
//!
//! Both languages share one IR path: [`crate::ir::IrFn::normalised_tokens`]
//! is populated by the per-language converter (Rust `function_item`,
//! Python `function_definition`, top-level only), so the detector no
//! longer reparses or branches on language for the function-level
//! pipeline. The intra-fn if-branch pass walks IR [`IrStmtKind::If`] and
//! runs for Rust and Python: F2b flags fully identical branches
//! (`if_same_then_else`), F2c flags branches sharing a leading statement
//! run (`branches_sharing_code`, shared-prefix variant). See
//! `clone-drift-v0.md` F2b / F2c.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};
use crate::ir::{
    IrBlock, IrExpr, IrExprKind, IrFile, IrIfStmt, IrStmt, IrStmtKind, NormalisedToken,
};
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
    normalized: Vec<NormalisedToken>,
    /// Sorted, deduplicated u64 fingerprints of the function's token
    /// n-grams (see [`build_ngrams`]). Set semantics over fingerprints
    /// instead of materialised token windows: Jaccard is computed by
    /// a linear merge over the two sorted slices.
    ngrams: Vec<u64>,
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
        &[
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::Tsx,
            Language::Go,
        ]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        let mut findings: Vec<Finding> = Vec::new();
        findings.extend(run_detect_for_language(
            ctx,
            Language::Rust,
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
            LanguageCitationStatus::Confirmed,
            &[
                "cordy-roy-icpc-2008",
                "bettenburg-msr-2009",
                "krinke-icsm-2007",
                "assi-tosem-2025",
            ],
        ));
        // R-2.d: TypeScript runs the same function-level NiCad-style
        // pipeline (the IR `normalised_tokens` are language-agnostic).
        // Status is Unconfirmed until the R-2.f survey grounds a
        // TypeScript citation; the cross-cutting concept keys carry over.
        // F2b (intra-fn if-branch clones) stays Rust-only for v0.
        findings.extend(run_detect_for_language(
            ctx,
            Language::TypeScript,
            LanguageCitationStatus::Unconfirmed,
            &[
                "cordy-roy-icpc-2008",
                "bettenburg-msr-2009",
                "krinke-icsm-2007",
            ],
        ));
        // `.tsx` reuses the TypeScript function-level pipeline verbatim
        // (the IR `normalised_tokens` are grammar-agnostic). Unconfirmed
        // for the same reason as `.ts`.
        findings.extend(run_detect_for_language(
            ctx,
            Language::Tsx,
            LanguageCitationStatus::Unconfirmed,
            &[
                "cordy-roy-icpc-2008",
                "bettenburg-msr-2009",
                "krinke-icsm-2007",
            ],
        ));
        // R-3.d: Go runs the same function-level NiCad-style pipeline over
        // the language-agnostic IR `normalised_tokens`. Top-level `func`
        // declarations participate (`!is_method`); methods are excluded as
        // for Rust/TS. Unconfirmed until the R-3.f survey grounds a Go
        // citation. F2b (intra-fn if-branch clones) stays Rust-only.
        findings.extend(run_detect_for_language(
            ctx,
            Language::Go,
            LanguageCitationStatus::Unconfirmed,
            &[
                "cordy-roy-icpc-2008",
                "bettenburg-msr-2009",
                "krinke-icsm-2007",
            ],
        ));
        // F2b / F2c: intra-fn if-then-else branch clone detection. The
        // function-level pipeline above operates on top-level `fn`
        // items only (granularity locked by F2). This pass surfaces
        // if/else whose branches normalise to identical source (F2b,
        // `if_same_then_else`) or share a leading/trailing run of
        // statements (F2c, `branches_sharing_code`). Same Type-1 / Type-2
        // clone signal at sub-function granularity — NiCad (Cordy-Roy
        // ICPC 2008) defines clone detection at "function or fragment"
        // granularity, so the citation set is unchanged. Rust uses the
        // C-like comment grammar; Python uses `#` and reuses the Python
        // function-level citation set (Confirmed via assi-tosem-2025).
        findings.extend(run_intra_fn_if_clones(
            ctx,
            Language::Rust,
            &IfCloneCfg {
                comment: CommentStyle::CLike,
                status: LanguageCitationStatus::Confirmed,
                citation_keys: vec![
                    "cordy-roy-icpc-2008",
                    "bettenburg-msr-2009",
                    "krinke-icsm-2007",
                ],
            },
        ));
        findings.extend(run_intra_fn_if_clones(
            ctx,
            Language::Python,
            &IfCloneCfg {
                comment: CommentStyle::Hash,
                status: LanguageCitationStatus::Confirmed,
                citation_keys: vec![
                    "cordy-roy-icpc-2008",
                    "bettenburg-msr-2009",
                    "krinke-icsm-2007",
                    "assi-tosem-2025",
                ],
            },
        ));

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
    citation_status: LanguageCitationStatus,
    citation_keys: &'static [&'static str],
) -> Vec<Finding> {
    // F5b: extract FnInfos paired with their per-file scope key so
    // clustering can run independently per scope. The parallel collect
    // preserves source order because rayon's collect from an indexed
    // parallel iterator is deterministic.
    let per_file: Vec<(String, Vec<FnInfo>)> = ctx
        .files
        .par_iter()
        .filter(|f| f.language == lang)
        .filter_map(|file| {
            extract_top_level_fns(file).map(|fns| {
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
                origin: Default::default(),
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

/// Collect top-level functions as `FnInfo`, reading the converter's
/// function-item-rooted [`crate::ir::IrFn::normalised_tokens`] directly.
///
/// The v0.5.x raw walk processed only root-level `function_item`
/// (Rust) / `function_definition` + top-level `decorated_definition`
/// (Python). [`IrFile::fns`] additionally captures impl / class methods
/// (`is_method == true`); the function-level pipeline excludes those
/// here (`!is_method`) to reproduce the v0.5.x function set byte-for-byte.
fn extract_top_level_fns(file: &IrFile) -> Option<Vec<FnInfo>> {
    if file.parse_recovered {
        return None;
    }
    let fns = file
        .fns
        .iter()
        .filter(|f| !f.is_method)
        .map(|f| {
            let normalized = f.normalised_tokens.clone();
            let ngrams = build_ngrams(&normalized, NGRAM_SIZE);
            FnInfo {
                location: ir_loc_to_core(&f.location),
                normalized,
                ngrams,
            }
        })
        .collect();
    Some(fns)
}

fn ir_loc_to_core(loc: &crate::ir::Location) -> Location {
    Location {
        file: loc.file.to_path_buf(),
        start_line: loc.start_line,
        start_col: loc.start_col,
        end_line: loc.end_line,
        end_col: loc.end_col,
    }
}

/// Fold each n-gram window into a u64 fingerprint instead of
/// materialising it as a `Vec<NormalisedToken>`; the pairwise Jaccard
/// scan then compares integers instead of heap-allocated token vectors.
/// Fingerprints are content-derived (`SipHash` over the window), so they
/// are stable within a run — they are never persisted, and cross-run
/// determinism of findings is unaffected. Two distinct windows colliding
/// on the same u64 would merge two n-grams; at corpus scale (thousands
/// of n-grams per scope against a 2^64 space) the probability is
/// negligible relative to the detector's own similarity thresholds.
fn build_ngrams(seq: &[NormalisedToken], n: usize) -> Vec<u64> {
    if seq.len() < n {
        return Vec::new();
    }
    let mut grams: Vec<u64> = seq
        .windows(n)
        .map(|w| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            w.hash(&mut h);
            h.finish()
        })
        .collect();
    grams.sort_unstable();
    grams.dedup();
    grams
}

/// Jaccard similarity over two sorted, deduplicated fingerprint slices.
/// Single merge pass: `|A ∪ B| = |A| + |B| - |A ∩ B|`, so only the
/// intersection is counted.
fn jaccard(a: &[u64], b: &[u64]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut i = 0;
    let mut j = 0;
    let mut inter = 0usize;
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                inter += 1;
                i += 1;
                j += 1;
            }
        }
    }
    let union = a.len() + b.len() - inter;
    if union == 0 {
        return 0.0;
    }
    inter as f64 / union as f64
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
                .filter(move |&j| {
                    // Size-ratio prefilter: J(A,B) <= min(|A|,|B|)/max(|A|,|B|),
                    // so a pair whose smaller set is below THRESHOLD × the
                    // larger provably cannot qualify — skip it without
                    // touching the fingerprints. Exact for the current 0.5
                    // threshold (0.5 × integer is representable); the bound
                    // itself is conservative, so no qualifying pair is lost.
                    let small = fns[i].ngrams.len().min(fns[j].ngrams.len());
                    let large = fns[i].ngrams.len().max(fns[j].ngrams.len());
                    if (small as f64) < SIMILARITY_THRESHOLD * (large as f64) {
                        return false;
                    }
                    jaccard(&fns[i].ngrams, &fns[j].ngrams) >= SIMILARITY_THRESHOLD
                })
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

/// Comment syntax used by the F2b/F2c source-equality normaliser.
#[derive(Clone, Copy)]
enum CommentStyle {
    /// `//` line + `/* */` block comments (Rust, TypeScript, Go).
    CLike,
    /// `#` line comments (Python).
    Hash,
}

/// Per-language configuration for the intra-fn if-branch clone pass
/// (F2b identical-branch + F2c shared-prefix/suffix).
struct IfCloneCfg {
    comment: CommentStyle,
    status: LanguageCitationStatus,
    citation_keys: Vec<&'static str>,
}

/// F2c minimum combined normalised-source length (chars) of the shared
/// prefix + suffix run before a branches-sharing-code finding is emitted.
/// A single short shared statement (`let x = 1;`) is below this floor and
/// is left unreported; the floor keeps F2c — the clippy
/// `branches_sharing_code` class, which clippy disables by default for
/// FP-proneness — conservative enough not to fire on the curated
/// wild corpora (verified: wild-rust / wild-python stay at zero
/// clone-drift findings). Catches the audit-corpus FN
/// `clippy_ui_branches_sharing_code_shared_at_top.rs:15` whose shared
/// prefix `println!("Hello World!");` clears the floor.
pub const BRANCH_SHARING_MIN_CHARS: usize = 20;

fn run_intra_fn_if_clones(ctx: &DetectContext, lang: Language, cfg: &IfCloneCfg) -> Vec<Finding> {
    ctx.files
        .par_iter()
        .filter(|f| f.language == lang)
        .flat_map_iter(|file| {
            let mut local = Vec::new();
            if !file.parse_recovered {
                for ir_fn in &file.fns {
                    walk_if_branches_block(file, &ir_fn.body, cfg, &mut local);
                }
            }
            local
        })
        .collect()
}

/// Walk an [`IrBlock`] for `if`-same-then-else clone pairs (F2b),
/// recursing through every nested statement-bearing block.
///
/// The v0.5.x raw walk visited every `if_expression` node; the IR walk
/// reaches the same statement-position `if`s — the only positions any
/// audit / wild F2b finding occupies — plus those nested in
/// `while` / `loop` / `with` / `match`-arm blocks. Expression-position
/// `if`s hidden inside an opaque `IrStmtKind::Other` (e.g. a `let` RHS)
/// are out of v0 IR-F2b scope.
fn walk_if_branches_block(
    file: &IrFile,
    block: &IrBlock,
    cfg: &IfCloneCfg,
    out: &mut Vec<Finding>,
) {
    for stmt in &block.statements {
        match &stmt.kind {
            IrStmtKind::If(if_stmt) => {
                if let Some(f) = analyze_if_branches(file, if_stmt, cfg) {
                    out.push(f);
                }
                walk_if_branches_block(file, &if_stmt.consequence, cfg, out);
                if let Some(alt) = &if_stmt.alternative {
                    walk_if_branches_block(file, alt, cfg, out);
                }
            }
            IrStmtKind::While(w) => walk_if_branches_block(file, &w.body, cfg, out),
            IrStmtKind::Loop(l) => walk_if_branches_block(file, &l.body, cfg, out),
            IrStmtKind::With(wi) => walk_if_branches_block(file, &wi.body, cfg, out),
            IrStmtKind::Match(m) => {
                for arm in &m.arms {
                    walk_if_branches_expr(file, &arm.body, cfg, out);
                }
            }
            _ => {}
        }
    }
}

fn walk_if_branches_expr(file: &IrFile, expr: &IrExpr, cfg: &IfCloneCfg, out: &mut Vec<Finding>) {
    match &expr.kind {
        IrExprKind::If(if_stmt) => {
            if let Some(f) = analyze_if_branches(file, if_stmt, cfg) {
                out.push(f);
            }
            walk_if_branches_block(file, &if_stmt.consequence, cfg, out);
            if let Some(alt) = &if_stmt.alternative {
                walk_if_branches_block(file, alt, cfg, out);
            }
        }
        IrExprKind::Block(b) => walk_if_branches_block(file, b, cfg, out),
        IrExprKind::Loop(l) => walk_if_branches_block(file, &l.body, cfg, out),
        IrExprKind::Match(m) => {
            for arm in &m.arms {
                walk_if_branches_expr(file, &arm.body, cfg, out);
            }
        }
        _ => {}
    }
}

fn analyze_if_branches(file: &IrFile, if_stmt: &IrIfStmt, cfg: &IfCloneCfg) -> Option<Finding> {
    // Both F2b and F2c fire only on a flat `if { } else { }`. The
    // converter sets `alternative` to `Some` exactly for a flat else
    // block (else-if chains yield `None`), mirroring v0.5.x
    // `find_else_block_rust`.
    let consequence = &if_stmt.consequence;
    let alternative = if_stmt.alternative.as_ref()?;

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
    let conseq_src = normalize_block_source(block_source(file, consequence)?, cfg.comment);
    let alt_src = normalize_block_source(block_source(file, alternative)?, cfg.comment);

    // F2b: the whole consequence and alternative blocks are byte-for-byte
    // identical after normalisation (the `if_same_then_else` class).
    if conseq_src == alt_src {
        // Size gate on the consequence block's block-rooted normalised
        // token count (equals v0.5.x `normalize_rust(consequence).len()`).
        let branch_token_count = consequence.normalised_token_count;
        if branch_token_count < INTRA_FN_IF_MIN_TOKENS {
            return None;
        }
        return Some(make_if_clone_finding(
            if_stmt,
            consequence,
            alternative,
            cfg,
            format!(
                "if-then-else branches contain identical source ({branch_token_count} tokens) — likely a copy-paste duplicate"
            ),
            serde_json::json!({
                "kind": "intra-fn-if-same-then-else",
                "branch_token_count": branch_token_count,
                "intra_fn_if_min_tokens": INTRA_FN_IF_MIN_TOKENS,
            }),
        ));
    }

    // F2c: branches diverge overall but share a leading and/or trailing
    // run of identical statements (the clippy `branches_sharing_code`
    // class — clone-drift-v0.md F2c). Lifts the recorded clone-drift
    // recall bound (recall-audit-v0.md Bound C).
    analyze_branch_sharing(file, if_stmt, consequence, alternative, cfg)
}

/// F2c: detect a shared statement prefix/suffix between two divergent
/// branches. Returns a finding when the combined normalised length of the
/// shared run clears [`BRANCH_SHARING_MIN_CHARS`] and the branches still
/// diverge (at least one statement is unshared in each).
fn analyze_branch_sharing(
    file: &IrFile,
    if_stmt: &IrIfStmt,
    consequence: &IrBlock,
    alternative: &IrBlock,
    cfg: &IfCloneCfg,
) -> Option<Finding> {
    let a = norm_statements(file, &consequence.statements, cfg.comment)?;
    let b = norm_statements(file, &alternative.statements, cfg.comment)?;
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let min_len = a.len().min(b.len());

    // v0 scope: shared LEADING run only (the clippy "shared_at_top"
    // variant — the audit FN is `..._shared_at_top.rs`). A shared
    // trailing run ("shared_at_bottom") is the intentional fan-out-then-
    // common-tail pattern (`if {..} else {..}; self.buffer.write(x)`)
    // that dominates real code: an early F2c probe over `wild-corpus`
    // produced 16 such shared-suffix detections across curated libraries
    // (object, regex_syntax, hyper, …), exactly the noise clippy's
    // default-off `branches_sharing_code` is known for. Restricting v0 to
    // the leading run keeps wild-rust / wild-python at zero clone-drift
    // findings while still catching the audit FN. Shared-suffix is a
    // documented future scope lift (clone-drift-v0.md F2c "Non-goals").
    let mut prefix = 0;
    while prefix < min_len && a[prefix] == b[prefix] {
        prefix += 1;
    }
    if prefix == 0 {
        return None;
    }
    // Require genuine divergence: the shared prefix must not cover an
    // entire branch (that would be the F2b identical case).
    if prefix >= a.len() && prefix >= b.len() {
        return None;
    }

    let shared_chars: usize = a[..prefix].iter().map(|s| s.len()).sum();
    if shared_chars < BRANCH_SHARING_MIN_CHARS {
        return None;
    }

    Some(make_if_clone_finding(
        if_stmt,
        consequence,
        alternative,
        cfg,
        format!(
            "if-then-else branches share {prefix} leading statement(s) ({shared_chars} chars) then diverge — consider hoisting the shared code"
        ),
        serde_json::json!({
            "kind": "intra-fn-if-branches-sharing-code",
            "shared_prefix_statements": prefix,
            "shared_chars": shared_chars,
            "branch_sharing_min_chars": BRANCH_SHARING_MIN_CHARS,
        }),
    ))
}

/// Normalise each statement's source slice; `None` if any slice is
/// unrecoverable (keeps the prefix/suffix alignment honest).
fn norm_statements(file: &IrFile, stmts: &[IrStmt], style: CommentStyle) -> Option<Vec<String>> {
    let mut out = Vec::with_capacity(stmts.len());
    for s in stmts {
        let start = s.location.start_byte as usize;
        let end = s.location.end_byte as usize;
        out.push(normalize_block_source(file.source.get(start..end)?, style));
    }
    Some(out)
}

/// Shared finding builder for the F2b / F2c intra-fn if-branch passes.
fn make_if_clone_finding(
    if_stmt: &IrIfStmt,
    consequence: &IrBlock,
    alternative: &IrBlock,
    cfg: &IfCloneCfg,
    message: String,
    raw: serde_json::Value,
) -> Finding {
    Finding {
        detector_id: "clone-drift".to_string(),
        primary: ir_loc_to_core(&if_stmt.location),
        related: vec![
            ir_loc_to_core(&consequence.location),
            ir_loc_to_core(&alternative.location),
        ],
        message,
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: cfg.citation_keys.clone(),
            raw,
            language_citation_status: cfg.status,
        },
        origin: Default::default(),
    }
}

/// Source slice for a block, recovered from [`IrFile::source`] via the
/// block's byte span. Equals the v0.5.x `block.utf8_text(source)`.
fn block_source<'a>(file: &'a IrFile, block: &IrBlock) -> Option<&'a str> {
    let start = block.location.start_byte as usize;
    let end = block.location.end_byte as usize;
    file.source.get(start..end)
}

/// Source-text normalisation used by F2b / F2c: strip line and block
/// comments (per `style`), collapse internal whitespace runs to a single
/// space, trim leading and trailing whitespace. Two blocks normalising to
/// the same string are byte-for-byte identical source modulo formatting
/// and commentary. The comment stripper is intentionally naive (it does
/// not skip string literals); both branches are normalised the same way,
/// so the equality comparison stays consistent.
fn normalize_block_source(text: &str, style: CommentStyle) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = true;
    let mut iter = text.chars().peekable();
    while let Some(c) = iter.next() {
        if matches!(style, CommentStyle::Hash) && c == '#' {
            // Python line comment — consume to end of line.
            while let Some(&n) = iter.peek() {
                if n == '\n' {
                    break;
                }
                iter.next();
            }
            continue;
        }
        if matches!(style, CommentStyle::CLike) && c == '/' && iter.peek() == Some(&'/') {
            // Line comment — consume to end of line.
            while let Some(&n) = iter.peek() {
                if n == '\n' {
                    break;
                }
                iter.next();
            }
            continue;
        }
        if matches!(style, CommentStyle::CLike) && c == '/' && iter.peek() == Some(&'*') {
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
    out.trim().to_string()
}

fn partition(group: &[usize], fns: &[FnInfo]) -> Vec<Vec<usize>> {
    let mut by_form: HashMap<&Vec<NormalisedToken>, Vec<usize>> = HashMap::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `is_url_shaped` gates whether a trailing comment fragment is
    /// treated as a URL rather than prose (call site at the
    /// `is_url_shaped(rest)` guard above). The cases below are chosen so
    /// that each clause of the scheme predicate is decided independently:
    /// a table that only ever exercises alphanumeric schemes cannot tell
    /// `a || b` from `a && b`, because the alphanumeric disjunct alone
    /// already settles the result.
    #[test]
    fn is_url_shaped_accepts_and_rejects_by_scheme_shape() {
        let cases: &[(&str, bool)] = &[
            // Plain alphanumeric scheme — the common case.
            ("https://example.com", true),
            ("http://a", true),
            // Each non-alphanumeric character the scheme grammar allows,
            // in isolation. These are what distinguish the `||` chain
            // from a `&&` chain and pin each `==` comparison to its own
            // character.
            ("git+ssh://host/repo", true),
            ("my-scheme://host", true),
            ("com.example.app://host", true),
            // A character the scheme grammar does NOT allow. Every
            // `== c` in the chain must stay an equality test: flipping
            // any one of them to `!=` would let this through.
            ("a_b://host", false),
            ("sch eme://host", false),
            ("sch/eme://host", false),
            // Short schemes: the separator is three characters wide, so
            // the offset arithmetic for `rest` is only correct as
            // `scheme_end + 3`.
            ("ab://x", true),
            ("a://x", true),
            // Empty scheme.
            ("://host", false),
            // Present scheme, absent value.
            ("https://", false),
            ("a://", false),
            // No separator at all.
            ("", false),
            ("example.com", false),
            ("not a url", false),
            ("//example.com", false),
            (":/example.com", false),
        ];
        for (input, want) in cases {
            assert_eq!(
                is_url_shaped(input),
                *want,
                "is_url_shaped({input:?}) should be {want}"
            );
        }
    }

    #[test]
    fn jaccard_of_two_empty_sets_is_one() {
        assert_eq!(jaccard(&[], &[]), 1.0);
    }

    #[test]
    fn jaccard_is_zero_when_only_one_side_is_empty() {
        // Guards the `&&` in the empty-pair short circuit: under `||` a
        // single empty side would report a perfect match against a
        // non-empty set.
        assert_eq!(jaccard(&[], &[1, 2]), 0.0);
        assert_eq!(jaccard(&[1, 2], &[]), 0.0);
    }

    #[test]
    fn jaccard_is_zero_for_disjoint_sets() {
        assert_eq!(jaccard(&[1, 2], &[3, 4]), 0.0);
    }

    #[test]
    fn jaccard_is_one_for_identical_sets() {
        assert_eq!(jaccard(&[1, 2, 3], &[1, 2, 3]), 1.0);
    }

    #[test]
    fn jaccard_of_partial_overlap_is_intersection_over_union() {
        // |A| = 3, |B| = 2, |A ∩ B| = 2, so |A ∪ B| = 3 + 2 - 2 = 3.
        // Chosen so that intersection and union differ and neither is a
        // fixed point of the division: 2/3 is distinguishable from 2*3
        // and from 2%3.
        assert_eq!(jaccard(&[1, 2, 3], &[1, 3]), 2.0 / 3.0);
        // |A| = 4, |B| = 3, |A ∩ B| = 1, |A ∪ B| = 6.
        assert_eq!(jaccard(&[1, 2, 3, 4], &[4, 5, 6]), 1.0 / 6.0);
    }

    #[test]
    fn find_root_compress_returns_root_and_flattens_the_path() {
        // Chain 2 -> 1 -> 0 -> 3, with 3 as the self-parent root. The
        // root is deliberately neither 0 nor 1 so that a constant return
        // is distinguishable from the real lookup.
        let mut parent = vec![3, 0, 1, 3];
        assert_eq!(find_root_compress(&mut parent, 2), 3);
        assert_eq!(
            parent,
            vec![3, 3, 3, 3],
            "every node on the walked path should point straight at the root"
        );
    }

    #[test]
    fn find_root_compress_on_a_self_root_is_identity() {
        let mut parent = vec![0, 1, 2];
        assert_eq!(find_root_compress(&mut parent, 2), 2);
        assert_eq!(parent, vec![0, 1, 2], "no path to compress");
    }
}
