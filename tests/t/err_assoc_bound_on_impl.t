// A bound on a projection would make proving the impl's trait be about
// what `T::Out` stands for, which can be a bigger type than `W<T>`.
trait Tr {
    type Out;
}

trait Show {
    fn show(&self);
}

#[derive(Discard)]
struct W<T> { t: T }

impl<T: Tr<Out: Show>> Show for W<T> {
    fn show(&self) {}
}

fn main() {}
// error: a bound on an associated type, like `Out: Show`, can only be on a function's type parameter
