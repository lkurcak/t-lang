// A bound on a projection makes proving the impl's trait be about what
// `T::Out` stands for, which can be a bigger type than `W<T>`: an `X`
// whose `Out` is `W<X>` would have `W<X>: Show` need `W<X>: Show`. So it
// can only be of a closed trait, which `Show` isn't.
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
// error: this impl's bound `T::Out: Show` isn't on a type parameter of `W<T>`
