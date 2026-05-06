// pr-miner positive: begin_tx/commit_tx pairing rule violated by stuck_tx.
// First begin_tx/commit_tx fixture; pairs with pr_miner_008.rs.

fn tx_one(x: i32) -> i32 {
    begin_tx();
    let r = x + 17;
    commit_tx();
    r
}

fn tx_two(a: i32, b: i32) -> i32 {
    begin_tx();
    let r = a + b * 5;
    commit_tx();
    r
}

fn tx_three() -> bool {
    begin_tx();
    let r = true;
    commit_tx();
    r
}

fn tx_four(n: usize) -> usize {
    begin_tx();
    let r = n;
    commit_tx();
    r
}

fn tx_five(flag: bool) {
    begin_tx();
    let _ = flag;
    commit_tx();
}

fn tx_six(value: u128) -> u128 {
    begin_tx();
    let r = value.swap_bytes();
    commit_tx();
    r
}

fn tx_seven() {
    begin_tx();
    let _ = 19u32;
    commit_tx();
}

fn stuck_tx() {
    begin_tx();
    pr_miner_007_specific_helper();
}
