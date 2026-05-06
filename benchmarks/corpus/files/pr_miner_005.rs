// pr-miner positive: register_handler/unregister_handler pairing violated.

fn hook_one(x: i32) -> i32 {
    register_handler();
    let r = x.wrapping_add(1);
    unregister_handler();
    r
}

fn hook_two(a: i32, b: i32) -> i32 {
    register_handler();
    let r = a.wrapping_sub(b);
    unregister_handler();
    r
}

fn hook_three() -> bool {
    register_handler();
    let r = true;
    unregister_handler();
    r
}

fn hook_four(n: usize) -> usize {
    register_handler();
    let r = n.wrapping_mul(3);
    unregister_handler();
    r
}

fn hook_five(flag: bool) {
    register_handler();
    let _ = flag;
    unregister_handler();
}

fn hook_six(value: i16) -> i16 {
    register_handler();
    let r = value.rotate_left(2);
    unregister_handler();
    r
}

fn hook_seven() {
    register_handler();
    let _ = 0i8;
    unregister_handler();
}

fn dangling_handler() {
    register_handler();
    pr_miner_005_specific_helper();
}
