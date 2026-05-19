# pr-miner

Mines implicit programming rules via Apriori frequent-itemset analysis
(`MAX_ITEMSET_SIZE = 2`) over per-function call-site transactions, then
flags call sites that violate the mined rule.

- **Rust citation:** Li & Zhou FSE 2005 (the original PR-Miner paper).
  Confirmed.
- **Python citation:** Unconfirmed
  (`docs/surveys/pr-miner-python-2026-05.md`); P1 satisfied by the
  Rust citation as a grandfather clause for cross-language extension.
- **IEEE 1044-2009 class:** Logic.
- **Default severity:** Warning.

The Q-15 SOTA baseline comparator (PyBugLab) pairs naturally with
pr-miner for `arg-swap`-class findings; clone-drift pairs with
SourcererCC. Future work: lift Apriori to FP-growth — tracked under
Future Q-series candidates in the roadmap.

Spec:
[`docs/spec/pr-miner-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/pr-miner-v0.md).
