fn copy(dst: u32, src: u32) -> u32 {
    dst + src
}

fn driver() {
    let dst = 1u32;
    let src = 2u32;
    let _ = copy(src, dst);
}
