// Only `Discard` can be implemented for a refined type: an impl of any
// other trait is for every value of its type.
#[derive(Discard)]
struct S { n: i64 }

trait Tr { fn f(&self) -> i64; }

impl Tr for { s: S | s.n > 0 } {
    fn f(&self) -> i64 { self.n }
}

fn main() {}
// error: only `Discard` can be implemented for a refined type
