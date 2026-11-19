// A trait's default measure is final, whichever way generic code comes to
// a value of the impl's type: here through an associated type's bound.
trait S {
    #[measure]
    fn size(&self) -> usize;
    #[measure]
    fn small(&self) -> bool { self.size() < 2 }
}

trait Make {
    type Out: S;
    fn make(&self) -> { o: Self::Out | o.small() };
}

#[derive(Discard)]
struct Y {}

impl S for Y {
    #[measure]
    fn size(&self) -> usize { 5 }
    #[measure]
    fn small(&self) -> bool { true }
}

#[derive(Discard)]
struct X {}

impl Make for X {
    type Out = Y;
    fn make(&self) -> Y { Y {} }
}

fn g<M: Make<Out: Discard>>(m: &M) -> u8 {
    let o = m.make();
    let a = [7u8; 2];
    let r = a[o.size()];
    r
}

fn main() with {} {
    let _ = g(&X {});
}
// error: `small` has a body in trait `S`
