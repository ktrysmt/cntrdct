# Synthetic Q-15 fixture file — deliberately minimal so cntrdct's
# arg-swap detector produces zero findings on it. The Python counterpart
# of files/quiet_a.rs; lets the harness exercise pybuglab end-to-end
# with all-zero cells under the same skip-run discipline as the
# SourcererCC adapter.
def small():
    return 0
