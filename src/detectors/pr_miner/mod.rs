//! pr-miner detector — frequent-itemset rule violations (PR-Miner).
//!
//! Spec: `cntrdct/docs/spec/pr-miner-v0.md`.
//! Multi-language: `cntrdct/docs/spec/multilang-v0.md` (Pattern A). v0.0
//! supports Rust only; Python widens `supported_languages()` in v0.1.
//!
//! Algorithm:
//! 1. Extract one `Transaction` per top-level function in each supported-
//!    language file. The extractor records distinct call-site identifiers
//!    (last path/field segment) — full set retained for the violation
//!    scan (spec F4 R-B).
//! 2. Build the mining database by filtering transactions with fewer than
//!    `MIN_TRANSACTION_ITEMS` items. If the mining database is smaller
//!    than `MIN_DATABASE_SIZE`, return no findings (spec N5).
//! 3. Mine pair rules `{a} -> {b}` via bounded Apriori
//!    (`MAX_ITEMSET_SIZE = 2`).
//! 4. For each mined rule, scan the FULL transaction set (not the filtered
//!    mining DB) for violations: a function violates iff `a in items(T) &&
//!    b not in items(T)`. Spec F4 R-B: filtering is a noise-suppression
//!    knob for mining; a function with `a` alone is exactly the violation
//!    pattern PR-Miner is designed to surface.
//! 5. Emit one `Finding` per violation. `related` lists all DB functions
//!    that satisfy the rule, capped at `MAX_RELATED`.

#![deny(missing_docs)]

use std::collections::BTreeSet;

use crate::core::{
    AnomalyClass, Citation, DetectContext, Detector, DetectorError, Evidence, Finding, Language,
    LanguageCitationStatus, Location, Severity,
};

mod apriori;
mod extract_go;
mod extract_python;
mod extract_rust;
mod extract_typescript;

use apriori::{mine_pairs, Rule};

/// One extracted function. Carries the source language so that finding
/// builders can pick the per-language `LanguageCitationStatus` per spec
/// F5 / `docs/spec/citations-policy.md`. Items are the distinct
/// last-segment call identifiers in the body.
#[derive(Debug, Clone)]
pub(crate) struct Transaction {
    pub language: Language,
    pub items: BTreeSet<String>,
    pub location: Location,
}

/// Minimum joint support fraction. Pairs whose joint itemset appears in
/// fewer than `MIN_SUPPORT * |T|` transactions are pruned. Below this
/// threshold the rule is statistical noise; Li-Zhou used 0.01 for
/// kernel-scale corpora and we default higher for the modest seed corpus.
pub const MIN_SUPPORT: f64 = 0.05;

/// Minimum confidence. When `lhs` appears, `rhs` must follow in at least
/// `MIN_CONFIDENCE` of those transactions. Picks up "almost always paired"
/// patterns without flagging every co-incidence.
pub const MIN_CONFIDENCE: f64 = 0.85;

/// Maximum mined itemset size. v0 fixes this at 2 (pair rules only).
pub const MAX_ITEMSET_SIZE: usize = 2;

/// Functions whose body has fewer than this many distinct call items
/// are dropped from the MINING database (they cannot resolve a frequent
/// pair on their own). They are still scanned for violations per spec
/// F4 R-B — a function with one item is exactly the violation pattern.
pub const MIN_TRANSACTION_ITEMS: usize = 2;

/// Below this many transactions in the mining database, Apriori output
/// is too unstable to act on; the detector returns no findings.
pub const MIN_DATABASE_SIZE: usize = 20;

/// Maximum entries kept on `Finding.related`. Caps tractability of large
/// corpora where a rule is satisfied by hundreds of functions.
pub const MAX_RELATED: usize = 32;

/// Maximum item cardinality (as a fraction of mining-database size) for a
/// rule to survive post-filtering. A rule `{lhs} -> {rhs}` is dropped when
/// `lhs` OR `rhs` appears in *more than* `MAX_ITEM_CARDINALITY * |T|`
/// transactions. Spec F4b (R7 — item-cardinality post-filter): items that
/// "everyone" calls are by definition not paired-API candidates, so a rule
/// involving them is statistical co-occurrence rather than a contract.
/// Pairs with F4c (R6 stop-list) for FM-A elimination.
pub const MAX_ITEM_CARDINALITY: f64 = 0.5;

