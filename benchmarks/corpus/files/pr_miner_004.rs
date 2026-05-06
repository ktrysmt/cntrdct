// pr-miner positive: connect/disconnect pairing rule violated by stuck_socket.

fn talk_one(x: i32) -> i32 {
    connect();
    let r = x;
    disconnect();
    r
}

fn talk_two(a: i32, b: i32) -> i32 {
    connect();
    let r = a ^ b;
    disconnect();
    r
}

fn talk_three() -> bool {
    connect();
    let r = false;
    disconnect();
    r
}

fn talk_four(n: usize) -> usize {
    connect();
    let r = n / 2;
    disconnect();
    r
}

fn talk_five(flag: bool) {
    connect();
    let _ = !flag as u32;
    disconnect();
}

fn talk_six(value: u32) -> u32 {
    connect();
    let r = value.swap_bytes();
    disconnect();
    r
}

fn talk_seven() {
    connect();
    let _ = 3.14f64;
    disconnect();
}

fn stuck_socket() {
    connect();
    pr_miner_004_specific_helper();
}
