// pr-miner positive: lock/unlock pairing rule violated by missed_unlock_path.
// Second lock/unlock fixture; pairs with pr_miner_001.rs to provide enough
// satisfiers to clear MIN_SUPPORT against the v0.1 mining-database size.

fn read_one(x: i32) -> i32 {
    lock();
    let r = x + 11;
    unlock();
    r
}

fn read_two(a: i32, b: i32) -> i32 {
    lock();
    let r = a.saturating_mul(b);
    unlock();
    r
}

fn read_three() -> bool {
    lock();
    let r = false;
    unlock();
    r
}

fn read_four(n: usize) -> usize {
    lock();
    let r = n + 4;
    unlock();
    r
}

fn read_five(flag: bool) {
    lock();
    let _ = flag as u32;
    unlock();
}

fn read_six(value: u64) -> u64 {
    lock();
    let r = value | 1;
    unlock();
    r
}

fn read_seven() {
    lock();
    let _ = 11i64;
    unlock();
}

fn missed_unlock_path() {
    lock();
    pr_miner_003_specific_helper();
}