/// R6 stop-list for Rust. Items in this list are dropped from each
/// transaction before mining. Citation grounding: Li-Zhou ESEC/FSE 2005
/// §3.2 ("we filter common library calls"). The empirical FM-A
/// pathology in the wild Rust corpus is the `Err -> Ok` rule mined
/// across eight permissively-licensed crates; both items are stdlib
/// `Result`/`Option` constructors that co-occur in the majority of
/// fallible functions without describing a paired-API contract. The
/// list is intentionally narrow at v0 — adding `new` / `from` / `into`
/// would risk dropping legitimate paired APIs whose last-segment name
/// collides with stdlib constructors.
pub const RUST_STOPLIST: &[&str] = &[
    // Result / Option constructors — FM-A core.
    "Err", "Ok", "Some", "None",
];

/// R6 stop-list for Python. Counterpart to `RUST_STOPLIST`. The
/// empirical FM-A pathology on the Python wild corpus is the
/// `TypeError -> isinstance` rule mined across click validators;
/// `TypeError`, `isinstance`, and the other built-in exception classes
/// are stop-listed for the same Li-Zhou §3.2 reason. Built-in
/// introspection / iteration functions (`len`, `range`, `print`,
/// `super`, `iter`, `next`, `getattr` / `setattr` / `hasattr`) are
/// included because they appear in virtually every non-trivial Python
/// function and would otherwise mine the same co-occurrence rules.
pub const PYTHON_STOPLIST: &[&str] = &[
    // Built-in exception classes — FM-A core.
    "Exception",
    "TypeError",
    "ValueError",
    "KeyError",
    "IndexError",
    "AttributeError",
    "RuntimeError",
    "OSError",
    "StopIteration",
    "NotImplementedError",
    "FileNotFoundError",
    "ImportError",
    // Built-in introspection.
    "isinstance",
    "issubclass",
    "getattr",
    "setattr",
    "hasattr",
    "delattr",
    "callable",
    // Built-in iteration / sequence helpers.
    "len",
    "range",
    "iter",
    "next",
    "enumerate",
    "zip",
    "map",
    "filter",
    // Built-in I/O / debug.
    "print",
    "repr",
    // Inheritance / scoping.
    "super",
];

/// R6 stop-list for TypeScript (R-2.e). Last-segment call names that are
/// ubiquitous in idiomatic TypeScript and would otherwise mine spurious
/// co-occurrence rules (the same Li-Zhou §3.2 frequency pathology the
/// Rust / Python lists guard against): console logging, the `Array` /
/// `Object` / `JSON` / `Promise` static helpers reached by their last
/// segment, and the universal iteration / collection methods. Kept
/// deliberately small in v0; the R-2.e wild corpus is what would justify
/// any additions.
pub const TYPESCRIPT_STOPLIST: &[&str] = &[
    // Logging / debug — appears in nearly every non-trivial function.
    "log",
    "warn",
    "error",
    "info",
    "debug",
    "assert",
    // Collection / iteration methods reached by last segment.
    "map",
    "filter",
    "forEach",
    "reduce",
    "push",
    "pop",
    "join",
    "split",
    "slice",
    "keys",
    "values",
    "entries",
    "has",
    "get",
    "set",
    // Promise / async plumbing.
    "then",
    "catch",
    "resolve",
    "reject",
    "all",
    // Serialisation helpers.
    "stringify",
    "parse",
];

/// R6 stop-list for Go. Ubiquitous builtins, logging, and formatting
/// helpers whose presence in a function body carries no implicit-rule
/// signal (spec F4c). Pairing primitives a contributor might mine
/// (`Lock`/`Unlock`, `beginTx`/`commitTx`) are intentionally absent.
pub const GO_STOPLIST: &[&str] = &[
    // Builtins reached as a bare identifier head.
    "len", "cap", "make", "new", "append", "copy", "delete", "close", "print", "println",
    // fmt logging / formatting (last segment of `fmt.Printf` etc.).
    "Print", "Printf", "Println", "Sprint", "Sprintf", "Sprintln", "Fprintf",
    // error / log plumbing.
    "Error", "Errorf", "Fatal", "Fatalf", "Fatalln",
    // Common stringer / byte conversions.
    "String", "Bytes",
];

