// An impl can't give its own body for a trait's default measure, marked a
// measure or not: the error says the default is final, rather than to mark
// it.
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
    fn is_empty(&self) -> bool { false }
}

fn main() {}
// error: `is_empty` has a body in trait `S`, which is what it is of every type
