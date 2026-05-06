// pr-miner positive: lock/unlock pairing rule violated by missing_unlock.
// Seven satisfier functions follow lock(); ...; unlock(); the violator
// calls lock() but never unlock().

fn lock_compute_001(x: i32) -> i32 {
    lock();
    let r = x + 1;
    unlock();
    r
}

fn lock_compute_002(a: i32, b: i32) -> i32 {
    lock();
    let r = a * b;
    unlock();
    r
}

fn lock_compute_003() -> bool {
    lock();
    let r = true;
    unlock();
    r
}

fn lock_compute_004(n: usize) -> usize {
    lock();
    let r = n.saturating_sub(1);
    unlock();
    r
}

fn lock_compute_005(flag: bool) {
    lock();
    let _ = !flag;
    unlock();
}

fn lock_compute_006(value: i64) -> i64 {
    lock();
    let r = value << 1;
    unlock();
    r
}

fn lock_compute_007() {
    lock();
    let _ = 0u32;
    unlock();
}

fn missing_unlock() {
    lock();
    pr_miner_001_specific_helper();
}
