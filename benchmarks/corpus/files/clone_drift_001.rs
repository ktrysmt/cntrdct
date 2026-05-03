fn process_a(items: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    for it in items {
        if it > 0 {
            out.push(it * 2);
        }
    }
    out
}

fn process_b(items: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    for it in items {
        if it > 0 {
            out.push(it * 2);
        }
    }
    out
}

fn process_c(items: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    for it in items {
        if it > 0 {
            out.push(it * 2);
        }
    }
    out
}

fn process_d(items: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    for it in items {
        if it > 0 {
            out.push(it * 2);
        }
    }
    out
}

fn process_drifted(items: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    for it in items {
        if it > 0 && it < 100 {
            out.push(it * 2);
        }
    }
    out
}
