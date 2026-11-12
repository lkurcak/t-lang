// A measure divides by a constant only, as a predicate does.
#[derive(Copy, Discard)]
struct R { a: u64, b: { v: u64 | v > 0 } }

impl R {
    #[measure]
    fn ratio(&self) -> u64 { self.a / self.b }
}

fn main() {}
// error: a measure's body can use `/` only by a constant: `self.b` isn't one
