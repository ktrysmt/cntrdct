fn move_bytes(target: u32, source: u32) -> u32 {
    target + source
}

fn driver() {
    let target = 1u32;
    let source = 2u32;
    let _ = move_bytes(target, source);
}
