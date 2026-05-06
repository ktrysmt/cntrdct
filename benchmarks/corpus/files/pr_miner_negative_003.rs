// pr-miner negative: each function correctly pairs its own API call.
// Mixes pairs from several positive scenarios so the corpus shows clean
// negatives spread across rules.

fn safe_tx(x: i32) -> i32 {
    begin_tx();
    let r = x + 3;
    commit_tx();
    r
}

fn safe_socket(n: usize) -> usize {
    connect();
    let r = n;
    disconnect();
    r
}

fn safe_handler() -> bool {
    register_handler();
    let r = true;
    unregister_handler();
    r
}

fn safe_buffer(value: u8) -> u8 {
    alloc();
    let r = value.wrapping_add(1);
    free();
    r
}

fn safe_section(flag: bool) {
    enter_section();
    let _ = !flag;
    exit_section();
}

fn safe_timer() {
    start_timer();
    let _ = 0u16;
    stop_timer();
}
