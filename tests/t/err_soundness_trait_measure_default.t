// A trait's default measure is final: generic code unfolds it, as what the
// measure is of every type, so an impl can't give its own
// (docs/86-trait-contracts.md). Here `X`'s would say it's empty with five.
trait S {
    #[measure]
    fn len(&self) -> usize;
    #[measure]
    fn is_empty(&self) -> bool { self.len() == 0 }
}

#[derive(Discard)]
struct X {}

impl S for X {
    #[measure]
    fn len(&self) -> usize { 5 }
    #[measure]
    fn is_empty(&self) -> bool { true }
}

fn g<T: S>(t: &T) -> u8 {
    let a = [7u8; 1];
    if t.is_empty() { a[t.len()] } else { 0 }
}

fn main() with {} {
    let _ = g(&X {});
}
// error: `is_empty` has a body in trait `S`, which is what it is of every type: generic code counts on it, so `X` can't give its own
