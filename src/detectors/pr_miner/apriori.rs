//! Bounded Apriori frequent-itemset miner (max itemset size 2).
//!
//! Methodology reference: Agrawal & Srikant, "Fast Algorithms for Mining
//! Association Rules", VLDB 1994. We do NOT cite this in `Detector::citations`
//! because it does not introduce the detector concept (per
//! `docs/spec/citations-policy.md`); the rule-violation framing comes from
//! `li-zhou-fse-2005`, which is the Detector citation.
//!
//! Bound `MAX_ITEMSET_SIZE = 2` from spec F3: in v0 we only mine pair rules
//! `{a} -> {b}`. Raising this bound is a v1 concern and would require an
//! FP-growth-style implementation for tractability.

use std::collections::BTreeMap;

/// One mined association rule of the shape `{lhs} -> {rhs}`.
///
/// Field semantics match spec F5 message format:
/// `"function calls {a} but never {b}; {N} of {M} similar functions
/// ({percent}%) call both"`, where `N = support_count`, `M = lhs_count`,
/// `percent = round(confidence * 100)`.
#[derive(Debug, Clone)]
pub struct Rule {
    pub lhs: String,
    pub rhs: String,
    /// Number of transactions in the mining database that contain BOTH
    /// `lhs` and `rhs` (the joint itemset's support count).
    pub support_count: usize,
    /// Number of transactions in the mining database that contain `lhs`.
    /// Used as the denominator of confidence and as the `M` in the F5
    /// message.
    pub lhs_count: usize,
    /// `support_count / lhs_count`. Always in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Total mining-database size at the time the rule was mined.
    pub transaction_count: usize,
}

