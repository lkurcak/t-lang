// A measure can't divide by zero.
#[derive(Copy, Discard)]
struct R { a: u64 }

impl R {
    #[measure]
    fn none(&self) -> u64 { self.a % 0 }
}

fn main() {}
// error: a measure's body can't divide by zero
