// Nor can an inherent impl be for one.
struct S { n: i64 }

impl { s: S | s.n > 0 } {
    fn f(self) -> i64 { let S { n } = self; n }
}

fn main() {}
// error: only `Discard` can be implemented for a refined type