static CITATIONS: &[Citation] = &[Citation {
    key: "li-zhou-fse-2005",
    authors: "Z. Li, Y. Zhou",
    title: "PR-Miner: Automatically Extracting Implicit Programming Rules and Detecting Violations in Large Software Code",
    venue: "ESEC/FSE 2005",
    year: 2005,
    doi: None,
    url: None,
    languages: &[Language::Rust],
}];

/// Detector entry point. See module docs for the algorithm contract.
#[derive(Debug, Default)]
pub struct PrMinerDetector;

impl PrMinerDetector {
    /// Construct a default-configured detector.
    pub fn new() -> Self {
        Self
    }
}

impl Detector for PrMinerDetector {
    fn id(&self) -> &'static str {
        "pr-miner"
    }

    fn name(&self) -> &'static str {
        "Implicit Rule Violation (PR-Miner)"
    }

    fn citations(&self) -> &'static [Citation] {
        CITATIONS
    }

    fn supported_languages(&self) -> &'static [Language] {
        // Rust + Python + TypeScript + Go. Python, TypeScript, and Go
        // findings carry LanguageCitationStatus::Unconfirmed per the
        // per-language surveys
        // (docs/surveys/pr-miner-{python,typescript,go}-*.md).
        &[
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::Go,
        ]
    }

    fn detect(&self, ctx: &DetectContext) -> Result<Vec<Finding>, DetectorError> {
        // Step 1: extract transactions from every supported-language file.
        let mut all_txns: Vec<Transaction> = Vec::new();
        for file in ctx.files {
            match file.language {
                Language::Rust => all_txns.extend(extract_rust::extract(file)),
                Language::Python => all_txns.extend(extract_python::extract(file)),
                Language::TypeScript => all_txns.extend(extract_typescript::extract(file)),
                Language::Go => all_txns.extend(extract_go::extract(file)),
            }
        }

        // Step 1b: R6 stop-list filter. Drop stop-listed items from each
        // transaction's item set so they never enter the mining database
        // OR the violation scan. Spec F4c. The filter is applied to the
        // canonical `Transaction::items` *in place* — both mining (Step 2)
        // and violation detection (Step 4) read from the same filtered
        // set, so a function whose only items were stop-listed cannot
        // appear as a violator either.
        for txn in &mut all_txns {
            let stoplist: &[&str] = stoplist_for(txn.language);
            txn.items.retain(|item| !stoplist.contains(&item.as_str()));
        }

        // Step 2: build the mining database (post MIN_TRANSACTION_ITEMS
        // filter). Mining and violation scanning use different sets per
        // spec F4 R-B.
        let mining_items: Vec<Vec<String>> = all_txns
            .iter()
            .filter(|t| t.items.len() >= MIN_TRANSACTION_ITEMS)
            .map(|t| t.items.iter().cloned().collect())
            .collect();

        if mining_items.len() < MIN_DATABASE_SIZE {
            return Ok(Vec::new());
        }

        // Step 3: mine pair rules.
        let rules: Vec<Rule> = mine_pairs(
            &mining_items,
            MIN_SUPPORT,
            MIN_CONFIDENCE,
            MAX_ITEM_CARDINALITY,
        );

        // Step 4: for each rule, scan the FULL transaction set for
        // violations and collect satisfying functions for `related`.
        let mut findings: Vec<Finding> = Vec::new();
        for rule in &rules {
            let satisfying: Vec<&Transaction> = all_txns
                .iter()
                .filter(|t| t.items.contains(&rule.lhs) && t.items.contains(&rule.rhs))
                .collect();
            let related_capped = satisfying.len() > MAX_RELATED;
            let related_locations: Vec<Location> = satisfying
                .iter()
                .take(MAX_RELATED)
                .map(|t| t.location.clone())
                .collect();

            for txn in &all_txns {
                if txn.items.contains(&rule.lhs) && !txn.items.contains(&rule.rhs) {
                    findings.push(make_finding(
                        rule,
                        txn.language,
                        &txn.location,
                        &related_locations,
                        related_capped,
                    ));
                }
            }
        }

        // Step 5: deterministic order per spec F6.
        findings.sort_by(|a, b| {
            a.primary
                .file
                .cmp(&b.primary.file)
                .then_with(|| a.primary.start_line.cmp(&b.primary.start_line))
                .then_with(|| {
                    let lhs_a = a
                        .evidence
                        .raw
                        .get("rule_lhs")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let lhs_b = b
                        .evidence
                        .raw
                        .get("rule_lhs")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    lhs_a.cmp(lhs_b)
                })
                .then_with(|| {
                    let rhs_a = a
                        .evidence
                        .raw
                        .get("rule_rhs")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let rhs_b = b
                        .evidence
                        .raw
                        .get("rule_rhs")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    rhs_a.cmp(rhs_b)
                })
        });

        Ok(findings)
    }
}

