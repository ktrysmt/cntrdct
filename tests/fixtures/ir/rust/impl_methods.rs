/// A small struct with two methods.
struct Counter {
    value: i32,
}

impl Counter {
    /// Build a new counter.
    pub fn new(start: i32) -> Counter {
        Counter { value: start }
    }

    pub fn add(&mut self, delta: i32) {
        self.value = self.value + delta;
    }
}
