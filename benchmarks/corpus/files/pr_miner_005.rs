// pr-miner positive: alloc/free pairing rule violated by leaked_arena.
// First alloc/free fixture; pairs with pr_miner_006.rs.

fn arena_one(x: i32) -> i32 {
    alloc();
    let r = x.wrapping_neg();
    free();
    r
}

fn arena_two(a: i32, b: i32) -> i32 {
    alloc();
    let r = a.wrapping_mul(b);
    free();
    r
}

fn arena_three() -> bool {
    alloc();
    let r = true;
    free();
    r
}

fn arena_four(n: usize) -> usize {
    alloc();
    let r = n.wrapping_add(15);
    free();
    r
}

fn arena_five(flag: bool) {
    alloc();
    let _ = flag;
    free();
}

fn arena_six(value: i16) -> i16 {
    alloc();
    let r = value.rotate_right(3);
    free();
    r
}

fn arena_seven() {
    alloc();
    let _ = 14i8;
    free();
}

fn leaked_arena() {
    alloc();
    pr_miner_005_specific_helper();
}
