// pr-miner positive: open_file/close_file pairing rule violated by leaky_reader.
// Seven satisfiers correctly close the handle; the violator opens but never closes.

fn read_one(path: i32) -> i32 {
    open_file();
    let r = path;
    close_file();
    r
}

fn read_two(a: i32, b: i32) -> i32 {
    open_file();
    let r = a + b;
    close_file();
    r
}

fn read_three() -> bool {
    open_file();
    let r = false;
    close_file();
    r
}

fn read_four(n: usize) -> usize {
    open_file();
    let r = n + 2;
    close_file();
    r
}

fn read_five(flag: bool) {
    open_file();
    let _ = flag as i32;
    close_file();
}

fn read_six(value: u64) -> u64 {
    open_file();
    let r = value & 0xff;
    close_file();
    r
}

fn read_seven() {
    open_file();
    let _ = 1i64;
    close_file();
}

fn leaky_reader() {
    open_file();
    pr_miner_002_specific_helper();
}
