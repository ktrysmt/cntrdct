// pr-miner negative: every function correctly pairs open_file/close_file.

fn safe_read_one(x: i32) -> i32 {
    open_file();
    let r = x * 2;
    close_file();
    r
}

fn safe_read_two() -> bool {
    open_file();
    let r = false;
    close_file();
    r
}

fn safe_read_three(n: usize) -> usize {
    open_file();
    let r = n;
    close_file();
    r
}

fn safe_read_four(flag: bool) {
    open_file();
    let _ = flag as u8;
    close_file();
}

fn safe_read_five(value: u32) -> u32 {
    open_file();
    let r = value.reverse_bits();
    close_file();
    r
}
