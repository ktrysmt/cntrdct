// pr-miner negative: every function correctly pairs lock/unlock.

fn safe_lock_one(x: i32) -> i32 {
    lock();
    let r = x + 10;
    unlock();
    r
}

fn safe_lock_two(s: bool) -> bool {
    lock();
    let r = !s;
    unlock();
    r
}

fn safe_lock_three(n: usize) -> usize {
    lock();
    let r = n.saturating_add(2);
    unlock();
    r
}

fn safe_lock_four() {
    lock();
    let _ = 'z';
    unlock();
}

fn safe_lock_five(value: i32) -> i32 {
    lock();
    let r = value.abs();
    unlock();
    r
}