fn make_finding(
    rule: &Rule,
    language: Language,
    primary: &Location,
    related: &[Location],
    related_capped: bool,
) -> Finding {
    let percent = (rule.confidence * 100.0).round() as u32;
    let status = match language {
        // li-zhou-fse-2005 is grandfathered as Rust-grounded under
        // citations-policy.md clause (b).
        Language::Rust => LanguageCitationStatus::Confirmed,
        // Python survey (docs/surveys/pr-miner-python-2026-05.md)
        // returned no qualifying citation; ship Unconfirmed per the
        // comment-code precedent. Future languages default to
        // Unconfirmed too — the survey is what flips them to
        // Confirmed.
        _ => LanguageCitationStatus::Unconfirmed,
    };
    Finding {
        detector_id: "pr-miner".to_string(),
        primary: primary.clone(),
        related: related.to_vec(),
        message: format!(
            "function calls {} but never {}; {} of {} similar functions ({}%) call both",
            rule.lhs, rule.rhs, rule.support_count, rule.lhs_count, percent
        ),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["li-zhou-fse-2005"],
            raw: serde_json::json!({
                "rule_lhs": rule.lhs,
                "rule_rhs": rule.rhs,
                "support": rule.support_count,
                "confidence": rule.confidence,
                "transaction_count": rule.transaction_count,
                "related_capped": related_capped,
            }),
            language_citation_status: status,
        },
    }
}

/// Return the R6 stop-list for the given language. Languages without an
/// explicit list return an empty slice — adding a new language to
/// `supported_languages()` is one entry here away from being a no-op
/// stop-list. Spec F4c.
fn stoplist_for(language: Language) -> &'static [&'static str] {
    match language {
        Language::Rust => RUST_STOPLIST,
        Language::Python => PYTHON_STOPLIST,
        Language::TypeScript => TYPESCRIPT_STOPLIST,
        Language::Go => GO_STOPLIST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stoplist_for_rust_carries_result_constructors() {
        let sl = stoplist_for(Language::Rust);
        for item in &["Err", "Ok", "Some", "None"] {
            assert!(
                sl.contains(item),
                "Rust stop-list missing {item}; expected for FM-A coverage"
            );
        }
    }

    #[test]
    fn stoplist_for_python_carries_fma_pathology_items() {
        let sl = stoplist_for(Language::Python);
        for item in &["TypeError", "isinstance"] {
            assert!(
                sl.contains(item),
                "Python stop-list missing {item}; expected for FM-A coverage"
            );
        }
    }

    #[test]
    fn rust_stoplist_excludes_potentially_legitimate_paired_apis() {
        // The narrow v0 list deliberately does NOT include `new` /
        // `from` / `into` — those names collide with legitimate
        // paired-API conventions (`Mutex::new`, `Type::from`). Adding
        // them risks dropping real rules; the spec defers that
        // decision to a future revision once we have larger corpora.
        let sl = stoplist_for(Language::Rust);
        for item in &["new", "from", "into", "to_string", "clone"] {
            assert!(
                !sl.contains(item),
                "Rust stop-list must not include {item} at v0 (risks legitimate paired-API drop)"
            );
        }
    }
}
