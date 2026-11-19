// A trait's default measure is final, whichever way code comes to a value
// of the impl's type: here an opaque result, known only by its bounds.
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

fn mk() -> impl S + Discard { X {} }

fn main() with {} {
    let x = mk();
    let a = [7u8; 1];
    if x.is_empty() { let _ = a[x.len()]; }
}
// error: `is_empty` has a body in trait `S`
