// A bound on a projection makes proving the impl's trait be about what
// `T::Out` stands for, which can be a bigger type than `W<T>`: an `X`
// whose `Out` is `W<X>` would have `W<X>: Show` need `W<X>: Show`. So it
// can't be of the trait the impl is of, or one that leads back to it.
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
// error: proving that a type implements `Show` could go on forever: this impl needs `T::Out: Show`
