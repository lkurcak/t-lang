// A generic impl can't give its own body for a trait's default measure
// either, which a default method's contract may count on generically.
trait S {
    #[measure]
    fn len(&self) -> usize;
    #[measure]
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn check(&self) -> { b: bool | b == self.is_empty() };
}

#[derive(Discard)]
struct X<T> { t: T }

impl<T> S for X<T> {
    #[measure]
    fn len(&self) -> usize { 5 }
    #[measure]
    fn is_empty(&self) -> bool { true }
    fn check(&self) -> bool { true }
}

fn g<T: S>(t: &T) -> u8 {
    let a = [7u8; 1];
    if t.check() { a[t.len()] } else { 0 }
}

fn main() with {} {
    let _ = g(&X { t: 1u8 });
}
// error: `is_empty` has a body in trait `S`
