// pr-miner positive: begin_tx/commit_tx pairing rule violated by panic_in_tx.
// Second begin_tx/commit_tx fixture; pairs with pr_miner_007.rs.

fn ledger_one(x: i32) -> i32 {
    begin_tx();
    let r = x;
    commit_tx();
    r
}

fn ledger_two(a: i32, b: i32) -> i32 {
    begin_tx();
    let r = a.checked_add(b * 9).unwrap_or(0);
    commit_tx();
    r
}

fn ledger_three() -> bool {
    begin_tx();
    let r = false;
    commit_tx();
    r
}

fn ledger_four(n: usize) -> usize {
    begin_tx();
    let r = n + 21;
    commit_tx();
    r
}

fn ledger_five(flag: bool) {
    begin_tx();
    let _ = !flag;
    commit_tx();
}

fn ledger_six(value: u64) -> u64 {
    begin_tx();
    let r = value.leading_zeros() as u64;
    commit_tx();
    r
}

fn ledger_seven() {
    begin_tx();
    let _ = 23u16;
    commit_tx();
}

fn panic_in_tx() {
    begin_tx();
    pr_miner_008_specific_helper();
}
