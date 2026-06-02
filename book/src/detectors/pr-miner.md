# pr-miner

Mines implicit programming rules via Apriori frequent-itemset
analysis over per-function call-site transactions, then flags
function bodies that violate the mined rule. The contradiction is
between a statistically-supported pairing (e.g. `lock` → `unlock`)
and a single function that calls one without the other.

| Property | Value |
|---|---|
| Detector ID | `pr-miner` |
| Languages | Rust, Python |
| IEEE 1044-2009 class | Logic |
| Default severity | Warning |
| Spec | [`docs/spec/pr-miner-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/pr-miner-v0.md) |

## How it works

1. Each top-level function in the scan becomes one *transaction* whose
   *items* are the distinct call-site names inside its body.
2. Apriori mines association rules `{a} → {b}` with `support ≥
   MIN_SUPPORT = 0.05` and `confidence ≥ MIN_CONFIDENCE = 0.85`.
3. Any function whose body contains `a` but never `b` is a
   *violator* and receives one Finding.
4. `Finding.related` lists every function in the database that
   satisfies the rule (capped at `MAX_RELATED = 32`).

`MAX_ITEMSET_SIZE = 2` keeps the algorithm to pairs in v0. Lifting
to FP-growth and N > 2 itemsets is tracked under Future Q-series
candidates in the roadmap.

## What it flags

```rust
fn handle_request(conn: &Mutex<Conn>) -> Result<()> {
    let guard = lock(conn);
    process(&guard);
    // <- flagged: 9 of 10 functions that call lock(...) also call unlock(...)
    Ok(())
}
```

The message format is:

```text
function calls lock but never unlock; 9 of 10 similar functions (90%) call both
```

The N / M / percent figures come straight from the rule's
support and confidence numbers; the `evidence.raw` payload carries
the full `rule_lhs`, `rule_rhs`, `support`, `confidence`,
`transaction_count`, and `related_capped` fields so a reviewer can
audit the inference without re-running the miner.

## What it does not flag

- Closures, dynamic dispatch (`obj[i]()`), and other non-identifier
  call heads. The extractor only sees `call_expression` /
  `call` nodes whose head is a `path` / `identifier` /
  `attribute`.
- Functions with fewer than `MIN_TRANSACTION_ITEMS = 2` distinct
  call items. Single-call bodies have no signal.
- Any rule when the transaction database is smaller than
  `MIN_DATABASE_SIZE = 20`. Apriori on tiny corpora is too noisy to
  act on.
- Inter-procedural rules (lock acquired in `f`, released in `g`).

## Language coverage

- Rust: `li-zhou-fse-2005` is grandfathered as a confirmed citation
  per
  [`docs/spec/citations-policy.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/citations-policy.md).
- Python: unconfirmed citation per
  [`docs/surveys/pr-miner-python-2026-05.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/surveys/pr-miner-python-2026-05.md).
  P1 is satisfied by the Rust citation as a cross-language extension.

## Empirical FP profile

The v0.1 calibration labels 16 TP / 22 FP across the seed and wild
corpora (`posterior_tp ≈ 0.43`). The dominant failure mode (21 / 22
FPs) is stdlib-constructor co-occurrence: `Err(...)` and `Ok(...)`
both reduce to the items `Err` / `Ok` under the last-segment rule,
so Apriori mines them as a paired API even though no contract is
being violated. The spec's "v1 mitigations under consideration"
section discusses three tightening options (stop-list, fully-
qualified paths, cardinality post-filter); none are shipped yet, and
the Layer 2 ranker is what currently keeps these false positives
near the bottom of the output.

The Q-14 recall audit closed `pr-miner` at 2/0/1.00 against the
Semgrep `open-never-closed` rule on the audit corpus — the
narrow-but-clean recall signal complements the wild-corpus FP
picture.

## Configuration

```toml
[detectors.pr-miner]
enabled = false
```

Per-language disable:

```toml
[languages.python]
suppress = ["pr-miner"]
```

In-source: `#[cntrdct::allow(pr-miner)]` (Rust) or
`# cntrdct: allow(pr-miner)` (Python).

## Related

- [arg-swap](./arg-swap.md) — different family of same-file API-
  contract violations.
- [Statistical priors (P4)](../concepts/priors.md) — pr-miner is the
  only shipped detector at v0.4.3 carrying `prior_method = "wilson"`
  (n = 38, above the Q-11 `SMALL_SAMPLE_THRESHOLD = 30` switch).
