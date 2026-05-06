// pr-miner positive: open_file/close_file pairing rule violated by abort_path.
// Second open_file/close_file fixture; pairs with pr_miner_002.rs.

fn handle_one(x: i32) -> i32 {
    open_file();
    let r = x ^ 7;
    close_file();
    r
}

fn handle_two(a: i32, b: i32) -> i32 {
    open_file();
    let r = a.saturating_sub(b);
    close_file();
    r
}

fn handle_three() -> bool {
    open_file();
    let r = true;
    close_file();
    r
}

fn handle_four(n: usize) -> usize {
    open_file();
    let r = n - 1;
    close_file();
    r
}

fn handle_five(flag: bool) {
    open_file();
    let _ = !flag;
    close_file();
}

fn handle_six(value: u32) -> u32 {
    open_file();
    let r = value.count_zeros();
    close_file();
    r
}

fn handle_seven() {
    open_file();
    let _ = 13i32;
    close_file();
}

fn abort_path() {
    open_file();
    pr_miner_004_specific_helper();
}
