// pr-miner positive: start_timer/stop_timer pairing violated by hanging_timer.

fn time_one(x: i32) -> i32 {
    start_timer();
    let r = x;
    stop_timer();
    r
}

fn time_two(a: i32, b: i32) -> i32 {
    start_timer();
    let r = a + b * 2;
    stop_timer();
    r
}

fn time_three() -> bool {
    start_timer();
    let r = true;
    stop_timer();
    r
}

fn time_four(n: usize) -> usize {
    start_timer();
    let r = n;
    stop_timer();
    r
}

fn time_five(flag: bool) {
    start_timer();
    let _ = flag;
    stop_timer();
}

fn time_six(value: u64) -> u64 {
    start_timer();
    let r = value.leading_zeros() as u64;
    stop_timer();
    r
}

fn time_seven() {
    start_timer();
    let _ = 9u32;
    stop_timer();
}

fn hanging_timer() {
    start_timer();
    pr_miner_008_specific_helper();
}