/// Mine pair-rules from `transactions`, each represented as a sorted set of
/// item names. Returns rules satisfying the spec F3 + F4b constraints:
///
/// - `support(lhs U rhs) / |T| >= min_support`
/// - `confidence(lhs -> rhs) >= min_confidence`
/// - `lhs != rhs` (disjoint, trivially true for size-2 itemsets)
/// - `count(lhs) / |T| <= max_item_cardinality` AND
///   `count(rhs) / |T| <= max_item_cardinality` (F4b R7 item-cardinality
///   post-filter — items that "everyone" calls are statistical co-occurrence,
///   not paired-API rules).
///
/// Determinism: items inside each transaction must already be sorted by
/// the caller (extractor returns `BTreeSet`); we additionally iterate over
/// the candidate set in sorted order via `BTreeMap`.
pub fn mine_pairs(
    transactions: &[Vec<String>],
    min_support: f64,
    min_confidence: f64,
    max_item_cardinality: f64,
) -> Vec<Rule> {
    let n = transactions.len();
    if n == 0 {
        return Vec::new();
    }

    // Step 1: count single-item support.
    let mut item_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for txn in transactions {
        for item in txn {
            *item_counts.entry(item.as_str()).or_insert(0) += 1;
        }
    }

    // Step 2: prune items below min_support to L1 (Apriori antimonotone
    // pruning: a pair containing an infrequent item is itself infrequent,
    // so we never need to count it).
    let l1: Vec<&str> = item_counts
        .iter()
        .filter(|(_, c)| (**c as f64) / (n as f64) >= min_support)
        .map(|(k, _)| *k)
        .collect();

    if l1.len() < 2 {
        return Vec::new();
    }

    // Step 3: count pair support. Iterate over (i, j) with i < j inside
    // each transaction; the items list is already sorted, so the `<`
    // comparison gives canonical pair ordering and we visit each unordered
    // pair once per transaction.
    let l1_set: std::collections::BTreeSet<&str> = l1.iter().copied().collect();
    let mut pair_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for txn in transactions {
        let frequent: Vec<&str> = txn
            .iter()
            .map(String::as_str)
            .filter(|s| l1_set.contains(s))
            .collect();
        for i in 0..frequent.len() {
            for j in (i + 1)..frequent.len() {
                let a = frequent[i];
                let b = frequent[j];
                let key = if a < b {
                    (a.to_string(), b.to_string())
                } else {
                    (b.to_string(), a.to_string())
                };
                *pair_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    // Step 4: from each frequent pair, emit both directions if confidence
    // qualifies. Spec F3 treats `{a} -> {b}` and `{b} -> {a}` as separate
    // rules (T11); we do not merge.
    //
    // F4b R7: drop pairs where EITHER side's item cardinality exceeds
    // `max_item_cardinality * |T|`. Such items are statistically
    // ubiquitous, so any rule involving them describes co-occurrence
    // rather than a paired-API contract.
    let mut rules: Vec<Rule> = Vec::new();
    for ((a, b), &count) in &pair_counts {
        if (count as f64) / (n as f64) < min_support {
            continue;
        }
        let count_a = item_counts[a.as_str()];
        let count_b = item_counts[b.as_str()];
        let card_a = count_a as f64 / n as f64;
        let card_b = count_b as f64 / n as f64;
        if card_a > max_item_cardinality || card_b > max_item_cardinality {
            continue;
        }
        let conf_ab = count as f64 / count_a as f64;
        let conf_ba = count as f64 / count_b as f64;
        if conf_ab >= min_confidence {
            rules.push(Rule {
                lhs: a.clone(),
                rhs: b.clone(),
                support_count: count,
                lhs_count: count_a,
                confidence: conf_ab,
                transaction_count: n,
            });
        }
        if conf_ba >= min_confidence {
            rules.push(Rule {
                lhs: b.clone(),
                rhs: a.clone(),
                support_count: count,
                lhs_count: count_b,
                confidence: conf_ba,
                transaction_count: n,
            });
        }
    }

    rules.sort_by(|x, y| x.lhs.cmp(&y.lhs).then_with(|| x.rhs.cmp(&y.rhs)));
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(items: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = items.iter().map(|s| s.to_string()).collect();
        v.sort();
        v.dedup();
        v
    }

    #[test]
    fn mines_strong_pair() {
        let mut txns = Vec::new();
        for _ in 0..9 {
            txns.push(t(&["acquire", "release"]));
        }
        txns.push(t(&["acquire", "helper"]));
        for i in 0..10 {
            txns.push(t(&[&format!("filler_a_{}", i), &format!("filler_b_{}", i)]));
        }
        let rules = mine_pairs(&txns, 0.05, 0.85, 0.5);
        let acquire_release = rules
            .iter()
            .find(|r| r.lhs == "acquire" && r.rhs == "release")
            .expect("acquire -> release must qualify");
        assert_eq!(acquire_release.support_count, 9);
        assert_eq!(acquire_release.lhs_count, 10);
        assert!((acquire_release.confidence - 0.9).abs() < 1e-9);
    }

    #[test]
    fn drops_low_confidence_pair() {
        let mut txns = Vec::new();
        for _ in 0..9 {
            txns.push(t(&["acquire", "helper"]));
        }
        txns.push(t(&["acquire", "release"]));
        for i in 0..10 {
            txns.push(t(&[&format!("p_a_{}", i), &format!("p_b_{}", i)]));
        }
        let rules = mine_pairs(&txns, 0.05, 0.85, 0.5);
        assert!(rules
            .iter()
            .all(|r| !(r.lhs == "acquire" && r.rhs == "release")));
    }

    #[test]
    fn drops_low_support_pair() {
        let mut txns = Vec::new();
        // 4 transactions with the pair, 16 with unrelated singletons-times-two.
        for _ in 0..4 {
            txns.push(t(&["acquire", "release"]));
        }
        for i in 0..16 {
            txns.push(t(&[&format!("p_a_{}", i), &format!("p_b_{}", i)]));
        }
        // Support 4/20 = 0.20 — actually frequent. To exercise the support
        // floor we tighten min_support to 0.25.
        let rules = mine_pairs(&txns, 0.25, 0.85, 0.5);
        assert!(rules
            .iter()
            .all(|r| !(r.lhs == "acquire" && r.rhs == "release")));
    }

    #[test]
    fn empty_input_yields_no_rules() {
        let rules = mine_pairs(&[], 0.05, 0.85, 0.5);
        assert!(rules.is_empty());
    }

    // F4b R7: stdlib-constructor pathology — `ubiquitous` is in every
    // transaction, `partner` co-occurs strongly enough to mine a rule
    // under the support / confidence floors. The cardinality post-filter
    // must drop the rule.
    #[test]
    fn drops_rule_when_lhs_cardinality_exceeds_threshold() {
        let mut txns = Vec::new();
        for _ in 0..26 {
            txns.push(t(&["ubiquitous", "partner"]));
        }
        for _ in 0..4 {
            txns.push(t(&["ubiquitous", "unrelated"]));
        }
        // |T| = 30; ubiquitous: 30/30 = 1.0, partner: 26/30 = 0.866.
        // Threshold 0.5 drops the rule (both sides exceed).
        let rules = mine_pairs(&txns, 0.05, 0.85, 0.5);
        assert!(
            !rules
                .iter()
                .any(|r| r.lhs == "ubiquitous" && r.rhs == "partner"),
            "ubiquitous -> partner must be dropped at cardinality 1.0 > 0.5: rules {:?}",
            rules
        );
    }

    // Symmetric to above: when only `rhs` is ubiquitous, the rule still
    // drops. The lhs stays well below the threshold so the failure mode
    // lives purely on the rhs side.
    #[test]
    fn drops_rule_when_rhs_cardinality_exceeds_threshold() {
        let mut txns = Vec::new();
        for _ in 0..10 {
            txns.push(t(&["acquire", "ubiquitous"]));
        }
        for _ in 0..20 {
            txns.push(t(&["filler", "ubiquitous"]));
        }
        // |T| = 30; acquire: 10/30 = 0.33; ubiquitous: 30/30 = 1.0.
        // The pair (acquire, ubiquitous) qualifies under support+confidence
        // but is dropped because ubiquitous's cardinality exceeds 0.5.
        let rules = mine_pairs(&txns, 0.05, 0.85, 0.5);
        assert!(
            !rules
                .iter()
                .any(|r| r.lhs == "acquire" && r.rhs == "ubiquitous"),
            "acquire -> ubiquitous must be dropped at rhs cardinality 1.0 > 0.5: rules {:?}",
            rules
        );
    }

    // Real paired-API shape: both sides comfortably below the threshold.
    // The rule must survive.
    #[test]
    fn keeps_rule_when_both_cardinalities_below_threshold() {
        let mut txns = Vec::new();
        for _ in 0..9 {
            txns.push(t(&["lock", "unlock"]));
        }
        txns.push(t(&["lock", "helper"]));
        for i in 0..20 {
            txns.push(t(&[&format!("p_a_{}", i), &format!("p_b_{}", i)]));
        }
        // |T| = 30; lock: 10/30 = 0.33; unlock: 9/30 = 0.30. Both < 0.5.
        let rules = mine_pairs(&txns, 0.05, 0.85, 0.5);
        assert!(
            rules.iter().any(|r| r.lhs == "lock" && r.rhs == "unlock"),
            "lock -> unlock must survive when both cardinalities are below threshold: rules {:?}",
            rules
        );
    }

    // Strict `>` boundary: an item at exactly the threshold survives.
    // Distinguishes "exceeds N%" from "matches or exceeds N%".
    #[test]
    fn keeps_rule_when_cardinality_equals_threshold_exactly() {
        let mut txns = Vec::new();
        for _ in 0..10 {
            txns.push(t(&["a", "b"]));
        }
        for i in 0..10 {
            txns.push(t(&[&format!("c_{}", i), &format!("d_{}", i)]));
        }
        // |T| = 20; a: 10/20 = 0.5 exactly, b: 10/20 = 0.5 exactly. Survives.
        let rules = mine_pairs(&txns, 0.05, 0.85, 0.5);
        assert!(
            rules.iter().any(|r| r.lhs == "a" && r.rhs == "b"),
            "a -> b at exact-threshold cardinality must survive: rules {:?}",
            rules
        );
    }

    #[test]
    fn deterministic_order() {
        let mut txns = Vec::new();
        for _ in 0..5 {
            txns.push(t(&["a", "b"]));
            txns.push(t(&["c", "d"]));
        }
        for i in 0..10 {
            txns.push(t(&[&format!("x_{}", i), &format!("y_{}", i)]));
        }
        let rules1 = mine_pairs(&txns, 0.05, 0.85, 0.5);
        let rules2 = mine_pairs(&txns, 0.05, 0.85, 0.5);
        let names1: Vec<(String, String)> = rules1
            .iter()
            .map(|r| (r.lhs.clone(), r.rhs.clone()))
            .collect();
        let names2: Vec<(String, String)> = rules2
            .iter()
            .map(|r| (r.lhs.clone(), r.rhs.clone()))
            .collect();
        assert_eq!(names1, names2);
    }
}
