// Synthetic Q-15 fixture file — deliberately too short to trigger
// clone-drift's MIN_FN_TOKENS guard so the corpus has zero cntrdct
// findings. The comparison harness then exercises its end-to-end path
// with all-zero cells, which is the smallest test surface that still
// produces a well-formed BaselineComparisonReport JSON.
fn small() -> u32 {
    0
}
