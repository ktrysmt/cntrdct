fn run(items: &[i32]) -> i32 {
    let total = sum(items);
    for it in iter(items) {
        accumulate(it);
    }
    total
}

fn pick(flag: bool) -> i32 {
    let v = { return early(flag); };
    v
}
