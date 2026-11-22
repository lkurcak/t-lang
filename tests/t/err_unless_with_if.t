// An effect has a condition or an `unless`, not both
// (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f<T>(_x: &T, n: u8) with panic if n > 3 unless T: Small {}

fn main() {}
// error: `panic` has a condition and an `unless`: say one
