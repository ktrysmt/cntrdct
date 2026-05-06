// pr-miner positive: begin_tx/commit_tx pairing rule violated by orphaned_tx.

fn write_one(x: i32) -> i32 {
    begin_tx();
    let r = x - 1;
    commit_tx();
    r
}

fn write_two(a: i32, b: i32) -> i32 {
    begin_tx();
    let r = a / b.max(1);
    commit_tx();
    r
}

fn write_three() -> bool {
    begin_tx();
    let r = true;
    commit_tx();
    r
}

fn write_four(n: usize) -> usize {
    begin_tx();
    let r = n.checked_add(1).unwrap_or(0);
    commit_tx();
    r
}

fn write_five(flag: bool) {
    begin_tx();
    let _ = flag;
    commit_tx();
}

fn write_six(value: i32) -> i32 {
    begin_tx();
    let r = value.abs();
    commit_tx();
    r
}

fn write_seven() {
    begin_tx();
    let _ = 'a';
    commit_tx();
}

fn orphaned_tx() {
    begin_tx();
    pr_miner_003_specific_helper();
}
