// pr-miner positive: enter_section/exit_section pairing violated by stuck_section.

fn step_one(x: i32) -> i32 {
    enter_section();
    let r = x;
    exit_section();
    r
}

fn step_two(a: i32, b: i32) -> i32 {
    enter_section();
    let r = a.pow(b.max(0) as u32);
    exit_section();
    r
}

fn step_three() -> bool {
    enter_section();
    let r = true;
    exit_section();
    r
}

fn step_four(n: usize) -> usize {
    enter_section();
    let r = n;
    exit_section();
    r
}

fn step_five(flag: bool) {
    enter_section();
    let _ = !flag;
    exit_section();
}

fn step_six(value: i64) -> i64 {
    enter_section();
    let r = value.signum();
    exit_section();
    r
}

fn step_seven() {
    enter_section();
    let _ = 100u128;
    exit_section();
}

fn stuck_section() {
    enter_section();
    pr_miner_007_specific_helper();
}
