fn classify(x: i32) -> &'static str {
    if x < 0 {
        match x {
            -1 => "minus_one",
            _ => "negative",
        }
    } else if x == 0 {
        "zero"
    } else {
        "positive"
    }
}
