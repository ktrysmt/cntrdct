// pr-miner positive: alloc/free pairing rule violated by leaked_buffer.
// Second alloc/free fixture; pairs with pr_miner_005.rs.

fn buf_one(x: i32) -> i32 {
    alloc();
    let r = x;
    free();
    r
}

fn buf_two(a: i32, b: i32) -> i32 {
    alloc();
    let r = a.max(b);
    free();
    r
}

fn buf_three() -> bool {
    alloc();
    let r = false;
    free();
    r
}

fn buf_four(n: usize) -> usize {
    alloc();
    let r = n.min(7);
    free();
    r
}

fn buf_five(flag: bool) {
    alloc();
    let _ = flag as u8;
    free();
}

fn buf_six(value: u8) -> u8 {
    alloc();
    let r = value.count_ones() as u8;
    free();
    r
}

fn buf_seven() {
    alloc();
    let _ = 7u16;
    free();
}

fn leaked_buffer() {
    alloc();
    pr_miner_006_specific_helper();
}
