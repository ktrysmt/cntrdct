fn outer(a: i32, b: i32) -> i32 {
    inner(a, plus(b, 1))
}

fn inner(x: i32, y: i32) -> i32 {
    x + y
}

fn plus(x: i32, y: i32) -> i32 {
    x + y
}
