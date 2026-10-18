trait A {
    type X;
}

trait B {}

impl<T: A> B for <T as A>::X {}

fn main() {}
// error: an impl can't be for a projection
