// A type has one impl of `Discard` for a refined type, at most.
struct S { n: i64 }

impl S {
    #[measure]
    fn n(&self) -> i64 { self.n }
}

impl Discard for { s: S | s.n() == 0 } {
    fn discard(self) { let S { n: _ } = self; }
}

impl Discard for { s: S | s.n() == 1 } {
    fn discard(self) { let S { n: _ } = self; }
}

fn main() {}
// error: `S` implements `Discard` for a refined type twice
