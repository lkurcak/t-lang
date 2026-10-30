//! A counter that only counts up: its field is its own.

/// Counts up from zero.
#[derive(Discard)]
export struct Counter { n: i64 }

impl Counter {
    export fn new() -> Counter { Counter { n: 0 } }
    export fn up(&mut self) with panic { self.n = self.n + 1; }
    export fn get(&self) -> i64 { self.n }
}

export mod show;
